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
        }
    }

    /// Builder method to set fusion weights.
    pub fn with_fusion_weights(
        mut self,
        weights: crate::search::fusion::FusionWeights,
    ) -> Self {
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

    /// Get the fusion weights, using defaults if not specified.
    pub fn get_fusion_weights(&self) -> crate::search::fusion::FusionWeights {
        self.fusion_weights
            .clone()
            .unwrap_or_else(crate::search::fusion::FusionWeights::default)
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
        assert_eq!(weights.vector, 0.4);
        assert_eq!(weights.graph, 0.2);
    }
}
