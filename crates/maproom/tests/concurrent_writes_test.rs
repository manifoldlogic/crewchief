//! Concurrent write tests for SQLite connection pool with enhanced PRAGMAs.
//!
//! These tests verify that the enhanced PRAGMA configuration (busy_timeout=30s,
//! wal_autocheckpoint, cache_size, mmap_size) allows concurrent writes without
//! SQLITE_BUSY errors.
//!
//! Run with: cargo test --test concurrent_writes_test

use maproom::db::sqlite::SqliteStore;
use maproom::db::StoreChunks;
use maproom::db::StoreCore;
use maproom::db::{ChunkRecord, FileRecord};
use tempfile::tempdir;
use tokio::task::JoinSet;

/// Helper to create a test file record
fn create_file_record(
    repo_id: i64,
    worktree_id: i64,
    commit_id: i64,
    relpath: &str,
    content_hash: &str,
) -> FileRecord {
    FileRecord {
        repo_id,
        worktree_id,
        commit_id,
        relpath: relpath.to_string(),
        language: Some("rust".to_string()),
        content_hash: content_hash.to_string(),
        size_bytes: 1000,
        last_modified: None,
    }
}

/// Helper to create a test chunk record
fn create_chunk_record(
    file_id: i64,
    worktree_id: i64,
    blob_sha: &str,
    symbol_name: &str,
    kind: &str,
    preview: &str,
) -> ChunkRecord {
    ChunkRecord {
        file_id,
        worktree_id,
        blob_sha: blob_sha.to_string(),
        symbol_name: Some(symbol_name.to_string()),
        kind: kind.to_string(),
        signature: Some(format!("fn {}()", symbol_name)),
        docstring: Some(format!("{} documentation", symbol_name)),
        start_line: 1,
        end_line: 10,
        preview: preview.to_string(),
        ts_doc_text: format!("{} {}", symbol_name, kind),
        recency_score: 1.0,
        churn_score: 0.5,
        metadata: None,
    }
}

#[tokio::test]
async fn test_concurrent_writes_no_sqlite_busy() {
    // Create a file-based database to test concurrent access with enhanced PRAGMA settings
    // Note: In-memory databases don't share state across connections
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("concurrent_test.db");
    let store = SqliteStore::connect(db_path.to_str().unwrap())
        .await
        .unwrap();

    // Set up repo/worktree/commit hierarchy
    let repo_id = store
        .get_or_create_repo("test-repo", "/test/path")
        .await
        .unwrap();
    let worktree_id = store
        .get_or_create_worktree(repo_id, "main", "/test/path")
        .await
        .unwrap();
    let commit_id = store
        .get_or_create_commit(repo_id, "abc123", None)
        .await
        .unwrap();

    // Spawn 5 concurrent tasks, each inserting multiple files and chunks
    let mut set = JoinSet::new();
    for thread_id in 0..5 {
        let store_clone = store.clone();
        set.spawn(async move {
            // Each thread inserts 20 files with chunks
            for i in 0..20 {
                let relpath = format!("src/thread_{}/file_{}.rs", thread_id, i);
                let content_hash = format!("hash_{}_{}", thread_id, i);

                // Insert file
                let file =
                    create_file_record(repo_id, worktree_id, commit_id, &relpath, &content_hash);
                let file_id = store_clone
                    .upsert_file(&file)
                    .await
                    .expect("File insert should not get SQLITE_BUSY");

                // Insert chunk
                let blob_sha = format!("blob_{}_{}", thread_id, i);
                let symbol_name = format!("function_{}_{}", thread_id, i);
                let preview = format!("fn {}() {{ }}", symbol_name);
                let chunk = create_chunk_record(
                    file_id,
                    worktree_id,
                    &blob_sha,
                    &symbol_name,
                    "function",
                    &preview,
                );
                store_clone
                    .insert_chunk(&chunk)
                    .await
                    .expect("Chunk insert should not get SQLITE_BUSY");
            }

            // Return the thread ID for verification
            thread_id
        });
    }

    // Collect results - all tasks should complete without SQLITE_BUSY errors
    let mut completed_threads = Vec::new();
    while let Some(result) = set.join_next().await {
        match result {
            Ok(thread_id) => completed_threads.push(thread_id),
            Err(e) => panic!("Task failed: {}", e),
        }
    }

    // Verify all 5 threads completed
    assert_eq!(completed_threads.len(), 5);
    completed_threads.sort();
    assert_eq!(completed_threads, vec![0, 1, 2, 3, 4]);

    // Verify all files were inserted (5 threads * 20 files = 100 total)
    let file_count = store
        .run(|conn| {
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
            Ok(count)
        })
        .await
        .unwrap();
    assert_eq!(file_count, 100, "Should have inserted 100 files");

    // Verify all chunks were inserted
    let chunk_count = store
        .run(|conn| {
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))?;
            Ok(count)
        })
        .await
        .unwrap();
    assert_eq!(chunk_count, 100, "Should have inserted 100 chunks");
}

#[tokio::test]
async fn test_concurrent_repo_operations_no_sqlite_busy() {
    // Create a file-based database to test concurrent metadata operations
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("concurrent_repos_test.db");
    let store = SqliteStore::connect(db_path.to_str().unwrap())
        .await
        .unwrap();

    // Spawn 5 concurrent tasks, each creating repos/worktrees/commits
    let mut set = JoinSet::new();
    for thread_id in 0..5 {
        let store_clone = store.clone();
        set.spawn(async move {
            // Each thread creates 10 repos with worktrees and commits
            for i in 0..10 {
                let repo_name = format!("repo_{}_{}", thread_id, i);
                let repo_path = format!("/test/{}/{}", thread_id, i);

                // Create repo
                let repo_id = store_clone
                    .get_or_create_repo(&repo_name, &repo_path)
                    .await
                    .expect("Repo creation should not get SQLITE_BUSY");

                // Create worktree
                let worktree_id = store_clone
                    .get_or_create_worktree(repo_id, "main", &repo_path)
                    .await
                    .expect("Worktree creation should not get SQLITE_BUSY");

                // Create commit
                let commit_sha = format!("commit_{}_{}", thread_id, i);
                let _commit_id = store_clone
                    .get_or_create_commit(repo_id, &commit_sha, None)
                    .await
                    .expect("Commit creation should not get SQLITE_BUSY");

                // Verify IDs are valid
                assert!(repo_id > 0);
                assert!(worktree_id > 0);
            }
            thread_id
        });
    }

    // Collect results
    let mut completed_threads = Vec::new();
    while let Some(result) = set.join_next().await {
        match result {
            Ok(thread_id) => completed_threads.push(thread_id),
            Err(e) => panic!("Task failed: {}", e),
        }
    }

    // Verify all 5 threads completed
    assert_eq!(completed_threads.len(), 5);
    completed_threads.sort();
    assert_eq!(completed_threads, vec![0, 1, 2, 3, 4]);

    // Verify we created 50 repos (5 threads * 10 repos each)
    let repo_count = store
        .run(|conn| {
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM repos", [], |row| row.get(0))?;
            Ok(count)
        })
        .await
        .unwrap();
    assert_eq!(repo_count, 50, "Should have created 50 repos");

    // Verify we created 50 worktrees
    let worktree_count = store
        .run(|conn| {
            let count: i64 =
                conn.query_row("SELECT COUNT(*) FROM worktrees", [], |row| row.get(0))?;
            Ok(count)
        })
        .await
        .unwrap();
    assert_eq!(worktree_count, 50, "Should have created 50 worktrees");
}
