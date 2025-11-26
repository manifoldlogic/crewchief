//! Database access layer for Maproom.
//!
//! This module provides database connectivity, connection pooling, and query utilities.

pub mod cleanup;
pub mod columns;
pub mod connection;
pub mod index_state;
pub mod materialized_views;
pub mod pool;
pub mod queries;
pub mod postgres;
#[cfg(feature = "sqlite")]
pub mod sqlite;
pub mod factory;

// Re-export cleanup types for convenience
pub use cleanup::{
    CleanupError, CleanupReport, StaleWorktree, StaleWorktreeDetector, WorktreeCleaner,
};

// Re-export column selection types for convenience
pub use columns::{select_columns_for_dimension, ColumnSet};

// Re-export index state functions for convenience
pub use index_state::{get_last_indexed_tree, update_index_state, UpdateStats};

// Re-export pool types for convenience
pub use pool::{create_pool, pool_stats, PgPool, PoolStats};

// Re-export query functions for convenience
// TODO: Phase this out in favor of VectorStore trait
pub use queries::*;

use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashSet;

// ... (rest of the file - Trait definitions) ...
/// Record for inserting/updating a file
#[derive(Debug, Clone)]
pub struct FileRecord {
    pub repo_id: i64,
    pub worktree_id: i64,
    pub commit_id: i64,
    pub relpath: String,
    pub language: Option<String>,
    pub content_hash: String,
    pub size_bytes: i32,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
}

/// Record for inserting a chunk
#[derive(Debug, Clone)]
pub struct ChunkRecord {
    pub file_id: i64,
    pub blob_sha: String,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub start_line: i32,
    pub end_line: i32,
    pub preview: String,
    pub ts_doc_text: String,
    pub recency_score: f32,
    pub churn_score: f32,
    pub metadata: Option<serde_json::Value>,
    pub worktree_id: i64,
}

/// Search result returned by vector/FTS search
#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub score: f64,
    pub file_relpath: String,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub start_line: i32,
    pub end_line: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind_mult: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_mult: Option<f64>,
}

/// Common interface for Vector/FTS storage backends.
#[async_trait]
pub trait VectorStore: Send + Sync {
    // --- Repository & Worktree ---
    async fn get_or_create_repo(&self, name: &str, root_path: &str) -> anyhow::Result<i64>;
    async fn get_or_create_worktree(
        &self,
        repo_id: i64,
        name: &str,
        abs_path: &str,
    ) -> anyhow::Result<i64>;
    async fn get_or_create_commit(
        &self,
        repo_id: i64,
        sha: &str,
        committed_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> anyhow::Result<i64>;

    // --- Indexing ---
    async fn upsert_file(&self, file: &FileRecord) -> anyhow::Result<i64>;
    async fn insert_chunk(&self, chunk: &ChunkRecord) -> anyhow::Result<i64>;
    async fn insert_chunks_batch(&self, chunks: &[ChunkRecord]) -> anyhow::Result<Vec<i64>>;
    async fn insert_chunk_edge(
        &self,
        src_chunk_id: i64,
        dst_chunk_id: i64,
        edge_type: &str,
    ) -> anyhow::Result<()>;

    // --- Embeddings ---
    async fn upsert_embeddings(
        &self,
        chunk_id: i64,
        code_embedding: Option<&[f32]>,
        text_embedding: Option<&[f32]>,
        dimension: usize,
    ) -> anyhow::Result<()>;
    
    async fn batch_upsert_embeddings(
        &self,
        embeddings: &[(i64, Option<Vec<f32>>, Option<Vec<f32>>)],
        dimension: usize,
    ) -> anyhow::Result<()>;

    // --- Search ---
    async fn search_chunks_fts(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        k: i64,
        debug: bool,
    ) -> anyhow::Result<Vec<SearchHit>>;

    /// Vector similarity search using embedding
    async fn search_chunks_vector(
        &self,
        repo: &str,
        worktree: Option<&str>,
        embedding: &[f32],
        k: i64,
        debug: bool,
    ) -> anyhow::Result<Vec<SearchHit>>;

    // --- Lookup ---
    async fn find_chunk_by_symbol(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        symbol_name: &str,
        relpath: Option<&str>,
    ) -> anyhow::Result<Option<i64>>;

    // --- Migrations ---
    async fn migrate(&self) -> anyhow::Result<()>;
    
    // --- Stats/Maintenance ---
    async fn get_applied_migrations(&self) -> anyhow::Result<HashSet<i32>>;
}
