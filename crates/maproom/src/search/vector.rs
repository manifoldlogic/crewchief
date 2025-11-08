//! Vector similarity search executor using pgvector.
//!
//! This module implements vector similarity search with three modes:
//! - Code: Uses embedding from code_embeddings table
//! - Text: Uses embedding from code_embeddings table
//! - Hybrid: Uses embedding from code_embeddings table (same as Code mode)
//!
//! # Content-Addressed Embedding Storage (BLOBSHA-3001)
//!
//! The executor uses the code_embeddings table for deduplicated embedding storage:
//! - Embeddings are indexed by blob_sha (content hash)
//! - Multiple chunks with identical content share one embedding
//! - Typical deduplication savings: 70-90% of embedding storage
//!
//! ## JOIN Pattern
//!
//! All vector searches JOIN chunks with code_embeddings on blob_sha:
//!
//! ```sql
//! SELECT c.id, e.embedding
//! FROM maproom.chunks c
//! JOIN maproom.code_embeddings e ON c.blob_sha = e.blob_sha
//! WHERE e.embedding <=> $1 < 0.5
//! ```
//!
//! ## Query Embedding Dimension
//!
//! The SQL query dynamically adapts to the query embedding dimension:
//! - 768-dim query: `$1::vector(768)` - searches all chunks with embeddings
//! - 1536-dim query: `$1::vector(1536)` - searches all chunks with embeddings
//!
//! ## Result Metadata
//!
//! Each `RankedResult` includes an `embedding_dimension` field ("1536")
//! indicating which embedding type was used for the similarity calculation.
//!
//! ## Performance Considerations
//!
//! - HNSW index on code_embeddings enables fast similarity search
//! - JOIN uses indexed foreign key (chunks.blob_sha → code_embeddings.blob_sha)
//! - Smaller index footprint due to deduplication
//! - Query performance equal or better than direct embedding access

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
    /// - `query_embedding`: Query vector (typically 1536 dimensions)
    /// - `mode`: Search mode (Code, Text, or Auto - all use same embedding)
    /// - `repo_id`: Repository ID to filter results
    /// - `worktree_id`: Optional worktree ID for additional filtering
    /// - `limit`: Maximum number of results (will over-fetch by 3x)
    ///
    /// # Returns
    /// RankedResults with similarity scores (0.0-1.0, where 1.0 is identical)
    ///
    /// # Search Modes
    /// - **Code**: Uses embedding from code_embeddings table
    /// - **Text**: Uses embedding from code_embeddings table
    /// - **Auto/Hybrid**: Uses embedding from code_embeddings table
    ///
    /// # SQL Query Pattern
    /// All modes JOIN with code_embeddings table:
    /// ```sql
    /// SELECT
    ///   c.id,
    ///   1 - (e.embedding <=> $1::vector) as similarity
    /// FROM maproom.chunks c
    /// JOIN maproom.code_embeddings e ON c.blob_sha = e.blob_sha
    /// JOIN maproom.files f ON f.id = c.file_id
    /// WHERE e.embedding IS NOT NULL
    ///   AND f.repo_id = $2
    ///   AND ($3::bigint IS NULL OR f.worktree_id = $3)
    /// ORDER BY similarity DESC
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
    /// Joins chunks with code_embeddings table to access deduplicated embeddings.
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
        // JOIN with code_embeddings table for content-addressed embedding storage
        let sql = format!(
            r#"
            SELECT
              c.id,
              CASE
                WHEN e.embedding IS NOT NULL THEN
                  1 - (e.embedding <=> $1::vector({}))
                ELSE 0
              END as similarity,
              CASE
                WHEN e.embedding IS NOT NULL THEN '1536'
                ELSE NULL
              END as embedding_dimension
            FROM maproom.chunks c
            JOIN maproom.code_embeddings e ON c.blob_sha = e.blob_sha
            JOIN maproom.files f ON f.id = c.file_id
            WHERE e.embedding IS NOT NULL
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
    /// Joins chunks with code_embeddings table to access deduplicated embeddings.
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
        // JOIN with code_embeddings table for content-addressed embedding storage
        let sql = format!(
            r#"
            SELECT
              c.id,
              CASE
                WHEN e.embedding IS NOT NULL THEN
                  1 - (e.embedding <=> $1::vector({}))
                ELSE 0
              END as similarity,
              CASE
                WHEN e.embedding IS NOT NULL THEN '1536'
                ELSE NULL
              END as embedding_dimension
            FROM maproom.chunks c
            JOIN maproom.code_embeddings e ON c.blob_sha = e.blob_sha
            JOIN maproom.files f ON f.id = c.file_id
            WHERE e.embedding IS NOT NULL
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
    /// Joins chunks with code_embeddings table to access deduplicated embeddings.
    /// For hybrid mode, we use the same embedding for both code and text similarity
    /// since code_embeddings stores a single embedding per unique content blob.
    async fn execute_hybrid_mode(
        client: &Client,
        query_embedding: &Vector,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: i64,
    ) -> Result<(&'static str, Vec<RankedResult>), VectorError> {
        let dimension = query_embedding.len();

        // Build SQL with dimension from query embedding length
        // JOIN with code_embeddings table for content-addressed embedding storage
        // Use same embedding for both code and text components in hybrid scoring
        let sql = format!(
            r#"
            SELECT
              c.id,
              CASE
                WHEN e.embedding IS NOT NULL THEN
                  1 - (e.embedding <=> $1::vector({}))
                ELSE 0
              END as similarity,
              CASE
                WHEN e.embedding IS NOT NULL THEN '1536'
                ELSE NULL
              END as embedding_dimension
            FROM maproom.chunks c
            JOIN maproom.code_embeddings e ON c.blob_sha = e.blob_sha
            JOIN maproom.files f ON f.id = c.file_id
            WHERE e.embedding IS NOT NULL
              AND f.repo_id = $2
              AND ($3::bigint IS NULL OR f.worktree_id = $3)
            ORDER BY similarity DESC
            LIMIT $4
            "#,
            dimension
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
