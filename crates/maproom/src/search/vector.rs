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

use crate::db::SqliteStore;
use crate::embedding::cache::Vector;
use crate::search::executor_types::{RankedResult, RankedResults, SearchSource};
use crate::search::types::SearchMode;
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
    #[instrument(skip(store, query_embedding), fields(embedding_dim = query_embedding.len()))]
    pub async fn execute(
        store: &SqliteStore,
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

        // TODO(IDXABS-2003): This is a placeholder implementation.
        // Vector search using sqlite-vec needs to be integrated with the search pipeline.
        // The SqliteStore has search_chunks_vector() but returns SearchHit, not RankedResult.
        // This needs adapter logic to convert between types.
        // See ticket IDXABS-4001 for search functionality updates.

        warn!("Vector search is not fully implemented for SqliteStore backend");
        Ok(RankedResults::empty(SearchSource::Vector))
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
