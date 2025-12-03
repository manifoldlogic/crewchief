//! Graph-based importance scoring using chunk_edges.
//!
//! This module implements PageRank-like importance scoring based on
//! incoming edges from the chunk_edges table. Different edge types
//! contribute different weights to the importance score.

use crate::db::SqliteStore;
use crate::search::executor_types::{RankedResult, RankedResults, SearchSource};
use tracing::{debug, instrument};

/// Graph importance executor.
///
/// Calculates PageRank-like scores from chunk_edges table:
/// - Incoming calls: 0.3 weight
/// - Incoming imports: 0.2 weight
/// - Test coverage: 0.1 weight
///
/// Uses logarithmic scaling to dampen extreme values.
/// Over-fetches results (limit * 2) for fusion.
pub struct GraphExecutor;

impl GraphExecutor {
    /// Execute graph importance query.
    ///
    /// # Parameters
    /// - `client`: Database client
    /// - `repo_id`: Repository ID to filter results
    /// - `worktree_id`: Optional worktree ID for additional filtering
    /// - `limit`: Maximum number of results (will over-fetch by 2x)
    ///
    /// # Returns
    /// RankedResults with graph importance scores normalized to 0.0-1.0 range
    ///
    /// # SQL Query
    /// ```sql
    /// WITH edge_counts AS (
    ///   SELECT
    ///     dst_chunk_id as chunk_id,
    ///     COUNT(*) FILTER (WHERE type = 'calls') as callers,
    ///     COUNT(*) FILTER (WHERE type = 'imports') as importers,
    ///     COUNT(*) FILTER (WHERE type = 'test_of') as tests
    ///   FROM maproom.chunk_edges
    ///   GROUP BY dst_chunk_id
    /// )
    /// SELECT
    ///   c.id,
    ///   COALESCE(
    ///     LOG(2 + COALESCE(e.callers, 0)) * 0.3 +
    ///     LOG(2 + COALESCE(e.importers, 0)) * 0.2 +
    ///     LOG(2 + COALESCE(e.tests, 0)) * 0.1,
    ///     0
    ///   ) as graph_score
    /// FROM maproom.chunks c
    /// JOIN maproom.files f ON f.id = c.file_id
    /// LEFT JOIN edge_counts e ON e.chunk_id = c.id
    /// WHERE f.repo_id = $1
    ///   AND ($2::bigint IS NULL OR f.worktree_id = $2)
    /// ORDER BY graph_score DESC
    /// LIMIT $3;
    /// ```
    #[instrument(skip(store))]
    pub async fn execute(
        store: &SqliteStore,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
    ) -> Result<RankedResults, GraphError> {
        // Over-fetch by 2x for fusion (graph signals are less selective than FTS/vector)
        let _fetch_limit = (limit * 2) as i64;

        debug!("Executing graph importance query (limit: {})", limit);

        // Delegate to SqliteStore's graph importance calculation
        let hits = store
            .calculate_graph_importance(repo_id, worktree_id, _fetch_limit as usize)
            .await
            .map_err(|e| GraphError::Database(e.to_string()))?;

        // Convert SearchHit to RankedResult
        let results: Vec<RankedResult> = hits
            .into_iter()
            .enumerate()
            .map(|(i, hit)| RankedResult::new(hit.chunk_id, hit.score as f32, i + 1))
            .collect();

        debug!("Graph search returned {} results", results.len());
        Ok(RankedResults::new(results, SearchSource::Graph))
    }

    /// Execute graph importance query for specific chunk IDs.
    ///
    /// This variant calculates graph scores only for a given set of chunks,
    /// useful when combining with other search results.
    #[instrument(skip(store, chunk_ids), fields(chunk_count = chunk_ids.len()))]
    pub async fn execute_for_chunks(
        store: &SqliteStore,
        chunk_ids: &[i64],
        repo_id: i64,
        worktree_id: Option<i64>,
    ) -> Result<RankedResults, GraphError> {
        if chunk_ids.is_empty() {
            return Ok(RankedResults::empty(SearchSource::Graph));
        }

        debug!(
            "Executing graph importance query for {} specific chunks",
            chunk_ids.len()
        );

        // Delegate to SqliteStore's graph importance calculation for specific chunks
        let hits = store
            .calculate_graph_importance_for_chunks(chunk_ids, repo_id, worktree_id)
            .await
            .map_err(|e| GraphError::Database(e.to_string()))?;

        // Convert SearchHit to RankedResult
        let results: Vec<RankedResult> = hits
            .into_iter()
            .enumerate()
            .map(|(i, hit)| RankedResult::new(hit.chunk_id, hit.score as f32, i + 1))
            .collect();

        debug!("Graph search for chunks returned {} results", results.len());
        Ok(RankedResults::new(results, SearchSource::Graph))
    }
}

/// Errors that can occur during graph query execution.
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    /// Database query error
    #[error("Database error: {0}")]
    Database(String),

    /// No edge data available
    #[error("No graph data available")]
    NoGraphData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_executor_exists() {
        // Verify the executor type exists
        let _executor = GraphExecutor;
    }

    // Note: Full integration tests with real database are in tests/search/executors_test.rs
}
