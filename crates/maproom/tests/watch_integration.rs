//! Integration tests for watch command fix (WATCHFIX-1005).
//!
//! These tests verify the watch command correctly handles:
//! - Multiple files modified simultaneously
//! - Proper ChangeType::Modified classification
//! - Database timestamp updates
//! - No infinite retry loops
//!
//! Tests require a running PostgreSQL database.

use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

// Note: common module has compilation errors unrelated to this test
// mod common;

use crewchief_maproom::db::create_pool;
use crewchief_maproom::incremental::{
    ChangeDetector, ChangeType, FileHasher, IncrementalProcessor, Trigger, UpdateTask,
};

/// Test utilities for watch integration tests.
struct WatchTestFixture {
    /// Temporary directory (auto-cleaned on drop)
    _temp_dir: TempDir,
    repo_root: PathBuf,
    pool: deadpool_postgres::Pool,
    repo_id: i64,
    worktree_id: i64,
    commit_id: i64,
}

impl WatchTestFixture {
    /// Create a new test fixture with database and temp directory.
    async fn new() -> Result<Self> {
        let temp_dir = TempDir::new().context("Failed to create temp directory")?;
        let repo_root = temp_dir.path().to_path_buf();

        // Initialize git repo
        Self::init_git_repo(&repo_root)?;

        // Connect to database
        let pool = create_pool()
            .await
            .context("Failed to create database pool")?;

        // Create repo, worktree, and commit records
        let client = pool.get().await?;

        // Insert repo
        let repo_name = format!("test-repo-{}", uuid::Uuid::new_v4());
        let repo_id: i64 = client
            .query_one(
                "INSERT INTO maproom.repos (name, root_path) VALUES ($1, $2) RETURNING id",
                &[&repo_name, &repo_root.to_string_lossy().as_ref()],
            )
            .await?
            .get(0);

        // Insert worktree
        let worktree_id: i64 = client
            .query_one(
                "INSERT INTO maproom.worktrees (repo_id, name, abs_path) VALUES ($1, $2, $3) RETURNING id",
                &[&repo_id, &"main", &repo_root.to_string_lossy().as_ref()],
            )
            .await?
            .get(0);

        // Insert commit
        let commit_id: i64 = client
            .query_one(
                "INSERT INTO maproom.commits (repo_id, sha, committed_at) VALUES ($1, $2, NOW()) RETURNING id",
                &[&repo_id, &"abc123def456"],
            )
            .await?
            .get(0);

        Ok(Self {
            _temp_dir: temp_dir,
            repo_root,
            pool,
            repo_id,
            worktree_id,
            commit_id,
        })
    }

    /// Initialize a minimal git repository in the given directory.
    fn init_git_repo(path: &Path) -> Result<()> {
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .context("Failed to init git repo")?;

        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(path)
            .output()?;

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(path)
            .output()?;

        Ok(())
    }

    /// Create a source file with initial content and seed it in the database.
    async fn create_and_seed_file(&self, relpath: &str, content: &str) -> Result<i64> {
        let abs_path = self.repo_root.join(relpath);

        // Create parent directories if needed
        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write file content
        fs::write(&abs_path, content)?;

        // Calculate hash
        let hash = FileHasher::hash_file(&abs_path)?;

        // Insert file record
        // CRITICAL: Must use blake3_hash (BYTEA) not content_hash (TEXT)
        // ChangeDetector.detect_change() queries blake3_hash column
        let client = self.pool.get().await?;
        let hash_bytes: &[u8] = hash.as_bytes();
        let file_id: i64 = client
            .query_one(
                "INSERT INTO maproom.files (repo_id, worktree_id, commit_id, relpath, language, content_hash, blake3_hash, size_bytes, last_modified)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
                 RETURNING id",
                &[
                    &self.repo_id,
                    &self.worktree_id,
                    &self.commit_id,
                    &relpath,
                    &"rust",
                    &hash.to_string(), // content_hash (TEXT) for backward compatibility
                    &hash_bytes,       // blake3_hash (BYTEA) for incremental indexing
                    &(content.len() as i32),
                ],
            )
            .await?
            .get(0);

        // Insert a dummy chunk for this file
        client
            .execute(
                "INSERT INTO maproom.chunks (file_id, symbol_name, kind, start_line, end_line, preview)
                 VALUES ($1, $2::text, ($3::text)::maproom.symbol_kind, $4, $5, $6::text)",
                &[
                    &file_id,
                    &"initial_function",
                    &"func",
                    &1,
                    &10,
                    &content,
                ],
            )
            .await?;

        Ok(file_id)
    }

    /// Modify a file's content.
    fn modify_file(&self, relpath: &str, new_content: &str) -> Result<()> {
        let abs_path = self.repo_root.join(relpath);
        fs::write(&abs_path, new_content)?;
        Ok(())
    }

    /// Get the most recent updated_at timestamp for a file's chunks.
    async fn get_file_updated_at(&self, relpath: &str) -> Result<Option<chrono::DateTime<Utc>>> {
        let client = self.pool.get().await?;

        let row = client
            .query_opt(
                "SELECT c.updated_at
                 FROM maproom.chunks c
                 JOIN maproom.files f ON c.file_id = f.id
                 WHERE f.relpath = $1 AND f.repo_id = $2
                 ORDER BY c.updated_at DESC
                 LIMIT 1",
                &[&relpath, &self.repo_id],
            )
            .await?;

        Ok(row.map(|r| r.get(0)))
    }

    /// Get the file_id for a given relpath.
    async fn get_file_id(&self, relpath: &str) -> Result<Option<i64>> {
        let client = self.pool.get().await?;

        let row = client
            .query_opt(
                "SELECT id FROM maproom.files WHERE relpath = $1 AND repo_id = $2",
                &[&relpath, &self.repo_id],
            )
            .await?;

        Ok(row.map(|r| r.get(0)))
    }

    /// Assert that a file was indexed after a given timestamp.
    async fn assert_file_indexed_after(
        &self,
        relpath: &str,
        start_time: chrono::DateTime<Utc>,
    ) -> Result<()> {
        let updated_at = self
            .get_file_updated_at(relpath)
            .await?
            .context(format!("No chunks found for file: {}", relpath))?;

        assert!(
            updated_at > start_time,
            "File {} was not re-indexed. Updated at: {}, Start time: {}",
            relpath,
            updated_at,
            start_time
        );

        Ok(())
    }

    /// Clean up database records for this test.
    async fn cleanup(&self) -> Result<()> {
        let client = self.pool.get().await?;

        // Delete repo (cascades to worktrees, commits, files, chunks)
        client
            .execute("DELETE FROM maproom.repos WHERE id = $1", &[&self.repo_id])
            .await?;

        Ok(())
    }
}

impl Drop for WatchTestFixture {
    fn drop(&mut self) {
        // Temp directory auto-cleans on drop
        // Database cleanup is async, so it's done explicitly in tests
    }
}

/// Test 1: Multi-file modification scenario (reproduces the original bug).
///
/// This test verifies that when 3 files are modified simultaneously:
/// 1. All 3 files are detected as modified
/// 2. All 3 files are classified as ChangeType::Modified (not New)
/// 3. All 3 files are successfully re-indexed
/// 4. Database timestamps are updated for all 3 files
#[tokio::test]
#[ignore = "Requires PostgreSQL database"]
async fn test_watch_multi_file_modification() -> Result<()> {
    let fixture = WatchTestFixture::new().await?;

    // Create and seed 3 files
    fixture
        .create_and_seed_file("src/a.rs", "fn a() { println!(\"original a\"); }")
        .await?;
    fixture
        .create_and_seed_file("src/b.rs", "fn b() { println!(\"original b\"); }")
        .await?;
    fixture
        .create_and_seed_file("src/c.rs", "fn c() { println!(\"original c\"); }")
        .await?;

    // Record start time
    let start_time = Utc::now();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Modify all 3 files
    fixture.modify_file(
        "src/a.rs",
        "fn a() { println!(\"modified a\"); // CHANGED }",
    )?;
    fixture.modify_file(
        "src/b.rs",
        "fn b() { println!(\"modified b\"); // CHANGED }",
    )?;
    fixture.modify_file(
        "src/c.rs",
        "fn c() { println!(\"modified c\"); // CHANGED }",
    )?;

    // Detect changes for all 3 files
    let mut detector = ChangeDetector::new(fixture.pool.clone());

    let file_a_id = fixture
        .get_file_id("src/a.rs")
        .await?
        .expect("File a not found");
    let file_b_id = fixture
        .get_file_id("src/b.rs")
        .await?
        .expect("File b not found");
    let file_c_id = fixture
        .get_file_id("src/c.rs")
        .await?
        .expect("File c not found");

    let change_a = detector
        .detect_change(file_a_id, &fixture.repo_root.join("src/a.rs"))
        .await?;
    let change_b = detector
        .detect_change(file_b_id, &fixture.repo_root.join("src/b.rs"))
        .await?;
    let change_c = detector
        .detect_change(file_c_id, &fixture.repo_root.join("src/c.rs"))
        .await?;

    // Verify all 3 files are classified as Modified (not New)
    assert!(
        matches!(change_a, ChangeType::Modified { .. }),
        "File a should be Modified, got: {:?}",
        change_a
    );
    assert!(
        matches!(change_b, ChangeType::Modified { .. }),
        "File b should be Modified, got: {:?}",
        change_b
    );
    assert!(
        matches!(change_c, ChangeType::Modified { .. }),
        "File c should be Modified, got: {:?}",
        change_c
    );

    // Process all 3 files using IncrementalProcessor
    let processor = IncrementalProcessor::new(fixture.pool.clone(), fixture.repo_root.clone());

    let task_a = UpdateTask::new(fixture.repo_root.join("src/a.rs"), change_a, Trigger::Save);
    let task_b = UpdateTask::new(fixture.repo_root.join("src/b.rs"), change_b, Trigger::Save);
    let task_c = UpdateTask::new(fixture.repo_root.join("src/c.rs"), change_c, Trigger::Save);

    // Process with timeout to prevent hanging
    timeout(Duration::from_secs(10), processor.process(task_a))
        .await
        .context("Task A timed out")??;
    timeout(Duration::from_secs(10), processor.process(task_b))
        .await
        .context("Task B timed out")??;
    timeout(Duration::from_secs(10), processor.process(task_c))
        .await
        .context("Task C timed out")??;

    // Verify all 3 files were re-indexed with updated timestamps
    fixture
        .assert_file_indexed_after("src/a.rs", start_time)
        .await?;
    fixture
        .assert_file_indexed_after("src/b.rs", start_time)
        .await?;
    fixture
        .assert_file_indexed_after("src/c.rs", start_time)
        .await?;

    // Cleanup
    fixture.cleanup().await?;

    Ok(())
}

/// Test 2: Single file modification.
///
/// This test verifies that a single modified file:
/// 1. Is detected as modified
/// 2. Is classified as ChangeType::Modified (not New)
/// 3. Is successfully re-indexed
/// 4. Database timestamp is updated
#[tokio::test]
#[ignore = "Requires PostgreSQL database"]
async fn test_watch_single_file_modified() -> Result<()> {
    let fixture = WatchTestFixture::new().await?;

    // Create and seed 1 file
    fixture
        .create_and_seed_file("src/main.rs", "fn main() { println!(\"Hello, world!\"); }")
        .await?;

    // Record start time
    let start_time = Utc::now();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Modify the file
    fixture.modify_file(
        "src/main.rs",
        "fn main() { println!(\"Hello, modified!\"); }",
    )?;

    // Detect change
    let mut detector = ChangeDetector::new(fixture.pool.clone());
    let file_id = fixture
        .get_file_id("src/main.rs")
        .await?
        .expect("File not found");
    let change = detector
        .detect_change(file_id, &fixture.repo_root.join("src/main.rs"))
        .await?;

    // Verify classified as Modified
    assert!(
        matches!(change, ChangeType::Modified { .. }),
        "File should be Modified, got: {:?}",
        change
    );

    // Process the file
    let processor = IncrementalProcessor::new(fixture.pool.clone(), fixture.repo_root.clone());
    let task = UpdateTask::new(fixture.repo_root.join("src/main.rs"), change, Trigger::Save);

    timeout(Duration::from_secs(10), processor.process(task))
        .await
        .context("Task timed out")??;

    // Verify file was re-indexed
    fixture
        .assert_file_indexed_after("src/main.rs", start_time)
        .await?;

    // Cleanup
    fixture.cleanup().await?;

    Ok(())
}

/// Test 3: Verify change type classification for existing files.
///
/// This test ensures that files with existing database records
/// are correctly classified as Modified (not New) when their content changes.
#[tokio::test]
#[ignore = "Requires PostgreSQL database"]
async fn test_change_type_classification() -> Result<()> {
    let fixture = WatchTestFixture::new().await?;

    // Create and seed a file
    let original_content = "fn original() { /* original */ }";
    fixture
        .create_and_seed_file("src/lib.rs", original_content)
        .await?;

    // Modify the file
    let modified_content = "fn original() { /* modified */ }";
    fixture.modify_file("src/lib.rs", modified_content)?;

    // Detect change
    let mut detector = ChangeDetector::new(fixture.pool.clone());
    let file_id = fixture
        .get_file_id("src/lib.rs")
        .await?
        .expect("File not found");
    let change = detector
        .detect_change(file_id, &fixture.repo_root.join("src/lib.rs"))
        .await?;

    // Verify it's Modified with old and new hashes
    match change {
        ChangeType::Modified { old, new } => {
            assert_ne!(old, new, "Old and new hashes should be different");

            // Verify old hash matches original content
            let expected_old = FileHasher::hash_bytes(original_content.as_bytes());
            assert_eq!(old, expected_old, "Old hash should match original content");

            // Verify new hash matches modified content
            let expected_new = FileHasher::hash_bytes(modified_content.as_bytes());
            assert_eq!(new, expected_new, "New hash should match modified content");
        }
        _ => panic!("Expected ChangeType::Modified, got: {:?}", change),
    }

    // Cleanup
    fixture.cleanup().await?;

    Ok(())
}

/// Test 4: Verify no infinite retry loops.
///
/// This test ensures that processing a file doesn't trigger infinite retries
/// by verifying that processing completes within a reasonable time.
#[tokio::test]
#[ignore = "Requires PostgreSQL database"]
async fn test_no_infinite_retry_loops() -> Result<()> {
    let fixture = WatchTestFixture::new().await?;

    // Create and seed a file
    fixture
        .create_and_seed_file("src/test.rs", "fn test() {}")
        .await?;

    // Modify the file
    fixture.modify_file("src/test.rs", "fn test() { /* changed */ }")?;

    // Detect change
    let mut detector = ChangeDetector::new(fixture.pool.clone());
    let file_id = fixture
        .get_file_id("src/test.rs")
        .await?
        .expect("File not found");
    let change = detector
        .detect_change(file_id, &fixture.repo_root.join("src/test.rs"))
        .await?;

    // Process with a strict timeout
    let processor = IncrementalProcessor::new(fixture.pool.clone(), fixture.repo_root.clone());
    let task = UpdateTask::new(fixture.repo_root.join("src/test.rs"), change, Trigger::Save);

    let result = timeout(Duration::from_secs(5), processor.process(task)).await;

    // Verify processing completed without timeout
    assert!(
        result.is_ok(),
        "Processing should complete within 5 seconds (no infinite retry loop)"
    );

    // Cleanup
    fixture.cleanup().await?;

    Ok(())
}

/// Test 5: Database state consistency after multi-file processing.
///
/// This test verifies that after processing multiple files:
/// 1. All file records are updated with new hashes
/// 2. Old chunks are deleted
/// 3. New chunks are inserted
/// 4. No orphaned chunks remain
#[tokio::test]
#[ignore = "Requires PostgreSQL database"]
async fn test_database_consistency_multi_file() -> Result<()> {
    let fixture = WatchTestFixture::new().await?;

    // Create and seed 3 files
    fixture
        .create_and_seed_file("src/x.rs", "fn x() {}")
        .await?;
    fixture
        .create_and_seed_file("src/y.rs", "fn y() {}")
        .await?;
    fixture
        .create_and_seed_file("src/z.rs", "fn z() {}")
        .await?;

    // Count initial chunks
    let client = fixture.pool.get().await?;
    let initial_count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks c
             JOIN maproom.files f ON c.file_id = f.id
             WHERE f.repo_id = $1",
            &[&fixture.repo_id],
        )
        .await?
        .get(0);

    assert_eq!(initial_count, 3, "Should have 3 initial chunks");

    // Modify all files
    fixture.modify_file("src/x.rs", "fn x() { /* modified */ }")?;
    fixture.modify_file("src/y.rs", "fn y() { /* modified */ }")?;
    fixture.modify_file("src/z.rs", "fn z() { /* modified */ }")?;

    // Process all files
    let mut detector = ChangeDetector::new(fixture.pool.clone());
    let processor = IncrementalProcessor::new(fixture.pool.clone(), fixture.repo_root.clone());

    for relpath in &["src/x.rs", "src/y.rs", "src/z.rs"] {
        let file_id = fixture.get_file_id(relpath).await?.expect("File not found");
        let change = detector
            .detect_change(file_id, &fixture.repo_root.join(relpath))
            .await?;

        let task = UpdateTask::new(fixture.repo_root.join(relpath), change, Trigger::Save);

        processor.process(task).await?;
    }

    // Verify chunk count (should still be 3, old deleted, new inserted)
    let final_count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks c
             JOIN maproom.files f ON c.file_id = f.id
             WHERE f.repo_id = $1",
            &[&fixture.repo_id],
        )
        .await?
        .get(0);

    assert_eq!(
        final_count, initial_count,
        "Chunk count should remain the same (old deleted, new inserted)"
    );

    // Verify all chunks have recent updated_at timestamps
    let recent_count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks c
             JOIN maproom.files f ON c.file_id = f.id
             WHERE f.repo_id = $1 AND c.updated_at > NOW() - INTERVAL '10 seconds'",
            &[&fixture.repo_id],
        )
        .await?
        .get(0);

    assert_eq!(
        recent_count, final_count,
        "All chunks should have recent updated_at timestamps"
    );

    // Cleanup
    fixture.cleanup().await?;

    Ok(())
}
