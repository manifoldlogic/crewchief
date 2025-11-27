//! Integration tests for concurrent incremental indexing operations.
//!
//! Tests concurrent file updates to verify:
//! - No data corruption under concurrent load
//! - Database transaction isolation works correctly
//! - All changes are eventually indexed
//! - Lock contention is handled properly
//! - Deadlock prevention mechanisms work
//!
//! These tests simulate real-world scenarios where multiple files change
//! simultaneously (e.g., git checkout, bulk refactoring).

use anyhow::Result;
use crewchief_maproom::db::{create_pool, PgPool};
use crewchief_maproom::incremental::{
    ChangeType, FileHasher, IncrementalProcessor, UpdateTask, Trigger,
};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio;
use tokio::task::JoinSet;

/// Test helper for concurrent operations.
struct ConcurrentTestRepo {
    temp_dir: TempDir,
    pool: PgPool,
    repo_id: i64,
    worktree_id: i64,
    commit_id: i64,
}

impl ConcurrentTestRepo {
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
                &[&"concurrent_test_repo", &repo_path.to_string_lossy().as_ref()],
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
                &[&repo_id, &"concurrent_test_commit"],
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

    /// Count total files in database.
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

    /// Count total chunks across all files.
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
#[ignore = "requires PostgreSQL database"]
async fn test_concurrent_file_creation() {
    let repo = ConcurrentTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = Arc::new(IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf()));

    let num_files = 50;
    let mut tasks = JoinSet::new();

    // Spawn concurrent file creation tasks
    for i in 0..num_files {
        let repo_path = repo.temp_dir.path().to_path_buf();
        let processor_clone = processor.clone();

        tasks.spawn(async move {
            let relpath = format!("src/concurrent_{}.ts", i);
            let content = format!("export const value{} = {};", i, i);

            // Write file
            let path = repo_path.join(&relpath);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("Failed to create directory");
            }
            fs::write(&path, &content).expect("Failed to write file");

            // Hash and process
            let hash = FileHasher::hash_file(&path).expect("Failed to hash file");
            let task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Save);

            processor_clone
                .process(task)
                .await
                .expect("Failed to process file")
        });
    }

    // Wait for all tasks to complete
    while let Some(result) = tasks.join_next().await {
        result.expect("Task panicked");
    }

    // Verify all files were created
    let file_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(
        file_count, num_files,
        "Expected {} files, found {}",
        num_files, file_count
    );

    // Verify chunks were created
    let chunk_count = repo.count_all_chunks().await.expect("Failed to count chunks");
    assert!(
        chunk_count >= num_files,
        "Expected at least {} chunks, found {}",
        num_files,
        chunk_count
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_concurrent_modifications() {
    let repo = ConcurrentTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = Arc::new(IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf()));

    let num_files = 20;

    // First, create all files sequentially
    for i in 0..num_files {
        let content = format!("export const initial{} = {};", i, i);
        let path = repo
            .write_file(&format!("src/modify_{}.ts", i), &content)
            .expect("Failed to write file");
        let hash = FileHasher::hash_file(&path).expect("Failed to hash file");
        let task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Save);
        processor
            .process(task)
            .await
            .expect("Failed to create initial file");
    }

    let initial_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(initial_count, num_files, "Initial file count mismatch");

    // Now modify all files concurrently
    let mut tasks = JoinSet::new();

    for i in 0..num_files {
        let repo_path = repo.temp_dir.path().to_path_buf();
        let processor_clone = processor.clone();

        tasks.spawn(async move {
            let relpath = format!("src/modify_{}.ts", i);
            let old_content = format!("export const initial{} = {};", i, i);
            let new_content = format!("export const modified{} = {}; // updated", i, i * 2);

            let old_hash = FileHasher::hash_bytes(old_content.as_bytes());
            let path = repo_path.join(&relpath);

            // Modify file
            fs::write(&path, &new_content).expect("Failed to modify file");
            let new_hash = FileHasher::hash_file(&path).expect("Failed to hash modified file");

            // Process modification
            let task = UpdateTask::new(
                path,
                ChangeType::Modified {
                    old: old_hash,
                    new: new_hash,
                },
                Trigger::Save,
            );

            processor_clone
                .process(task)
                .await
                .expect("Failed to process modification")
        });
    }

    // Wait for all modifications to complete
    while let Some(result) = tasks.join_next().await {
        result.expect("Modification task panicked");
    }

    // Verify file count unchanged (modifications don't add/remove files)
    let final_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(
        final_count, num_files,
        "File count changed during modifications"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_concurrent_mixed_operations() {
    let repo = ConcurrentTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = Arc::new(IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf()));

    // Create some initial files
    let num_initial = 10;
    for i in 0..num_initial {
        let content = format!("export const initial{} = {};", i, i);
        let path = repo
            .write_file(&format!("src/initial_{}.ts", i), &content)
            .expect("Failed to write file");
        let hash = FileHasher::hash_file(&path).expect("Failed to hash file");
        let task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Save);
        processor
            .process(task)
            .await
            .expect("Failed to create file");
    }

    let mut tasks = JoinSet::new();

    // Concurrently:
    // - Create new files (10-19)
    // - Modify existing files (0-4)
    // - Delete files (5-9)

    // Create new files
    for i in 10..20 {
        let repo_path = repo.temp_dir.path().to_path_buf();
        let processor_clone = processor.clone();

        tasks.spawn(async move {
            let relpath = format!("src/new_{}.ts", i);
            let content = format!("export const new{} = {};", i, i);
            let path = repo_path.join(&relpath);

            fs::write(&path, &content).expect("Failed to write file");
            let hash = FileHasher::hash_file(&path).expect("Failed to hash file");
            let task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Save);

            processor_clone
                .process(task)
                .await
                .expect("Failed to process new file")
        });
    }

    // Modify existing files
    for i in 0..5 {
        let repo_path = repo.temp_dir.path().to_path_buf();
        let processor_clone = processor.clone();

        tasks.spawn(async move {
            let relpath = format!("src/initial_{}.ts", i);
            let old_content = format!("export const initial{} = {};", i, i);
            let new_content = format!("export const modified{} = {};", i, i * 10);

            let old_hash = FileHasher::hash_bytes(old_content.as_bytes());
            let path = repo_path.join(&relpath);

            fs::write(&path, &new_content).expect("Failed to modify file");
            let new_hash = FileHasher::hash_file(&path).expect("Failed to hash file");

            let task = UpdateTask::new(
                path,
                ChangeType::Modified {
                    old: old_hash,
                    new: new_hash,
                },
                Trigger::Save,
            );

            processor_clone
                .process(task)
                .await
                .expect("Failed to process modification")
        });
    }

    // Delete files
    for i in 5..10 {
        let repo_path = repo.temp_dir.path().to_path_buf();
        let processor_clone = processor.clone();

        tasks.spawn(async move {
            let relpath = format!("src/initial_{}.ts", i);
            let content = format!("export const initial{} = {};", i, i);
            let hash = FileHasher::hash_bytes(content.as_bytes());
            let path = repo_path.join(&relpath);

            let task = UpdateTask::new(path, ChangeType::Deleted(hash), Trigger::Save);

            processor_clone
                .process(task)
                .await
                .expect("Failed to process deletion")
        });
    }

    // Wait for all tasks
    while let Some(result) = tasks.join_next().await {
        result.expect("Task panicked");
    }

    // Verify final state:
    // - 5 modified files (0-4)
    // - 10 new files (10-19)
    // - 5 deleted files (5-9)
    // Total: 15 files
    let final_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(
        final_count, 15,
        "Expected 15 files after mixed operations, found {}",
        final_count
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_transaction_isolation() {
    let repo = ConcurrentTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = Arc::new(IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf()));

    // Create a file
    let content = "export const shared = 1;";
    let path = repo
        .write_file("src/shared.ts", content)
        .expect("Failed to write file");
    let initial_hash = FileHasher::hash_file(&path).expect("Failed to hash file");

    let task = UpdateTask::new(
        path.clone(),
        ChangeType::New(initial_hash.clone()),
        Trigger::Save,
    );
    processor
        .process(task)
        .await
        .expect("Failed to create file");

    // Simulate concurrent modifications to the same file
    // This should not cause corruption due to transaction isolation
    let mut tasks = JoinSet::new();

    for i in 0..5 {
        let path_clone = path.clone();
        let processor_clone = processor.clone();
        let old_hash = initial_hash.clone();

        tasks.spawn(async move {
            let new_content = format!("export const shared = {}; // version {}", i, i);
            fs::write(&path_clone, &new_content).expect("Failed to write file");
            let new_hash = FileHasher::hash_file(&path_clone).expect("Failed to hash file");

            let task = UpdateTask::new(
                path_clone,
                ChangeType::Modified {
                    old: old_hash,
                    new: new_hash,
                },
                Trigger::Save,
            );

            // Some may fail due to hash conflicts, but no corruption should occur
            let _ = processor_clone.process(task).await;
        });
    }

    while let Some(result) = tasks.join_next().await {
        result.expect("Task panicked");
    }

    // Verify database consistency - file should exist and have valid data
    let client = repo.pool.get().await.expect("Failed to get client");
    let file_count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.files WHERE worktree_id = $1 AND relpath = $2",
            &[&repo.worktree_id, &"src/shared.ts"],
        )
        .await
        .expect("Failed to count files")
        .get(0);

    assert_eq!(
        file_count, 1,
        "File count should be exactly 1 despite concurrent modifications"
    );

    // Verify no orphaned chunks
    let orphaned: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks WHERE file_id NOT IN (SELECT id FROM maproom.files)",
            &[],
        )
        .await
        .expect("Failed to query orphaned chunks")
        .get(0);

    assert_eq!(orphaned, 0, "Found orphaned chunks after concurrent updates");

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_no_deadlocks_under_load() {
    let repo = ConcurrentTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = Arc::new(IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf()));

    // Create many files concurrently to stress test lock contention
    let num_files = 100;
    let mut tasks = JoinSet::new();

    for i in 0..num_files {
        let repo_path = repo.temp_dir.path().to_path_buf();
        let processor_clone = processor.clone();

        tasks.spawn(async move {
            let relpath = format!("src/stress_{}.ts", i);
            let content = format!("export const stress{} = {};", i, i);
            let path = repo_path.join(&relpath);

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("Failed to create directory");
            }
            fs::write(&path, &content).expect("Failed to write file");

            let hash = FileHasher::hash_file(&path).expect("Failed to hash file");
            let task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Save);

            processor_clone.process(task).await
        });
    }

    // Wait for all tasks with timeout to detect deadlocks
    let timeout = tokio::time::Duration::from_secs(60);
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        match tokio::time::timeout_at(deadline, tasks.join_next()).await {
            Ok(Some(task_result)) => {
                task_result
                    .expect("Task panicked")
                    .expect("Task failed to process");
            }
            Ok(None) => break, // All tasks completed
            Err(_) => panic!("Deadlock detected: tasks did not complete within timeout"),
        }
    }

    // Verify all files were processed
    let file_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(
        file_count, num_files,
        "Not all files were processed - possible deadlock or failure"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}
