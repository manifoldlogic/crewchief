//! Stale Worktree Detection Module
//!
//! This module provides functionality to detect worktrees whose abs_path no longer exists on disk.
//! Uses parallel async validation for performance with large numbers of worktrees.
//!
//! IDXCLEAN-1001: Foundational component for cleanup system.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::db::SqliteStore;
use tracing::{debug, warn};

/// Errors specific to cleanup operations
#[derive(Error, Debug)]
pub enum CleanupError {
    /// Database transaction failed during cleanup
    #[error("Database transaction failed during cleanup: {0}")]
    TransactionFailed(String),

    /// Failed to validate worktree path on disk
    #[error("Failed to validate worktree path {path}: {source}")]
    ValidationFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Worktree not found in database
    #[error("Worktree {id} not found in database")]
    WorktreeNotFound { id: i64 },

    /// Database connection failed
    #[error("Database connection failed: {0}")]
    ConnectionFailed(String),

    /// Cleanup operation cancelled by user
    #[error("Cleanup operation cancelled by user")]
    Cancelled,
}

/// Information about a worktree from the database
#[derive(Debug, Clone)]
struct Worktree {
    id: i64,
    repo_id: i64,
    name: String,
    abs_path: String,
}

/// Stale worktree detection result with metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StaleWorktree {
    /// Worktree ID from database
    pub id: i64,
    /// Repository ID this worktree belongs to
    pub repo_id: i64,
    /// Worktree name (typically branch name)
    pub name: String,
    /// Absolute path that no longer exists
    pub abs_path: String,
    /// Whether the path exists on disk
    pub exists: bool,
    /// Number of chunks indexed for this worktree
    pub chunk_count: i64,
}

/// Detector for identifying stale worktrees
pub struct StaleWorktreeDetector<'a> {
    store: &'a SqliteStore,
}

impl<'a> StaleWorktreeDetector<'a> {
    /// Create a new stale worktree detector
    ///
    /// # Arguments
    /// * `store` - SQLite database store
    ///
    /// # Example
    /// ```no_run
    /// use crewchief_maproom::db::cleanup::StaleWorktreeDetector;
    /// use crewchief_maproom::db;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let store = db::connect().await?;
    ///
    /// let detector = StaleWorktreeDetector::new(&store);
    /// let stale_worktrees = detector.detect_stale_worktrees().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(store: &'a SqliteStore) -> Self {
        Self { store }
    }

    /// Detect all stale worktrees in the database
    ///
    /// Queries all worktrees from the database and validates their paths in parallel.
    /// Returns only worktrees whose abs_path does not exist on disk.
    ///
    /// # Performance
    /// Uses parallel async validation to achieve <1 second for 100 worktrees.
    ///
    /// # Error Handling
    /// - Permission denied errors are treated as "exists" (conservative approach)
    /// - Individual validation failures are logged but don't stop the process
    ///
    /// # Returns
    /// Vector of stale worktrees with metadata including chunk counts
    pub async fn detect_stale_worktrees(&self) -> Result<Vec<StaleWorktree>> {
        debug!("Starting stale worktree detection");

        // Query all worktrees from database
        let worktrees = self.query_all_worktrees().await?;
        debug!("Found {} worktrees in database", worktrees.len());

        // Validate all worktrees in parallel
        let validation_futures = worktrees.into_iter().map(|wt| self.validate_worktree(wt));

        let results = futures::future::join_all(validation_futures).await;

        // Filter to only stale worktrees (where exists=false)
        let stale_worktrees: Vec<StaleWorktree> = results
            .into_iter()
            .filter_map(|r| match r {
                Ok(wt) if !wt.exists => Some(wt),
                Ok(_) => None,
                Err(e) => {
                    warn!("Failed to validate worktree: {}", e);
                    None
                }
            })
            .collect();

        debug!(
            "Detection complete: found {} stale worktrees",
            stale_worktrees.len()
        );

        Ok(stale_worktrees)
    }

    /// Query all worktrees from the database
    async fn query_all_worktrees(&self) -> Result<Vec<Worktree>> {
        self.store.run(move |conn| {
            let mut stmt = conn.prepare("SELECT id, repo_id, name, abs_path FROM worktrees ORDER BY id")?;
            let worktrees = stmt.query_map([], |row| {
                Ok(Worktree {
                    id: row.get(0)?,
                    repo_id: row.get(1)?,
                    name: row.get(2)?,
                    abs_path: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
            Ok(worktrees)
        }).await
    }

    /// Validate a single worktree and return its status
    ///
    /// Checks if the worktree's abs_path exists on disk and counts chunks.
    /// Permission denied errors are treated as "exists" to avoid false positives.
    async fn validate_worktree(&self, wt: Worktree) -> Result<StaleWorktree> {
        // Check if path exists on disk
        let exists = match tokio::fs::try_exists(&wt.abs_path).await {
            Ok(exists) => exists,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                warn!(
                    "Permission denied checking path '{}' for worktree '{}' (id={}), treating as exists",
                    wt.abs_path, wt.name, wt.id
                );
                true // Conservative: treat permission denied as exists
            }
            Err(e) => {
                return Err(e).with_context(|| {
                    format!(
                        "Failed to check existence of path '{}' for worktree '{}'",
                        wt.abs_path, wt.name
                    )
                })?;
            }
        };

        // Count chunks for this worktree
        let chunk_count = self.count_chunks_for_worktree(wt.id).await?;

        Ok(StaleWorktree {
            id: wt.id,
            repo_id: wt.repo_id,
            name: wt.name,
            abs_path: wt.abs_path,
            exists,
            chunk_count,
        })
    }

    /// Count chunks for a specific worktree
    ///
    /// Counts chunks via the chunk_worktrees junction table.
    async fn count_chunks_for_worktree(&self, worktree_id: i64) -> Result<i64> {
        self.store.get_worktree_chunk_count(worktree_id).await
    }
}

/// Report of cleanup operations with statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupReport {
    /// Total number of stale worktrees found
    pub total_stale: usize,
    /// Number of worktrees successfully deleted
    pub deleted_count: usize,
    /// Total number of chunks cleaned (deleted or had worktree removed)
    pub chunks_cleaned: i64,
    /// Number of deletions that failed
    pub failed_count: usize,
    /// IDs of successfully deleted worktrees
    pub deleted_ids: Vec<i64>,
    /// Failed deletions with error messages
    pub failed_deletions: Vec<(i64, String)>,
}

impl CleanupReport {
    /// Success rate as percentage (0.0-1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_stale == 0 {
            return 1.0;
        }
        self.deleted_count as f64 / self.total_stale as f64
    }

    /// Check if any deletions failed
    ///
    /// Returns true if failed_count > 0
    pub fn has_failures(&self) -> bool {
        self.failed_count > 0
    }
}

/// Cleaner for safely deleting stale worktrees
pub struct WorktreeCleaner<'a> {
    store: &'a SqliteStore,
    dry_run: bool,
}

impl<'a> WorktreeCleaner<'a> {
    /// Create a new worktree cleaner
    ///
    /// # Arguments
    /// * `store` - SQLite database store
    /// * `dry_run` - If true, no actual deletions are performed
    ///
    /// # Example
    /// ```no_run
    /// use crewchief_maproom::db::cleanup::{WorktreeCleaner, StaleWorktreeDetector};
    /// use crewchief_maproom::db;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let store = db::connect().await?;
    ///
    /// // Detect stale worktrees
    /// let detector = StaleWorktreeDetector::new(&store);
    /// let stale_worktrees = detector.detect_stale_worktrees().await?;
    ///
    /// // Clean up stale worktrees
    /// let cleaner = WorktreeCleaner::new(&store, false);
    /// let report = cleaner.cleanup_stale_worktrees(stale_worktrees).await?;
    ///
    /// println!("Deleted {} worktrees, cleaned {} chunks",
    ///     report.deleted_count, report.chunks_cleaned);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(store: &'a SqliteStore, dry_run: bool) -> Self {
        Self { store, dry_run }
    }

    /// Clean up stale worktrees
    ///
    /// Deletes stale worktrees from the database. In SQLite, this uses the chunk_worktrees
    /// junction table to track multi-worktree chunks. All deletions occur within a single
    /// transaction for atomicity.
    ///
    /// # Algorithm
    /// For each stale worktree:
    /// 1. Remove entries from chunk_worktrees junction table
    /// 2. Garbage collect chunks with no remaining worktree associations
    /// 3. Delete the worktree record
    ///
    /// # Safety
    /// - Multi-worktree chunks are preserved (only junction entries removed)
    /// - Single-worktree chunks are garbage collected
    /// - All operations in transaction (atomic commit)
    /// - Partial failures are collected but don't abort transaction
    ///
    /// # Arguments
    /// * `stale` - Vector of stale worktrees to delete
    ///
    /// # Returns
    /// CleanupReport with statistics and any failures
    pub async fn cleanup_stale_worktrees(
        &self,
        stale: Vec<StaleWorktree>,
    ) -> Result<CleanupReport> {
        if self.dry_run {
            return Ok(self.create_dry_run_report(&stale));
        }

        let store = self.store.clone();
        store.run(move |conn| {
            let tx = conn.transaction()?;

            let mut deleted_ids = Vec::new();
            let mut chunks_cleaned = 0i64;
            let mut failed_deletions = Vec::new();

            // Process each worktree deletion within the same transaction
            for wt in &stale {
                match Self::delete_worktree_tx(&tx, wt.id) {
                    Ok(cleaned) => {
                        deleted_ids.push(wt.id);
                        chunks_cleaned += cleaned;
                        tracing::info!(
                            worktree_id = wt.id,
                            name = %wt.name,
                            abs_path = %wt.abs_path,
                            chunks_cleaned = cleaned,
                            "Deleted stale worktree"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            worktree_id = wt.id,
                            name = %wt.name,
                            error = %e,
                            "Failed to delete stale worktree"
                        );
                        failed_deletions.push((wt.id, e.to_string()));
                    }
                }
            }

            // Commit all deletions at once
            tx.commit()
                .context("Failed to commit cleanup transaction")?;

            Ok(CleanupReport {
                total_stale: stale.len(),
                deleted_count: deleted_ids.len(),
                chunks_cleaned,
                failed_count: failed_deletions.len(),
                deleted_ids,
                failed_deletions,
            })
        }).await
    }

    /// Delete a single worktree within a transaction
    ///
    /// In SQLite, we use the chunk_worktrees junction table instead of JSONB arrays.
    ///
    /// # Steps
    /// 1. Remove entries from chunk_worktrees junction table for this worktree
    /// 2. Garbage collect chunks with no remaining worktree associations
    /// 3. Delete worktree record
    ///
    /// # Arguments
    /// * `tx` - Transaction to execute deletions within
    /// * `worktree_id` - ID of worktree to delete
    ///
    /// # Returns
    /// Number of chunks garbage collected (had no remaining worktree associations)
    fn delete_worktree_tx(
        tx: &rusqlite::Transaction<'_>,
        worktree_id: i64,
    ) -> Result<i64> {
        use rusqlite::params;

        // Step 1: Remove entries from chunk_worktrees junction table
        tx.execute(
            "DELETE FROM chunk_worktrees WHERE worktree_id = ?1",
            params![worktree_id],
        )
        .with_context(|| format!("Failed to remove worktree {} from chunk_worktrees", worktree_id))?;

        // Step 2: Garbage collection - delete chunks with no remaining worktree associations
        // These are chunks that belonged ONLY to the deleted worktree
        let deleted = tx.execute(
            r#"
            DELETE FROM chunks
            WHERE id NOT IN (SELECT DISTINCT chunk_id FROM chunk_worktrees)
            "#,
            params![],
        )
        .with_context(|| {
            format!(
                "Failed to garbage collect chunks for worktree {}",
                worktree_id
            )
        })?;

        // Step 3: Delete worktree record
        // This also cascades to worktree_index_state via ON DELETE CASCADE
        tx.execute(
            "DELETE FROM worktrees WHERE id = ?1",
            params![worktree_id],
        )
        .with_context(|| format!("Failed to delete worktree record {}", worktree_id))?;

        Ok(deleted as i64)
    }

    /// Create dry-run report without making any changes
    fn create_dry_run_report(&self, stale: &[StaleWorktree]) -> CleanupReport {
        tracing::info!(
            stale_count = stale.len(),
            "Dry-run mode: would delete {} worktrees",
            stale.len()
        );

        CleanupReport {
            total_stale: stale.len(),
            deleted_count: 0,
            chunks_cleaned: 0,
            failed_count: 0,
            deleted_ids: Vec::new(),
            failed_deletions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stale_worktree_serialization() {
        let stale = StaleWorktree {
            id: 1,
            repo_id: 1,
            name: "test-branch".to_string(),
            abs_path: "/tmp/test-repo/.crewchief/test-branch".to_string(),
            exists: false,
            chunk_count: 42,
        };

        // Test JSON serialization
        let json = serde_json::to_string(&stale).unwrap();
        assert!(json.contains("test-branch"));
        assert!(json.contains("\"exists\":false"));
        assert!(json.contains("\"chunk_count\":42"));

        // Test deserialization
        let deserialized: StaleWorktree = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, stale);
    }

    #[test]
    fn test_stale_worktree_equality() {
        let stale1 = StaleWorktree {
            id: 1,
            repo_id: 1,
            name: "branch1".to_string(),
            abs_path: "/path/to/branch1".to_string(),
            exists: false,
            chunk_count: 10,
        };

        let stale2 = StaleWorktree {
            id: 1,
            repo_id: 1,
            name: "branch1".to_string(),
            abs_path: "/path/to/branch1".to_string(),
            exists: false,
            chunk_count: 10,
        };

        let stale3 = StaleWorktree {
            id: 2,
            repo_id: 1,
            name: "branch2".to_string(),
            abs_path: "/path/to/branch2".to_string(),
            exists: true,
            chunk_count: 20,
        };

        assert_eq!(stale1, stale2);
        assert_ne!(stale1, stale3);
    }

    #[test]
    fn test_cleanup_report_success_rate() {
        let report = CleanupReport {
            total_stale: 10,
            deleted_count: 8,
            chunks_cleaned: 150,
            failed_count: 2,
            deleted_ids: vec![1, 2, 3, 4, 5, 6, 7, 8],
            failed_deletions: vec![(9, "error1".to_string()), (10, "error2".to_string())],
        };

        assert_eq!(report.success_rate(), 0.8);
    }

    #[test]
    fn test_cleanup_report_success_rate_empty() {
        let report = CleanupReport {
            total_stale: 0,
            deleted_count: 0,
            chunks_cleaned: 0,
            failed_count: 0,
            deleted_ids: Vec::new(),
            failed_deletions: Vec::new(),
        };

        assert_eq!(report.success_rate(), 1.0);
    }

    #[test]
    fn test_cleanup_report_serialization() {
        let report = CleanupReport {
            total_stale: 5,
            deleted_count: 3,
            chunks_cleaned: 42,
            failed_count: 2,
            deleted_ids: vec![1, 2, 3],
            failed_deletions: vec![
                (4, "Database connection lost".to_string()),
                (5, "Permission denied".to_string()),
            ],
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"total_stale\":5"));
        assert!(json.contains("\"deleted_count\":3"));
        assert!(json.contains("\"chunks_cleaned\":42"));

        let deserialized: CleanupReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_stale, 5);
        assert_eq!(deserialized.deleted_count, 3);
        assert_eq!(deserialized.chunks_cleaned, 42);
    }

    #[test]
    fn test_cleanup_error_messages() {
        let err = CleanupError::WorktreeNotFound { id: 42 };
        let msg = err.to_string();
        assert!(
            msg.contains("Worktree 42 not found"),
            "Error message should contain worktree ID"
        );
        assert!(
            msg.contains("database"),
            "Error message should mention database"
        );

        let err = CleanupError::Cancelled;
        let msg = err.to_string();
        assert!(
            msg.contains("cancelled"),
            "Error message should contain 'cancelled'"
        );
        assert!(msg.contains("user"), "Error message should mention user");
    }

    #[test]
    fn test_has_failures_method() {
        let report_no_failures = CleanupReport {
            total_stale: 10,
            deleted_count: 10,
            chunks_cleaned: 100,
            failed_count: 0,
            deleted_ids: vec![],
            failed_deletions: vec![],
        };
        assert!(
            !report_no_failures.has_failures(),
            "Should return false when no failures"
        );

        let report_with_failures = CleanupReport {
            total_stale: 10,
            deleted_count: 8,
            chunks_cleaned: 80,
            failed_count: 2,
            deleted_ids: vec![],
            failed_deletions: vec![],
        };
        assert!(
            report_with_failures.has_failures(),
            "Should return true when failures exist"
        );
    }
}
