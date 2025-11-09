//! Graph-based importance scoring using chunk_edges.
//!
//! This module implements PageRank-like importance scoring based on
//! incoming edges from the chunk_edges table. Different edge types
//! contribute different weights to the importance score.

use crate::search::executor_types::{RankedResult, RankedResults, SearchSource};
use tokio_postgres::Client;
use tracing::{debug, instrument, warn};

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
    #[instrument(skip(client))]
    pub async fn execute(
        client: &Client,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
    ) -> Result<RankedResults, GraphError> {
        // Over-fetch by 2x for fusion (graph signals are less selective than FTS/vector)
        let fetch_limit = (limit * 2) as i64;

        debug!(
            "Executing graph importance query (limit: {}, over-fetch: {})",
            limit, fetch_limit
        );

        let sql = r#"
            WITH edge_counts AS (
              SELECT
                dst_chunk_id as chunk_id,
                COUNT(*) FILTER (WHERE type = 'calls') as callers,
                COUNT(*) FILTER (WHERE type = 'imports') as importers,
                COUNT(*) FILTER (WHERE type = 'test_of') as tests
              FROM maproom.chunk_edges
              GROUP BY dst_chunk_id
            )
            SELECT
              c.id,
              COALESCE(
                LOG(2 + COALESCE(e.callers, 0)) * 0.3 +
                LOG(2 + COALESCE(e.importers, 0)) * 0.2 +
                LOG(2 + COALESCE(e.tests, 0)) * 0.1,
                0
              ) as graph_score
            FROM maproom.chunks c
            JOIN maproom.files f ON f.id = c.file_id
            LEFT JOIN edge_counts e ON e.chunk_id = c.id
            WHERE f.repo_id = $1
              AND ($2::bigint IS NULL OR f.worktree_id = $2)
            ORDER BY graph_score DESC
            LIMIT $3
        "#;

        let stmt = client.prepare(sql).await.map_err(|e| {
            warn!("Failed to prepare graph query: {}", e);
            GraphError::Database(format!("Failed to prepare query: {}", e))
        })?;

        let rows = client
            .query(&stmt, &[&repo_id, &worktree_id, &fetch_limit])
            .await
            .map_err(|e| {
                warn!("Failed to execute graph query: {}", e);
                GraphError::Database(format!("Query execution failed: {}", e))
            })?;

        debug!("Graph query returned {} rows", rows.len());

        if rows.is_empty() {
            return Ok(RankedResults::empty(SearchSource::Graph));
        }

        // Extract results and find max score for normalization
        let raw_results: Vec<(i64, f64)> = rows
            .iter()
            .map(|row| {
                let chunk_id: i64 = row.get(0);
                let score: f64 = row.get(1);
                (chunk_id, score)
            })
            .collect();

        let max_score = raw_results.iter().map(|(_, s)| *s).fold(0.0f64, f64::max);

        // Normalize scores to 0.0-1.0 range
        let ranked_results: Vec<RankedResult> = raw_results
            .iter()
            .enumerate()
            .map(|(idx, (chunk_id, score))| {
                let normalized_score = if max_score > 0.0 {
                    (score / max_score) as f32
                } else {
                    0.0
                };
                RankedResult::new(*chunk_id, normalized_score, idx + 1)
            })
            .collect();

        debug!(
            "Graph executor returning {} results (max_score: {:.4})",
            ranked_results.len(),
            max_score
        );

        Ok(RankedResults::new(ranked_results, SearchSource::Graph))
    }

    /// Execute graph importance query for specific chunk IDs.
    ///
    /// This variant calculates graph scores only for a given set of chunks,
    /// useful when combining with other search results.
    #[instrument(skip(client, chunk_ids), fields(chunk_count = chunk_ids.len()))]
    pub async fn execute_for_chunks(
        client: &Client,
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

        let sql = r#"
            WITH edge_counts AS (
              SELECT
                dst_chunk_id as chunk_id,
                COUNT(*) FILTER (WHERE type = 'calls') as callers,
                COUNT(*) FILTER (WHERE type = 'imports') as importers,
                COUNT(*) FILTER (WHERE type = 'test_of') as tests
              FROM maproom.chunk_edges
              WHERE dst_chunk_id = ANY($1)
              GROUP BY dst_chunk_id
            )
            SELECT
              c.id,
              COALESCE(
                LOG(2 + COALESCE(e.callers, 0)) * 0.3 +
                LOG(2 + COALESCE(e.importers, 0)) * 0.2 +
                LOG(2 + COALESCE(e.tests, 0)) * 0.1,
                0
              ) as graph_score
            FROM maproom.chunks c
            JOIN maproom.files f ON f.id = c.file_id
            LEFT JOIN edge_counts e ON e.chunk_id = c.id
            WHERE c.id = ANY($1)
              AND f.repo_id = $2
              AND ($3::bigint IS NULL OR f.worktree_id = $3)
            ORDER BY graph_score DESC
        "#;

        let stmt = client.prepare(sql).await.map_err(|e| {
            warn!("Failed to prepare chunk-specific graph query: {}", e);
            GraphError::Database(format!("Failed to prepare query: {}", e))
        })?;

        let rows = client
            .query(&stmt, &[&chunk_ids, &repo_id, &worktree_id])
            .await
            .map_err(|e| {
                warn!("Failed to execute chunk-specific graph query: {}", e);
                GraphError::Database(format!("Query execution failed: {}", e))
            })?;

        if rows.is_empty() {
            return Ok(RankedResults::empty(SearchSource::Graph));
        }

        // Extract and normalize results
        let raw_results: Vec<(i64, f64)> = rows
            .iter()
            .map(|row| {
                let chunk_id: i64 = row.get(0);
                let score: f64 = row.get(1);
                (chunk_id, score)
            })
            .collect();

        let max_score = raw_results.iter().map(|(_, s)| *s).fold(0.0f64, f64::max);

        let ranked_results: Vec<RankedResult> = raw_results
            .iter()
            .enumerate()
            .map(|(idx, (chunk_id, score))| {
                let normalized_score = if max_score > 0.0 {
                    (score / max_score) as f32
                } else {
                    0.0
                };
                RankedResult::new(*chunk_id, normalized_score, idx + 1)
            })
            .collect();

        debug!(
            "Graph executor (chunk-specific) returning {} results",
            ranked_results.len()
        );

        Ok(RankedResults::new(ranked_results, SearchSource::Graph))
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
