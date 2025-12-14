//! Final search result structures with complete metadata.
//!
//! This module defines the final output format for hybrid search queries,
//! including complete chunk details, metadata, and timing information.

use crate::search::executor_types::SearchSource;
use crate::search::types::SearchMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete search results with ranked chunks and metadata.
///
/// This is the top-level structure returned by the SearchPipeline,
/// containing everything needed to display and analyze search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalSearchResults {
    /// Original query string
    pub query: String,

    /// Ranked search results with full chunk details
    pub results: Vec<ChunkSearchResult>,

    /// Search execution metadata and statistics
    pub metadata: SearchMetadata,
}

impl FinalSearchResults {
    /// Create new FinalSearchResults.
    pub fn new(query: String, results: Vec<ChunkSearchResult>, metadata: SearchMetadata) -> Self {
        Self {
            query,
            results,
            metadata,
        }
    }

    /// Check if there are any results.
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Get the number of results.
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Get the top N results.
    pub fn top_n(&self, n: usize) -> &[ChunkSearchResult] {
        let end = n.min(self.results.len());
        &self.results[..end]
    }
}

/// A single search result with complete chunk details and scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkSearchResult {
    /// Chunk ID from maproom.chunks table
    pub chunk_id: i64,

    /// File ID from maproom.files table
    pub file_id: i64,

    /// Relative path to the file
    pub relpath: String,

    /// Optional symbol name (function, class, etc.)
    pub symbol_name: Option<String>,

    /// Chunk kind (function, class, interface, etc.)
    pub kind: String,

    /// Starting line number (1-based)
    pub start_line: i32,

    /// Ending line number (1-based)
    pub end_line: i32,

    /// Preview text from the chunk
    pub preview: String,

    /// Final fused score (0.0-1.0)
    pub score: f32,

    /// Individual scores from each search source
    pub source_scores: HashMap<SearchSource, f32>,

    /// Confidence signals for result quality assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ConfidenceSignals>,
}

impl ChunkSearchResult {
    /// Create a new ChunkSearchResult.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chunk_id: i64,
        file_id: i64,
        relpath: String,
        symbol_name: Option<String>,
        kind: String,
        start_line: i32,
        end_line: i32,
        preview: String,
        score: f32,
        source_scores: HashMap<SearchSource, f32>,
    ) -> Self {
        Self {
            chunk_id,
            file_id,
            relpath,
            symbol_name,
            kind,
            start_line,
            end_line,
            preview,
            score,
            source_scores,
            confidence: None,
        }
    }

    /// Get the line range as a formatted string.
    pub fn line_range(&self) -> String {
        format!("{}-{}", self.start_line, self.end_line)
    }

    /// Get the number of lines in this chunk.
    pub fn line_count(&self) -> i32 {
        self.end_line - self.start_line + 1
    }
}

/// Query understanding metadata for successful searches (Phase 2).
///
/// This structure provides transparency about how the query was interpreted,
/// what filters were applied, and timing breakdown. All data is assembled from
/// existing in-memory structures - no new computation is performed.
///
/// TYPE_SYNC: packages/daemon-client/src/client.ts::QueryUnderstanding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryUnderstanding {
    /// Detected search mode
    pub mode: SearchMode,

    /// Tokenized query terms
    pub tokens: Vec<String>,

    /// Expanded query terms (synonyms, variations)
    pub expanded_terms: Vec<String>,

    /// Applied filters
    pub filters: QueryFilters,

    /// Fusion strategy name (e.g., "reciprocal_rank_fusion", "basic_weighted")
    pub fusion_strategy: String,

    /// Timing breakdown for search stages
    pub timing: TimingBreakdown,
}

impl QueryUnderstanding {
    /// Create from processed query data and timing information.
    ///
    /// This is a convenience constructor that assembles QueryUnderstanding from
    /// data already available in the search pipeline.
    pub fn from_query_data(
        mode: SearchMode,
        tokens: Vec<String>,
        expanded_terms: Vec<String>,
        filters: QueryFilters,
        fusion_strategy: String,
        timing: TimingBreakdown,
    ) -> Self {
        Self {
            mode,
            tokens,
            expanded_terms,
            filters,
            fusion_strategy,
            timing,
        }
    }
}

/// Filters applied to the search query.
///
/// TYPE_SYNC: packages/daemon-client/src/client.ts::QueryFilters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilters {
    /// Repository ID being searched
    pub repo_id: i64,

    /// Optional worktree ID filter
    pub worktree_id: Option<i64>,

    /// File type filters (e.g., ["ts", "tsx", "js"])
    pub file_types: Vec<String>,

    /// Recency threshold filter (e.g., "7 days", "1 month")
    pub recency_threshold: Option<String>,
}

/// Timing breakdown for search execution stages.
///
/// TYPE_SYNC: packages/daemon-client/src/client.ts::TimingBreakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingBreakdown {
    /// Time spent processing the query (ms)
    pub query_processing_ms: f64,

    /// Time spent executing searches (ms)
    pub search_execution_ms: f64,

    /// Time spent fusing scores (ms)
    pub score_fusion_ms: f64,

    /// Time spent assembling final results (ms)
    pub result_assembly_ms: f64,

    /// Total time across all stages (ms)
    pub total_ms: f64,
}

impl TimingBreakdown {
    /// Create a new TimingBreakdown with automatic total calculation.
    ///
    /// The total_ms field is calculated as the sum of all individual timings.
    pub fn new(
        query_processing_ms: f64,
        search_execution_ms: f64,
        score_fusion_ms: f64,
        result_assembly_ms: f64,
    ) -> Self {
        let total_ms =
            query_processing_ms + search_execution_ms + score_fusion_ms + result_assembly_ms;

        Self {
            query_processing_ms,
            search_execution_ms,
            score_fusion_ms,
            result_assembly_ms,
            total_ms,
        }
    }
}

/// Metadata about search execution and results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetadata {
    /// Query processing details
    pub query_processing: QueryProcessingDetails,

    /// Result counts from each search strategy
    pub result_counts: HashMap<SearchSource, usize>,

    /// Timing breakdown for each stage
    pub timing: SearchTiming,

    /// Total number of unique chunks found before fusion
    pub total_unique_chunks: usize,

    /// Number of results returned after fusion and limit
    pub returned_results: usize,

    /// Query understanding metadata (added in Phase 2)
    ///
    /// This field provides transparency about query interpretation, filters,
    /// and timing. It's optional for backward compatibility and is omitted
    /// from JSON when None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub understanding: Option<QueryUnderstanding>,
}

impl SearchMetadata {
    /// Create new SearchMetadata.
    pub fn new(
        query_processing: QueryProcessingDetails,
        result_counts: HashMap<SearchSource, usize>,
        timing: SearchTiming,
        total_unique_chunks: usize,
        returned_results: usize,
    ) -> Self {
        Self {
            query_processing,
            result_counts,
            timing,
            total_unique_chunks,
            returned_results,
            understanding: None,
        }
    }

    /// Create new SearchMetadata with query understanding.
    pub fn with_understanding(
        query_processing: QueryProcessingDetails,
        result_counts: HashMap<SearchSource, usize>,
        timing: SearchTiming,
        total_unique_chunks: usize,
        returned_results: usize,
        understanding: QueryUnderstanding,
    ) -> Self {
        Self {
            query_processing,
            result_counts,
            timing,
            total_unique_chunks,
            returned_results,
            understanding: Some(understanding),
        }
    }

    /// Get total execution time in milliseconds.
    pub fn total_time_ms(&self) -> f64 {
        self.timing.query_processing_ms
            + self.timing.search_execution_ms
            + self.timing.fusion_ms
            + self.timing.assembly_ms
    }

    /// Check if search met the performance target (< 50ms total).
    pub fn met_performance_target(&self) -> bool {
        self.total_time_ms() < 50.0
    }
}

/// Details about query processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryProcessingDetails {
    /// Original query string
    pub original: String,

    /// Detected search mode
    pub mode: SearchMode,

    /// Number of tokens extracted
    pub token_count: usize,

    /// Number of expanded terms added
    pub expanded_term_count: usize,

    /// FTS query string generated
    pub fts_query: String,

    /// Whether embedding was generated successfully
    pub has_embedding: bool,
}

impl QueryProcessingDetails {
    /// Create new QueryProcessingDetails.
    pub fn new(
        original: String,
        mode: SearchMode,
        token_count: usize,
        expanded_term_count: usize,
        fts_query: String,
        has_embedding: bool,
    ) -> Self {
        Self {
            original,
            mode,
            token_count,
            expanded_term_count,
            fts_query,
            has_embedding,
        }
    }
}

/// Timing breakdown for search execution stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchTiming {
    /// Time spent processing the query (ms)
    pub query_processing_ms: f64,

    /// Time spent executing parallel searches (ms)
    pub search_execution_ms: f64,

    /// Time spent fusing scores (ms)
    pub fusion_ms: f64,

    /// Time spent assembling final results with chunk details (ms)
    pub assembly_ms: f64,

    /// Optional reranking time (ms) - unused in Phase 2
    pub reranking_ms: Option<f64>,
}

impl SearchTiming {
    /// Create new SearchTiming.
    pub fn new(
        query_processing_ms: f64,
        search_execution_ms: f64,
        fusion_ms: f64,
        assembly_ms: f64,
    ) -> Self {
        Self {
            query_processing_ms,
            search_execution_ms,
            fusion_ms,
            assembly_ms,
            reranking_ms: None,
        }
    }

    /// Create SearchTiming with all stages at 0.0.
    pub fn zero() -> Self {
        Self {
            query_processing_ms: 0.0,
            search_execution_ms: 0.0,
            fusion_ms: 0.0,
            assembly_ms: 0.0,
            reranking_ms: None,
        }
    }
}

/// Options for configuring search execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Repository ID to search within
    pub repo_id: i64,

    /// Optional worktree ID for filtering
    pub worktree_id: Option<i64>,

    /// Maximum number of results to return
    pub limit: usize,

    /// Fusion weights for combining scores
    pub fusion_weights: Option<crate::search::fusion::FusionWeights>,

    /// Whether to skip vector search (faster, text-only)
    pub skip_vector: bool,

    /// Whether to skip graph search
    pub skip_graph: bool,

    /// Whether to skip signal search
    pub skip_signals: bool,

    /// Whether to deduplicate results across worktrees.
    ///
    /// When enabled (default), results with the same identity
    /// (relpath, symbol_name, start_line) are grouped, and only
    /// the highest-scoring instance is returned.
    ///
    /// Default: `true`
    #[serde(default = "default_deduplicate")]
    pub deduplicate: bool,

    /// File type filters (e.g., ["ts", "tsx", "js"])
    #[serde(default)]
    pub file_types: Vec<String>,

    /// Recency threshold filter (e.g., "7 days", "1 month")
    #[serde(default)]
    pub recency_threshold: Option<String>,

    /// Whether to include confidence signals in search results.
    ///
    /// When true, each result will include confidence field with quality signals.
    /// When false (default), confidence is None for backward compatibility.
    ///
    /// Default: `false`
    #[serde(default)]
    pub include_confidence: bool,
}

fn default_deduplicate() -> bool {
    true
}

/// Confidence signals for search result quality assessment.
///
/// This structure provides transparency about result quality through three
/// core signals computed from existing search pipeline data. All fields are
/// derived from in-memory structures with O(1) computation per result.
///
/// TYPE_SYNC: packages/daemon-client/src/types.ts::ConfidenceSignals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceSignals {
    /// Number of search sources that found this result (FTS, vector, graph, signals).
    /// Higher values indicate stronger cross-strategy agreement.
    pub source_count: usize,

    /// Score difference between this result and the next result.
    /// Larger gaps indicate clearer quality separation. 0.0 for the last result.
    pub score_gap: f32,

    /// Whether this result received an exact match boost during scoring.
    /// Exact matches (exact_match_multiplier >= 2.9) have higher confidence.
    pub is_exact_match: bool,
}

impl SearchOptions {
    /// Create new SearchOptions with required parameters.
    pub fn new(repo_id: i64, worktree_id: Option<i64>, limit: usize) -> Self {
        Self {
            repo_id,
            worktree_id,
            limit,
            fusion_weights: None,
            skip_vector: false,
            skip_graph: false,
            skip_signals: false,
            deduplicate: true,
            file_types: vec![],
            recency_threshold: None,
            include_confidence: false,
        }
    }

    /// Builder method to set fusion weights.
    pub fn with_fusion_weights(mut self, weights: crate::search::fusion::FusionWeights) -> Self {
        self.fusion_weights = Some(weights);
        self
    }

    /// Builder method to skip vector search.
    pub fn with_skip_vector(mut self, skip: bool) -> Self {
        self.skip_vector = skip;
        self
    }

    /// Builder method to skip graph search.
    pub fn with_skip_graph(mut self, skip: bool) -> Self {
        self.skip_graph = skip;
        self
    }

    /// Builder method to skip signal search.
    pub fn with_skip_signals(mut self, skip: bool) -> Self {
        self.skip_signals = skip;
        self
    }

    /// Builder method to disable deduplication.
    pub fn without_dedup(mut self) -> Self {
        self.deduplicate = false;
        self
    }

    /// Builder method to set deduplication explicitly.
    pub fn with_deduplicate(mut self, deduplicate: bool) -> Self {
        self.deduplicate = deduplicate;
        self
    }

    /// Get the fusion weights, using defaults if not specified.
    pub fn get_fusion_weights(&self) -> crate::search::fusion::FusionWeights {
        self.fusion_weights.clone().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_final_search_results_empty() {
        let results = FinalSearchResults::new(
            "test query".to_string(),
            vec![],
            SearchMetadata::new(
                QueryProcessingDetails::new(
                    "test query".to_string(),
                    SearchMode::Auto,
                    2,
                    0,
                    "test & query".to_string(),
                    true,
                ),
                HashMap::new(),
                SearchTiming::zero(),
                0,
                0,
            ),
        );

        assert!(results.is_empty());
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_chunk_search_result_line_range() {
        let result = ChunkSearchResult::new(
            1,
            1,
            "src/main.rs".to_string(),
            Some("main".to_string()),
            "function".to_string(),
            10,
            25,
            "fn main() {".to_string(),
            0.95,
            HashMap::new(),
        );

        assert_eq!(result.line_range(), "10-25");
        assert_eq!(result.line_count(), 16);
    }

    #[test]
    fn test_search_metadata_total_time() {
        let timing = SearchTiming::new(5.0, 30.0, 2.0, 8.0);
        let metadata = SearchMetadata::new(
            QueryProcessingDetails::new(
                "test".to_string(),
                SearchMode::Auto,
                1,
                0,
                "test".to_string(),
                true,
            ),
            HashMap::new(),
            timing,
            50,
            10,
        );

        assert_eq!(metadata.total_time_ms(), 45.0);
        assert!(metadata.met_performance_target());
    }

    #[test]
    fn test_search_metadata_performance_target_exceeded() {
        let timing = SearchTiming::new(15.0, 40.0, 5.0, 10.0);
        let metadata = SearchMetadata::new(
            QueryProcessingDetails::new(
                "test".to_string(),
                SearchMode::Auto,
                1,
                0,
                "test".to_string(),
                true,
            ),
            HashMap::new(),
            timing,
            50,
            10,
        );

        assert_eq!(metadata.total_time_ms(), 70.0);
        assert!(!metadata.met_performance_target());
    }

    #[test]
    fn test_search_options_builder() {
        let options = SearchOptions::new(1, Some(2), 10)
            .with_skip_vector(true)
            .with_skip_graph(false);

        assert_eq!(options.repo_id, 1);
        assert_eq!(options.worktree_id, Some(2));
        assert_eq!(options.limit, 10);
        assert!(options.skip_vector);
        assert!(!options.skip_graph);
    }

    #[test]
    fn test_search_options_default_weights() {
        let options = SearchOptions::new(1, None, 10);
        let weights = options.get_fusion_weights();

        assert_eq!(weights.fts, 0.4);
        assert_eq!(weights.vector, 0.35);
        assert_eq!(weights.graph, 0.1);
        assert_eq!(weights.recency, 0.1);
        assert_eq!(weights.churn, 0.05);
    }

    #[test]
    fn test_search_options_deduplicate_default() {
        let options = SearchOptions::new(1, None, 10);
        assert!(
            options.deduplicate,
            "Deduplication should be enabled by default"
        );
    }

    #[test]
    fn test_search_options_without_dedup() {
        let options = SearchOptions::new(1, None, 10).without_dedup();
        assert!(
            !options.deduplicate,
            "without_dedup should disable deduplication"
        );
    }

    #[test]
    fn test_search_options_with_deduplicate() {
        let options = SearchOptions::new(1, None, 10)
            .without_dedup()
            .with_deduplicate(true);
        assert!(
            options.deduplicate,
            "with_deduplicate(true) should enable deduplication"
        );

        let options = SearchOptions::new(1, None, 10).with_deduplicate(false);
        assert!(
            !options.deduplicate,
            "with_deduplicate(false) should disable deduplication"
        );
    }

    #[test]
    fn test_timing_breakdown_total_calculation() {
        let timing = TimingBreakdown::new(4.2, 35.8, 2.1, 6.4);

        assert_eq!(timing.total_ms, 48.5);
        assert_eq!(timing.query_processing_ms, 4.2);
        assert_eq!(timing.search_execution_ms, 35.8);
        assert_eq!(timing.score_fusion_ms, 2.1);
        assert_eq!(timing.result_assembly_ms, 6.4);
    }

    #[test]
    fn test_query_understanding_serialization() {
        let understanding = QueryUnderstanding {
            mode: SearchMode::Auto,
            tokens: vec!["authenticate".to_string(), "user".to_string()],
            expanded_terms: vec!["auth".to_string(), "login".to_string()],
            filters: QueryFilters {
                repo_id: 1,
                worktree_id: Some(2),
                file_types: vec![],
                recency_threshold: None,
            },
            fusion_strategy: "reciprocal_rank_fusion".to_string(),
            timing: TimingBreakdown::new(4.2, 35.8, 2.1, 6.4),
        };

        let json = serde_json::to_string(&understanding).unwrap();
        assert!(json.contains("authenticate"));
        assert!(json.contains("reciprocal_rank_fusion"));
        assert!(json.contains("\"total_ms\":48.5"));
    }

    #[test]
    fn test_optional_understanding_field_serialization() {
        let metadata = SearchMetadata {
            query_processing: QueryProcessingDetails::new(
                "test query".to_string(),
                SearchMode::Auto,
                2,
                0,
                "test & query".to_string(),
                true,
            ),
            result_counts: HashMap::new(),
            timing: SearchTiming::zero(),
            total_unique_chunks: 0,
            returned_results: 0,
            understanding: None,
        };

        let json = serde_json::to_value(&metadata).unwrap();
        // When None, field should be omitted (skip_serializing_if)
        assert!(json.get("understanding").is_none());

        let metadata_with_understanding = SearchMetadata {
            query_processing: QueryProcessingDetails::new(
                "test query".to_string(),
                SearchMode::Auto,
                2,
                0,
                "test & query".to_string(),
                true,
            ),
            result_counts: HashMap::new(),
            timing: SearchTiming::zero(),
            total_unique_chunks: 0,
            returned_results: 0,
            understanding: Some(QueryUnderstanding {
                mode: SearchMode::Code,
                tokens: vec!["test".to_string()],
                expanded_terms: vec![],
                filters: QueryFilters {
                    repo_id: 1,
                    worktree_id: None,
                    file_types: vec![],
                    recency_threshold: None,
                },
                fusion_strategy: "basic_weighted".to_string(),
                timing: TimingBreakdown::new(1.0, 2.0, 3.0, 4.0),
            }),
        };

        let json = serde_json::to_value(&metadata_with_understanding).unwrap();
        assert!(json.get("understanding").is_some());
        let understanding = json.get("understanding").unwrap();
        assert_eq!(understanding.get("mode").unwrap(), "code");
        assert_eq!(
            understanding.get("fusion_strategy").unwrap(),
            "basic_weighted"
        );
    }

    #[test]
    fn test_query_filters_serialization() {
        let filters = QueryFilters {
            repo_id: 42,
            worktree_id: Some(123),
            file_types: vec!["ts".to_string(), "tsx".to_string()],
            recency_threshold: Some("7 days".to_string()),
        };

        let json = serde_json::to_value(&filters).unwrap();
        assert_eq!(json.get("repo_id").unwrap(), 42);
        assert_eq!(json.get("worktree_id").unwrap(), 123);

        let file_types = json.get("file_types").unwrap().as_array().unwrap();
        assert_eq!(file_types.len(), 2);
        assert_eq!(file_types[0], "ts");
        assert_eq!(file_types[1], "tsx");

        assert_eq!(json.get("recency_threshold").unwrap(), "7 days");
    }

    #[test]
    fn test_query_understanding_from_query_data() {
        let timing = TimingBreakdown::new(5.0, 10.0, 2.0, 3.0);
        let filters = QueryFilters {
            repo_id: 1,
            worktree_id: None,
            file_types: vec!["rs".to_string()],
            recency_threshold: None,
        };

        let understanding = QueryUnderstanding::from_query_data(
            SearchMode::Code,
            vec!["search".to_string(), "query".to_string()],
            vec!["find".to_string()],
            filters,
            "reciprocal_rank_fusion".to_string(),
            timing,
        );

        assert_eq!(understanding.mode, SearchMode::Code);
        assert_eq!(understanding.tokens.len(), 2);
        assert_eq!(understanding.expanded_terms.len(), 1);
        assert_eq!(understanding.fusion_strategy, "reciprocal_rank_fusion");
        assert_eq!(understanding.timing.total_ms, 20.0);
    }
}
