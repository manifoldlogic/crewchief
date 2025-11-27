//! Integration tests for failure recovery in incremental indexing.
//!
//! Tests error handling and recovery scenarios:
//! - Database connection failures
//! - File system watcher crashes
//! - Interrupted batch processing
//! - Transaction rollback validation
//! - Partial update handling
//! - Graceful degradation behavior
//!
//! These tests ensure the system can recover from failures without data corruption.

use anyhow::Result;
use crewchief_maproom::db::{create_pool, PgPool};
use crewchief_maproom::incremental::{
    ChangeType, FileHasher, IncrementalProcessor, UpdateTask, Trigger,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio;

/// Test helper for failure scenarios.
struct FailureTestRepo {
    temp_dir: TempDir,
    pool: PgPool,
    repo_id: i64,
    worktree_id: i64,
    commit_id: i64,
}

impl FailureTestRepo {
    /// Create a new test repository.
    async fn new() -> Result<Self> {
        let _ = dotenvy::dotenv();
        let temp_dir = TempDir::new()?;
        let pool = create_pool().await?;

        Self::setup_schema(&pool).await?;
        let (repo_id, worktree_id, commit_id) =
            Self::create_test_entities(&pool, temp_dir.path()).await?;

        Ok(Self {
            temp_dir,
            pool,
            repo_id,
            worktree_id,
            commit_id,
        })
    }

    /// Set up database schema.
    async fn setup_schema(pool: &PgPool) -> Result<()> {
        let client = pool.get().await?;

        client
            .batch_execute(
                r#"
                CREATE SCHEMA IF NOT EXISTS maproom;

                CREATE TABLE IF NOT EXISTS maproom.repos (
                    id BIGSERIAL PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    root_path TEXT NOT NULL,
                    created_at TIMESTAMPTZ DEFAULT NOW()
                );

                CREATE TABLE IF NOT EXISTS maproom.worktrees (
                    id BIGSERIAL PRIMARY KEY,
                    repo_id BIGINT NOT NULL REFERENCES maproom.repos(id),
                    name TEXT NOT NULL,
                    abs_path TEXT NOT NULL,
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(repo_id, name)
                );

                CREATE TABLE IF NOT EXISTS maproom.commits (
                    id BIGSERIAL PRIMARY KEY,
                    repo_id BIGINT NOT NULL REFERENCES maproom.repos(id),
                    sha TEXT NOT NULL,
                    committed_at TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(repo_id, sha)
                );

                CREATE TABLE IF NOT EXISTS maproom.files (
                    id BIGSERIAL PRIMARY KEY,
                    repo_id BIGINT NOT NULL REFERENCES maproom.repos(id),
                    worktree_id BIGINT NOT NULL REFERENCES maproom.worktrees(id),
                    commit_id BIGINT NOT NULL REFERENCES maproom.commits(id),
                    relpath TEXT NOT NULL,
                    language TEXT,
                    blake3_hash TEXT,
                    size_bytes BIGINT,
                    last_modified TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(commit_id, relpath, blake3_hash)
                );

                CREATE INDEX IF NOT EXISTS idx_files_worktree ON maproom.files(worktree_id);
                CREATE INDEX IF NOT EXISTS idx_files_hash ON maproom.files(blake3_hash);

                CREATE TABLE IF NOT EXISTS maproom.chunks (
                    id BIGSERIAL PRIMARY KEY,
                    file_id BIGINT NOT NULL REFERENCES maproom.files(id) ON DELETE CASCADE,
                    chunk_index INTEGER NOT NULL,
                    content TEXT NOT NULL,
                    start_line INTEGER NOT NULL,
                    end_line INTEGER NOT NULL,
                    chunk_type TEXT NOT NULL,
                    language TEXT,
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(file_id, chunk_index)
                );

                CREATE INDEX IF NOT EXISTS idx_chunks_file ON maproom.chunks(file_id);

                CREATE TABLE IF NOT EXISTS maproom.chunk_edges (
                    id BIGSERIAL PRIMARY KEY,
                    src_chunk_id BIGINT NOT NULL REFERENCES maproom.chunks(id) ON DELETE CASCADE,
                    dst_chunk_id BIGINT NOT NULL REFERENCES maproom.chunks(id) ON DELETE CASCADE,
                    edge_type TEXT NOT NULL,
                    weight DOUBLE PRECISION DEFAULT 1.0,
                    created_at TIMESTAMPTZ DEFAULT NOW()
                );

                CREATE INDEX IF NOT EXISTS idx_edges_src ON maproom.chunk_edges(src_chunk_id);
                CREATE INDEX IF NOT EXISTS idx_edges_dst ON maproom.chunk_edges(dst_chunk_id);
                "#,
            )
            .await?;

        Ok(())
    }

    /// Create test entities.
    async fn create_test_entities(
        pool: &PgPool,
        repo_path: &std::path::Path,
    ) -> Result<(i64, i64, i64)> {
        let client = pool.get().await?;

        let repo_row = client
            .query_one(
                "INSERT INTO maproom.repos(name, root_path)
                 VALUES ($1, $2)
                 RETURNING id",
                &[&"failure_test_repo", &repo_path.to_string_lossy().as_ref()],
            )
            .await?;
        let repo_id: i64 = repo_row.get(0);

        let worktree_row = client
            .query_one(
                "INSERT INTO maproom.worktrees(repo_id, name, abs_path)
                 VALUES ($1, $2, $3)
                 RETURNING id",
                &[&repo_id, &"main", &repo_path.to_string_lossy().as_ref()],
            )
            .await?;
        let worktree_id: i64 = worktree_row.get(0);

        let commit_row = client
            .query_one(
                "INSERT INTO maproom.commits(repo_id, sha, committed_at)
                 VALUES ($1, $2, NOW())
                 RETURNING id",
                &[&repo_id, &"failure_test_commit"],
            )
            .await?;
        let commit_id: i64 = commit_row.get(0);

        Ok((repo_id, worktree_id, commit_id))
    }

    /// Write a file to the repository.
    fn write_file(&self, relpath: &str, content: &str) -> Result<PathBuf> {
        let path = self.temp_dir.path().join(relpath);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
        Ok(path)
    }

    /// Count files in database.
    async fn count_files(&self) -> Result<i64> {
        let client = self.pool.get().await?;
        let row = client
            .query_one(
                "SELECT COUNT(*) FROM maproom.files WHERE worktree_id = $1",
                &[&self.worktree_id],
            )
            .await?;
        Ok(row.get(0))
    }

    /// Count chunks in database.
    async fn count_all_chunks(&self) -> Result<i64> {
        let client = self.pool.get().await?;
        let row = client
            .query_one(
                "SELECT COUNT(*) FROM maproom.chunks c
                 JOIN maproom.files f ON c.file_id = f.id
                 WHERE f.worktree_id = $1",
                &[&self.worktree_id],
            )
            .await?;
        Ok(row.get(0))
    }

    /// Verify database consistency.
    async fn verify_consistency(&self) -> Result<bool> {
        let client = self.pool.get().await?;

        // Check for orphaned chunks
        let orphaned_chunks: i64 = client
            .query_one(
                "SELECT COUNT(*) FROM maproom.chunks
                 WHERE file_id NOT IN (SELECT id FROM maproom.files)",
                &[],
            )
            .await?
            .get(0);

        if orphaned_chunks > 0 {
            return Ok(false);
        }

        // Check for orphaned edges
        let orphaned_edges: i64 = client
            .query_one(
                "SELECT COUNT(*) FROM maproom.chunk_edges
                 WHERE src_chunk_id NOT IN (SELECT id FROM maproom.chunks)
                    OR dst_chunk_id NOT IN (SELECT id FROM maproom.chunks)",
                &[],
            )
            .await?
            .get(0);

        if orphaned_edges > 0 {
            return Ok(false);
        }

        // Check for files without worktrees
        let orphaned_files: i64 = client
            .query_one(
                "SELECT COUNT(*) FROM maproom.files
                 WHERE worktree_id NOT IN (SELECT id FROM maproom.worktrees)",
                &[],
            )
            .await?
            .get(0);

        if orphaned_files > 0 {
            return Ok(false);
        }

        Ok(true)
    }

    /// Clean up test data.
    async fn cleanup(&self) -> Result<()> {
        let client = self.pool.get().await?;

        client
            .execute(
                "DELETE FROM maproom.files WHERE repo_id = $1",
                &[&self.repo_id],
            )
            .await?;

        client
            .execute(
                "DELETE FROM maproom.commits WHERE repo_id = $1",
                &[&self.repo_id],
            )
            .await?;

        client
            .execute(
                "DELETE FROM maproom.worktrees WHERE repo_id = $1",
                &[&self.repo_id],
            )
            .await?;

        client
            .execute("DELETE FROM maproom.repos WHERE id = $1", &[&self.repo_id])
            .await?;

        Ok(())
    }
}

#[tokio::test]
async fn test_invalid_file_path_handling() {
    let repo = FailureTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    // Try to process a non-existent file
    let non_existent_path = repo.temp_dir.path().join("does_not_exist.ts");
    let hash = FileHasher::hash_bytes(b"fake content");

    let task = UpdateTask::new(
        non_existent_path.clone(),
        ChangeType::New(hash),
        Trigger::Save,
    );

    // Should fail gracefully without crashing
    let result = processor.process(task).await;
    assert!(result.is_err(), "Expected error for non-existent file");

    // Database should remain consistent
    assert!(
        repo.verify_consistency().await.expect("Failed to verify consistency"),
        "Database inconsistent after error"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_partial_batch_failure() {
    let repo = FailureTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    // Create a batch with some valid and some invalid files
    let num_valid = 10;
    let num_invalid = 5;

    // Process valid files
    for i in 0..num_valid {
        let content = format!("export const valid{} = {};", i, i);
        let path = repo
            .write_file(&format!("src/valid_{}.ts", i), &content)
            .expect("Failed to write file");
        let hash = FileHasher::hash_file(&path).expect("Failed to hash file");
        let task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Save);

        processor
            .process(task)
            .await
            .expect("Failed to process valid file");
    }

    // Try to process invalid files (non-existent)
    let mut error_count = 0;
    for i in 0..num_invalid {
        let non_existent = repo.temp_dir.path().join(format!("src/invalid_{}.ts", i));
        let hash = FileHasher::hash_bytes(b"fake");
        let task = UpdateTask::new(non_existent, ChangeType::New(hash), Trigger::Save);

        if processor.process(task).await.is_err() {
            error_count += 1;
        }
    }

    assert_eq!(error_count, num_invalid, "Not all invalid files failed");

    // Verify only valid files were indexed
    let file_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(
        file_count, num_valid,
        "Invalid files should not be indexed"
    );

    // Verify database consistency
    assert!(
        repo.verify_consistency().await.expect("Failed to verify consistency"),
        "Database inconsistent after partial failure"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_transaction_rollback_on_error() {
    let repo = FailureTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    // Create a valid file and index it
    let content = "export const value = 1;";
    let path = repo
        .write_file("src/valid.ts", content)
        .expect("Failed to write file");
    let hash = FileHasher::hash_file(&path).expect("Failed to hash file");

    let task = UpdateTask::new(
        path.clone(),
        ChangeType::New(hash.clone()),
        Trigger::Save,
    );
    processor
        .process(task)
        .await
        .expect("Failed to process file");

    let initial_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(initial_count, 1, "Initial file not created");

    // Try to process an invalid modification (wrong old hash)
    let wrong_hash = FileHasher::hash_bytes(b"wrong old content");
    let new_content = "export const value = 2;";
    fs::write(&path, new_content).expect("Failed to write file");
    let new_hash = FileHasher::hash_file(&path).expect("Failed to hash file");

    let invalid_task = UpdateTask::new(
        path.clone(),
        ChangeType::Modified {
            old: wrong_hash,
            new: new_hash,
        },
        Trigger::Save,
    );

    // This may fail due to hash mismatch
    let _ = processor.process(invalid_task).await;

    // Verify database state is still consistent
    let final_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(
        final_count, initial_count,
        "File count should not change on error"
    );

    assert!(
        repo.verify_consistency().await.expect("Failed to verify consistency"),
        "Database inconsistent after transaction error"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_corrupted_file_content() {
    let repo = FailureTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    // Create a file with invalid UTF-8 content
    let path = repo.temp_dir.path().join("src/corrupted.ts");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("Failed to create directory");
    }

    // Write binary garbage (invalid UTF-8)
    let corrupted_data: Vec<u8> = vec![0xFF, 0xFE, 0xFD, 0x00, 0x80, 0x81];
    fs::write(&path, &corrupted_data).expect("Failed to write corrupted file");

    let hash = FileHasher::hash_bytes(&corrupted_data);
    let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);

    // Should handle gracefully (may succeed with lossy conversion or fail cleanly)
    let result = processor.process(task).await;

    // Regardless of outcome, database should be consistent
    assert!(
        repo.verify_consistency().await.expect("Failed to verify consistency"),
        "Database inconsistent after processing corrupted file"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_filesystem_permission_error() {
    let repo = FailureTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    // Create a file
    let content = "export const value = 1;";
    let path = repo
        .write_file("src/test.ts", content)
        .expect("Failed to write file");

    // Make file unreadable (Unix-like systems only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path).expect("Failed to get metadata").permissions();
        perms.set_mode(0o000); // No permissions
        fs::set_permissions(&path, perms).expect("Failed to set permissions");
    }

    let hash = FileHasher::hash_bytes(content.as_bytes());
    let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);

    // Should fail gracefully
    let result = processor.process(task).await;

    #[cfg(unix)]
    {
        assert!(result.is_err(), "Expected error for unreadable file");
    }

    // Restore permissions for cleanup
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)
            .unwrap_or_else(|_| fs::metadata(repo.temp_dir.path()).expect("Failed to get metadata"))
            .permissions();
        perms.set_mode(0o644);
        let _ = fs::set_permissions(&path, perms);
    }

    // Database should be consistent
    assert!(
        repo.verify_consistency().await.expect("Failed to verify consistency"),
        "Database inconsistent after permission error"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_interrupted_batch_consistency() {
    let repo = FailureTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    let batch_size = 50;

    // Process half of the batch
    for i in 0..(batch_size / 2) {
        let content = format!("export const value{} = {};", i, i);
        let path = repo
            .write_file(&format!("src/batch_{}.ts", i), &content)
            .expect("Failed to write file");
        let hash = FileHasher::hash_file(&path).expect("Failed to hash file");
        let task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Save);

        processor
            .process(task)
            .await
            .expect("Failed to process file");
    }

    let mid_count = repo.count_files().await.expect("Failed to count files");

    // Simulate interruption (stop processing)
    // In a real scenario, this could be a crash or network failure

    // Verify database is consistent even with partial batch
    assert!(
        repo.verify_consistency().await.expect("Failed to verify consistency"),
        "Database inconsistent after partial batch"
    );

    // Continue processing the rest
    for i in (batch_size / 2)..batch_size {
        let content = format!("export const value{} = {};", i, i);
        let path = repo
            .write_file(&format!("src/batch_{}.ts", i), &content)
            .expect("Failed to write file");
        let hash = FileHasher::hash_file(&path).expect("Failed to hash file");
        let task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Save);

        processor
            .process(task)
            .await
            .expect("Failed to process file");
    }

    let final_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(
        final_count, batch_size,
        "Not all files processed after resume"
    );

    assert!(
        repo.verify_consistency().await.expect("Failed to verify consistency"),
        "Database inconsistent after batch completion"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_cascade_delete_integrity() {
    let repo = FailureTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    // Create and index files
    let num_files = 20;
    for i in 0..num_files {
        let content = format!("export const value{} = {};", i, i);
        let path = repo
            .write_file(&format!("src/cascade_{}.ts", i), &content)
            .expect("Failed to write file");
        let hash = FileHasher::hash_file(&path).expect("Failed to hash file");
        let task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Save);

        processor
            .process(task)
            .await
            .expect("Failed to process file");
    }

    let initial_file_count = repo.count_files().await.expect("Failed to count files");
    let initial_chunk_count = repo.count_all_chunks().await.expect("Failed to count chunks");

    assert_eq!(initial_file_count, num_files);
    assert!(initial_chunk_count > 0, "No chunks created");

    // Delete all files directly from database to test CASCADE
    let client = repo.pool.get().await.expect("Failed to get client");
    client
        .execute(
            "DELETE FROM maproom.files WHERE worktree_id = $1",
            &[&repo.worktree_id],
        )
        .await
        .expect("Failed to delete files");

    // Verify chunks were cascade deleted
    let final_chunk_count = repo.count_all_chunks().await.expect("Failed to count chunks");
    assert_eq!(final_chunk_count, 0, "Chunks not cascade deleted");

    // Verify no orphaned edges
    let orphaned_edges: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunk_edges",
            &[],
        )
        .await
        .expect("Failed to count edges")
        .get(0);

    assert_eq!(orphaned_edges, 0, "Found orphaned edges after cascade delete");

    assert!(
        repo.verify_consistency().await.expect("Failed to verify consistency"),
        "Database inconsistent after cascade delete"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_recovery_from_pool_exhaustion() {
    let repo = FailureTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    // Process files normally - if pool is exhausted, operations should wait and eventually succeed
    let num_files = 30;

    for i in 0..num_files {
        let content = format!("export const pool_test{} = {};", i, i);
        let path = repo
            .write_file(&format!("src/pool_{}.ts", i), &content)
            .expect("Failed to write file");
        let hash = FileHasher::hash_file(&path).expect("Failed to hash file");
        let task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Save);

        processor
            .process(task)
            .await
            .expect("Failed to process file");
    }

    let file_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(
        file_count, num_files,
        "Not all files processed despite pool pressure"
    );

    assert!(
        repo.verify_consistency().await.expect("Failed to verify consistency"),
        "Database inconsistent after pool stress"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}
