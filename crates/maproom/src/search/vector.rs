//! Vector similarity search executor using pgvector.
//!
//! This module implements vector similarity search with three modes:
//! - Code: Prioritize code_embedding similarity
//! - Text: Prioritize text_embedding similarity
//! - Hybrid: Combine both (60% code, 40% text)
//!
//! # Mixed Embedding Support (MPEMBED-4003)
//!
//! The executor supports chunks with different embedding dimensions:
//! - **768-dim**: From Ollama (nomic-embed-text) or Google Vertex AI (stored in *_ollama columns)
//! - **1536-dim**: From OpenAI (text-embedding-3-small, stored in original columns)
//!
//! ## COALESCE Preference Pattern
//!
//! When a chunk has both 768-dim and 1536-dim embeddings, the executor prefers 768-dim:
//!
//! ```sql
//! COALESCE(c.code_embedding_ollama, c.code_embedding)
//! ```
//!
//! This preference order (768 > 1536) is based on:
//! - More recent embeddings (Ollama/Google added later)
//! - Better performance for semantic code search tasks
//! - Lower dimensionality reduces computational cost
//!
//! ## Query Embedding Dimension
//!
//! The SQL query dynamically adapts to the query embedding dimension:
//! - 768-dim query: `$1::vector(768)` - searches all chunks, comparing with COALESCE result
//! - 1536-dim query: `$1::vector(1536)` - searches all chunks, comparing with COALESCE result
//!
//! ## Result Metadata
//!
//! Each `RankedResult` includes an `embedding_dimension` field ("768" or "1536")
//! indicating which embedding type was actually used for the similarity calculation.
//!
//! ## Performance Considerations
//!
//! - COALESCE expressions may not use indexes optimally (PostgreSQL limitation)
//! - Both *_ollama and original embedding columns have ivfflat indexes
//! - PostgreSQL should use the index for the first non-NULL column in COALESCE
//! - Performance regression target: < 5% vs baseline (see MPEMBED-0002)

use crate::embedding::cache::Vector;
use crate::search::executor_types::{RankedResult, RankedResults, SearchSource};
use crate::search::types::SearchMode;
use tokio_postgres::Client;
use tracing::{debug, instrument, warn};

/// Vector similarity search executor.
///
/// Uses pgvector cosine distance operator (<=>) for efficient similarity search.
/// Over-fetches results (limit * 3) to provide more candidates for fusion.
pub struct VectorExecutor;

impl VectorExecutor {
    /// Execute vector similarity search.
    ///
    /// # Parameters
    /// - `client`: Database client
    /// - `query_embedding`: Query vector (1536 dimensions)
    /// - `mode`: Search mode (Code, Text, or Auto for hybrid)
    /// - `repo_id`: Repository ID to filter results
    /// - `worktree_id`: Optional worktree ID for additional filtering
    /// - `limit`: Maximum number of results (will over-fetch by 3x)
    ///
    /// # Returns
    /// RankedResults with similarity scores (0.0-1.0, where 1.0 is identical)
    ///
    /// # Search Modes
    /// - **Code**: Uses code_embedding only
    /// - **Text**: Uses text_embedding only
    /// - **Auto/Hybrid**: Combines code (60%) and text (40%) similarities
    ///
    /// # SQL Query
    /// The query adapts based on mode:
    /// ```sql
    /// -- For hybrid mode:
    /// SELECT
    ///   c.id,
    ///   1 - (c.code_embedding <=> $1::vector) as code_similarity,
    ///   1 - (c.text_embedding <=> $1::vector) as text_similarity,
    ///   (
    ///     (1 - (c.code_embedding <=> $1::vector)) * 0.6 +
    ///     (1 - (c.text_embedding <=> $1::vector)) * 0.4
    ///   ) as combined_score
    /// FROM maproom.chunks c
    /// JOIN maproom.files f ON f.id = c.file_id
    /// WHERE c.code_embedding IS NOT NULL
    ///   AND c.text_embedding IS NOT NULL
    ///   AND f.repo_id = $2
    ///   AND ($3::bigint IS NULL OR f.worktree_id = $3)
    /// ORDER BY combined_score DESC
    /// LIMIT $4;
    /// ```
    #[instrument(skip(client, query_embedding), fields(embedding_dim = query_embedding.len()))]
    pub async fn execute(
        client: &Client,
        query_embedding: &Vector,
        mode: SearchMode,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
    ) -> Result<RankedResults, VectorError> {
        if query_embedding.is_empty() {
            warn!("Empty query embedding provided");
            return Ok(RankedResults::empty(SearchSource::Vector));
        }

        // Over-fetch by 3x for fusion
        let fetch_limit = (limit * 3) as i64;

        debug!(
            "Executing vector search (mode: {:?}, limit: {}, over-fetch: {})",
            mode, limit, fetch_limit
        );

        let (_sql, results) = match mode {
            SearchMode::Code => {
                Self::execute_code_mode(client, query_embedding, repo_id, worktree_id, fetch_limit)
                    .await?
            }
            SearchMode::Text => {
                Self::execute_text_mode(client, query_embedding, repo_id, worktree_id, fetch_limit)
                    .await?
            }
            SearchMode::Auto => {
                Self::execute_hybrid_mode(client, query_embedding, repo_id, worktree_id, fetch_limit)
                    .await?
            }
        };

        debug!(
            "Vector search ({:?} mode) returned {} results",
            mode,
            results.len()
        );

        Ok(RankedResults::new(results, SearchSource::Vector))
    }

    /// Execute code-focused vector search.
    ///
    /// Uses COALESCE to prefer 768-dim code_embedding_ollama over 1536-dim code_embedding.
    /// This allows searching across chunks with different embedding providers.
    async fn execute_code_mode(
        client: &Client,
        query_embedding: &Vector,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: i64,
    ) -> Result<(&'static str, Vec<RankedResult>), VectorError> {
        let dimension = query_embedding.len();

        // Build SQL with dimension from query embedding length
        let sql = format!(
            r#"
            SELECT
              c.id,
              CASE
                WHEN COALESCE(c.code_embedding_ollama, c.code_embedding) IS NOT NULL THEN
                  1 - (COALESCE(c.code_embedding_ollama, c.code_embedding) <=> $1::vector({}))
                ELSE 0
              END as similarity,
              CASE
                WHEN c.code_embedding_ollama IS NOT NULL THEN '768'
                WHEN c.code_embedding IS NOT NULL THEN '1536'
                ELSE NULL
              END as embedding_dimension
            FROM maproom.chunks c
            JOIN maproom.files f ON f.id = c.file_id
            WHERE (c.code_embedding_ollama IS NOT NULL OR c.code_embedding IS NOT NULL)
              AND f.repo_id = $2
              AND ($3::bigint IS NULL OR f.worktree_id = $3)
            ORDER BY similarity DESC
            LIMIT $4
            "#,
            dimension
        );

        let stmt = client.prepare(&sql).await.map_err(|e| {
            warn!("Failed to prepare code vector query: {}", e);
            VectorError::Database(format!("Failed to prepare query: {}", e))
        })?;

        let rows = client
            .query(&stmt, &[&query_embedding, &repo_id, &worktree_id, &limit])
            .await
            .map_err(|e| {
                warn!("Failed to execute code vector query: {}", e);
                VectorError::Database(format!("Query execution failed: {}", e))
            })?;

        let results = Self::process_rows_with_dimension(rows)?;
        Ok(("code", results))
    }

    /// Execute text-focused vector search.
    ///
    /// Uses COALESCE to prefer 768-dim text_embedding_ollama over 1536-dim text_embedding.
    /// This allows searching across chunks with different embedding providers.
    async fn execute_text_mode(
        client: &Client,
        query_embedding: &Vector,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: i64,
    ) -> Result<(&'static str, Vec<RankedResult>), VectorError> {
        let dimension = query_embedding.len();

        // Build SQL with dimension from query embedding length
        let sql = format!(
            r#"
            SELECT
              c.id,
              CASE
                WHEN COALESCE(c.text_embedding_ollama, c.text_embedding) IS NOT NULL THEN
                  1 - (COALESCE(c.text_embedding_ollama, c.text_embedding) <=> $1::vector({}))
                ELSE 0
              END as similarity,
              CASE
                WHEN c.text_embedding_ollama IS NOT NULL THEN '768'
                WHEN c.text_embedding IS NOT NULL THEN '1536'
                ELSE NULL
              END as embedding_dimension
            FROM maproom.chunks c
            JOIN maproom.files f ON f.id = c.file_id
            WHERE (c.text_embedding_ollama IS NOT NULL OR c.text_embedding IS NOT NULL)
              AND f.repo_id = $2
              AND ($3::bigint IS NULL OR f.worktree_id = $3)
            ORDER BY similarity DESC
            LIMIT $4
            "#,
            dimension
        );

        let stmt = client.prepare(&sql).await.map_err(|e| {
            warn!("Failed to prepare text vector query: {}", e);
            VectorError::Database(format!("Failed to prepare query: {}", e))
        })?;

        let rows = client
            .query(&stmt, &[&query_embedding, &repo_id, &worktree_id, &limit])
            .await
            .map_err(|e| {
                warn!("Failed to execute text vector query: {}", e);
                VectorError::Database(format!("Query execution failed: {}", e))
            })?;

        let results = Self::process_rows_with_dimension(rows)?;
        Ok(("text", results))
    }

    /// Execute hybrid vector search (60% code, 40% text).
    ///
    /// Uses COALESCE to prefer 768-dim embeddings over 1536-dim embeddings for both
    /// code and text. This allows searching across chunks with different embedding providers.
    async fn execute_hybrid_mode(
        client: &Client,
        query_embedding: &Vector,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: i64,
    ) -> Result<(&'static str, Vec<RankedResult>), VectorError> {
        let dimension = query_embedding.len();

        // Build SQL with dimension from query embedding length
        let sql = format!(
            r#"
            SELECT
              c.id,
              (
                CASE
                  WHEN COALESCE(c.code_embedding_ollama, c.code_embedding) IS NOT NULL THEN
                    (1 - (COALESCE(c.code_embedding_ollama, c.code_embedding) <=> $1::vector({}))) * 0.6
                  ELSE 0
                END +
                CASE
                  WHEN COALESCE(c.text_embedding_ollama, c.text_embedding) IS NOT NULL THEN
                    (1 - (COALESCE(c.text_embedding_ollama, c.text_embedding) <=> $1::vector({}))) * 0.4
                  ELSE 0
                END
              ) as similarity,
              CASE
                WHEN c.code_embedding_ollama IS NOT NULL OR c.text_embedding_ollama IS NOT NULL THEN '768'
                WHEN c.code_embedding IS NOT NULL OR c.text_embedding IS NOT NULL THEN '1536'
                ELSE NULL
              END as embedding_dimension
            FROM maproom.chunks c
            JOIN maproom.files f ON f.id = c.file_id
            WHERE (c.code_embedding_ollama IS NOT NULL OR c.code_embedding IS NOT NULL
                   OR c.text_embedding_ollama IS NOT NULL OR c.text_embedding IS NOT NULL)
              AND f.repo_id = $2
              AND ($3::bigint IS NULL OR f.worktree_id = $3)
            ORDER BY similarity DESC
            LIMIT $4
            "#,
            dimension, dimension
        );

        let stmt = client.prepare(&sql).await.map_err(|e| {
            warn!("Failed to prepare hybrid vector query: {}", e);
            VectorError::Database(format!("Failed to prepare query: {}", e))
        })?;

        let rows = client
            .query(&stmt, &[&query_embedding, &repo_id, &worktree_id, &limit])
            .await
            .map_err(|e| {
                warn!("Failed to execute hybrid vector query: {}", e);
                VectorError::Database(format!("Query execution failed: {}", e))
            })?;

        let results = Self::process_rows_with_dimension(rows)?;
        Ok(("hybrid", results))
    }

    /// Process query rows into RankedResults with embedding dimension information.
    fn process_rows_with_dimension(
        rows: Vec<tokio_postgres::Row>,
    ) -> Result<Vec<RankedResult>, VectorError> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let results: Vec<RankedResult> = rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let chunk_id: i64 = row.get(0);
                let similarity: f32 = row.get(1);
                let embedding_dimension: Option<String> = row.get(2);
                // Clamp to 0.0-1.0 range (should already be in range, but ensure it)
                let score = similarity.clamp(0.0, 1.0);
                RankedResult::new_with_dimension(chunk_id, score, idx + 1, embedding_dimension)
            })
            .collect();

        Ok(results)
    }
}

/// Errors that can occur during vector search execution.
#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    /// Database query error
    #[error("Database error: {0}")]
    Database(String),

    /// Missing or invalid embeddings
    #[error("Embedding error: {0}")]
    Embedding(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_executor_exists() {
        // Verify the executor type exists
        let _executor = VectorExecutor;
    }

    // Note: Full integration tests with real database are in tests/search/executors_test.rs
}
