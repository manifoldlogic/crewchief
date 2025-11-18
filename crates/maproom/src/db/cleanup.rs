//! Stale Worktree Detection Module
//!
//! This module provides functionality to detect worktrees whose abs_path no longer exists on disk.
//! Uses parallel async validation for performance with large numbers of worktrees.
//!
//! IDXCLEAN-1001: Foundational component for cleanup system.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio_postgres::Client;
use tracing::{debug, warn};

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
    client: &'a Client,
}

impl<'a> StaleWorktreeDetector<'a> {
    /// Create a new stale worktree detector
    ///
    /// # Arguments
    /// * `client` - PostgreSQL database client
    ///
    /// # Example
    /// ```no_run
    /// use crewchief_maproom::db::cleanup::StaleWorktreeDetector;
    /// use tokio_postgres::NoTls;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let (client, connection) = tokio_postgres::connect("postgresql://...", NoTls).await?;
    /// tokio::spawn(async move { connection.await });
    ///
    /// let detector = StaleWorktreeDetector::new(&client);
    /// let stale_worktrees = detector.detect_stale_worktrees().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: &'a Client) -> Self {
        Self { client }
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
        let validation_futures = worktrees
            .into_iter()
            .map(|wt| self.validate_worktree(wt));

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
        let rows = self
            .client
            .query(
                "SELECT id, repo_id, name, abs_path FROM maproom.worktrees ORDER BY id",
                &[],
            )
            .await
            .context("Failed to query worktrees from database")?;

        let worktrees = rows
            .into_iter()
            .map(|row| Worktree {
                id: row.get(0),
                repo_id: row.get(1),
                name: row.get(2),
                abs_path: row.get(3),
            })
            .collect();

        Ok(worktrees)
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
    /// Counts chunks where the worktree_id appears in the worktree_ids JSONB array.
    async fn count_chunks_for_worktree(&self, worktree_id: i64) -> Result<i64> {
        let row = self
            .client
            .query_one(
                "SELECT COUNT(*) FROM maproom.chunks WHERE worktree_ids ? $1::text",
                &[&worktree_id.to_string()],
            )
            .await
            .with_context(|| format!("Failed to count chunks for worktree {}", worktree_id))?;

        let count: i64 = row.get(0);
        Ok(count)
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
}
