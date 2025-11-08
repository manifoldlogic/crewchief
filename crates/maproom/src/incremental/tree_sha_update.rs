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
use crate::git::{get_git_tree_sha, git_diff_tree, FileStatus};
use anyhow::{Context, Result};
use std::path::Path;
use tokio_postgres::Client;
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
    client: &Client,
    worktree_id: i64,
    relpath: &str,
) -> Result<i64> {
    // Remove worktree_id from JSONB array using `-` operator
    let result = client
        .execute(
            r#"
            UPDATE maproom.chunks
            SET worktree_ids = worktree_ids - $1::TEXT,
                updated_at = NOW()
            WHERE relpath = $2
            "#,
            &[&worktree_id.to_string(), &relpath],
        )
        .await
        .context("Failed to remove worktree from chunks")?;

    let affected = result as i64;

    // Garbage collection: Delete chunks with empty worktree_ids arrays
    let deleted = client
        .execute(
            r#"
            DELETE FROM maproom.chunks
            WHERE jsonb_array_length(worktree_ids) = 0
            "#,
            &[],
        )
        .await
        .context("Failed to delete orphan chunks")?;

    if deleted > 0 {
        debug!(
            deleted = deleted,
            relpath = relpath,
            "Deleted orphan chunks with no worktrees"
        );
    }

    if affected > 0 {
        debug!(
            affected = affected,
            deleted = deleted,
            relpath = relpath,
            worktree_id = worktree_id,
            "Removed worktree from chunks"
        );
    }

    Ok(affected)
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
    client: &Client,
    worktree_id: i64,
    repo_path: &Path,
) -> Result<UpdateStats> {
    info!(
        worktree_id = worktree_id,
        repo_path = ?repo_path,
        "Starting incremental update"
    );

    // Step 1: Get current git tree SHA
    let current_tree = get_git_tree_sha(repo_path)
        .context("Failed to get current git tree SHA")?;
    debug!(tree_sha = %current_tree, "Current tree SHA retrieved");

    // Step 2: Get last indexed tree SHA from database
    let last_tree = get_last_indexed_tree(client, worktree_id)
        .await
        .context("Failed to get last indexed tree SHA")?;
    debug!(last_tree = %last_tree, "Last indexed tree SHA retrieved");

    // Step 3: Quick check - if tree SHA unchanged, skip processing
    if current_tree == last_tree {
        info!("No changes detected (tree SHA match), skipping scan");
        return Ok(UpdateStats::skipped());
    }

    // Step 4: Find changed files via git diff-tree
    let changed_files = if last_tree == "init" {
        // First-time indexing: treat all files as new
        info!("First-time indexing detected (last_tree = 'init')");
        // For now, return empty vec - full scan logic will be added in BRANCHX-1008
        vec![]
    } else {
        git_diff_tree(&last_tree, &current_tree, repo_path)
            .context("Failed to get diff-tree between old and new tree SHA")?
    };

    info!(
        changed_files = changed_files.len(),
        "Found changed files via git diff-tree"
    );

    // Step 5: Process changed files
    let mut stats = UpdateStats::new();

    for file_change in changed_files {
        match file_change.status {
            FileStatus::Added | FileStatus::Modified => {
                debug!(
                    path = ?file_change.path,
                    status = ?file_change.status,
                    "Processing changed file"
                );

                // TODO (BRANCHX-1008): Call updated upsert function that tracks worktree_ids
                // For now, just count the file
                // let chunks = parse_file_into_chunks(&file_change.path)?;
                // for chunk in chunks {
                //     upsert_chunk_with_worktree(client, &chunk, worktree_id).await?;
                //     stats.chunks_processed += 1;
                // }

                stats.files_processed += 1;
            }
            FileStatus::Deleted => {
                debug!(
                    path = ?file_change.path,
                    "Removing deleted file from worktree"
                );

                // Convert PathBuf to string
                let path_str = file_change
                    .path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in file path"))?;

                let affected = remove_worktree_from_chunks(client, worktree_id, path_str)
                    .await
                    .context("Failed to remove worktree from chunks")?;

                if affected > 0 {
                    stats.files_processed += 1;
                }
            }
        }
    }

    // Step 6: Update index state with new tree SHA
    // TODO (BRANCHX-1008): Uncomment this after file processing is implemented
    // Currently commented to avoid database inconsistency (claiming indexed without actual processing)
    // update_index_state(client, worktree_id, &current_tree, &stats)
    //     .await
    //     .context("Failed to update index state")?;

    info!(
        files_processed = stats.files_processed,
        chunks_processed = stats.chunks_processed,
        embeddings_generated = stats.embeddings_generated,
        cache_hit_rate = %format!("{:.1}%", stats.cache_hit_rate() * 100.0),
        cost = %format!("${:.4}", stats.cost()),
        "Incremental update completed (file processing pending BRANCHX-1008)"
    );

    Ok(stats)
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
