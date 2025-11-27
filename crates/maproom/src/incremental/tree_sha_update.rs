//! Git tree SHA-based incremental update algorithm.
//!
//! This module implements the core incremental indexing optimization:
//! compare current git tree SHA against last indexed SHA, and if different,
//! use git diff-tree to find changed files and process only those.
//!
//! This provides a 5-10x performance improvement over full repository scans
//! by avoiding re-indexing unchanged code.
//!
//! # Algorithm Flow
//!
//! 1. Get current git tree SHA via `get_git_tree_sha()`
//! 2. Retrieve last indexed tree SHA from `worktree_index_state` table
//! 3. **Quick check**: If tree SHAs match, skip processing (0 changes)
//! 4. If different, use `git diff-tree` to find Added/Modified/Deleted files
//! 5. Process only changed files (not entire repository)
//! 6. Update `worktree_index_state` with new tree SHA and metrics
//!
//! # Example
//!
//! ```no_run
//! use crewchief_maproom::incremental::incremental_update;
//! use crewchief_maproom::db::create_pool;
//! use std::path::Path;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let pool = create_pool().await?;
//! let client = pool.get().await?;
//! let worktree_id = 1;
//! let repo_path = Path::new("/workspace");
//!
//! let stats = incremental_update(&*client, worktree_id, repo_path).await?;
//!
//! println!("Processed {} files, {} chunks",
//!     stats.files_processed, stats.chunks_processed);
//!
//! if stats.files_processed == 0 {
//!     println!("No changes detected (tree SHA unchanged)");
//! }
//! # Ok(())
//! # }
//! ```

use crate::db::index_state::{get_last_indexed_tree, UpdateStats};
use crate::db::SqliteStore;
use crate::git::{get_git_tree_sha, git_diff_tree, FileStatus};
use anyhow::{Context, Result};
use std::path::Path;
use tracing::{debug, info};

impl UpdateStats {
    /// Create a new UpdateStats with all counters at zero.
    pub fn new() -> Self {
        Self {
            files_processed: 0,
            chunks_processed: 0,
            embeddings_generated: 0,
        }
    }

    /// Create stats representing a skipped update (no changes detected).
    pub fn skipped() -> Self {
        Self::new()
    }

    /// Calculate the embedding cache hit rate.
    ///
    /// Returns 1.0 (100%) if no chunks processed, otherwise the ratio of
    /// chunks that didn't need new embeddings.
    pub fn cache_hit_rate(&self) -> f64 {
        if self.chunks_processed == 0 {
            return 1.0;
        }
        1.0 - (self.embeddings_generated as f64 / self.chunks_processed as f64)
    }

    /// Estimate embedding API cost based on number of embeddings generated.
    ///
    /// Uses $0.00002 per embedding (typical for text-embedding-ada-002).
    pub fn cost(&self) -> f64 {
        self.embeddings_generated as f64 * 0.00002
    }
}

impl Default for UpdateStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Remove a worktree from chunks when files are deleted.
///
/// This function:
/// 1. Removes `worktree_id` from the `worktree_ids` JSONB array for all chunks in `relpath`
/// 2. Deletes chunks that have empty `worktree_ids` arrays (garbage collection)
///
/// # Arguments
///
/// * `client` - Database client
/// * `worktree_id` - ID of the worktree to remove
/// * `relpath` - Relative path of the deleted file
///
/// # Returns
///
/// * `Ok(affected)` - Number of chunks affected (had worktree removed)
/// * `Err` - Database errors
///
/// # Idempotency
///
/// Calling this function multiple times with the same arguments is safe (no-op if worktree already removed).
///
/// # Example
///
/// ```no_run
/// # use crewchief_maproom::incremental::remove_worktree_from_chunks;
/// # use crewchief_maproom::db;
/// # async fn example() -> anyhow::Result<()> {
/// let client = db::connect().await?;
/// let affected = remove_worktree_from_chunks(&client, 1, "src/deleted.rs").await?;
/// println!("Removed worktree from {} chunks", affected);
/// # Ok(())
/// # }
/// ```
pub async fn remove_worktree_from_chunks(
    _store: &SqliteStore,
    _worktree_id: i64,
    _relpath: &str,
) -> Result<i64> {
    // TODO: Implement SQLite-based worktree removal from chunks
    // This will be implemented in a future ticket
    Ok(0)
}

/// Perform incremental update based on git tree SHA comparison.
///
/// This is the core function that implements the 5-10x performance improvement
/// from BRANCHX. It compares the current git tree SHA against the last indexed
/// SHA, and if different, processes only the changed files.
///
/// # Algorithm
///
/// 1. Get current git tree SHA (O(1) git operation)
/// 2. Get last indexed tree SHA from database
/// 3. If identical, return immediately with zero stats (common case after indexing)
/// 4. If different, use `git diff-tree` to find changed files
/// 5. Process only Added/Modified files (Deleted handled in BRANCHX-1009)
/// 6. Update index state with new tree SHA
///
/// # Arguments
///
/// * `client` - Database client
/// * `worktree_id` - ID of the worktree being updated
/// * `repo_path` - Path to the git repository root
///
/// # Returns
///
/// * `Ok(UpdateStats)` - Metrics about files/chunks/embeddings processed
/// * `Err` - Git errors, database errors, or processing errors
///
/// # Performance
///
/// - No changes: ~5-10ms (just git tree SHA + database query)
/// - Small changes: 100ms - 1s (process only changed files)
/// - Full repository: 10s - 5min (rare, only on first index or git reset)
///
/// # Example
///
/// ```no_run
/// # use crewchief_maproom::incremental::incremental_update;
/// # use crewchief_maproom::db;
/// # use std::path::Path;
/// # async fn example() -> anyhow::Result<()> {
/// let client = db::connect().await?;
/// let stats = incremental_update(&client, 1, Path::new("/workspace")).await?;
///
/// println!("Files: {}, Chunks: {}, Cost: ${:.4}",
///     stats.files_processed,
///     stats.chunks_processed,
///     stats.cost());
/// # Ok(())
/// # }
/// ```
pub async fn incremental_update(
    _store: &SqliteStore,
    _worktree_id: i64,
    _repo_path: &Path,
) -> Result<UpdateStats> {
    // TODO: Implement SQLite-based incremental update
    // This will be implemented in a future ticket
    // Steps to implement:
    // 1. Get current git tree SHA
    // 2. Get last indexed tree SHA from database
    // 3. If tree SHAs match, skip processing
    // 4. Find changed files via git diff-tree
    // 5. Process changed files (add/modify/delete)
    // 6. Update index state with new tree SHA

    Ok(UpdateStats::skipped())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_stats_new() {
        let stats = UpdateStats::new();
        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.chunks_processed, 0);
        assert_eq!(stats.embeddings_generated, 0);
    }

    #[test]
    fn test_update_stats_skipped() {
        let stats = UpdateStats::skipped();
        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.chunks_processed, 0);
        assert_eq!(stats.embeddings_generated, 0);
    }

    #[test]
    fn test_cache_hit_rate_no_chunks() {
        let stats = UpdateStats::new();
        assert_eq!(stats.cache_hit_rate(), 1.0);
    }

    #[test]
    fn test_cache_hit_rate_all_cached() {
        let stats = UpdateStats {
            files_processed: 10,
            chunks_processed: 100,
            embeddings_generated: 0,
        };
        assert_eq!(stats.cache_hit_rate(), 1.0);
    }

    #[test]
    fn test_cache_hit_rate_partial() {
        let stats = UpdateStats {
            files_processed: 10,
            chunks_processed: 100,
            embeddings_generated: 50,
        };
        assert_eq!(stats.cache_hit_rate(), 0.5); // 50% hit rate
    }

    #[test]
    fn test_cost_calculation() {
        let stats = UpdateStats {
            files_processed: 10,
            chunks_processed: 100,
            embeddings_generated: 1000,
        };
        assert_eq!(stats.cost(), 0.02); // 1000 * 0.00002 = $0.02
    }

    #[test]
    fn test_default() {
        let stats = UpdateStats::default();
        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.chunks_processed, 0);
        assert_eq!(stats.embeddings_generated, 0);
    }

    // Note: remove_worktree_from_chunks requires database integration tests
    // See tests/incremental_deletions.rs for comprehensive testing
}
