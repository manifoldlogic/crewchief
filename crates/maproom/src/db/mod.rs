//! Database access layer for Maproom.
//!
//! This module provides SQLite database connectivity and query utilities.
//! PostgreSQL support has been removed - SQLite is the only backend.

pub mod cleanup;
pub mod columns;
pub mod connection;
pub mod index_state;
pub mod sqlite;

// Re-export SqliteStore as the primary store type
pub use sqlite::SqliteStore;

// Re-export cleanup types for convenience
pub use cleanup::{
    CleanupError, CleanupReport, StaleWorktree, StaleWorktreeDetector, WorktreeCleaner,
};

// Re-export column selection types for convenience
pub use columns::{select_columns_for_dimension, ColumnSet};

// Re-export index state functions for convenience
pub use index_state::{get_last_indexed_tree, update_index_state, UpdateStats};

// Re-export connection utilities
pub use connection::get_database_url;

use serde::Serialize;

/// Connect to the SQLite database.
///
/// Uses `MAPROOM_DATABASE_URL` env var if set, otherwise defaults to
/// `~/.maproom/maproom.db`.
///
/// # Examples
///
/// ```no_run
/// use crewchief_maproom::db;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let store = db::connect().await?;
///     // Use store for indexing, search, etc.
///     Ok(())
/// }
/// ```
pub async fn connect() -> anyhow::Result<SqliteStore> {
    let url = connection::get_database_url()?;
    SqliteStore::connect(&url).await
}

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
    pub chunk_id: i64,
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

/// Full chunk data for display/context - read-only view of a chunk
#[derive(Debug, Clone, Serialize)]
pub struct ChunkFull {
    pub id: i64,
    pub file_id: i64,
    pub blob_sha: String,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub start_line: i32,
    pub end_line: i32,
    pub preview: String,
    pub file_path: String, // Denormalized from file table
}

/// Lightweight chunk reference for lists/navigation
#[derive(Debug, Clone, Serialize)]
pub struct ChunkSummary {
    pub id: i64,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub start_line: i32,
    pub end_line: i32,
    pub file_path: String,
}

/// Context around a chunk - surrounding and related chunks
#[derive(Debug, Clone, Serialize)]
pub struct ChunkContext {
    pub chunk: ChunkFull,
    pub file_path: String,
    pub surrounding_chunks: Vec<ChunkSummary>, // Chunks before/after by line number
}

/// Repository metadata
#[derive(Debug, Clone, Serialize)]
pub struct RepoInfo {
    pub id: i64,
    pub name: String,
    pub root_path: String,
}

/// Worktree metadata
#[derive(Debug, Clone, Serialize)]
pub struct WorktreeInfo {
    pub id: i64,
    pub repo_id: i64,
    pub name: String,
    pub abs_path: String,
}

/// Result from deleting a single worktree's data
#[derive(Debug, Clone, Default)]
pub struct WorktreeCleanupResult {
    pub chunks_deleted: u64,
    pub files_deleted: u64,
    pub embeddings_deleted: u64,
}

/// Chunk data needed for embedding generation
#[derive(Debug, Clone)]
pub struct ChunkForEmbedding {
    pub id: i64,
    pub blob_sha: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub preview: String,
}
