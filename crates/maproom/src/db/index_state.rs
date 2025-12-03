//! Worktree index state management functions.
//!
//! This module provides functions to track the indexing state of each worktree,
//! storing the last indexed git tree SHA and associated metrics. This enables
//! incremental indexing by comparing the current tree SHA against the last
//! indexed state.
//!
//! # Example
//!
//! ```no_run
//! use crewchief_maproom::db::{self, UpdateStats};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let store = db::connect().await?;
//! let worktree_id = 1;
//!
//! // Check if worktree has been indexed
//! let last_tree = db::get_last_indexed_tree(&store, worktree_id).await?;
//! if last_tree == "init" {
//!     println!("First-time indexing required");
//! }
//!
//! // After indexing, update the state
//! let stats = UpdateStats {
//!     files_processed: 100,
//!     chunks_processed: 500,
//!     embeddings_generated: 500,
//! };
//! db::update_index_state(&store, worktree_id, "abc123...", &stats).await?;
//! # Ok(())
//! # }
//! ```

use crate::db::SqliteStore;
use anyhow::Result;

/// Metrics for tracking indexing progress and costs.
///
/// These metrics help track resource usage and provide visibility into
/// the indexing process, particularly useful for cost estimation when
/// using paid embedding providers.
#[derive(Debug, Clone)]
pub struct UpdateStats {
    /// Number of files processed during indexing
    pub files_processed: i32,
    /// Number of code chunks created/updated
    pub chunks_processed: i32,
    /// Number of embeddings generated (impacts API costs)
    pub embeddings_generated: i32,
}

/// Retrieves the last indexed git tree SHA for a given worktree.
///
/// Returns `"init"` if the worktree has never been indexed, signaling
/// that a full indexing pass is required.
///
/// # Arguments
///
/// * `store` - SQLite database store
/// * `worktree_id` - ID of the worktree to query
///
/// # Returns
///
/// * `Ok(String)` - The last indexed tree SHA, or "init" if never indexed
/// * `Err` - Database query error
///
/// # Example
///
/// ```no_run
/// # use crewchief_maproom::db;
/// # async fn example() -> anyhow::Result<()> {
/// # let store = db::connect().await?;
/// let tree_sha = db::get_last_indexed_tree(&store, 1).await?;
/// match tree_sha.as_str() {
///     "init" => println!("Never indexed, full scan required"),
///     sha => println!("Last indexed at tree {}", sha),
/// }
/// # Ok(())
/// # }
/// ```
pub async fn get_last_indexed_tree(store: &SqliteStore, worktree_id: i64) -> Result<String> {
    store
        .run(move |conn| {
            use rusqlite::{params, OptionalExtension};

            let result: Option<String> = conn
                .query_row(
                    "SELECT tree_sha FROM index_state WHERE worktree_id = ?1",
                    params![worktree_id],
                    |row| row.get(0),
                )
                .optional()?;

            Ok(result.unwrap_or_else(|| "init".to_string()))
        })
        .await
}

/// Updates the index state for a worktree, inserting new or updating existing records.
///
/// Uses SQLite's `INSERT ... ON CONFLICT DO UPDATE` (upsert) pattern to handle
/// both first-time indexing (INSERT) and subsequent updates (UPDATE) with a
/// single query.
///
/// The `last_indexed` timestamp is automatically set to the current time on every update.
///
/// # Arguments
///
/// * `store` - SQLite database store
/// * `worktree_id` - ID of the worktree being indexed
/// * `tree_sha` - Current git tree SHA (40-character hex string)
/// * `stats` - Indexing metrics for progress tracking
///
/// # Returns
///
/// * `Ok(())` - State successfully updated
/// * `Err` - Database query error
///
/// # Example
///
/// ```no_run
/// # use crewchief_maproom::db::{self, UpdateStats};
/// # async fn example() -> anyhow::Result<()> {
/// # let store = db::connect().await?;
/// let stats = UpdateStats {
///     files_processed: 150,
///     chunks_processed: 750,
///     embeddings_generated: 750,
/// };
/// db::update_index_state(&store, 1, "a1b2c3d4...", &stats).await?;
/// println!("Index state updated");
/// # Ok(())
/// # }
/// ```
pub async fn update_index_state(
    store: &SqliteStore,
    worktree_id: i64,
    tree_sha: &str,
    stats: &UpdateStats,
) -> Result<()> {
    let tree_sha = tree_sha.to_string();
    let chunks_processed = stats.chunks_processed;
    let embeddings_generated = stats.embeddings_generated;

    store
        .run(move |conn| {
            use rusqlite::params;

            conn.execute(
                r#"
            INSERT INTO index_state
              (worktree_id, tree_sha, last_indexed, chunks_processed, embeddings_generated)
            VALUES (?1, ?2, datetime('now'), ?3, ?4)
            ON CONFLICT (worktree_id) DO UPDATE
            SET
              tree_sha = excluded.tree_sha,
              last_indexed = datetime('now'),
              chunks_processed = excluded.chunks_processed,
              embeddings_generated = excluded.embeddings_generated
            "#,
                params![
                    worktree_id,
                    tree_sha,
                    chunks_processed,
                    embeddings_generated,
                ],
            )?;

            Ok(())
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These are unit tests that verify the function signatures and logic structure.
    // Full integration tests that require a live PostgreSQL database are in BRANCHX-1006.

    #[test]
    fn test_update_stats_creation() {
        let stats = UpdateStats {
            files_processed: 100,
            chunks_processed: 500,
            embeddings_generated: 500,
        };

        assert_eq!(stats.files_processed, 100);
        assert_eq!(stats.chunks_processed, 500);
        assert_eq!(stats.embeddings_generated, 500);
    }

    #[test]
    fn test_update_stats_clone() {
        let stats = UpdateStats {
            files_processed: 50,
            chunks_processed: 250,
            embeddings_generated: 250,
        };

        let cloned = stats.clone();
        assert_eq!(cloned.files_processed, 50);
        assert_eq!(cloned.chunks_processed, 250);
        assert_eq!(cloned.embeddings_generated, 250);
    }

    #[test]
    fn test_update_stats_debug() {
        let stats = UpdateStats {
            files_processed: 10,
            chunks_processed: 50,
            embeddings_generated: 50,
        };

        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("files_processed"));
        assert!(debug_str.contains("chunks_processed"));
        assert!(debug_str.contains("embeddings_generated"));
    }
}
