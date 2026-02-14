//! IDXCLEAN-1001: Stale Worktree Detection Integration Tests
//!
//! Integration tests for stale worktree detection using SQLite in-memory database.
//!
//! Tests verify:
//! - Detection of worktrees with non-existent paths
//! - Preservation of worktrees with valid paths
//! - Parallel validation performance
//! - Error handling for edge cases

use anyhow::Result;
use crewchief_maproom::db::cleanup::StaleWorktreeDetector;
use crewchief_maproom::db::sqlite::SqliteStore;
use crewchief_maproom::db::StoreCore;
use crewchief_maproom::db::{ChunkRecord, FileRecord};
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

// Counter for unique test database names
static TEST_DB_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Create a shared in-memory SQLite store with migrations applied
/// Uses file:memdb?mode=memory&cache=shared for proper pooled connection support
async fn setup_test_store() -> SqliteStore {
    let counter = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_name = format!(
        "file:memdb_cleanup_detect_{}?mode=memory&cache=shared",
        counter
    );
    let store = SqliteStore::connect(&db_name).await.unwrap();
    store.migrate().await.unwrap();
    store
}

/// Insert test data: repo, worktree, and chunks
/// Returns (repo_id, worktree_id)
async fn insert_test_data(
    store: &SqliteStore,
    worktree_name: &str,
    abs_path: &str,
    chunk_count: i32,
) -> Result<(i64, i64)> {
    // Create repo (shared across all worktrees in a test)
    let repo_id = store
        .get_or_create_repo("test-repo", "/tmp/test-repo")
        .await?;

    // Create worktree
    let worktree_id = store
        .get_or_create_worktree(repo_id, worktree_name, abs_path)
        .await?;

    // Create commit (unique per worktree)
    let commit_sha = format!("commit-{}", worktree_name.replace('-', ""));
    let commit_id = store
        .get_or_create_commit(repo_id, &commit_sha, None)
        .await?;

    // Create file (unique per worktree using worktree_name)
    let file = FileRecord {
        repo_id,
        worktree_id,
        commit_id,
        relpath: format!("src/{}/test.rs", worktree_name),
        language: Some("rust".to_string()),
        content_hash: format!("hash_{}", worktree_name),
        size_bytes: 1000,
        last_modified: None,
    };
    let file_id = store.upsert_file(&file).await?;

    // Create chunks with worktree association via junction table
    for i in 0..chunk_count {
        let chunk = ChunkRecord {
            file_id,
            blob_sha: format!("blob_sha_{}_{}", worktree_name, i),
            symbol_name: Some(format!("test_func_{}_{}", worktree_name, i)),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: i * 10,
            end_line: (i + 1) * 10,
            preview: format!("fn test_func_{}() {{}}", i),
            ts_doc_text: format!("test function {} {}", worktree_name, i),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
            worktree_id,
        };
        store.insert_chunk(&chunk).await?;
    }

    Ok((repo_id, worktree_id))
}

#[tokio::test]
async fn test_detects_stale_worktree() -> Result<()> {
    // Setup test database
    let store = setup_test_store().await;

    // Insert test data with non-existent path
    let non_existent_path = "/tmp/non_existent_worktree_12345";
    insert_test_data(&store, "test-branch", non_existent_path, 5).await?;

    // Run detection
    let detector = StaleWorktreeDetector::new(&store);
    let stale_worktrees = detector.detect_stale_worktrees().await?;

    // Verify: should find 1 stale worktree
    assert_eq!(
        stale_worktrees.len(),
        1,
        "Expected 1 stale worktree, found {}",
        stale_worktrees.len()
    );

    let stale = &stale_worktrees[0];
    assert_eq!(stale.name, "test-branch");
    assert_eq!(stale.abs_path, non_existent_path);
    assert!(!stale.exists, "Worktree should be marked as not existing");
    assert_eq!(stale.chunk_count, 5, "Expected 5 chunks");

    Ok(())
}

#[tokio::test]
async fn test_preserves_valid_worktree() -> Result<()> {
    // Setup test database
    let store = setup_test_store().await;

    // Create a temporary directory that exists
    let temp_dir = TempDir::new()?;
    let existing_path = temp_dir.path().to_str().unwrap();

    // Insert test data with existing path
    insert_test_data(&store, "test-branch", existing_path, 3).await?;

    // Run detection
    let detector = StaleWorktreeDetector::new(&store);
    let stale_worktrees = detector.detect_stale_worktrees().await?;

    // Verify: should find 0 stale worktrees (the path exists)
    assert_eq!(
        stale_worktrees.len(),
        0,
        "Expected 0 stale worktrees for existing path, found {}",
        stale_worktrees.len()
    );

    Ok(())
}

#[tokio::test]
async fn test_mixed_worktrees() -> Result<()> {
    // Setup test database
    let store = setup_test_store().await;

    // Use UUID-like unique prefixes for worktree names to avoid conflicts
    let uuid = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    // Create one valid worktree
    let temp_dir = TempDir::new()?;
    let valid_path = temp_dir.path().to_str().unwrap();
    insert_test_data(&store, &format!("valid-{}", uuid), valid_path, 10).await?;

    // Create two stale worktrees with unique names
    insert_test_data(
        &store,
        &format!("stale1-{}", uuid),
        "/tmp/stale_worktree_1_xyz",
        5,
    )
    .await?;
    insert_test_data(
        &store,
        &format!("stale2-{}", uuid),
        "/tmp/stale_worktree_2_xyz",
        8,
    )
    .await?;

    // Run detection
    let detector = StaleWorktreeDetector::new(&store);
    let stale_worktrees = detector.detect_stale_worktrees().await?;

    // Verify: should find 2 stale worktrees
    assert_eq!(
        stale_worktrees.len(),
        2,
        "Expected 2 stale worktrees, found {}",
        stale_worktrees.len()
    );

    // Verify chunk counts are correct
    let total_chunks: i64 = stale_worktrees.iter().map(|w| w.chunk_count).sum();
    assert_eq!(
        total_chunks, 13,
        "Expected 13 total chunks in stale worktrees"
    );

    Ok(())
}

#[tokio::test]
async fn test_empty_database() -> Result<()> {
    // Setup test database
    let store = setup_test_store().await;

    // Run detection on empty database
    let detector = StaleWorktreeDetector::new(&store);
    let stale_worktrees = detector.detect_stale_worktrees().await?;

    // Verify: should find 0 stale worktrees
    assert_eq!(
        stale_worktrees.len(),
        0,
        "Expected 0 stale worktrees in empty database"
    );

    Ok(())
}

#[tokio::test]
async fn test_worktree_with_no_chunks() -> Result<()> {
    // Setup test database
    let store = setup_test_store().await;

    // Create repo
    let repo_id = store
        .get_or_create_repo("test-repo", "/tmp/test-repo")
        .await?;

    // Create worktree with no chunks
    store
        .get_or_create_worktree(repo_id, "empty-branch", "/tmp/non_existent_empty")
        .await?;

    // Run detection
    let detector = StaleWorktreeDetector::new(&store);
    let stale_worktrees = detector.detect_stale_worktrees().await?;

    // Verify: should find 1 stale worktree with 0 chunks
    assert_eq!(stale_worktrees.len(), 1);
    assert_eq!(stale_worktrees[0].chunk_count, 0);
    assert_eq!(stale_worktrees[0].name, "empty-branch");

    Ok(())
}

#[tokio::test]
async fn test_parallel_performance() -> Result<()> {
    // Setup test database
    let store = setup_test_store().await;

    // Insert 50 worktrees (a reasonable test size) with unique names
    for i in 0..50 {
        insert_test_data(
            &store,
            &format!("test-branch-{}", i),
            &format!("/tmp/test_worktree_{}", i),
            2,
        )
        .await?;
    }

    // Run detection and measure time
    let start = std::time::Instant::now();
    let detector = StaleWorktreeDetector::new(&store);
    let stale_worktrees = detector.detect_stale_worktrees().await?;
    let duration = start.elapsed();

    // Verify results
    assert_eq!(stale_worktrees.len(), 50, "Expected 50 stale worktrees");

    // Verify performance: should complete in under 2 seconds for 50 worktrees
    // (Target is <1s for 100, so 50 should be well under 1s, using 2s as safe margin)
    assert!(
        duration.as_secs() < 2,
        "Detection took too long: {:?} (expected < 2s)",
        duration
    );

    println!("Performance test: detected 50 worktrees in {:?}", duration);

    Ok(())
}
