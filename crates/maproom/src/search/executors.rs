//! Parallel search execution coordinator.
//!
//! This module implements the SearchExecutors struct that coordinates
//! parallel execution of FTS, vector, graph, and signal searches using
//! tokio::join! for optimal performance.

use crate::db::SqliteStore;
use crate::profile_scope;
use crate::search::executor_types::RankedResults;
use crate::search::fts::{FTSError, FTSExecutor};
use crate::search::graph::{GraphError, GraphExecutor};
use crate::search::signals::{SignalError, SignalExecutor};
use crate::search::types::ProcessedQuery;
use crate::search::vector::{VectorError, VectorExecutor};
use std::time::Instant;
use tracing::{debug, info, instrument, warn};

/// Parallel search execution coordinator.
///
/// Orchestrates parallel execution of all search strategies:
/// - Full-text search (FTS)
/// - Vector similarity search
/// - Graph importance signals
/// - Temporal signals (recency/churn)
///
/// All searches run concurrently using tokio::join! to minimize latency.
pub struct SearchExecutors {
    /// Database store for query execution
    store: SqliteStore,
}

impl SearchExecutors {
    /// Create a new SearchExecutors coordinator.
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }

    /// Execute all search strategies in parallel.
    ///
    /// # Parameters
    /// - `query`: Processed query with tokens, embedding, and mode
    /// - `repo_id`: Repository ID to filter results
    /// - `worktree_id`: Optional worktree ID for additional filtering
    /// - `limit`: Maximum number of results per search strategy
    ///
    /// # Returns
    /// Tuple of (FTS, Vector, Graph, Signals) RankedResults
    ///
    /// # Performance
    /// - Target: < 100ms total execution time
    /// - FTS: ~20-30ms
    /// - Vector: ~30-50ms
    /// - Graph: ~10-20ms
    /// - Signals: ~5-10ms
    /// - Parallel execution should complete in ~50-80ms
    #[instrument(skip(self, query), fields(original = %query.original, mode = ?query.mode))]
    pub async fn execute_all(
        &self,
        query: &ProcessedQuery,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
    ) -> Result<SearchResults, ExecutorError> {
        profile_scope!("search_execute_all");
        let start = Instant::now();

        info!(
            "Executing parallel search (repo: {}, worktree: {:?}, limit: {})",
            repo_id, worktree_id, limit
        );

        // Prepare parameters
        let fts_query = query.fts_query_string();
        let original_query = query.original.clone();
        let query_embedding = query.embedding.clone();
        let search_mode = query.mode;

        // Normalize query for exact match detection (SEMRANK-2004b)
        let normalized_query = crate::search::fts::normalize_for_exact_match(&original_query);

        // Execute all searches in parallel using tokio::join!
        let (fts_result, vector_result, graph_result, signal_result) = tokio::join!(
            // Full-text search with normalized query for exact matching
            FTSExecutor::execute(
                &self.store,
                &fts_query,
                &normalized_query,
                repo_id,
                worktree_id,
                limit
            ),
            // Vector similarity search
            VectorExecutor::execute(
                &self.store,
                &query_embedding,
                search_mode,
                repo_id,
                worktree_id,
                limit
            ),
            // Graph importance (None config = legacy mode, Phase 2 will pass config)
            GraphExecutor::execute(&self.store, repo_id, worktree_id, limit, None),
            // Temporal signals
            SignalExecutor::execute(&self.store, repo_id, worktree_id),
        );

        let elapsed = start.elapsed();

        // Handle errors from each executor
        let fts_results = match fts_result {
            Ok(results) => results,
            Err(e) => {
                warn!("FTS search failed: {}", e);
                RankedResults::empty(crate::search::executor_types::SearchSource::FTS)
            }
        };

        let vector_results = match vector_result {
            Ok(results) => results,
            Err(e) => {
                warn!("Vector search failed: {}", e);
                RankedResults::empty(crate::search::executor_types::SearchSource::Vector)
            }
        };

        let graph_results = match graph_result {
            Ok(results) => results,
            Err(e) => {
                warn!("Graph search failed: {}", e);
                RankedResults::empty(crate::search::executor_types::SearchSource::Graph)
            }
        };

        let signal_results = match signal_result {
            Ok(results) => results,
            Err(e) => {
                warn!("Signal search failed: {}", e);
                RankedResults::empty(crate::search::executor_types::SearchSource::Signals)
            }
        };

        info!(
            "Parallel search completed in {:.2}ms (FTS: {}, Vector: {}, Graph: {}, Signals: {})",
            elapsed.as_secs_f64() * 1000.0,
            fts_results.len(),
            vector_results.len(),
            graph_results.len(),
            signal_results.len()
        );

        // Check if we met the performance target
        if elapsed.as_millis() > 100 {
            warn!(
                "Search exceeded 100ms target: {:.2}ms",
                elapsed.as_secs_f64() * 1000.0
            );
        }

        Ok(SearchResults {
            fts: fts_results,
            vector: vector_results,
            graph: graph_results,
            signals: signal_results,
            execution_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Execute only FTS and vector searches (fast path for simple queries).
    ///
    /// This is a lighter-weight variant that skips graph and signal queries
    /// when they're not needed (e.g., for quick lookups).
    #[instrument(skip(self, query))]
    pub async fn execute_fast(
        &self,
        query: &ProcessedQuery,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
    ) -> Result<(RankedResults, RankedResults), ExecutorError> {
        let start = Instant::now();

        debug!("Executing fast search (FTS + Vector only)");

        let fts_query = query.fts_query_string();
        let original_query = query.original.clone();
        let query_embedding = query.embedding.clone();
        let search_mode = query.mode;

        // Normalize query for exact match detection (SEMRANK-2004b)
        let normalized_query = crate::search::fts::normalize_for_exact_match(&original_query);

        let (fts_result, vector_result) = tokio::join!(
            // Full-text search with normalized query for exact matching
            FTSExecutor::execute(
                &self.store,
                &fts_query,
                &normalized_query,
                repo_id,
                worktree_id,
                limit
            ),
            VectorExecutor::execute(
                &self.store,
                &query_embedding,
                search_mode,
                repo_id,
                worktree_id,
                limit
            ),
        );

        let elapsed = start.elapsed();

        let fts_results = fts_result.map_err(ExecutorError::FTS)?;
        let vector_results = vector_result.map_err(ExecutorError::Vector)?;

        debug!(
            "Fast search completed in {:.2}ms (FTS: {}, Vector: {})",
            elapsed.as_secs_f64() * 1000.0,
            fts_results.len(),
            vector_results.len()
        );

        Ok((fts_results, vector_results))
    }

    /// Get reference to database store for custom queries.
    pub fn store(&self) -> &SqliteStore {
        &self.store
    }
}

/// Results from parallel search execution.
#[derive(Debug, Clone)]
pub struct SearchResults {
    /// Full-text search results
    pub fts: RankedResults,
    /// Vector similarity results
    pub vector: RankedResults,
    /// Graph importance results
    pub graph: RankedResults,
    /// Temporal signal results
    pub signals: RankedResults,
    /// Total execution time in milliseconds
    pub execution_time_ms: f64,
}

impl SearchResults {
    /// Get total number of unique chunks across all result sets.
    pub fn total_unique_chunks(&self) -> usize {
        let mut chunk_ids = std::collections::HashSet::new();
        chunk_ids.extend(self.fts.chunk_ids());
        chunk_ids.extend(self.vector.chunk_ids());
        chunk_ids.extend(self.graph.chunk_ids());
        chunk_ids.extend(self.signals.chunk_ids());
        chunk_ids.len()
    }

    /// Check if execution met the 100ms performance target.
    pub fn met_performance_target(&self) -> bool {
        self.execution_time_ms <= 100.0
    }

    /// Get a summary of result counts.
    pub fn summary(&self) -> String {
        format!(
            "FTS: {}, Vector: {}, Graph: {}, Signals: {} ({:.1}ms)",
            self.fts.len(),
            self.vector.len(),
            self.graph.len(),
            self.signals.len(),
            self.execution_time_ms
        )
    }
}

/// Errors that can occur during search execution.
#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    /// Full-text search error
    #[error("FTS search failed: {0}")]
    FTS(#[from] FTSError),

    /// Vector search error
    #[error("Vector search failed: {0}")]
    Vector(#[from] VectorError),

    /// Graph search error
    #[error("Graph search failed: {0}")]
    Graph(#[from] GraphError),

    /// Signal search error
    #[error("Signal search failed: {0}")]
    Signal(#[from] SignalError),

    /// All searches failed
    #[error("All search strategies failed")]
    AllFailed,

    /// Database connection error
    #[error("Database connection error: {0}")]
    Connection(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_results_summary() {
        use crate::search::executor_types::{RankedResult, RankedResults, SearchSource};

        let fts = RankedResults::new(vec![RankedResult::new(1, 0.9, 1)], SearchSource::FTS);
        let vector = RankedResults::new(
            vec![RankedResult::new(2, 0.8, 1), RankedResult::new(3, 0.7, 2)],
            SearchSource::Vector,
        );
        let graph = RankedResults::empty(SearchSource::Graph);
        let signals = RankedResults::empty(SearchSource::Signals);

        let results = SearchResults {
            fts,
            vector,
            graph,
            signals,
            execution_time_ms: 75.5,
        };

        assert_eq!(results.total_unique_chunks(), 3);
        assert!(results.met_performance_target());
        let summary = results.summary();
        assert!(summary.contains("FTS: 1"));
        assert!(summary.contains("Vector: 2"));
        // Check that execution time is present (allow for rounding variations)
        assert!(summary.contains("75") && summary.contains("ms"));
    }

    // Note: Full integration tests with real database are in tests/search/executors_test.rs
}
