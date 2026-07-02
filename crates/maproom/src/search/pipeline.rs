//! Search pipeline orchestration for hybrid search.
//!
//! This module implements the SearchPipeline that coordinates the complete
//! hybrid search flow:
//! 1. Query processing (tokenization, embedding, expansion)
//! 2. Parallel search execution (FTS, vector, graph, signals)
//! 3. Score fusion (combining results from multiple strategies)
//! 4. Result assembly (enriching with chunk details)
//! 5. Optional reranking (placeholder for Phase 3)
//!
//! # Architecture
//!
//! The pipeline follows a staged approach with clear separation of concerns:
//! - QueryProcessor handles query understanding
//! - SearchExecutors handle parallel retrieval
//! - ScoreFusion handles result combination
//! - Result assembly enriches with database details
//!
//! # Performance
//!
//! Target: < 50ms end-to-end for k=10 results
//! - Query processing: ~5ms
//! - Search execution: ~30-40ms (parallel)
//! - Fusion: ~2-5ms
//! - Assembly: ~5-10ms

use crate::config::SearchConfig;
use crate::metrics::get_metrics;
use crate::search::dedup::{self, DeduplicationConfig};
use crate::search::executor_types::SearchSource;
use crate::search::executors::SearchExecutors;
use crate::search::fusion::{BasicWeightedFusion, FusedResult, FusionWeights, ScoreFusion};
use crate::search::query_processor::QueryProcessor;
use crate::search::results::{
    ChunkSearchResult, FinalSearchResults, QueryFilters, QueryProcessingDetails,
    QueryUnderstanding, SearchMetadata, SearchOptions, SearchTiming, TimingBreakdown,
};
use crate::search::types::ProcessedQuery;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, instrument, trace, warn};

/// Main search pipeline coordinator.
///
/// Orchestrates the complete hybrid search flow from query string to
/// ranked results with full chunk details.
pub struct SearchPipeline {
    /// Query processor for tokenization, embedding, and expansion
    processor: Arc<QueryProcessor>,

    /// Search executors for parallel FTS, vector, graph, and signal searches
    executors: SearchExecutors,

    /// Score fusion strategy (Basic weighted fusion in Phase 2)
    fusion: Box<dyn ScoreFusion>,

    /// Search configuration for quality-weighted graph scoring (SRCHREL-2003)
    config: Option<SearchConfig>,

    /// Optional cross-encoder for reranking (None in Phase 2)
    #[allow(dead_code)]
    reranker: Option<()>, // Placeholder for Phase 3
}

impl SearchPipeline {
    /// Create a new SearchPipeline with default fusion strategy.
    ///
    /// Uses BasicWeightedFusion with default weights.
    pub fn new(processor: Arc<QueryProcessor>, executors: SearchExecutors) -> Self {
        Self {
            processor,
            executors,
            fusion: Box::new(BasicWeightedFusion::new()),
            config: None,
            reranker: None,
        }
    }

    /// Create a SearchPipeline with custom fusion strategy.
    pub fn with_fusion(
        processor: Arc<QueryProcessor>,
        executors: SearchExecutors,
        fusion: Box<dyn ScoreFusion>,
    ) -> Self {
        Self {
            processor,
            executors,
            fusion,
            config: None,
            reranker: None,
        }
    }

    /// Create a SearchPipeline with search configuration (SRCHREL-2003).
    ///
    /// Uses the provided configuration for quality-weighted graph scoring
    /// and fusion weight overrides.
    pub fn with_config(
        processor: Arc<QueryProcessor>,
        executors: SearchExecutors,
        config: SearchConfig,
    ) -> Self {
        Self {
            processor,
            executors,
            fusion: Box::new(BasicWeightedFusion::new()),
            config: Some(config),
            reranker: None,
        }
    }

    /// Create a SearchPipeline with both config and custom fusion strategy.
    pub fn with_config_and_fusion(
        processor: Arc<QueryProcessor>,
        executors: SearchExecutors,
        config: SearchConfig,
        fusion: Box<dyn ScoreFusion>,
    ) -> Self {
        Self {
            processor,
            executors,
            fusion,
            config: Some(config),
            reranker: None,
        }
    }

    /// Get reference to the current configuration.
    pub fn config(&self) -> Option<&SearchConfig> {
        self.config.as_ref()
    }

    /// Update the search configuration (supports hot reload).
    pub fn set_config(&mut self, config: SearchConfig) {
        self.config = Some(config);
    }

    /// Get reference to the database store (for testing and utilities).
    pub fn store(&self) -> &(dyn crate::db::Store + Send + Sync) {
        self.executors.store()
    }

    /// Calculate fusion weights, applying config overrides if present (SRCHREL-2003).
    ///
    /// If `config.graph_importance.fusion_weight_override` is set, uses that value
    /// for the graph weight and renormalizes other weights proportionally.
    ///
    /// # Weight Override Logic
    /// 1. Start with weights from options or defaults
    /// 2. If graph weight override is set, apply it
    /// 3. Renormalize other weights to sum to (1.0 - graph_override)
    ///
    /// # Example
    /// If original weights are {fts: 0.4, vector: 0.35, graph: 0.1, recency: 0.1, churn: 0.05}
    /// and override is 0.2, the result will be:
    /// - remaining = 1.0 - 0.2 = 0.8
    /// - scale = 0.8 / (0.4 + 0.35 + 0.1 + 0.05) = 0.8 / 0.9 = 0.889
    /// - {fts: 0.356, vector: 0.311, graph: 0.2, recency: 0.089, churn: 0.044}
    fn calculate_fusion_weights(&self, options: &SearchOptions) -> FusionWeights {
        let mut weights = options.get_fusion_weights();

        // Apply config override if present
        if let Some(config) = &self.config {
            if let Some(graph_override) = config.graph_importance.fusion_weight_override {
                debug!(
                    "Applying graph fusion weight override: {} -> {}",
                    weights.graph, graph_override
                );

                // Calculate how much weight is currently distributed to non-graph signals
                let non_graph_sum = weights.fts + weights.vector + weights.recency + weights.churn;

                if non_graph_sum > 0.0001 {
                    // Calculate the remaining weight after applying the override
                    let remaining = 1.0 - graph_override;

                    // Scale non-graph weights to fit in the remaining space
                    let scale = remaining / non_graph_sum;

                    weights.fts *= scale;
                    weights.vector *= scale;
                    weights.recency *= scale;
                    weights.churn *= scale;
                    weights.graph = graph_override;

                    debug!(
                        "Renormalized fusion weights: fts={:.3}, vector={:.3}, graph={:.3}, recency={:.3}, churn={:.3}",
                        weights.fts, weights.vector, weights.graph, weights.recency, weights.churn
                    );
                } else {
                    // Edge case: all non-graph weights are zero, just set graph weight
                    weights.graph = graph_override;
                }
            }
        }

        weights
    }

    /// Execute the complete search pipeline.
    ///
    /// # Flow
    /// 1. Process query (tokenization, embedding, expansion)
    /// 2. Execute parallel searches (FTS, vector, graph, signals)
    /// 3. Fuse scores from multiple strategies
    /// 4. Assemble final results with chunk details
    /// 5. Optional reranking (not implemented in Phase 2)
    ///
    /// # Performance Target
    /// < 50ms end-to-end for k=10 results
    #[instrument(skip(self), fields(query = %query, limit = options.limit))]
    pub async fn search(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Result<FinalSearchResults, PipelineError> {
        let total_start = Instant::now();
        let metrics = get_metrics();

        info!(
            "Starting search pipeline: '{}' (repo: {}, limit: {})",
            query, options.repo_id, options.limit
        );

        // Stage 1: Process query
        let process_start = Instant::now();
        let processed = match self.processor.process(query).await {
            Ok(p) => p,
            Err(e) => {
                metrics.record_error("query_processing");
                metrics.increment_queries("unknown", false);
                return Err(e.into());
            }
        };
        let query_processing_ms = process_start.elapsed().as_secs_f64() * 1000.0;

        let mode_str = format!("{:?}", processed.mode).to_lowercase();

        debug!(
            "Query processed in {:.2}ms: {} tokens, mode={:?}",
            query_processing_ms,
            processed.tokens.len(),
            processed.mode
        );

        trace!(
            "Processed query details: tokens={:?}, expanded_terms={:?}, fts_query={}",
            processed.tokens,
            processed.expanded_terms,
            processed.fts_query_string()
        );

        // Stage 2: Execute parallel searches
        let search_start = Instant::now();
        let search_results = match self
            .executors
            .execute_all(
                &processed,
                options.repo_id,
                options.worktree_id,
                options.limit * 3,    // Fetch more for better fusion
                self.config.as_ref(), // Pass config for quality-weighted graph scoring (SRCHREL-2003)
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                metrics.record_error("search_execution");
                metrics.increment_queries(&mode_str, false);
                let total_time = total_start.elapsed().as_secs_f64();
                metrics.record_query_latency(total_time, &mode_str, false);
                return Err(e.into());
            }
        };
        let search_execution_ms = search_start.elapsed().as_secs_f64() * 1000.0;

        debug!(
            "Parallel search completed in {:.2}ms: {}",
            search_execution_ms,
            search_results.summary()
        );

        trace!(
            "Search results breakdown: FTS={}, Vector={}, Graph={}, Signals={}",
            search_results.fts.len(),
            search_results.vector.len(),
            search_results.graph.len(),
            search_results.signals.len()
        );

        // Stage 3: Fuse scores
        let fusion_start = Instant::now();
        let fusion_weights = self.calculate_fusion_weights(&options);

        // Extract metadata before moving search_results
        let total_unique_chunks = search_results.total_unique_chunks();
        let execution_time_ms = search_results.execution_time_ms;
        let fts_count = search_results.fts.len();
        let vector_count = search_results.vector.len();
        let graph_count = search_results.graph.len();
        let signals_count = search_results.signals.len();

        // Fetch more results when deduplication is enabled to ensure
        // we can satisfy the limit after removing duplicates
        let fusion_limit = if options.deduplicate {
            options.limit * 3
        } else {
            options.limit
        };

        let fused_results = self.fusion.fuse(
            vec![
                search_results.fts,
                search_results.vector,
                search_results.graph,
                search_results.signals,
            ],
            &fusion_weights,
            fusion_limit,
        );
        let score_fusion_ms = fusion_start.elapsed().as_secs_f64() * 1000.0;

        // Record fusion time metric
        metrics.record_fusion_time(score_fusion_ms / 1000.0, "basic_weighted");

        debug!(
            "Score fusion completed in {:.2}ms: {} results",
            score_fusion_ms,
            fused_results.len()
        );

        trace!(
            "Top 3 fused results: {:?}",
            fused_results
                .iter()
                .take(3)
                .map(|r| (r.chunk_id, r.score))
                .collect::<Vec<_>>()
        );

        // Note: Deduplication is applied in assemble_results after fetching
        // chunk details (which provide relpath, symbol_name, start_line needed
        // for the identity key).

        // Stage 4: Assemble final results with chunk details
        let assembly_start = Instant::now();
        let final_results = self
            .assemble_results(
                query,
                &processed,
                fused_results,
                total_unique_chunks,
                fts_count,
                vector_count,
                graph_count,
                signals_count,
                execution_time_ms,
                query_processing_ms,
                score_fusion_ms,
                options,
            )
            .await?;
        let result_assembly_ms = assembly_start.elapsed().as_secs_f64() * 1000.0;

        debug!(
            "Result assembly completed in {:.2}ms: {} results",
            result_assembly_ms,
            final_results.results.len()
        );

        let total_time = total_start.elapsed().as_secs_f64() * 1000.0;

        // Record comprehensive metrics
        metrics.record_query_latency(total_time / 1000.0, &mode_str, true);
        metrics.record_result_count(final_results.results.len(), &mode_str);
        metrics.increment_queries(&mode_str, true);

        info!(
            "Search pipeline completed in {:.2}ms (target: <50ms), returned {} results",
            total_time,
            final_results.results.len()
        );

        if total_time > 50.0 {
            warn!(
                "Search exceeded 50ms target: {:.2}ms (processing: {:.2}ms, search: {:.2}ms, fusion: {:.2}ms, assembly: {:.2}ms)",
                total_time, query_processing_ms, search_execution_ms, score_fusion_ms, result_assembly_ms
            );
        }

        trace!(
            "Pipeline timing breakdown: total={:.2}ms, processing={:.2}ms, search={:.2}ms, fusion={:.2}ms, assembly={:.2}ms",
            total_time, query_processing_ms, search_execution_ms, score_fusion_ms, result_assembly_ms
        );

        Ok(final_results)
    }

    /// Assemble final results by enriching fused results with chunk details.
    ///
    /// Fetches complete chunk information from the database including:
    /// - File path, symbol name, kind
    /// - Line ranges
    /// - Preview text
    #[allow(clippy::too_many_arguments)] // Pipeline metrics need individual tracking for diagnostics
    #[instrument(skip(self, processed, fused_results))]
    async fn assemble_results(
        &self,
        query: &str,
        processed: &ProcessedQuery,
        fused_results: Vec<FusedResult>,
        total_unique_chunks: usize,
        fts_count: usize,
        vector_count: usize,
        graph_count: usize,
        signals_count: usize,
        search_execution_time_ms: f64,
        query_processing_ms: f64,
        score_fusion_ms: f64,
        options: SearchOptions,
    ) -> Result<FinalSearchResults, PipelineError> {
        // Extract chunk IDs to fetch
        let chunk_ids: Vec<i64> = fused_results.iter().map(|r| r.chunk_id).collect();

        if chunk_ids.is_empty() {
            debug!("No results to assemble");
            return Ok(FinalSearchResults::new(
                query.to_string(),
                vec![],
                self.build_metadata(
                    processed,
                    0,
                    0,
                    fts_count,
                    vector_count,
                    graph_count,
                    signals_count,
                    query_processing_ms,
                    search_execution_time_ms,
                    score_fusion_ms,
                    0.0, // result_assembly_ms (negligible for empty results)
                    &options,
                ),
            ));
        }

        // Fetch chunk details from database
        let chunk_details = self.fetch_chunk_details(&chunk_ids).await?;

        // Build result counts map
        let mut result_counts = HashMap::new();
        result_counts.insert(SearchSource::FTS, fts_count);
        result_counts.insert(SearchSource::Vector, vector_count);
        result_counts.insert(SearchSource::Graph, graph_count);
        result_counts.insert(SearchSource::Signals, signals_count);

        // Merge fused scores with chunk details
        let mut assembled_results = Vec::new();

        // Auto-enable confidence if related chunks requested
        let enable_confidence = options.include_confidence || options.include_related;

        for (index, fused) in fused_results.iter().enumerate() {
            if let Some(details) = chunk_details.get(&fused.chunk_id) {
                // Compute confidence if requested (auto-enabled by include_related)
                let confidence = if enable_confidence {
                    Some(crate::search::confidence::compute_result_confidence(
                        fused,
                        &fused_results,
                        index,
                        fused.exact_match_multiplier,
                    ))
                } else {
                    None
                };

                let mut result = ChunkSearchResult::new(
                    fused.chunk_id,
                    details.file_id,
                    details.relpath.clone(),
                    details.symbol_name.clone(),
                    details.kind.clone(),
                    details.start_line,
                    details.end_line,
                    details.preview.clone(),
                    fused.score,
                    fused.source_scores.clone(),
                );
                result.confidence = confidence;
                assembled_results.push(result);
            } else {
                warn!(
                    "Chunk {} found in fusion but missing from database",
                    fused.chunk_id
                );
            }
        }

        // Relationship expansion (after confidence scoring, before deduplication)
        const MAX_CONCURRENT_EXPANSIONS: usize = 3;

        if options.include_related {
            let mut expansion_count = 0;

            for result in &mut assembled_results {
                if expansion_count >= MAX_CONCURRENT_EXPANSIONS {
                    break; // Hard cap
                }

                // Only expand high-confidence results
                if let Some(conf) = &result.confidence {
                    if conf.source_count >= 2 || conf.is_exact_match {
                        match crate::search::relationships::find_top_related_chunks(
                            self.executors.store(),
                            result.chunk_id,
                            5,
                        )
                        .await
                        {
                            Ok(related) => {
                                result.related = Some(related);
                                expansion_count += 1;
                            }
                            Err(e) => {
                                // Log error but don't fail entire search
                                tracing::warn!(
                                    "Failed to find related chunks for {}: {}",
                                    result.chunk_id,
                                    e
                                );
                            }
                        }
                    }
                }
            }

            debug!(
                "Relationship expansion: {} results expanded (max: {})",
                expansion_count, MAX_CONCURRENT_EXPANSIONS
            );
        }

        // Apply deduplication if enabled
        let final_results = if options.deduplicate {
            let config = DeduplicationConfig::default();
            let pre_dedup_count = assembled_results.len();
            let deduped = dedup::deduplicate(assembled_results, &config);
            debug!(
                "Deduplication: {} -> {} results (removed {} duplicates)",
                pre_dedup_count,
                deduped.len(),
                pre_dedup_count - deduped.len()
            );
            // Apply the original limit after deduplication
            deduped.into_iter().take(options.limit).collect()
        } else {
            assembled_results
        };

        // Measure assembly time from the start of assemble_results
        // Note: This is approximate since we're measuring within the function
        // The actual result_assembly_ms will be passed in from the caller
        let metadata = self.build_metadata(
            processed,
            total_unique_chunks,
            final_results.len(),
            fts_count,
            vector_count,
            graph_count,
            signals_count,
            query_processing_ms,
            search_execution_time_ms,
            score_fusion_ms,
            0.0, // result_assembly_ms - will be updated by caller if needed
            &options,
        );

        Ok(FinalSearchResults::new(
            query.to_string(),
            final_results,
            metadata,
        ))
    }

    /// Fetch chunk details from database for the given chunk IDs.
    ///
    /// Returns a HashMap of chunk_id -> ChunkDetails for efficient lookup.
    async fn fetch_chunk_details(
        &self,
        chunk_ids: &[i64],
    ) -> Result<HashMap<i64, ChunkDetails>, PipelineError> {
        // TODO(IDXABS-2003): This needs to be implemented via the Store trait.
        // The store trait doesn't have a bulk chunk details fetch method yet.
        // This should query the chunks and files tables to get the needed fields.
        // See ticket IDXABS-4001 for search functionality updates.

        debug!(
            "fetch_chunk_details called for {} chunk IDs (not yet implemented)",
            chunk_ids.len()
        );
        Ok(HashMap::new())
    }

    /// Build search metadata from pipeline execution details.
    #[allow(clippy::too_many_arguments)] // Pipeline metrics need individual tracking for diagnostics
    fn build_metadata(
        &self,
        processed: &ProcessedQuery,
        total_unique: usize,
        returned: usize,
        fts_count: usize,
        vector_count: usize,
        graph_count: usize,
        signals_count: usize,
        query_processing_ms: f64,
        search_execution_time_ms: f64,
        score_fusion_ms: f64,
        result_assembly_ms: f64,
        options: &SearchOptions,
    ) -> SearchMetadata {
        let query_details = QueryProcessingDetails::new(
            processed.original.clone(),
            processed.mode,
            processed.tokens.len(),
            processed.expanded_terms.len(),
            processed.fts_query_string(),
            !processed.embedding.is_empty(),
        );

        let mut result_counts = HashMap::new();
        result_counts.insert(SearchSource::FTS, fts_count);
        result_counts.insert(SearchSource::Vector, vector_count);
        result_counts.insert(SearchSource::Graph, graph_count);
        result_counts.insert(SearchSource::Signals, signals_count);

        // Use actual timing data from pipeline measurements
        let timing = SearchTiming::new(
            query_processing_ms,
            search_execution_time_ms,
            score_fusion_ms,
            result_assembly_ms,
        );

        // Assemble QueryUnderstanding metadata (Phase 2)
        let filters = QueryFilters {
            repo_id: options.repo_id,
            worktree_id: options.worktree_id,
            file_types: options.file_types.clone(),
            recency_threshold: options.recency_threshold.clone(),
        };

        let timing_breakdown = TimingBreakdown::new(
            query_processing_ms,
            search_execution_time_ms,
            score_fusion_ms,
            result_assembly_ms,
        );

        let understanding = QueryUnderstanding::from_query_data(
            processed.mode,
            processed.tokens.clone(),
            processed.expanded_terms.clone(),
            filters,
            "basic_weighted".to_string(), // fusion strategy name
            timing_breakdown,
        );

        SearchMetadata::with_understanding(
            query_details,
            result_counts,
            timing,
            total_unique,
            returned,
            understanding,
        )
    }
}

/// Internal structure for chunk details fetched from database.
#[derive(Debug, Clone)]
struct ChunkDetails {
    file_id: i64,
    relpath: String,
    symbol_name: Option<String>,
    kind: String,
    start_line: i32,
    end_line: i32,
    preview: String,
}

/// Errors that can occur during pipeline execution.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    /// Query processing failed
    #[error("Query processing failed: {0}")]
    QueryProcessing(#[from] crate::search::query_processor::QueryProcessorError),

    /// Search execution failed
    #[error("Search execution failed: {0}")]
    SearchExecution(#[from] crate::search::executors::ExecutorError),

    /// Database query failed
    #[error("Database error: {0}")]
    Database(String),

    /// Result assembly failed
    #[error("Result assembly failed: {0}")]
    Assembly(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GraphImportanceConfig;

    // Note: Full integration tests are in tests/search_pipeline_integration_test.rs
    // These are unit tests for helper functions

    #[test]
    fn test_chunk_details_structure() {
        let details = ChunkDetails {
            file_id: 1,
            relpath: "src/main.rs".to_string(),
            symbol_name: Some("main".to_string()),
            kind: "function".to_string(),
            start_line: 10,
            end_line: 25,
            preview: "fn main() {".to_string(),
        };

        assert_eq!(details.file_id, 1);
        assert_eq!(details.relpath, "src/main.rs");
        assert_eq!(details.symbol_name, Some("main".to_string()));
    }

    // ===== SRCHREL-2003: Fusion Weight Override Tests =====

    /// Helper to create a minimal SearchPipeline for testing calculate_fusion_weights.
    /// Note: This doesn't set up a real processor/executors, just tests the weight logic.
    fn create_test_pipeline_with_config(
        config: Option<SearchConfig>,
    ) -> TestPipelineWeightCalculator {
        TestPipelineWeightCalculator { config }
    }

    /// Test helper struct that mirrors SearchPipeline's weight calculation logic.
    struct TestPipelineWeightCalculator {
        config: Option<SearchConfig>,
    }

    impl TestPipelineWeightCalculator {
        fn calculate_fusion_weights(&self, options: &SearchOptions) -> FusionWeights {
            let mut weights = options.get_fusion_weights();

            if let Some(config) = &self.config {
                if let Some(graph_override) = config.graph_importance.fusion_weight_override {
                    let non_graph_sum =
                        weights.fts + weights.vector + weights.recency + weights.churn;

                    if non_graph_sum > 0.0001 {
                        let remaining = 1.0 - graph_override;
                        let scale = remaining / non_graph_sum;

                        weights.fts *= scale;
                        weights.vector *= scale;
                        weights.recency *= scale;
                        weights.churn *= scale;
                        weights.graph = graph_override;
                    } else {
                        weights.graph = graph_override;
                    }
                }
            }

            weights
        }
    }

    #[test]
    fn test_calculate_fusion_weights_no_config() {
        // Without config, should use default weights
        let pipeline = create_test_pipeline_with_config(None);
        let options = SearchOptions::new(1, None, 10);

        let weights = pipeline.calculate_fusion_weights(&options);

        // Should match FusionWeights::default()
        assert!(
            (weights.fts - 0.4).abs() < f32::EPSILON,
            "FTS weight should be 0.4"
        );
        assert!(
            (weights.vector - 0.35).abs() < f32::EPSILON,
            "Vector weight should be 0.35"
        );
        assert!(
            (weights.graph - 0.1).abs() < f32::EPSILON,
            "Graph weight should be 0.1"
        );
        assert!(
            (weights.recency - 0.1).abs() < f32::EPSILON,
            "Recency weight should be 0.1"
        );
        assert!(
            (weights.churn - 0.05).abs() < f32::EPSILON,
            "Churn weight should be 0.05"
        );
    }

    #[test]
    fn test_calculate_fusion_weights_config_without_override() {
        // Config without override should use default weights
        let config = SearchConfig::default();
        assert!(
            config.graph_importance.fusion_weight_override.is_none(),
            "Default config should not have fusion override"
        );

        let pipeline = create_test_pipeline_with_config(Some(config));
        let options = SearchOptions::new(1, None, 10);

        let weights = pipeline.calculate_fusion_weights(&options);

        // Should match FusionWeights::default()
        assert!((weights.fts - 0.4).abs() < f32::EPSILON);
        assert!((weights.graph - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calculate_fusion_weights_with_override() {
        // Config with override should renormalize other weights
        let mut config = SearchConfig::default();
        config.graph_importance.fusion_weight_override = Some(0.2);

        let pipeline = create_test_pipeline_with_config(Some(config));
        let options = SearchOptions::new(1, None, 10);

        let weights = pipeline.calculate_fusion_weights(&options);

        // Graph should be exactly the override
        assert!(
            (weights.graph - 0.2).abs() < f32::EPSILON,
            "Graph weight should be override value 0.2"
        );

        // Other weights should be renormalized
        // Original non-graph sum: 0.4 + 0.35 + 0.1 + 0.05 = 0.9
        // Remaining: 1.0 - 0.2 = 0.8
        // Scale: 0.8 / 0.9 = 0.8889
        let expected_scale = 0.8 / 0.9;
        assert!(
            (weights.fts - 0.4 * expected_scale).abs() < 0.001,
            "FTS should be scaled: expected {}, got {}",
            0.4 * expected_scale,
            weights.fts
        );
        assert!(
            (weights.vector - 0.35 * expected_scale).abs() < 0.001,
            "Vector should be scaled"
        );
        assert!(
            (weights.recency - 0.1 * expected_scale).abs() < 0.001,
            "Recency should be scaled"
        );
        assert!(
            (weights.churn - 0.05 * expected_scale).abs() < 0.001,
            "Churn should be scaled"
        );

        // Total should still be 1.0
        let total = weights.fts + weights.vector + weights.graph + weights.recency + weights.churn;
        assert!(
            (total - 1.0).abs() < 0.001,
            "Total weights should be 1.0, got {}",
            total
        );
    }

    #[test]
    fn test_calculate_fusion_weights_high_override() {
        // High override value (e.g., 0.5) should still normalize correctly
        let mut config = SearchConfig::default();
        config.graph_importance.fusion_weight_override = Some(0.5);

        let pipeline = create_test_pipeline_with_config(Some(config));
        let options = SearchOptions::new(1, None, 10);

        let weights = pipeline.calculate_fusion_weights(&options);

        assert!(
            (weights.graph - 0.5).abs() < f32::EPSILON,
            "Graph weight should be 0.5"
        );

        // Total should still be 1.0
        let total = weights.fts + weights.vector + weights.graph + weights.recency + weights.churn;
        assert!(
            (total - 1.0).abs() < 0.001,
            "Total weights should be 1.0, got {}",
            total
        );
    }

    #[test]
    fn test_calculate_fusion_weights_preserves_custom_options() {
        // Options with custom weights should be respected, then override applied
        let mut config = SearchConfig::default();
        config.graph_importance.fusion_weight_override = Some(0.15);

        let pipeline = create_test_pipeline_with_config(Some(config));

        // Create options with custom fusion weights
        let custom_weights = FusionWeights::new(0.5, 0.3, 0.1, 0.07, 0.03);
        let options = SearchOptions::new(1, None, 10).with_fusion_weights(custom_weights);

        let weights = pipeline.calculate_fusion_weights(&options);

        // Graph should be exactly the override
        assert!(
            (weights.graph - 0.15).abs() < f32::EPSILON,
            "Graph weight should be override value 0.15"
        );

        // Original non-graph sum: 0.5 + 0.3 + 0.07 + 0.03 = 0.9
        // Remaining: 1.0 - 0.15 = 0.85
        // Scale: 0.85 / 0.9 = 0.9444
        let expected_scale = 0.85 / 0.9;
        assert!(
            (weights.fts - 0.5 * expected_scale).abs() < 0.001,
            "FTS should be scaled from custom weight"
        );
    }

    #[test]
    fn test_graph_importance_config_default_no_override() {
        // Verify GraphImportanceConfig default has no fusion override
        let config = GraphImportanceConfig::default();
        assert!(
            config.fusion_weight_override.is_none(),
            "Default GraphImportanceConfig should have no fusion override"
        );
    }
}
