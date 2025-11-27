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

use crate::metrics::get_metrics;
use crate::search::dedup::{self, DeduplicationConfig};
use crate::search::executor_types::SearchSource;
use crate::search::executors::SearchExecutors;
use crate::search::fusion::{BasicWeightedFusion, FusedResult, ScoreFusion};
use crate::search::query_processor::QueryProcessor;
use crate::search::results::{
    ChunkSearchResult, FinalSearchResults, QueryProcessingDetails, SearchMetadata, SearchOptions,
    SearchTiming,
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
            reranker: None,
        }
    }

    /// Get reference to the database store (for testing and utilities).
    pub fn store(&self) -> &crate::db::SqliteStore {
        self.executors.store()
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
        let processing_time = process_start.elapsed().as_secs_f64() * 1000.0;

        let mode_str = format!("{:?}", processed.mode).to_lowercase();

        debug!(
            "Query processed in {:.2}ms: {} tokens, mode={:?}",
            processing_time,
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
                options.limit * 3, // Fetch more for better fusion
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
        let search_time = search_start.elapsed().as_secs_f64() * 1000.0;

        debug!(
            "Parallel search completed in {:.2}ms: {}",
            search_time,
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
        let fusion_weights = options.get_fusion_weights();

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
        let fusion_time = fusion_start.elapsed().as_secs_f64() * 1000.0;

        // Record fusion time metric
        metrics.record_fusion_time(fusion_time / 1000.0, "basic_weighted");

        debug!(
            "Score fusion completed in {:.2}ms: {} results",
            fusion_time,
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
                options,
            )
            .await?;
        let assembly_time = assembly_start.elapsed().as_secs_f64() * 1000.0;

        debug!(
            "Result assembly completed in {:.2}ms: {} results",
            assembly_time,
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
                total_time, processing_time, search_time, fusion_time, assembly_time
            );
        }

        trace!(
            "Pipeline timing breakdown: total={:.2}ms, processing={:.2}ms, search={:.2}ms, fusion={:.2}ms, assembly={:.2}ms",
            total_time, processing_time, search_time, fusion_time, assembly_time
        );

        Ok(final_results)
    }

    /// Assemble final results by enriching fused results with chunk details.
    ///
    /// Fetches complete chunk information from the database including:
    /// - File path, symbol name, kind
    /// - Line ranges
    /// - Preview text
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
                    search_execution_time_ms,
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
        for fused in fused_results {
            if let Some(details) = chunk_details.get(&fused.chunk_id) {
                assembled_results.push(ChunkSearchResult::new(
                    fused.chunk_id,
                    details.file_id,
                    details.relpath.clone(),
                    details.symbol_name.clone(),
                    details.kind.clone(),
                    details.start_line,
                    details.end_line,
                    details.preview.clone(),
                    fused.score,
                    fused.source_scores,
                ));
            } else {
                warn!(
                    "Chunk {} found in fusion but missing from database",
                    fused.chunk_id
                );
            }
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

        let metadata = self.build_metadata(
            processed,
            total_unique_chunks,
            final_results.len(),
            fts_count,
            vector_count,
            graph_count,
            signals_count,
            search_execution_time_ms,
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
        // TODO(IDXABS-2003): This needs to be implemented using SqliteStore.
        // SqliteStore doesn't have a bulk chunk details fetch method yet.
        // This should query the chunks and files tables to get the needed fields.
        // See ticket IDXABS-4001 for search functionality updates.

        debug!("fetch_chunk_details called for {} chunk IDs (not yet implemented)", chunk_ids.len());
        Ok(HashMap::new())
    }

    /// Build search metadata from pipeline execution details.
    fn build_metadata(
        &self,
        processed: &ProcessedQuery,
        total_unique: usize,
        returned: usize,
        fts_count: usize,
        vector_count: usize,
        graph_count: usize,
        signals_count: usize,
        search_execution_time_ms: f64,
        _options: &SearchOptions,
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

        // Timing is tracked at the search() method level, so we estimate here
        // In a production system, we'd pass actual timing data through
        let timing = SearchTiming::new(
            5.0,                      // query processing estimate
            search_execution_time_ms, // actual search time
            2.0,                      // fusion estimate
            5.0,                      // assembly estimate
        );

        SearchMetadata::new(query_details, result_counts, timing, total_unique, returned)
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
}
