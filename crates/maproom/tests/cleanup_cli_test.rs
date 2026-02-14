//! Integration tests for cleanup-stale CLI command execution logic
//!
//! Tests the three-phase workflow: detection → report → deletion
//! Uses in-memory SQLite database for testing.
//!
//! IDXCLEAN-3003: CLI Integration Tests

use anyhow::Result;
use crewchief_maproom::db::cleanup::{StaleWorktreeDetector, WorktreeCleaner};
use crewchief_maproom::db::sqlite::SqliteStore;
use crewchief_maproom::db::StoreChunks;
use crewchief_maproom::db::StoreCore;
use crewchief_maproom::db::StoreMigration;
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
        "file:memdb_cleanup_cli_{}?mode=memory&cache=shared",
        counter
    );
    let store = SqliteStore::connect(&db_name).await.unwrap();
    store.migrate().await.unwrap();
    store
}

/// Test helper: Create a test repo
async fn create_test_repo(store: &SqliteStore, name: &str) -> i64 {
    store
        .get_or_create_repo(name, &format!("/tmp/test/{}", name))
        .await
        .unwrap()
}

/// Test helper: Create a test worktree
async fn create_test_worktree(
    store: &SqliteStore,
    repo_id: i64,
    name: &str,
    abs_path: &str,
) -> i64 {
    store
        .get_or_create_worktree(repo_id, name, abs_path)
        .await
        .unwrap()
}

/// Test helper: Create a test chunk associated with a worktree
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
        preview: "test preview".to_string(),
        ts_doc_text: "test function".to_string(),
        recency_score: 1.0,
        churn_score: 0.2,
        metadata: None,
        worktree_id,
    };
    store.insert_chunk(&chunk).await.unwrap()
}

/// Test helper: Check if worktree exists
async fn worktree_exists(store: &SqliteStore, worktree_id: i64) -> bool {
    store
        .run(move |conn| {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM worktrees WHERE id = ?1)",
                    rusqlite::params![worktree_id],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            Ok(exists)
        })
        .await
        .unwrap()
}

/// Test dry-run mode: detection and reporting without deletion
#[tokio::test]
async fn test_cleanup_dry_run_no_deletion() -> Result<()> {
    // Setup: Connect to test database
    let store = setup_test_store().await;

    // Create a test repository
    let repo_id = create_test_repo(&store, "test-repo-dryrun").await;

    // Create a stale worktree (path that doesn't exist)
    let stale_worktree_id = create_test_worktree(
        &store,
        repo_id,
        "stale-branch-dryrun",
        "/tmp/nonexistent/path/dryrun",
    )
    .await;

    // Create a chunk for the worktree
    create_test_chunk(&store, repo_id, stale_worktree_id, "test.rs").await;

    // Phase 1: Detection
    let detector = StaleWorktreeDetector::new(&store);
    let stale = detector.detect_stale_worktrees().await?;

    // Verify detection found the stale worktree
    let found_stale = stale.iter().any(|wt| wt.id == stale_worktree_id);
    assert!(
        found_stale,
        "Detection phase should find the stale worktree"
    );

    // Phase 2: Simulate dry-run (no deletion) using dry_run mode
    let cleaner = WorktreeCleaner::new(&store, true); // dry_run = true
    let report = cleaner.cleanup_stale_worktrees(stale).await?;

    // Verify dry-run report
    assert_eq!(report.total_stale, 1);
    assert_eq!(report.deleted_count, 0); // No actual deletions in dry-run
    assert_eq!(report.chunks_cleaned, 0);

    // Phase 3: Verify worktree still exists in database
    assert!(
        worktree_exists(&store, stale_worktree_id).await,
        "Dry-run mode should not delete worktree from database"
    );

    Ok(())
}

/// Test confirm mode: actual deletion of stale worktrees
#[tokio::test]
async fn test_cleanup_confirm_deletes_worktrees() -> Result<()> {
    // Setup: Connect to test database
    let store = setup_test_store().await;

    // Create a test repository
    let repo_id = create_test_repo(&store, "test-repo-confirm").await;

    // Create a stale worktree (path that doesn't exist)
    let stale_worktree_id = create_test_worktree(
        &store,
        repo_id,
        "stale-branch-confirm",
        "/tmp/nonexistent/path/confirm",
    )
    .await;

    // Create a chunk for the worktree
    create_test_chunk(&store, repo_id, stale_worktree_id, "test.rs").await;

    // Phase 1: Detection
    let detector = StaleWorktreeDetector::new(&store);
    let stale = detector.detect_stale_worktrees().await?;

    // Verify detection found the stale worktree
    let found_stale = stale.iter().find(|wt| wt.id == stale_worktree_id).cloned();
    assert!(
        found_stale.is_some(),
        "Detection phase should find the stale worktree"
    );

    // Phase 2: Report (simulated - actual CLI would print to stdout)
    let stale_to_delete = vec![found_stale.unwrap()];

    // Phase 3: Deletion with confirmation
    let cleaner = WorktreeCleaner::new(&store, false); // dry_run = false
    let report = cleaner.cleanup_stale_worktrees(stale_to_delete).await?;

    // Verify cleanup report
    assert_eq!(report.total_stale, 1, "Should have 1 stale worktree");
    assert_eq!(report.deleted_count, 1, "Should have deleted 1 worktree");
    assert_eq!(report.failed_count, 0, "Should have no failures");

    // Phase 4: Verify worktree no longer exists in database
    assert!(
        !worktree_exists(&store, stale_worktree_id).await,
        "Confirm mode should delete worktree from database"
    );

    Ok(())
}

/// Test verbose mode: verify additional details are available
#[tokio::test]
async fn test_cleanup_verbose_mode() -> Result<()> {
    // Setup: Connect to test database
    let store = setup_test_store().await;

    // Create a test repository
    let repo_id = create_test_repo(&store, "test-repo-verbose").await;

    // Create a stale worktree
    let stale_worktree_id = create_test_worktree(
        &store,
        repo_id,
        "stale-branch-verbose",
        "/tmp/nonexistent/path/verbose",
    )
    .await;

    // Phase 1: Detection
    let detector = StaleWorktreeDetector::new(&store);
    let stale = detector.detect_stale_worktrees().await?;

    // Verify detection found the stale worktree and has verbose details
    let found_stale = stale.iter().find(|wt| wt.id == stale_worktree_id);
    assert!(found_stale.is_some(), "Should find stale worktree");

    let stale_wt = found_stale.unwrap();
    // Verify all verbose fields are populated
    assert_eq!(stale_wt.id, stale_worktree_id);
    assert_eq!(stale_wt.repo_id, repo_id);
    assert_eq!(stale_wt.name, "stale-branch-verbose");
    assert_eq!(stale_wt.abs_path, "/tmp/nonexistent/path/verbose");
    assert!(!stale_wt.exists, "Path should not exist");
    // chunk_count should be 0 since we didn't create any chunks
    assert_eq!(stale_wt.chunk_count, 0);

    Ok(())
}

/// Test error handling: database connection is already handled by SQLite setup
/// (This test validates that the store creation pattern works)
#[tokio::test]
async fn test_cleanup_valid_connection() {
    // Create a valid in-memory store
    let store = SqliteStore::connect(":memory:").await;
    assert!(store.is_ok(), "Should connect to in-memory database");

    let store = store.unwrap();
    let result = store.migrate().await;
    assert!(result.is_ok(), "Should migrate successfully");
}

/// Test exit code 2 scenario: no stale worktrees found
#[tokio::test]
async fn test_cleanup_no_stale_worktrees() -> Result<()> {
    // Setup: Connect to test database
    let store = setup_test_store().await;

    // Create a test repository
    let repo_id = create_test_repo(&store, "test-repo-clean").await;

    // Create a VALID worktree (with existing path)
    let temp_dir = TempDir::new()?;
    let valid_path = temp_dir.path().to_str().unwrap();
    create_test_worktree(&store, repo_id, "valid-branch", valid_path).await;

    // Phase 1: Detection on database with only valid worktrees
    let detector = StaleWorktreeDetector::new(&store);
    let stale = detector.detect_stale_worktrees().await?;

    // Filter to only non-existent paths (the actual stale ones)
    let actual_stale: Vec<_> = stale.into_iter().filter(|wt| !wt.exists).collect();

    // Verify: no stale worktrees found
    assert!(
        actual_stale.is_empty(),
        "No stale worktrees should be found for valid paths"
    );

    // In the CLI, this would result in exit code 2
    Ok(())
}

/// Test full workflow: detection → filtering → deletion
#[tokio::test]
async fn test_full_cleanup_workflow() -> Result<()> {
    // Setup: Connect to test database
    let store = setup_test_store().await;

    // Create a test repository
    let repo_id = create_test_repo(&store, "test-repo-workflow").await;

    // Create mix of valid and stale worktrees
    let temp_dir = TempDir::new()?;
    let valid_path = temp_dir.path().to_str().unwrap();

    let valid_id = create_test_worktree(&store, repo_id, "valid-main", valid_path).await;
    let stale1_id = create_test_worktree(
        &store,
        repo_id,
        "stale-feature1",
        "/tmp/nonexistent/feature1",
    )
    .await;
    let stale2_id = create_test_worktree(
        &store,
        repo_id,
        "stale-feature2",
        "/tmp/nonexistent/feature2",
    )
    .await;

    // Create chunks for stale worktrees
    create_test_chunk(&store, repo_id, stale1_id, "feature1.rs").await;
    create_test_chunk(&store, repo_id, stale2_id, "feature2.rs").await;

    // Step 1: Detection
    let detector = StaleWorktreeDetector::new(&store);
    let all_worktrees = detector.detect_stale_worktrees().await?;

    // Step 2: Filter to stale only
    let stale_worktrees: Vec<_> = all_worktrees.into_iter().filter(|wt| !wt.exists).collect();
    assert_eq!(stale_worktrees.len(), 2, "Should find 2 stale worktrees");

    // Step 3: Confirm and delete
    let cleaner = WorktreeCleaner::new(&store, false);
    let report = cleaner.cleanup_stale_worktrees(stale_worktrees).await?;

    // Verify report
    assert_eq!(report.total_stale, 2);
    assert_eq!(report.deleted_count, 2);
    assert_eq!(report.chunks_cleaned, 2);
    assert_eq!(report.failed_count, 0);

    // Verify stale worktrees deleted, valid preserved
    assert!(!worktree_exists(&store, stale1_id).await);
    assert!(!worktree_exists(&store, stale2_id).await);
    assert!(worktree_exists(&store, valid_id).await);

    Ok(())
}

/// Test chunk count accuracy in detection report
#[tokio::test]
async fn test_chunk_count_in_report() -> Result<()> {
    // Setup: Connect to test database
    let store = setup_test_store().await;

    // Create a test repository
    let repo_id = create_test_repo(&store, "test-repo-chunks").await;

    // Create a stale worktree with multiple chunks
    let stale_id =
        create_test_worktree(&store, repo_id, "stale-multi", "/tmp/nonexistent/multi").await;

    // Create 5 chunks
    for i in 0..5 {
        create_test_chunk(&store, repo_id, stale_id, &format!("file{}.rs", i)).await;
    }

    // Detection
    let detector = StaleWorktreeDetector::new(&store);
    let stale = detector.detect_stale_worktrees().await?;

    // Find our worktree
    let found = stale.iter().find(|wt| wt.id == stale_id);
    assert!(found.is_some());

    // Verify chunk count
    let wt = found.unwrap();
    assert_eq!(wt.chunk_count, 5, "Should report 5 chunks");

    Ok(())
}

/// Test report success rate calculation
#[tokio::test]
async fn test_report_success_rate() -> Result<()> {
    // Setup: Connect to test database
    let store = setup_test_store().await;

    // Create a test repository
    let repo_id = create_test_repo(&store, "test-repo-rate").await;

    // Create 3 stale worktrees
    let stale1_id = create_test_worktree(&store, repo_id, "stale1", "/tmp/nonexistent/s1").await;
    let stale2_id = create_test_worktree(&store, repo_id, "stale2", "/tmp/nonexistent/s2").await;
    let stale3_id = create_test_worktree(&store, repo_id, "stale3", "/tmp/nonexistent/s3").await;

    // Detection
    let detector = StaleWorktreeDetector::new(&store);
    let stale = detector.detect_stale_worktrees().await?;

    // Filter to our worktrees
    let our_stale: Vec<_> = stale
        .into_iter()
        .filter(|wt| [stale1_id, stale2_id, stale3_id].contains(&wt.id))
        .collect();

    // Delete all
    let cleaner = WorktreeCleaner::new(&store, false);
    let report = cleaner.cleanup_stale_worktrees(our_stale).await?;

    // Verify success rate
    assert_eq!(report.success_rate(), 1.0);
    assert!(!report.has_failures());

    Ok(())
}
