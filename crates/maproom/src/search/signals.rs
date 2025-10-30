//! Temporal signal integration for recency and churn scores.
//!
//! This module retrieves recency and churn scores from the chunks table
//! and combines them into a unified signal score.

use crate::search::executor_types::{RankedResult, RankedResults, SearchSource};
use tokio_postgres::Client;
use tracing::{debug, instrument, warn};

/// Signal weights for combining recency and churn.
#[derive(Debug, Clone, Copy)]
pub struct SignalWeights {
    /// Weight for recency score (default: 0.3)
    pub recency: f32,
    /// Weight for churn score (default: 0.2)
    pub churn: f32,
}

impl Default for SignalWeights {
    fn default() -> Self {
        Self {
            recency: 0.3,
            churn: 0.2,
        }
    }
}

/// Temporal signal executor.
///
/// Retrieves and combines recency_score and churn_score from chunks table.
/// Returns all chunks with signal scores (no limit) for flexible fusion.
pub struct SignalExecutor;

impl SignalExecutor {
    /// Execute signal query with default weights.
    ///
    /// # Parameters
    /// - `client`: Database client
    /// - `repo_id`: Repository ID to filter results
    /// - `worktree_id`: Optional worktree ID for additional filtering
    ///
    /// # Returns
    /// RankedResults with combined signal scores (0.0-1.0 range)
    #[instrument(skip(client))]
    pub async fn execute(
        client: &Client,
        repo_id: i64,
        worktree_id: Option<i64>,
    ) -> Result<RankedResults, SignalError> {
        Self::execute_with_weights(client, repo_id, worktree_id, SignalWeights::default()).await
    }

    /// Execute signal query with custom weights.
    ///
    /// # Parameters
    /// - `client`: Database client
    /// - `repo_id`: Repository ID to filter results
    /// - `worktree_id`: Optional worktree ID for additional filtering
    /// - `weights`: Custom weights for recency and churn
    ///
    /// # SQL Query
    /// ```sql
    /// SELECT
    ///   c.id,
    ///   c.recency_score,
    ///   c.churn_score,
    ///   (c.recency_score * $3 + c.churn_score * $4) as combined_signal
    /// FROM maproom.chunks c
    /// JOIN maproom.files f ON f.id = c.file_id
    /// WHERE f.repo_id = $1
    ///   AND ($2::bigint IS NULL OR f.worktree_id = $2)
    /// ORDER BY combined_signal DESC;
    /// ```
    #[instrument(skip(client))]
    pub async fn execute_with_weights(
        client: &Client,
        repo_id: i64,
        worktree_id: Option<i64>,
        weights: SignalWeights,
    ) -> Result<RankedResults, SignalError> {
        debug!(
            "Executing signal query (recency: {}, churn: {})",
            weights.recency, weights.churn
        );

        let sql = r#"
            SELECT
              c.id,
              c.recency_score,
              c.churn_score,
              (c.recency_score * $3 + c.churn_score * $4) as combined_signal
            FROM maproom.chunks c
            JOIN maproom.files f ON f.id = c.file_id
            WHERE f.repo_id = $1
              AND ($2::bigint IS NULL OR f.worktree_id = $2)
            ORDER BY combined_signal DESC
        "#;

        let stmt = client.prepare(sql).await.map_err(|e| {
            warn!("Failed to prepare signal query: {}", e);
            SignalError::Database(format!("Failed to prepare query: {}", e))
        })?;

        let rows = client
            .query(&stmt, &[&repo_id, &worktree_id, &weights.recency, &weights.churn])
            .await
            .map_err(|e| {
                warn!("Failed to execute signal query: {}", e);
                SignalError::Database(format!("Query execution failed: {}", e))
            })?;

        debug!("Signal query returned {} rows", rows.len());

        if rows.is_empty() {
            return Ok(RankedResults::empty(SearchSource::Signals));
        }

        // Extract results and find max score for normalization
        let raw_results: Vec<(i64, f32)> = rows
            .iter()
            .map(|row| {
                let chunk_id: i64 = row.get(0);
                let combined_signal: f32 = row.get(3);
                (chunk_id, combined_signal)
            })
            .collect();

        let max_score = raw_results.iter().map(|(_, s)| *s).fold(0.0f32, f32::max);

        // Normalize scores to 0.0-1.0 range
        let ranked_results: Vec<RankedResult> = raw_results
            .iter()
            .enumerate()
            .map(|(idx, (chunk_id, score))| {
                let normalized_score = if max_score > 0.0 {
                    score / max_score
                } else {
                    0.0
                };
                RankedResult::new(*chunk_id, normalized_score, idx + 1)
            })
            .collect();

        debug!(
            "Signal executor returning {} results (max_score: {:.4})",
            ranked_results.len(),
            max_score
        );

        Ok(RankedResults::new(ranked_results, SearchSource::Signals))
    }

    /// Execute signal query for specific chunk IDs.
    ///
    /// This variant calculates signal scores only for a given set of chunks,
    /// useful when combining with other search results.
    #[instrument(skip(client, chunk_ids), fields(chunk_count = chunk_ids.len()))]
    pub async fn execute_for_chunks(
        client: &Client,
        chunk_ids: &[i64],
        repo_id: i64,
        worktree_id: Option<i64>,
        weights: SignalWeights,
    ) -> Result<RankedResults, SignalError> {
        if chunk_ids.is_empty() {
            return Ok(RankedResults::empty(SearchSource::Signals));
        }

        debug!(
            "Executing signal query for {} specific chunks",
            chunk_ids.len()
        );

        let sql = r#"
            SELECT
              c.id,
              c.recency_score,
              c.churn_score,
              (c.recency_score * $4 + c.churn_score * $5) as combined_signal
            FROM maproom.chunks c
            JOIN maproom.files f ON f.id = c.file_id
            WHERE c.id = ANY($1)
              AND f.repo_id = $2
              AND ($3::bigint IS NULL OR f.worktree_id = $3)
            ORDER BY combined_signal DESC
        "#;

        let stmt = client.prepare(sql).await.map_err(|e| {
            warn!("Failed to prepare chunk-specific signal query: {}", e);
            SignalError::Database(format!("Failed to prepare query: {}", e))
        })?;

        let rows = client
            .query(
                &stmt,
                &[
                    &chunk_ids,
                    &repo_id,
                    &worktree_id,
                    &weights.recency,
                    &weights.churn,
                ],
            )
            .await
            .map_err(|e| {
                warn!("Failed to execute chunk-specific signal query: {}", e);
                SignalError::Database(format!("Query execution failed: {}", e))
            })?;

        if rows.is_empty() {
            return Ok(RankedResults::empty(SearchSource::Signals));
        }

        // Extract and normalize results
        let raw_results: Vec<(i64, f32)> = rows
            .iter()
            .map(|row| {
                let chunk_id: i64 = row.get(0);
                let combined_signal: f32 = row.get(3);
                (chunk_id, combined_signal)
            })
            .collect();

        let max_score = raw_results.iter().map(|(_, s)| *s).fold(0.0f32, f32::max);

        let ranked_results: Vec<RankedResult> = raw_results
            .iter()
            .enumerate()
            .map(|(idx, (chunk_id, score))| {
                let normalized_score = if max_score > 0.0 {
                    score / max_score
                } else {
                    0.0
                };
                RankedResult::new(*chunk_id, normalized_score, idx + 1)
            })
            .collect();

        debug!(
            "Signal executor (chunk-specific) returning {} results",
            ranked_results.len()
        );

        Ok(RankedResults::new(ranked_results, SearchSource::Signals))
    }
}

/// Errors that can occur during signal query execution.
#[derive(Debug, thiserror::Error)]
pub enum SignalError {
    /// Database query error
    #[error("Database error: {0}")]
    Database(String),

    /// Invalid signal weights
    #[error("Invalid signal weights: {0}")]
    InvalidWeights(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_weights_default() {
        let weights = SignalWeights::default();
        assert_eq!(weights.recency, 0.3);
        assert_eq!(weights.churn, 0.2);
    }

    #[test]
    fn test_signal_executor_exists() {
        // Verify the executor type exists
        let _executor = SignalExecutor;
    }

    // Note: Full integration tests with real database are in tests/search/executors_test.rs
}
