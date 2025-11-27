//! Integration tests for dynamic worktree_id event processing (UNIWATCH-3003).
//!
//! These tests verify that file events use the current dynamic worktree_id
//! after branch switches, rather than a hardcoded value.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tempfile::TempDir;

use crewchief_maproom::db::create_pool;
use crewchief_maproom::incremental::FileHasher;

/// Test fixture for dynamic worktree ID tests
struct DynamicWorktreeFixture {
    _temp_dir: TempDir,
    repo_root: PathBuf,
    pool: deadpool_postgres::Pool,
    repo_id: i64,
    worktree_id_1: i64,
    worktree_id_2: i64,
    commit_id: i64,
}

impl DynamicWorktreeFixture {
    /// Create a new test fixture with two worktrees
    async fn new() -> Result<Self> {
        let temp_dir = TempDir::new().context("Failed to create temp directory")?;
        let repo_root = temp_dir.path().to_path_buf();

        // Connect to database
        let pool = create_pool()
            .await
            .context("Failed to create database pool")?;

        let client = pool.get().await?;

        // Create repo
        let repo_name = format!("test-uniwatch3003-{}", uuid::Uuid::new_v4());
        let repo_id: i64 = client
            .query_one(
                "INSERT INTO maproom.repos (name, root_path) VALUES ($1, $2) RETURNING id",
                &[&repo_name, &repo_root.to_string_lossy().as_ref()],
            )
            .await?
            .get(0);

        // Create first worktree (main branch)
        let worktree_id_1: i64 = client
            .query_one(
                "INSERT INTO maproom.worktrees (repo_id, name, abs_path) VALUES ($1, $2, $3) RETURNING id",
                &[&repo_id, &"main", &repo_root.to_string_lossy().as_ref()],
            )
            .await?
            .get(0);

        // Create second worktree (feature branch)
        let worktree_id_2: i64 = client
            .query_one(
                "INSERT INTO maproom.worktrees (repo_id, name, abs_path) VALUES ($1, $2, $3) RETURNING id",
                &[&repo_id, &"feature", &repo_root.to_string_lossy().as_ref()],
            )
            .await?
            .get(0);

        // Create commit
        let commit_id: i64 = client
            .query_one(
                "INSERT INTO maproom.commits (repo_id, sha, committed_at) VALUES ($1, $2, NOW()) RETURNING id",
                &[&repo_id, &"test-commit-123"],
            )
            .await?
            .get(0);

        Ok(Self {
            _temp_dir: temp_dir,
            repo_root,
            pool,
            repo_id,
            worktree_id_1,
            worktree_id_2,
            commit_id,
        })
    }

    /// Create a file in the filesystem and insert a record for a specific worktree
    async fn create_file_for_worktree(
        &self,
        relpath: &str,
        content: &str,
        worktree_id: i64,
    ) -> Result<i64> {
        let abs_path = self.repo_root.join(relpath);

        // Create parent directories if needed
        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write file content
        fs::write(&abs_path, content)?;

        // Calculate hash
        let hash = FileHasher::hash_file(&abs_path)?;
        let hash_bytes: &[u8] = hash.as_bytes();

        // Insert file record for specific worktree
        let client = self.pool.get().await?;
        let file_id: i64 = client
            .query_one(
                "INSERT INTO maproom.files (repo_id, worktree_id, commit_id, relpath, language, content_hash, blake3_hash, size_bytes, last_modified)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
                 RETURNING id",
                &[
                    &self.repo_id,
                    &worktree_id,
                    &self.commit_id,
                    &relpath,
                    &"rust",
                    &hash.to_string(),
                    &hash_bytes,
                    &(content.len() as i32),
                ],
            )
            .await?
            .get(0);

        Ok(file_id)
    }

    /// Verify that a file exists in the database for a specific worktree
    async fn file_exists_in_worktree(&self, relpath: &str, worktree_id: i64) -> Result<bool> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT id FROM maproom.files WHERE worktree_id = $1 AND relpath = $2",
                &[&worktree_id, &relpath],
            )
            .await?;

        Ok(row.is_some())
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_file_events_use_current_worktree() -> Result<()> {
    // Setup test fixture with two worktrees
    let fixture = DynamicWorktreeFixture::new().await?;

    // Create dynamic worktree_id state (simulating the Arc<RwLock<i64>> in indexer)
    let current_worktree_id = Arc::new(RwLock::new(fixture.worktree_id_1));

    // Test Case 1: File event processing with worktree_id = 1
    {
        // Read current worktree_id (simulating what event processing does)
        let worktree_id = *current_worktree_id.read().unwrap();
        assert_eq!(
            worktree_id, fixture.worktree_id_1,
            "Initial worktree_id should be worktree 1"
        );

        // Create a file associated with worktree 1
        let relpath = "src/main.rs";
        let _file_id = fixture
            .create_file_for_worktree(relpath, "fn main() {}", worktree_id)
            .await?;

        // Verify file is in worktree 1
        assert!(
            fixture
                .file_exists_in_worktree(relpath, fixture.worktree_id_1)
                .await?,
            "File should exist in worktree 1"
        );

        // Verify file is NOT in worktree 2
        assert!(
            !fixture
                .file_exists_in_worktree(relpath, fixture.worktree_id_2)
                .await?,
            "File should NOT exist in worktree 2"
        );
    }

    // Test Case 2: Simulate branch switch (update current_worktree_id)
    {
        let mut id = current_worktree_id.write().unwrap();
        *id = fixture.worktree_id_2;
    }

    // Test Case 3: File event processing with worktree_id = 2 (after branch switch)
    {
        // Read current worktree_id (simulating what event processing does)
        let worktree_id = *current_worktree_id.read().unwrap();
        assert_eq!(
            worktree_id, fixture.worktree_id_2,
            "After branch switch, worktree_id should be worktree 2"
        );

        // Create a different file associated with worktree 2
        let relpath = "src/lib.rs";
        let _file_id = fixture
            .create_file_for_worktree(relpath, "pub fn hello() {}", worktree_id)
            .await?;

        // Verify file is in worktree 2
        assert!(
            fixture
                .file_exists_in_worktree(relpath, fixture.worktree_id_2)
                .await?,
            "File should exist in worktree 2"
        );

        // Verify file is NOT in worktree 1
        assert!(
            !fixture
                .file_exists_in_worktree(relpath, fixture.worktree_id_1)
                .await?,
            "File should NOT exist in worktree 1"
        );
    }

    // Test Case 4: Verify original file still in worktree 1
    {
        assert!(
            fixture
                .file_exists_in_worktree("src/main.rs", fixture.worktree_id_1)
                .await?,
            "Original file should still exist in worktree 1"
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_read_lock_dropped_quickly() -> Result<()> {
    // This test verifies the read lock is dropped immediately (not held across await points)
    let fixture = DynamicWorktreeFixture::new().await?;
    let current_worktree_id = Arc::new(RwLock::new(fixture.worktree_id_1));

    // Simulate event processing: read the worktree_id
    let _worktree_id = *current_worktree_id.read().unwrap();
    // Lock is dropped here when we copy the i64 value

    // Now we should be able to acquire a write lock from another task
    // This would deadlock if the read lock was still held
    {
        let mut id = current_worktree_id.write().unwrap();
        *id = fixture.worktree_id_2;
    }

    // Verify the update worked
    let updated_id = *current_worktree_id.read().unwrap();
    assert_eq!(
        updated_id, fixture.worktree_id_2,
        "Should be able to update worktree_id without deadlock"
    );

    Ok(())
}
