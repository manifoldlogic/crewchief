//! Integration tests for clean-ignored CLI command.
//!
//! Tests verify:
//! - End-to-end workflow: scan → create .maproomignore → clean-ignored → verify
//! - Chunks matching patterns are deleted
//! - Chunks not matching patterns are preserved
//! - Dry-run mode works correctly
//!
//! MRMIGNR-2001: clean-ignored command integration tests

use anyhow::Result;
use crewchief_maproom::cli::clean_ignored::clean_ignored;
use crewchief_maproom::db::sqlite::SqliteStore;
use crewchief_maproom::db::StoreCore;
use crewchief_maproom::db::{ChunkRecord, FileRecord};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

// Counter for unique test database names
static TEST_DB_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Create a shared in-memory SQLite store with migrations applied
async fn setup_test_store() -> SqliteStore {
    let counter = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_name = format!(
        "file:memdb_clean_ignored_{}?mode=memory&cache=shared",
        counter
    );
    let store = SqliteStore::connect(&db_name).await.unwrap();
    store.migrate().await.unwrap();
    store
}

/// Test helper: Create a test repository with .maproomignore file
async fn setup_test_repo_with_ignore(
    ignore_content: &str,
) -> Result<(TempDir, SqliteStore, String, String, i64, i64)> {
    // Create temp directory for test repo
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    // Write .maproomignore
    let ignore_path = repo_path.join(".maproomignore");
    let mut file = std::fs::File::create(&ignore_path)?;
    file.write_all(ignore_content.as_bytes())?;
    file.flush()?;

    // Create in-memory database for testing
    let store = setup_test_store().await;

    // Create repository and worktree
    let repo_name = "test-repo".to_string();
    let worktree_name = "main".to_string();
    let repo_id = store
        .get_or_create_repo(&repo_name, repo_path.to_str().unwrap())
        .await?;
    let worktree_id = store
        .get_or_create_worktree(repo_id, &worktree_name, repo_path.to_str().unwrap())
        .await?;

    Ok((
        temp_dir,
        store,
        repo_name,
        worktree_name,
        repo_id,
        worktree_id,
    ))
}

/// Test helper: Create a test chunk with given relpath
async fn create_test_chunk(
    store: &SqliteStore,
    repo_id: i64,
    worktree_id: i64,
    relpath: &str,
) -> i64 {
    // Create commit
    let commit_sha = format!("commit_{}", relpath.replace('/', "_"));
    let commit_id = store
        .get_or_create_commit(repo_id, &commit_sha, None)
        .await
        .unwrap();

    // Create file
    let file = FileRecord {
        repo_id,
        worktree_id,
        commit_id,
        relpath: relpath.to_string(),
        language: Some("rust".to_string()),
        content_hash: format!("hash_{}", relpath),
        size_bytes: 1000,
        last_modified: None,
    };
    let file_id = store.upsert_file(&file).await.unwrap();

    // Create chunk
    let chunk = ChunkRecord {
        file_id,
        blob_sha: format!("blob_sha_{}", relpath),
        symbol_name: Some(format!("test_symbol_{}", relpath)),
        kind: "function".to_string(),
        signature: None,
        docstring: None,
        start_line: 1,
        end_line: 10,
        preview: format!("preview for {}", relpath),
        ts_doc_text: format!("test function in {}", relpath),
        recency_score: 1.0,
        churn_score: 0.2,
        metadata: None,
        worktree_id,
    };
    store.insert_chunk(&chunk).await.unwrap()
}

/// Test helper: Check if chunk exists in database
async fn chunk_exists(store: &SqliteStore, chunk_id: i64) -> bool {
    store
        .run(move |conn| {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM chunks WHERE id = ?1)",
                    rusqlite::params![chunk_id],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            Ok(exists)
        })
        .await
        .unwrap()
}

/// Integration test: Clean-ignored removes chunks matching .maproomignore patterns
#[tokio::test]
async fn test_clean_ignored_removes_matching_chunks() -> Result<()> {
    // 1. Setup: Create test repository with files
    let (_temp_dir, store, repo_name, worktree_name, repo_id, worktree_id) =
        setup_test_repo_with_ignore("test/**\n*.log\n").await?;

    // 2. Index files: some matching patterns, some not
    let chunk1_id = create_test_chunk(&store, repo_id, worktree_id, "src/main.rs").await;
    let chunk2_id = create_test_chunk(&store, repo_id, worktree_id, "test/unit.rs").await;
    let chunk3_id = create_test_chunk(&store, repo_id, worktree_id, "test/integration.rs").await;
    let chunk4_id = create_test_chunk(&store, repo_id, worktree_id, "debug.log").await;
    let chunk5_id = create_test_chunk(&store, repo_id, worktree_id, "src/lib.rs").await;

    // Verify all chunks exist before cleanup
    assert!(chunk_exists(&store, chunk1_id).await);
    assert!(chunk_exists(&store, chunk2_id).await);
    assert!(chunk_exists(&store, chunk3_id).await);
    assert!(chunk_exists(&store, chunk4_id).await);
    assert!(chunk_exists(&store, chunk5_id).await);

    // 3. Run clean-ignored command
    clean_ignored(&store, &repo_name, &worktree_name, false).await?;

    // 4. Verify: chunks matching patterns are deleted
    // src/main.rs - does NOT match, should exist
    assert!(chunk_exists(&store, chunk1_id).await);

    // test/unit.rs - matches "test/**", should be deleted
    assert!(!chunk_exists(&store, chunk2_id).await);

    // test/integration.rs - matches "test/**", should be deleted
    assert!(!chunk_exists(&store, chunk3_id).await);

    // debug.log - matches "*.log", should be deleted
    assert!(!chunk_exists(&store, chunk4_id).await);

    // src/lib.rs - does NOT match, should exist
    assert!(chunk_exists(&store, chunk5_id).await);

    Ok(())
}

/// Integration test: Clean-ignored preserves non-matching chunks
#[tokio::test]
async fn test_clean_ignored_preserves_non_matching() -> Result<()> {
    // 1. Setup: Create test repository with ignore patterns
    let (_temp_dir, store, repo_name, worktree_name, repo_id, worktree_id) =
        setup_test_repo_with_ignore("*.tmp\nbuild/**\n").await?;

    // 2. Create chunks: some matching, some not matching
    let chunk1_id = create_test_chunk(&store, repo_id, worktree_id, "src/main.rs").await;
    let chunk2_id = create_test_chunk(&store, repo_id, worktree_id, "src/lib.rs").await;
    let chunk3_id = create_test_chunk(&store, repo_id, worktree_id, "data.tmp").await;
    let chunk4_id = create_test_chunk(&store, repo_id, worktree_id, "build/output.js").await;
    let chunk5_id = create_test_chunk(&store, repo_id, worktree_id, "README.md").await;

    // 3. Run clean-ignored command
    clean_ignored(&store, &repo_name, &worktree_name, false).await?;

    // 4. Verify: Only matching chunks are deleted, others preserved
    // src/main.rs - does NOT match, should exist
    assert!(chunk_exists(&store, chunk1_id).await);

    // src/lib.rs - does NOT match, should exist
    assert!(chunk_exists(&store, chunk2_id).await);

    // data.tmp - matches "*.tmp", should be deleted
    assert!(!chunk_exists(&store, chunk3_id).await);

    // build/output.js - matches "build/**", should be deleted
    assert!(!chunk_exists(&store, chunk4_id).await);

    // README.md - does NOT match, should exist
    assert!(chunk_exists(&store, chunk5_id).await);

    Ok(())
}

/// Integration test: Dry-run mode reports but doesn't delete
#[tokio::test]
async fn test_clean_ignored_dry_run() -> Result<()> {
    // 1. Setup: Create test repository with patterns
    let (_temp_dir, store, repo_name, worktree_name, repo_id, worktree_id) =
        setup_test_repo_with_ignore("*.log\ntest/**\n").await?;

    // 2. Create test chunks
    let chunk1_id = create_test_chunk(&store, repo_id, worktree_id, "src/main.rs").await;
    let chunk2_id = create_test_chunk(&store, repo_id, worktree_id, "test/unit.rs").await;
    let chunk3_id = create_test_chunk(&store, repo_id, worktree_id, "debug.log").await;

    // 3. Run clean-ignored in dry-run mode
    clean_ignored(&store, &repo_name, &worktree_name, true).await?;

    // 4. Verify: NO chunks are deleted (dry-run mode)
    assert!(chunk_exists(&store, chunk1_id).await);
    assert!(chunk_exists(&store, chunk2_id).await);
    assert!(chunk_exists(&store, chunk3_id).await);

    Ok(())
}

/// Integration test: Clean-ignored with no matching chunks
#[tokio::test]
async fn test_clean_ignored_no_matches() -> Result<()> {
    // 1. Setup: Create test repository with patterns that don't match anything
    let (_temp_dir, store, repo_name, worktree_name, repo_id, worktree_id) =
        setup_test_repo_with_ignore("nonexistent/**\n*.xyz\n").await?;

    // 2. Create chunks that don't match patterns
    let chunk1_id = create_test_chunk(&store, repo_id, worktree_id, "src/main.rs").await;
    let chunk2_id = create_test_chunk(&store, repo_id, worktree_id, "src/lib.rs").await;

    // 3. Run clean-ignored command
    clean_ignored(&store, &repo_name, &worktree_name, false).await?;

    // 4. Verify: All chunks still exist (none matched)
    assert!(chunk_exists(&store, chunk1_id).await);
    assert!(chunk_exists(&store, chunk2_id).await);

    Ok(())
}

/// Integration test: Clean-ignored with empty .maproomignore
#[tokio::test]
async fn test_clean_ignored_empty_ignore_file() -> Result<()> {
    // 1. Setup: Create test repository with empty .maproomignore
    let (_temp_dir, store, repo_name, worktree_name, repo_id, worktree_id) =
        setup_test_repo_with_ignore("").await?;

    // 2. Create test chunks
    let chunk1_id = create_test_chunk(&store, repo_id, worktree_id, "src/main.rs").await;
    let chunk2_id = create_test_chunk(&store, repo_id, worktree_id, "test/unit.rs").await;

    // 3. Run clean-ignored command (should be a no-op)
    clean_ignored(&store, &repo_name, &worktree_name, false).await?;

    // 4. Verify: All chunks still exist (empty ignore file)
    assert!(chunk_exists(&store, chunk1_id).await);
    assert!(chunk_exists(&store, chunk2_id).await);

    Ok(())
}

/// Integration test: Clean-ignored cleans up related data (edges, embeddings)
#[tokio::test]
async fn test_clean_ignored_cleans_related_data() -> Result<()> {
    // 1. Setup: Create test repository with patterns
    let (_temp_dir, store, repo_name, worktree_name, repo_id, worktree_id) =
        setup_test_repo_with_ignore("test/**\n").await?;

    // 2. Create test chunks with edges
    let chunk1_id = create_test_chunk(&store, repo_id, worktree_id, "src/main.rs").await;
    let chunk2_id = create_test_chunk(&store, repo_id, worktree_id, "test/unit.rs").await;

    // Create an edge between chunks (main.rs calls unit.rs)
    store
        .insert_chunk_edge(chunk1_id, chunk2_id, "calls")
        .await
        .unwrap();

    // Verify edge exists
    let edges_before = store
        .run(move |conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM chunk_edges WHERE src_chunk_id = ?1 OR dst_chunk_id = ?1",
                    rusqlite::params![chunk2_id],
                    |row| row.get(0),
                )
                .unwrap();
            Ok(count)
        })
        .await
        .unwrap();
    assert_eq!(edges_before, 1);

    // 3. Run clean-ignored command
    clean_ignored(&store, &repo_name, &worktree_name, false).await?;

    // 4. Verify: Chunk deleted and edges cleaned up
    assert!(!chunk_exists(&store, chunk2_id).await);

    // Verify edges involving deleted chunk are also removed
    let edges_after = store
        .run(move |conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM chunk_edges WHERE src_chunk_id = ?1 OR dst_chunk_id = ?1",
                    rusqlite::params![chunk2_id],
                    |row| row.get(0),
                )
                .unwrap();
            Ok(count)
        })
        .await
        .unwrap();
    assert_eq!(edges_after, 0);

    Ok(())
}
