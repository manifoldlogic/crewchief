//! Type definitions for search execution results.
//!
//! This module defines the result types used by search executors:
//! - RankedResults: Collection of ranked search results from a single source
//! - RankedResult: Individual result with chunk ID, score, and rank
//! - SearchSource: Enumeration of search strategy types

use serde::{Deserialize, Serialize};

/// Source of a ranked result set, indicating which search strategy produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchSource {
    /// Full-text search using PostgreSQL ts_rank_cd
    FTS,
    /// Vector similarity search using pgvector
    Vector,
    /// Graph-based importance from chunk_edges
    Graph,
    /// Recency and churn signal scores
    Signals,
}

impl SearchSource {
    /// Get a human-readable name for this search source.
    pub fn name(&self) -> &'static str {
        match self {
            SearchSource::FTS => "Full-Text Search",
            SearchSource::Vector => "Vector Similarity",
            SearchSource::Graph => "Graph Importance",
            SearchSource::Signals => "Temporal Signals",
        }
    }
}

/// Individual ranked search result.
///
/// Represents a single chunk with its relevance score and rank position
/// from a specific search strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedResult {
    /// Chunk ID from maproom.chunks table
    pub chunk_id: i64,

    /// Normalized score (0.0-1.0) indicating relevance
    pub score: f32,

    /// 1-based rank position in the result set
    pub rank: usize,

    /// Embedding dimension used for this result ("768" or "1536")
    /// None if result is not from vector search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dimension: Option<String>,

    /// Exact match multiplier applied during FTS scoring (3.0 for exact matches, 1.0 otherwise).
    /// None if result did not come from FTS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_match_multiplier: Option<f32>,
}

impl RankedResult {
    /// Create a new RankedResult.
    pub fn new(chunk_id: i64, score: f32, rank: usize) -> Self {
        Self {
            chunk_id,
            score,
            rank,
            embedding_dimension: None,
            exact_match_multiplier: None,
        }
    }

    /// Create a new RankedResult with embedding dimension information.
    pub fn new_with_dimension(
        chunk_id: i64,
        score: f32,
        rank: usize,
        embedding_dimension: Option<String>,
    ) -> Self {
        Self {
            chunk_id,
            score,
            rank,
            embedding_dimension,
            exact_match_multiplier: None,
        }
    }

    /// Create a new RankedResult with exact match multiplier (for FTS results).
    pub fn new_with_exact_match(
        chunk_id: i64,
        score: f32,
        rank: usize,
        exact_match_multiplier: Option<f32>,
    ) -> Self {
        Self {
            chunk_id,
            score,
            rank,
            embedding_dimension: None,
            exact_match_multiplier,
        }
    }
}

/// Collection of ranked results from a single search strategy.
///
/// This is the output format from each search executor (FTS, vector, graph, signals).
/// Results are sorted by score descending, with ranks assigned sequentially.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedResults {
    /// Ordered list of results (highest score first)
    pub results: Vec<RankedResult>,

    /// Source search strategy that produced these results
    pub source: SearchSource,
}

impl RankedResults {
    /// Create a new RankedResults collection.
    pub fn new(results: Vec<RankedResult>, source: SearchSource) -> Self {
        Self { results, source }
    }

    /// Create an empty RankedResults for the given source.
    pub fn empty(source: SearchSource) -> Self {
        Self {
            results: Vec::new(),
            source,
        }
    }

    /// Get the number of results.
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Check if there are no results.
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Get the top N results.
    pub fn top_n(&self, n: usize) -> Vec<&RankedResult> {
        self.results.iter().take(n).collect()
    }

    /// Get all chunk IDs in rank order.
    pub fn chunk_ids(&self) -> Vec<i64> {
        self.results.iter().map(|r| r.chunk_id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_source_name() {
        assert_eq!(SearchSource::FTS.name(), "Full-Text Search");
        assert_eq!(SearchSource::Vector.name(), "Vector Similarity");
        assert_eq!(SearchSource::Graph.name(), "Graph Importance");
        assert_eq!(SearchSource::Signals.name(), "Temporal Signals");
    }

    #[test]
    fn test_ranked_result_creation() {
        let result = RankedResult::new(123, 0.85, 1);
        assert_eq!(result.chunk_id, 123);
        assert_eq!(result.score, 0.85);
        assert_eq!(result.rank, 1);
    }

    #[test]
    fn test_ranked_results_empty() {
        let results = RankedResults::empty(SearchSource::FTS);
        assert!(results.is_empty());
        assert_eq!(results.len(), 0);
        assert_eq!(results.source, SearchSource::FTS);
    }

    #[test]
    fn test_ranked_results_operations() {
        let results = RankedResults::new(
            vec![
                RankedResult::new(1, 0.9, 1),
                RankedResult::new(2, 0.8, 2),
                RankedResult::new(3, 0.7, 3),
            ],
            SearchSource::Vector,
        );

        assert_eq!(results.len(), 3);
        assert!(!results.is_empty());

        let top_2 = results.top_n(2);
        assert_eq!(top_2.len(), 2);
        assert_eq!(top_2[0].chunk_id, 1);
        assert_eq!(top_2[1].chunk_id, 2);

        let ids = results.chunk_ids();
        assert_eq!(ids, vec![1, 2, 3]);
    }
}
