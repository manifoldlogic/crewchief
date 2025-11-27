//! Integration tests for incremental indexing file change scenarios.
//!
//! Tests all file operations in the incremental indexing pipeline:
//! - File creation (new files)
//! - File modification (content changes)
//! - File deletion
//! - File rename/move
//! - Mixed operations (create + modify + delete)
//!
//! Each test validates end-to-end integration with the database and verifies
//! index consistency after each operation.

use anyhow::Result;
use crewchief_maproom::db::{create_pool, PgPool};
use crewchief_maproom::incremental::{
    ChangeDetector, ChangeType, FileHasher, IncrementalProcessor, UpdateTask, Trigger,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio;

/// Test helper to set up a temporary repository with database.
struct TestRepo {
    temp_dir: TempDir,
    pool: PgPool,
    repo_id: i64,
    worktree_id: i64,
    commit_id: i64,
}

impl TestRepo {
    /// Create a new test repository with initialized database schema.
    async fn new() -> Result<Self> {
        let _ = dotenvy::dotenv();
        let temp_dir = TempDir::new()?;
        let pool = create_pool().await?;

        // Set up database schema
        Self::setup_schema(&pool).await?;

        // Create test repo, worktree, and commit
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

    /// Set up minimal database schema for testing.
    async fn setup_schema(pool: &PgPool) -> Result<()> {
        let client = pool.get().await?;

        // Create schema if it doesn't exist
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

    /// Create test repo, worktree, and commit entities.
    async fn create_test_entities(pool: &PgPool, repo_path: &std::path::Path) -> Result<(i64, i64, i64)> {
        let client = pool.get().await?;

        // Create repo
        let repo_row = client
            .query_one(
                "INSERT INTO maproom.repos(name, root_path)
                 VALUES ($1, $2)
                 RETURNING id",
                &[&"test_repo", &repo_path.to_string_lossy().as_ref()],
            )
            .await?;
        let repo_id: i64 = repo_row.get(0);

        // Create worktree
        let worktree_row = client
            .query_one(
                "INSERT INTO maproom.worktrees(repo_id, name, abs_path)
                 VALUES ($1, $2, $3)
                 RETURNING id",
                &[&repo_id, &"main", &repo_path.to_string_lossy().as_ref()],
            )
            .await?;
        let worktree_id: i64 = worktree_row.get(0);

        // Create commit
        let commit_row = client
            .query_one(
                "INSERT INTO maproom.commits(repo_id, sha, committed_at)
                 VALUES ($1, $2, NOW())
                 RETURNING id",
                &[&repo_id, &"test_commit_abc123"],
            )
            .await?;
        let commit_id: i64 = commit_row.get(0);

        Ok((repo_id, worktree_id, commit_id))
    }

    /// Write a file to the test repository.
    fn write_file(&self, relpath: &str, content: &str) -> Result<PathBuf> {
        let path = self.temp_dir.path().join(relpath);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
        Ok(path)
    }

    /// Delete a file from the test repository.
    fn delete_file(&self, relpath: &str) -> Result<()> {
        let path = self.temp_dir.path().join(relpath);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Rename/move a file in the test repository.
    fn rename_file(&self, old_relpath: &str, new_relpath: &str) -> Result<()> {
        let old_path = self.temp_dir.path().join(old_relpath);
        let new_path = self.temp_dir.path().join(new_relpath);
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(old_path, new_path)?;
        Ok(())
    }

    /// Get file_id from database by relpath.
    async fn get_file_id(&self, relpath: &str) -> Result<Option<i64>> {
        let client = self.pool.get().await?;
        let result = client
            .query_opt(
                "SELECT id FROM maproom.files WHERE worktree_id = $1 AND relpath = $2",
                &[&self.worktree_id, &relpath],
            )
            .await?;
        Ok(result.map(|row| row.get(0)))
    }

    /// Count chunks for a file.
    async fn count_chunks(&self, file_id: i64) -> Result<i64> {
        let client = self.pool.get().await?;
        let row = client
            .query_one(
                "SELECT COUNT(*) FROM maproom.chunks WHERE file_id = $1",
                &[&file_id],
            )
            .await?;
        Ok(row.get(0))
    }

    /// Verify file hash in database matches expected.
    async fn verify_file_hash(&self, relpath: &str, expected_hash: &str) -> Result<bool> {
        let client = self.pool.get().await?;
        let result = client
            .query_opt(
                "SELECT blake3_hash FROM maproom.files WHERE worktree_id = $1 AND relpath = $2",
                &[&self.worktree_id, &relpath],
            )
            .await?;

        if let Some(row) = result {
            let hash: Option<String> = row.get(0);
            Ok(hash.as_deref() == Some(expected_hash))
        } else {
            Ok(false)
        }
    }

    /// Clean up test data from database.
    async fn cleanup(&self) -> Result<()> {
        let client = self.pool.get().await?;

        // Delete in order respecting foreign keys
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
async fn test_file_creation() {
    let repo = TestRepo::new().await.expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    // Create a new file
    let content = "export function hello() { return 'world'; }";
    let path = repo
        .write_file("src/hello.ts", content)
        .expect("Failed to write file");
    let hash = FileHasher::hash_file(&path).expect("Failed to hash file");

    // Create update task for new file
    let task = UpdateTask::new(
        path.clone(),
        ChangeType::New(hash.clone()),
        Trigger::Save,
    );

    // Process the task
    processor
        .process(task)
        .await
        .expect("Failed to process new file");

    // Verify file was indexed
    let file_id = repo
        .get_file_id("src/hello.ts")
        .await
        .expect("Failed to query file")
        .expect("File not found in database");

    // Verify hash is stored
    assert!(
        repo.verify_file_hash("src/hello.ts", &hash.to_string())
            .await
            .expect("Failed to verify hash"),
        "File hash mismatch"
    );

    // Verify chunks were created
    let chunk_count = repo.count_chunks(file_id).await.expect("Failed to count chunks");
    assert!(chunk_count > 0, "No chunks created for new file");

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_file_modification() {
    let repo = TestRepo::new().await.expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    // Create and index initial file
    let initial_content = "export function old() { return 'old'; }";
    let path = repo
        .write_file("src/modified.ts", initial_content)
        .expect("Failed to write file");
    let old_hash = FileHasher::hash_file(&path).expect("Failed to hash file");

    let task = UpdateTask::new(
        path.clone(),
        ChangeType::New(old_hash.clone()),
        Trigger::Save,
    );
    processor
        .process(task)
        .await
        .expect("Failed to process initial file");

    let file_id = repo
        .get_file_id("src/modified.ts")
        .await
        .expect("Failed to query file")
        .expect("File not found");

    let initial_chunk_count = repo
        .count_chunks(file_id)
        .await
        .expect("Failed to count initial chunks");

    // Modify the file
    let new_content = "export function newFunc() { return 'updated content'; }";
    repo.write_file("src/modified.ts", new_content)
        .expect("Failed to update file");
    let new_hash = FileHasher::hash_file(&path).expect("Failed to hash modified file");

    // Process modification
    let task = UpdateTask::new(
        path.clone(),
        ChangeType::Modified {
            old: old_hash,
            new: new_hash.clone(),
        },
        Trigger::Save,
    );
    processor
        .process(task)
        .await
        .expect("Failed to process modified file");

    // Verify hash was updated
    assert!(
        repo.verify_file_hash("src/modified.ts", &new_hash.to_string())
            .await
            .expect("Failed to verify hash"),
        "File hash not updated after modification"
    );

    // Verify chunks were updated (count may change)
    let updated_chunk_count = repo
        .count_chunks(file_id)
        .await
        .expect("Failed to count updated chunks");
    assert!(
        updated_chunk_count > 0,
        "No chunks after modification"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_file_deletion() {
    let repo = TestRepo::new().await.expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    // Create and index a file
    let content = "export function toDelete() {}";
    let path = repo
        .write_file("src/delete_me.ts", content)
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

    let file_id = repo
        .get_file_id("src/delete_me.ts")
        .await
        .expect("Failed to query file")
        .expect("File not found");

    // Verify chunks exist
    let chunk_count = repo.count_chunks(file_id).await.expect("Failed to count chunks");
    assert!(chunk_count > 0, "No chunks before deletion");

    // Delete the file
    repo.delete_file("src/delete_me.ts")
        .expect("Failed to delete file");

    // Process deletion
    let task = UpdateTask::new(
        path.clone(),
        ChangeType::Deleted(hash),
        Trigger::Save,
    );
    processor
        .process(task)
        .await
        .expect("Failed to process deleted file");

    // Verify file entry is removed from database
    let file_exists = repo
        .get_file_id("src/delete_me.ts")
        .await
        .expect("Failed to query file")
        .is_some();
    assert!(!file_exists, "File still exists in database after deletion");

    // Verify chunks are removed (CASCADE delete)
    let chunk_count_after = repo
        .count_chunks(file_id)
        .await
        .expect("Failed to count chunks after deletion");
    assert_eq!(chunk_count_after, 0, "Chunks not deleted with file");

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_file_rename() {
    let repo = TestRepo::new().await.expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    // Create and index a file
    let content = "export function renamed() {}";
    let old_path = repo
        .write_file("src/old_name.ts", content)
        .expect("Failed to write file");
    let hash = FileHasher::hash_file(&old_path).expect("Failed to hash file");

    let task = UpdateTask::new(
        old_path.clone(),
        ChangeType::New(hash.clone()),
        Trigger::Save,
    );
    processor
        .process(task)
        .await
        .expect("Failed to process file");

    // Verify old file exists
    assert!(
        repo.get_file_id("src/old_name.ts")
            .await
            .expect("Failed to query file")
            .is_some(),
        "Old file not found"
    );

    // Rename the file
    repo.rename_file("src/old_name.ts", "src/new_name.ts")
        .expect("Failed to rename file");
    let new_path = repo.temp_dir.path().join("src/new_name.ts");

    // Process as deletion of old + creation of new
    let delete_task = UpdateTask::new(
        old_path.clone(),
        ChangeType::Deleted(hash.clone()),
        Trigger::Save,
    );
    processor
        .process(delete_task)
        .await
        .expect("Failed to process deletion");

    let create_task = UpdateTask::new(
        new_path.clone(),
        ChangeType::New(hash.clone()),
        Trigger::Save,
    );
    processor
        .process(create_task)
        .await
        .expect("Failed to process creation");

    // Verify old file is gone
    assert!(
        repo.get_file_id("src/old_name.ts")
            .await
            .expect("Failed to query old file")
            .is_none(),
        "Old file still exists after rename"
    );

    // Verify new file exists
    assert!(
        repo.get_file_id("src/new_name.ts")
            .await
            .expect("Failed to query new file")
            .is_some(),
        "New file not found after rename"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_mixed_operations() {
    let repo = TestRepo::new().await.expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    // Create multiple files
    let file1_path = repo
        .write_file("src/file1.ts", "export const a = 1;")
        .expect("Failed to write file1");
    let file2_path = repo
        .write_file("src/file2.ts", "export const b = 2;")
        .expect("Failed to write file2");
    let file3_path = repo
        .write_file("src/file3.ts", "export const c = 3;")
        .expect("Failed to write file3");

    // Index all files
    for (path, content) in [
        (&file1_path, "export const a = 1;"),
        (&file2_path, "export const b = 2;"),
        (&file3_path, "export const c = 3;"),
    ] {
        let hash = FileHasher::hash_file(path).expect("Failed to hash file");
        let task = UpdateTask::new(
            path.clone(),
            ChangeType::New(hash),
            Trigger::Save,
        );
        processor
            .process(task)
            .await
            .expect("Failed to process file");
    }

    // Verify all files indexed
    assert!(repo.get_file_id("src/file1.ts").await.unwrap().is_some());
    assert!(repo.get_file_id("src/file2.ts").await.unwrap().is_some());
    assert!(repo.get_file_id("src/file3.ts").await.unwrap().is_some());

    // Perform mixed operations:
    // - Modify file1
    // - Delete file2
    // - Create file4
    // - Leave file3 unchanged

    // Modify file1
    let old_hash1 = FileHasher::hash_bytes(b"export const a = 1;");
    repo.write_file("src/file1.ts", "export const a = 100; // modified")
        .expect("Failed to modify file1");
    let new_hash1 = FileHasher::hash_file(&file1_path).expect("Failed to hash modified file1");
    let modify_task = UpdateTask::new(
        file1_path.clone(),
        ChangeType::Modified {
            old: old_hash1,
            new: new_hash1.clone(),
        },
        Trigger::Save,
    );
    processor
        .process(modify_task)
        .await
        .expect("Failed to process modification");

    // Delete file2
    let hash2 = FileHasher::hash_bytes(b"export const b = 2;");
    repo.delete_file("src/file2.ts")
        .expect("Failed to delete file2");
    let delete_task = UpdateTask::new(
        file2_path.clone(),
        ChangeType::Deleted(hash2),
        Trigger::Save,
    );
    processor
        .process(delete_task)
        .await
        .expect("Failed to process deletion");

    // Create file4
    let file4_path = repo
        .write_file("src/file4.ts", "export const d = 4;")
        .expect("Failed to write file4");
    let hash4 = FileHasher::hash_file(&file4_path).expect("Failed to hash file4");
    let create_task = UpdateTask::new(
        file4_path.clone(),
        ChangeType::New(hash4),
        Trigger::Save,
    );
    processor
        .process(create_task)
        .await
        .expect("Failed to process creation");

    // Verify final state
    assert!(
        repo.verify_file_hash("src/file1.ts", &new_hash1.to_string())
            .await
            .unwrap(),
        "File1 not modified correctly"
    );
    assert!(
        repo.get_file_id("src/file2.ts").await.unwrap().is_none(),
        "File2 not deleted"
    );
    assert!(
        repo.get_file_id("src/file3.ts").await.unwrap().is_some(),
        "File3 should still exist"
    );
    assert!(
        repo.get_file_id("src/file4.ts").await.unwrap().is_some(),
        "File4 not created"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_index_consistency_after_operations() {
    let repo = TestRepo::new().await.expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    // Create and index a file
    let path = repo
        .write_file("src/consistency.ts", "export const x = 1;")
        .expect("Failed to write file");
    let hash = FileHasher::hash_file(&path).expect("Failed to hash file");
    let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);
    processor
        .process(task)
        .await
        .expect("Failed to process file");

    let file_id = repo
        .get_file_id("src/consistency.ts")
        .await
        .expect("Failed to query file")
        .expect("File not found");

    // Check chunk count
    let chunk_count = repo.count_chunks(file_id).await.expect("Failed to count chunks");
    assert!(chunk_count > 0, "No chunks created");

    // Verify no orphaned chunks exist (all chunks have valid file_id)
    let client = repo.pool.get().await.expect("Failed to get client");
    let orphaned_count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks WHERE file_id NOT IN (SELECT id FROM maproom.files)",
            &[],
        )
        .await
        .expect("Failed to query orphaned chunks")
        .get(0);
    assert_eq!(orphaned_count, 0, "Found orphaned chunks");

    repo.cleanup().await.expect("Failed to cleanup");
}
