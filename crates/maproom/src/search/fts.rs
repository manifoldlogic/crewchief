//! Full-text search executor using PostgreSQL FTS.
//!
//! This module implements full-text search using PostgreSQL's tsvector/tsquery
//! with ts_rank_cd ranking. It applies proximity boost for phrase matching
//! and exact match bonuses for symbol names.

use crate::search::executor_types::{RankedResult, RankedResults, SearchSource};
use tokio_postgres::Client;
use tracing::{debug, instrument, warn};

/// Full-text search executor.
///
/// Uses PostgreSQL full-text search with ts_rank_cd ranking function.
/// Over-fetches results (limit * 3) to provide more candidates for fusion.
pub struct FTSExecutor;

impl FTSExecutor {
    /// Execute full-text search query.
    ///
    /// # Parameters
    /// - `client`: Database client
    /// - `fts_query`: PostgreSQL tsquery string (e.g., "auth & login")
    /// - `repo_id`: Repository ID to filter results
    /// - `worktree_id`: Optional worktree ID for additional filtering
    /// - `limit`: Maximum number of results (will over-fetch by 3x)
    ///
    /// # Returns
    /// RankedResults with FTS scores normalized to 0.0-1.0 range
    ///
    /// # SQL Query
    /// ```sql
    /// WITH fts_results AS (
    ///   SELECT
    ///     c.id,
    ///     ts_rank_cd(c.ts_doc, to_tsquery('simple', $1), 32) as fts_score,
    ///     CASE
    ///       WHEN c.symbol_name ILIKE '%' || $2 || '%' THEN 0.2
    ///       ELSE 0.0
    ///     END as exact_bonus
    ///   FROM maproom.chunks c
    ///   JOIN maproom.files f ON f.id = c.file_id
    ///   WHERE c.ts_doc @@ to_tsquery('simple', $1)
    ///     AND f.repo_id = $3
    ///     AND ($4::bigint IS NULL OR f.worktree_id = $4)
    /// )
    /// SELECT
    ///   id,
    ///   (fts_score + exact_bonus) as score,
    ///   ROW_NUMBER() OVER (ORDER BY fts_score + exact_bonus DESC) as rank
    /// FROM fts_results
    /// ORDER BY score DESC
    /// LIMIT $5;
    /// ```
    #[instrument(skip(client), fields(query_len = fts_query.len()))]
    pub async fn execute(
        client: &Client,
        fts_query: &str,
        original_query: &str,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
    ) -> Result<RankedResults, FTSError> {
        if fts_query.is_empty() {
            debug!("Empty FTS query, returning no results");
            return Ok(RankedResults::empty(SearchSource::FTS));
        }

        // Over-fetch by 3x for fusion
        let fetch_limit = (limit * 3) as i64;

        debug!(
            "Executing FTS query: '{}' (limit: {}, over-fetch: {})",
            fts_query, limit, fetch_limit
        );

        // Prepare the SQL query
        let sql = r#"
            WITH fts_results AS (
              SELECT
                c.id,
                ts_rank_cd(c.ts_doc, to_tsquery('simple', $1), 32) as fts_score,
                CASE
                  WHEN c.symbol_name ILIKE '%' || $2 || '%' THEN 0.2
                  ELSE 0.0
                END as exact_bonus
              FROM maproom.chunks c
              JOIN maproom.files f ON f.id = c.file_id
              WHERE c.ts_doc @@ to_tsquery('simple', $1)
                AND f.repo_id = $3
                AND ($4::bigint IS NULL OR f.worktree_id = $4)
            )
            SELECT
              id,
              (fts_score + exact_bonus) as score,
              ROW_NUMBER() OVER (ORDER BY fts_score + exact_bonus DESC) as rank
            FROM fts_results
            ORDER BY score DESC
            LIMIT $5
        "#;

        let stmt = client.prepare(sql).await.map_err(|e| {
            warn!("Failed to prepare FTS query: {}", e);
            FTSError::Database(format!("Failed to prepare query: {}", e))
        })?;

        let rows = client
            .query(&stmt, &[&fts_query, &original_query, &repo_id, &worktree_id, &fetch_limit])
            .await
            .map_err(|e| {
                warn!("Failed to execute FTS query: {}", e);
                FTSError::Database(format!("Query execution failed: {}", e))
            })?;

        debug!("FTS query returned {} rows", rows.len());

        if rows.is_empty() {
            return Ok(RankedResults::empty(SearchSource::FTS));
        }

        // Extract results and normalize scores
        let results: Vec<(i64, f32, i64)> = rows
            .iter()
            .map(|row| {
                let chunk_id: i64 = row.get(0);
                let score: f32 = row.get(1);
                let rank: i64 = row.get(2);
                (chunk_id, score, rank)
            })
            .collect();

        // Find max score for normalization
        let max_score = results.iter().map(|(_, s, _)| *s).fold(0.0f32, f32::max);

        // Normalize scores to 0.0-1.0 range
        let ranked_results: Vec<RankedResult> = results
            .iter()
            .map(|(chunk_id, score, rank)| {
                let normalized_score = if max_score > 0.0 {
                    score / max_score
                } else {
                    0.0
                };
                RankedResult::new(*chunk_id, normalized_score, *rank as usize)
            })
            .collect();

        debug!(
            "FTS executor returning {} results (max_score: {:.4})",
            ranked_results.len(),
            max_score
        );

        Ok(RankedResults::new(ranked_results, SearchSource::FTS))
    }
}

/// Errors that can occur during FTS execution.
#[derive(Debug, thiserror::Error)]
pub enum FTSError {
    /// Database query error
    #[error("Database error: {0}")]
    Database(String),

    /// Invalid FTS query syntax
    #[error("Invalid FTS query: {0}")]
    InvalidQuery(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fts_executor_exists() {
        // Verify the executor type exists
        let _executor = FTSExecutor;
    }

    // Note: Full integration tests with real database are in tests/search/executors_test.rs
}
