//! Database access layer for Maproom.
//!
//! SQLite (`rusqlite` + `r2d2`) is the default backend. An optional PostgreSQL +
//! pgvector backend lives in [`postgres`], gated behind the `postgres` Cargo
//! feature; the default build is unchanged and free of `sqlx`/`pgvector`.

pub mod cleanup;
pub mod columns;
pub mod connection;
pub mod index_state;
#[cfg(feature = "postgres")]
pub mod postgres;
pub mod sqlite;
pub mod traits;
pub mod types;

// Re-export SqliteStore as the primary store type
pub use sqlite::SqliteStore;

// Re-export PostgresStore when the backend is compiled in
#[cfg(feature = "postgres")]
pub use postgres::PostgresStore;

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

// Re-export Store trait hierarchy
pub use traits::{
    Store, StoreChunks, StoreCleanup, StoreCore, StoreEmbeddings, StoreEncoding, StoreGraph,
    StoreIndexState, StoreMigration, StoreSearch,
};

// Re-export promoted types from types module
pub use types::{
    ChunkMetadata, EmbeddingRecord, EncodingRunRow, GraphResult, HybridResult, HybridWeights,
    ImportDirection, RankedSearchHit, SemanticRanking,
};

use serde::Serialize;

/// Connect to the SQLite database.
///
/// Uses `MAPROOM_DATABASE_URL` env var if set, otherwise defaults to
/// `~/.maproom/maproom.db`.
///
/// # Examples
///
/// ```no_run
/// use maproom::db;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
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

/// Truncate a string to max_length characters, appending "..." if truncated.
/// Uses chars().count() for character-based length checks (not .len() which is bytes).
/// Uses char_indices() for Unicode-safe truncation -- never splits multi-byte sequences.
pub fn truncate_preview(content: &str, max_length: usize) -> String {
    if content.chars().count() <= max_length {
        content.to_string()
    } else {
        let truncated: String = content.chars().take(max_length).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== truncate_preview() Unit Tests ====================

    #[test]
    fn test_truncate_preview_normal_truncation() {
        let result = truncate_preview("Hello, world! This is a long string", 10);
        assert_eq!(result, "Hello, wor...");
    }

    #[test]
    fn test_truncate_preview_no_truncation() {
        let result = truncate_preview("short", 200);
        assert_eq!(result, "short");
    }

    #[test]
    fn test_truncate_preview_exact_length() {
        let result = truncate_preview("12345", 5);
        assert_eq!(result, "12345");
    }

    #[test]
    fn test_truncate_preview_one_over() {
        let result = truncate_preview("123456", 5);
        assert_eq!(result, "12345...");
    }

    #[test]
    fn test_truncate_preview_empty_string() {
        let result = truncate_preview("", 200);
        assert_eq!(result, "");
    }

    #[test]
    fn test_truncate_preview_max_length_zero() {
        let result = truncate_preview("hello", 0);
        assert_eq!(result, "...");
    }

    #[test]
    fn test_truncate_preview_unicode_2byte() {
        // café is 4 characters, 5 bytes
        let result = truncate_preview("café", 4);
        assert_eq!(result, "café");
    }

    #[test]
    fn test_truncate_preview_unicode_cjk() {
        // 你好世界 is 4 characters, 12 bytes
        let result = truncate_preview("你好世界", 2);
        assert_eq!(result, "你好...");
    }

    #[test]
    fn test_truncate_preview_emoji() {
        // 5 single-codepoint emojis
        let result = truncate_preview("👍👎😀😁😂", 3);
        assert_eq!(result, "👍👎😀...");
    }

    #[test]
    fn test_truncate_preview_single_char() {
        let result = truncate_preview("a", 1);
        assert_eq!(result, "a");
    }

    #[test]
    fn test_truncate_preview_whitespace() {
        let result = truncate_preview("hello   ", 5);
        assert_eq!(result, "hello...");
    }

    #[test]
    fn test_truncate_preview_default_length() {
        // Create a 201-character string
        let long_string = "a".repeat(201);
        let result = truncate_preview(&long_string, 200);
        // Should be 200 'a's + "..."
        assert_eq!(result.len(), 203); // 200 + 3 for "..."
        assert!(result.ends_with("..."));
        assert_eq!(result.chars().filter(|c| *c == 'a').count(), 200);
    }

    // ==================== SearchHit Serialization Tests ====================

    #[test]
    fn test_searchhit_preview_none_omitted() {
        let hit = SearchHit {
            chunk_id: 1,
            score: 0.95,
            file_relpath: "test.rs".to_string(),
            symbol_name: Some("test_func".to_string()),
            kind: "func".to_string(),
            start_line: 10,
            end_line: 20,
            base_score: None,
            kind_mult: None,
            exact_mult: None,
            preview: None,
        };

        let json = serde_json::to_string(&hit).unwrap();
        // Verify "preview" key is NOT in JSON (field omitted, not null)
        assert!(!json.contains("\"preview\""));
    }

    #[test]
    fn test_searchhit_preview_some_included() {
        let hit = SearchHit {
            chunk_id: 1,
            score: 0.95,
            file_relpath: "test.rs".to_string(),
            symbol_name: Some("test_func".to_string()),
            kind: "func".to_string(),
            start_line: 10,
            end_line: 20,
            base_score: None,
            kind_mult: None,
            exact_mult: None,
            preview: Some("sample text".to_string()),
        };

        let json = serde_json::to_string(&hit).unwrap();
        // Verify "preview" key IS in JSON with correct value
        assert!(json.contains("\"preview\":\"sample text\""));
    }

    #[test]
    fn test_searchhit_preview_empty_string_included() {
        let hit = SearchHit {
            chunk_id: 1,
            score: 0.95,
            file_relpath: "test.rs".to_string(),
            symbol_name: Some("test_func".to_string()),
            kind: "func".to_string(),
            start_line: 10,
            end_line: 20,
            base_score: None,
            kind_mult: None,
            exact_mult: None,
            preview: Some("".to_string()),
        };

        let json = serde_json::to_string(&hit).unwrap();
        // Verify "preview" key IS in JSON (empty string is NOT omitted, only None is omitted)
        assert!(json.contains("\"preview\":\"\""));
    }
}
