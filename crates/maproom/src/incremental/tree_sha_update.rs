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
//! ```ignore
//! use maproom::incremental::incremental_update;
//! use maproom::db::create_pool;
//! use std::path::Path;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let pool = create_pool().await?;
//! let client = pool.get().await?;
//! let worktree_id = 1;
//! let repo_path = Path::new("/workspace");
//!
//! let stats = incremental_update(&*client, worktree_id, "my-repo", "main", repo_path).await?;
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

use crate::db::index_state::UpdateStats;
use crate::db::Store;
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
/// # use maproom::incremental::remove_worktree_from_chunks;
/// # use maproom::db;
/// # async fn example() -> anyhow::Result<()> {
/// let client = db::connect().await?;
/// let affected = remove_worktree_from_chunks(&client, 1, "src/deleted.rs").await?;
/// println!("Removed worktree from {} chunks", affected);
/// # Ok(())
/// # }
/// ```
pub async fn remove_worktree_from_chunks(
    store: &(dyn Store + Send + Sync),
    worktree_id: i64,
    relpath: &str,
) -> Result<i64> {
    store
        .remove_worktree_from_chunks(worktree_id, relpath)
        .await
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
/// # use maproom::incremental::incremental_update;
/// # use maproom::db;
/// # use std::path::Path;
/// # async fn example() -> anyhow::Result<()> {
/// let client = db::connect().await?;
/// let stats =
///     incremental_update(client.as_ref(), 1, "my-repo", "main", Path::new("/workspace")).await?;
///
/// println!("Files: {}, Chunks: {}, Cost: ${:.4}",
///     stats.files_processed,
///     stats.chunks_processed,
///     stats.cost());
/// # Ok(())
/// # }
/// ```
pub async fn incremental_update(
    store: &(dyn Store + Send + Sync),
    worktree_id: i64,
    repo: &str,
    worktree: &str,
    repo_path: &Path,
) -> Result<UpdateStats> {
    // 1. Get current git tree SHA
    let current_tree_sha = get_git_tree_sha(repo_path)
        .with_context(|| format!("Failed to get git tree SHA for {:?}", repo_path))?;

    debug!(
        worktree_id = worktree_id,
        tree_sha = %current_tree_sha,
        "Got current git tree SHA"
    );

    // 2. Get last indexed tree SHA from database
    // Returns "init" if no previous state exists
    let last_indexed = store
        .get_last_indexed_tree(worktree_id)
        .await
        .with_context(|| {
            format!(
                "Failed to get last indexed tree for worktree {}",
                worktree_id
            )
        })?;

    // 3. If tree SHAs match, skip processing (quick path)
    if last_indexed != "init" && last_indexed == current_tree_sha {
        info!(
            worktree_id = worktree_id,
            tree_sha = %current_tree_sha,
            "Tree SHA unchanged, skipping incremental update"
        );
        return Ok(UpdateStats::skipped());
    }

    // R-WATCH-5: a fresh worktree ("init") has no diff base — the first index
    // belongs to `scan`. Return WITHOUT stamping index_state (the old
    // fall-through here stamped tree_sha with zero work, poisoning the next
    // incremental scan into "No changes detected" against an empty index —
    // the R08 bug).
    if last_indexed == "init" {
        debug!(
            worktree_id = worktree_id,
            "No previous tree SHA; first index is scan's job — not stamping index_state"
        );
        return Ok(UpdateStats::skipped());
    }

    debug!(
        worktree_id = worktree_id,
        last_sha = %last_indexed,
        current_sha = %current_tree_sha,
        "Tree SHA changed, processing diff"
    );

    // 4. Find changed files via git diff-tree
    let changes = git_diff_tree(&last_indexed, &current_tree_sha, repo_path).with_context(|| {
        format!(
            "Failed to get diff-tree between {} and {}",
            last_indexed, current_tree_sha
        )
    })?;

    let mut stats = UpdateStats::new();

    // 5. Process the diff for real (R-WATCH-4; the old body was a debug! stub
    // that logged "Incremental update complete" with zero rows written — the
    // R07 phantom). Added/Modified paths are pre-validated (R-WATCH-8: any
    // failure is a hard Err BEFORE any index_state write, because
    // upsert_files silently skips unreadable paths and the R08 invariant
    // would be unenforceable), then submitted in ONE upsert_files call.
    let mut to_upsert: Vec<std::path::PathBuf> = Vec::new();
    for change in &changes {
        let relpath = change.path.to_string_lossy();
        match change.status {
            FileStatus::Added | FileStatus::Modified => {
                let abs = repo_path.join(&change.path);
                if abs.is_dir() {
                    // Legitimately non-indexable; scan would skip it too (OD-4).
                    stats.files_processed += 1;
                    continue;
                }
                if !abs.is_file() {
                    anyhow::bail!(
                        "diff entry {} is missing from the working tree; refusing to advance tree_sha",
                        relpath
                    );
                }
                std::fs::File::open(&abs).with_context(|| {
                    format!(
                        "diff entry {} is unreadable; refusing to advance tree_sha",
                        relpath
                    )
                })?;
                to_upsert.push(change.path.clone());
            }
            FileStatus::Deleted => {
                // R-GC-6: full unmap (junction + orphan GC + FTS + files rows).
                let affected = store
                    .unmap_superseded_file_chunks(worktree_id, &relpath, None)
                    .await?;
                debug!(
                    file = %relpath,
                    junction_rows = affected,
                    "Unmapped worktree from chunks"
                );
                stats.files_processed += 1;
            }
        }
    }

    if !to_upsert.is_empty() {
        let commit = crate::git::get_head_commit(repo_path)
            .with_context(|| format!("Failed to get HEAD commit for {:?}", repo_path))?;
        crate::indexer::upsert_files(store, repo, worktree, repo_path, &commit, &to_upsert)
            .await
            .context("incremental upsert of changed files failed")?;
        stats.files_processed += to_upsert.len() as i32;
    }

    // 6. Update index state with the new tree SHA — reached ONLY when every
    // diff entry was verifiably submitted (upsert Ok) or removed (delete Ok)
    // or is legitimately non-indexable (R-WATCH-5 invariant: tree_sha never
    // advances without verified work).
    store
        .update_index_state(worktree_id, &current_tree_sha, &stats)
        .await
        .with_context(|| format!("Failed to update index state for worktree {}", worktree_id))?;

    info!(
        worktree_id = worktree_id,
        files_processed = stats.files_processed,
        tree_sha = %current_tree_sha,
        "Incremental update complete"
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
