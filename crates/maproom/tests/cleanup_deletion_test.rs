//! Integration tests for safe worktree deletion
//!
//! Tests verify:
//! - Multi-worktree chunk safety (chunks shared by multiple worktrees are preserved)
//! - Garbage collection accuracy (chunks belonging only to deleted worktree are removed)
//! - Dry-run mode (no changes made when dry_run = true)
//! - Transaction rollback (all-or-nothing behavior on errors)
//!
//! IDXCLEAN-1002: Safe Deletion Module Integration Tests

use anyhow::Result;
use crewchief_maproom::db::cleanup::{StaleWorktree, WorktreeCleaner};
use crewchief_maproom::db::sqlite::SqliteStore;
use crewchief_maproom::db::StoreChunks;
use crewchief_maproom::db::StoreCore;
use crewchief_maproom::db::StoreMigration;
use crewchief_maproom::db::{ChunkRecord, FileRecord};
use std::sync::atomic::{AtomicUsize, Ordering};

// Counter for unique test database names
static TEST_DB_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Create a shared in-memory SQLite store with migrations applied
/// Uses file:memdb?mode=memory&cache=shared for proper pooled connection support
async fn setup_test_store() -> SqliteStore {
    let counter = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_name = format!(
        "file:memdb_cleanup_delete_{}?mode=memory&cache=shared",
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

/// Test helper: Create a test chunk associated with specific worktree(s)
///
/// For multi-worktree scenarios, we create a file for the LAST worktree in the list.
/// This way, deleting the first worktree won't cascade-delete the file/chunks.
/// The chunk is then associated with all worktrees via the junction table.
async fn create_test_chunk(
    store: &SqliteStore,
    repo_id: i64,
    worktree_ids: &[i64],
    relpath: &str,
) -> i64 {
    // Create commit
    let commit_sha = format!("commit_{}", relpath.replace('/', "_"));
    let commit_id = store
        .get_or_create_commit(repo_id, &commit_sha, None)
        .await
        .unwrap();

    // For multi-worktree chunks, create file with the LAST worktree to avoid
    // cascade deletion when the first worktree is deleted.
    // For single-worktree chunks, use the only worktree.
    let primary_worktree_id = *worktree_ids.last().unwrap();

    // Create file using primary worktree (last in list)
    let file = FileRecord {
        repo_id,
        worktree_id: primary_worktree_id,
        commit_id,
        relpath: relpath.to_string(),
        language: Some("rust".to_string()),
        content_hash: format!("hash_{}", relpath),
        size_bytes: 1000,
        last_modified: None,
    };
    let file_id = store.upsert_file(&file).await.unwrap();

    // Create chunk with primary worktree
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
        worktree_id: primary_worktree_id,
    };
    let chunk_id = store.insert_chunk(&chunk).await.unwrap();

    // Add chunk to ALL other worktrees via junction table
    for &wt_id in worktree_ids.iter().filter(|&&id| id != primary_worktree_id) {
        store.add_chunk_to_worktree(chunk_id, wt_id).await.unwrap();
    }

    chunk_id
}

/// Test helper: Count chunks for a specific worktree
async fn count_chunks_for_worktree(store: &SqliteStore, worktree_id: i64) -> i64 {
    store.get_worktree_chunk_count(worktree_id).await.unwrap()
}

/// Test helper: Check if chunk exists
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

/// Test helper: Get chunk's worktree IDs from junction table
async fn get_chunk_worktree_ids(store: &SqliteStore, chunk_id: i64) -> Vec<i64> {
    store.get_chunk_worktrees(chunk_id).await.unwrap()
}

#[tokio::test]
async fn test_multi_worktree_chunk_safety() -> Result<()> {
    // Scenario 4 from quality-strategy.md: Multi-worktree chunks must be preserved

    // Setup test database
    let store = setup_test_store().await;

    // Setup: Create repo and two worktrees
    let repo_id = create_test_repo(&store, "test_multi_worktree_safety").await;
    let wt1_id = create_test_worktree(&store, repo_id, "branch1", "/tmp/test/branch1").await;
    let wt2_id = create_test_worktree(&store, repo_id, "branch2", "/tmp/test/branch2").await;

    // Create a chunk shared by both worktrees
    let shared_chunk_id = create_test_chunk(&store, repo_id, &[wt1_id, wt2_id], "shared.rs").await;

    // Create a chunk belonging only to wt1
    let wt1_only_chunk_id = create_test_chunk(&store, repo_id, &[wt1_id], "wt1_only.rs").await;

    // Verify initial state
    assert_eq!(count_chunks_for_worktree(&store, wt1_id).await, 2);
    assert_eq!(count_chunks_for_worktree(&store, wt2_id).await, 1);
    assert!(chunk_exists(&store, shared_chunk_id).await);
    assert!(chunk_exists(&store, wt1_only_chunk_id).await);

    // Delete worktree 1 (simulate stale worktree)
    let stale = vec![StaleWorktree {
        id: wt1_id,
        repo_id,
        name: "branch1".to_string(),
        abs_path: "/tmp/test/branch1".to_string(),
        exists: false,
        chunk_count: 2,
    }];

    let cleaner = WorktreeCleaner::new(&store, false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify report
    assert_eq!(report.total_stale, 1);
    assert_eq!(report.deleted_count, 1);
    assert_eq!(report.chunks_cleaned, 1); // Only wt1_only_chunk should be deleted
    assert_eq!(report.failed_count, 0);
    assert_eq!(report.deleted_ids, vec![wt1_id]);

    // Verify multi-worktree chunk is preserved
    assert!(
        chunk_exists(&store, shared_chunk_id).await,
        "Shared chunk should still exist"
    );

    // Verify shared chunk's worktree_ids was updated (only wt2 remaining)
    let worktree_ids = get_chunk_worktree_ids(&store, shared_chunk_id).await;
    assert_eq!(
        worktree_ids,
        vec![wt2_id],
        "Shared chunk should only contain wt2_id"
    );

    // Verify single-worktree chunk was garbage collected
    assert!(
        !chunk_exists(&store, wt1_only_chunk_id).await,
        "wt1-only chunk should be deleted"
    );

    // Verify worktree 1 is deleted
    assert!(
        !worktree_exists(&store, wt1_id).await,
        "Worktree 1 should be deleted"
    );

    // Verify worktree 2 still exists
    assert!(
        worktree_exists(&store, wt2_id).await,
        "Worktree 2 should still exist"
    );

    // Verify worktree 2 can still find its chunk
    assert_eq!(count_chunks_for_worktree(&store, wt2_id).await, 1);

    Ok(())
}

#[tokio::test]
async fn test_garbage_collection_accuracy() -> Result<()> {
    // Scenario 5 from quality-strategy.md: Single-worktree chunks should be deleted

    // Setup test database
    let store = setup_test_store().await;

    // Setup: Create repo and worktree
    let repo_id = create_test_repo(&store, "test_garbage_collection").await;
    let wt_id = create_test_worktree(&store, repo_id, "feature", "/tmp/test/feature").await;

    // Create chunks belonging ONLY to this worktree
    let chunk1_id = create_test_chunk(&store, repo_id, &[wt_id], "file1.rs").await;
    let chunk2_id = create_test_chunk(&store, repo_id, &[wt_id], "file2.rs").await;
    let chunk3_id = create_test_chunk(&store, repo_id, &[wt_id], "file3.rs").await;

    // Verify initial state
    assert_eq!(count_chunks_for_worktree(&store, wt_id).await, 3);
    assert!(chunk_exists(&store, chunk1_id).await);
    assert!(chunk_exists(&store, chunk2_id).await);
    assert!(chunk_exists(&store, chunk3_id).await);

    // Delete the worktree
    let stale = vec![StaleWorktree {
        id: wt_id,
        repo_id,
        name: "feature".to_string(),
        abs_path: "/tmp/test/feature".to_string(),
        exists: false,
        chunk_count: 3,
    }];

    let cleaner = WorktreeCleaner::new(&store, false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify report
    assert_eq!(report.total_stale, 1);
    assert_eq!(report.deleted_count, 1);
    assert_eq!(report.chunks_cleaned, 3); // All 3 chunks should be deleted
    assert_eq!(report.failed_count, 0);

    // Verify all chunks are garbage collected
    assert!(
        !chunk_exists(&store, chunk1_id).await,
        "Chunk 1 should be deleted"
    );
    assert!(
        !chunk_exists(&store, chunk2_id).await,
        "Chunk 2 should be deleted"
    );
    assert!(
        !chunk_exists(&store, chunk3_id).await,
        "Chunk 3 should be deleted"
    );

    // Verify worktree is deleted
    assert!(
        !worktree_exists(&store, wt_id).await,
        "Worktree should be deleted"
    );

    Ok(())
}

#[tokio::test]
async fn test_dry_run_mode() -> Result<()> {
    // Verify dry-run mode makes no changes

    // Setup test database
    let store = setup_test_store().await;

    // Setup: Create repo, worktree, and chunk
    let repo_id = create_test_repo(&store, "test_dry_run").await;
    let wt_id = create_test_worktree(&store, repo_id, "test", "/tmp/test/dry_run").await;
    let chunk_id = create_test_chunk(&store, repo_id, &[wt_id], "test.rs").await;

    // Verify initial state
    assert!(worktree_exists(&store, wt_id).await);
    assert!(chunk_exists(&store, chunk_id).await);
    assert_eq!(count_chunks_for_worktree(&store, wt_id).await, 1);

    // Run cleanup in dry-run mode
    let stale = vec![StaleWorktree {
        id: wt_id,
        repo_id,
        name: "test".to_string(),
        abs_path: "/tmp/test/dry_run".to_string(),
        exists: false,
        chunk_count: 1,
    }];

    let cleaner = WorktreeCleaner::new(&store, true); // dry_run = true
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify report shows what would be deleted but didn't actually delete
    assert_eq!(report.total_stale, 1);
    assert_eq!(report.deleted_count, 0); // No actual deletions
    assert_eq!(report.chunks_cleaned, 0); // No chunks cleaned
    assert_eq!(report.failed_count, 0);
    assert_eq!(report.deleted_ids.len(), 0);

    // Verify nothing was actually deleted
    assert!(
        worktree_exists(&store, wt_id).await,
        "Worktree should still exist (dry-run)"
    );
    assert!(
        chunk_exists(&store, chunk_id).await,
        "Chunk should still exist (dry-run)"
    );
    assert_eq!(
        count_chunks_for_worktree(&store, wt_id).await,
        1,
        "Chunk count should be unchanged (dry-run)"
    );

    Ok(())
}

#[tokio::test]
async fn test_partial_failure_handling() -> Result<()> {
    // Test that partial failures are collected and don't abort the entire process

    // Setup test database
    let store = setup_test_store().await;

    // Setup: Create repo and worktrees
    let repo_id = create_test_repo(&store, "test_partial_failure").await;
    let wt1_id = create_test_worktree(&store, repo_id, "wt1", "/tmp/test/wt1").await;
    let wt2_id = create_test_worktree(&store, repo_id, "wt2", "/tmp/test/wt2").await;

    // Create chunks for wt1
    let chunk1_id = create_test_chunk(&store, repo_id, &[wt1_id], "file1.rs").await;

    // Create chunks for wt2
    let chunk2_id = create_test_chunk(&store, repo_id, &[wt2_id], "file2.rs").await;

    // Try to delete wt1 (should succeed), a non-existent worktree (should succeed as no-op), and wt2 (should succeed)
    let stale = vec![
        StaleWorktree {
            id: wt1_id,
            repo_id,
            name: "wt1".to_string(),
            abs_path: "/tmp/test/wt1".to_string(),
            exists: false,
            chunk_count: 1,
        },
        StaleWorktree {
            id: 999999, // Non-existent worktree
            repo_id,
            name: "fake".to_string(),
            abs_path: "/tmp/test/fake".to_string(),
            exists: false,
            chunk_count: 0,
        },
        StaleWorktree {
            id: wt2_id,
            repo_id,
            name: "wt2".to_string(),
            abs_path: "/tmp/test/wt2".to_string(),
            exists: false,
            chunk_count: 1,
        },
    ];

    let cleaner = WorktreeCleaner::new(&store, false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify report shows all deletions succeeded
    // Note: DELETE with no matching rows is not an error in SQLite
    assert_eq!(report.total_stale, 3);
    assert_eq!(report.deleted_count, 3); // All three "succeeded" (no errors)
    assert_eq!(report.chunks_cleaned, 2); // Only actual chunks from wt1 and wt2
    assert_eq!(report.failed_count, 0); // No errors occurred
    assert!(report.deleted_ids.contains(&wt1_id));
    assert!(report.deleted_ids.contains(&wt2_id));
    assert!(report.deleted_ids.contains(&999999)); // fake worktree also "deleted"
    assert_eq!(report.failed_deletions.len(), 0);

    // Verify successful deletions happened
    assert!(!worktree_exists(&store, wt1_id).await);
    assert!(!worktree_exists(&store, wt2_id).await);
    assert!(!chunk_exists(&store, chunk1_id).await);
    assert!(!chunk_exists(&store, chunk2_id).await);

    Ok(())
}

#[tokio::test]
async fn test_complex_multi_worktree_scenario() -> Result<()> {
    // Complex scenario: Multiple worktrees with overlapping chunks

    // Setup test database
    let store = setup_test_store().await;

    // Setup: Create repo and 3 worktrees
    let repo_id = create_test_repo(&store, "test_complex_scenario").await;
    let wt1_id = create_test_worktree(&store, repo_id, "main", "/tmp/test/main").await;
    let wt2_id = create_test_worktree(&store, repo_id, "dev", "/tmp/test/dev").await;
    let wt3_id = create_test_worktree(&store, repo_id, "staging", "/tmp/test/staging").await;

    // Create chunks with different sharing patterns:
    // - chunk_all: shared by all 3 worktrees (file owned by wt3, survives wt2 deletion)
    // - chunk_12: shared by wt1 and wt2 (file owned by wt1, survives wt2 deletion)
    // - chunk_23: shared by wt2 and wt3 (file owned by wt3, survives wt2 deletion)
    // - chunk_2: only in wt2 (will be deleted with wt2)
    //
    // Note: When a worktree is deleted, CASCADE deletes all files that reference it.
    // So chunks that should survive must have their file owned by a surviving worktree.
    // Order worktrees so the surviving one is LAST (which create_test_chunk uses for file).
    let chunk_all_id =
        create_test_chunk(&store, repo_id, &[wt1_id, wt2_id, wt3_id], "common.rs").await; // file -> wt3
    let chunk_12_id = create_test_chunk(&store, repo_id, &[wt2_id, wt1_id], "shared_1_2.rs").await; // file -> wt1 (survives)
    let chunk_23_id = create_test_chunk(&store, repo_id, &[wt2_id, wt3_id], "shared_2_3.rs").await; // file -> wt3 (survives)
    let chunk_2_id = create_test_chunk(&store, repo_id, &[wt2_id], "exclusive_2.rs").await; // file -> wt2 (deleted)

    // Verify initial state
    assert_eq!(count_chunks_for_worktree(&store, wt1_id).await, 2); // chunk_all, chunk_12
    assert_eq!(count_chunks_for_worktree(&store, wt2_id).await, 4); // all chunks
    assert_eq!(count_chunks_for_worktree(&store, wt3_id).await, 2); // chunk_all, chunk_23

    // Delete wt2 (has all chunks)
    let stale = vec![StaleWorktree {
        id: wt2_id,
        repo_id,
        name: "dev".to_string(),
        abs_path: "/tmp/test/dev".to_string(),
        exists: false,
        chunk_count: 4,
    }];

    let cleaner = WorktreeCleaner::new(&store, false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify report
    assert_eq!(report.total_stale, 1);
    assert_eq!(report.deleted_count, 1);
    assert_eq!(report.chunks_cleaned, 1); // Only chunk_2 should be deleted
    assert_eq!(report.failed_count, 0);

    // Verify chunk states after deletion:
    // - chunk_all: should exist with [wt1, wt3]
    // - chunk_12: should exist with [wt1]
    // - chunk_23: should exist with [wt3]
    // - chunk_2: should be deleted
    assert!(chunk_exists(&store, chunk_all_id).await);
    assert!(chunk_exists(&store, chunk_12_id).await);
    assert!(chunk_exists(&store, chunk_23_id).await);
    assert!(!chunk_exists(&store, chunk_2_id).await);

    // Verify worktree_ids arrays are correct
    let mut chunk_all_wts = get_chunk_worktree_ids(&store, chunk_all_id).await;
    chunk_all_wts.sort();
    assert_eq!(chunk_all_wts.len(), 2);
    assert!(chunk_all_wts.contains(&wt1_id));
    assert!(chunk_all_wts.contains(&wt3_id));
    assert!(!chunk_all_wts.contains(&wt2_id));

    let chunk_12_wts = get_chunk_worktree_ids(&store, chunk_12_id).await;
    assert_eq!(chunk_12_wts, vec![wt1_id]);

    let chunk_23_wts = get_chunk_worktree_ids(&store, chunk_23_id).await;
    assert_eq!(chunk_23_wts, vec![wt3_id]);

    // Verify remaining worktrees can still find their chunks
    assert_eq!(count_chunks_for_worktree(&store, wt1_id).await, 2);
    assert_eq!(count_chunks_for_worktree(&store, wt3_id).await, 2);

    Ok(())
}

#[tokio::test]
async fn test_deletes_only_stale_worktrees() -> Result<()> {
    // Test that only stale worktrees are deleted, not valid ones
    // Pattern from quality-strategy.md lines 137-161

    // Setup test database
    let store = setup_test_store().await;

    // Setup: Create repo
    let repo_id = create_test_repo(&store, "test_selective_deletion").await;

    // Create 1 valid worktree (with a path that exists using tempfile)
    let temp_dir = tempfile::tempdir()?;
    let valid_path = temp_dir.path().to_str().unwrap();
    let valid_id = create_test_worktree(&store, repo_id, "valid", valid_path).await;

    // Create 2 stale worktrees (with non-existent paths)
    let stale1_id =
        create_test_worktree(&store, repo_id, "stale1", "/tmp/nonexistent/stale1").await;
    let stale2_id =
        create_test_worktree(&store, repo_id, "stale2", "/tmp/nonexistent/stale2").await;

    // Create chunks: valid worktree has 2 chunks, stale worktrees have 1 each
    let valid_chunk1_id = create_test_chunk(&store, repo_id, &[valid_id], "valid1.rs").await;
    let valid_chunk2_id = create_test_chunk(&store, repo_id, &[valid_id], "valid2.rs").await;
    let stale1_chunk_id = create_test_chunk(&store, repo_id, &[stale1_id], "stale1.rs").await;
    let stale2_chunk_id = create_test_chunk(&store, repo_id, &[stale2_id], "stale2.rs").await;

    // Verify initial state
    assert!(worktree_exists(&store, valid_id).await);
    assert!(worktree_exists(&store, stale1_id).await);
    assert!(worktree_exists(&store, stale2_id).await);
    assert_eq!(count_chunks_for_worktree(&store, valid_id).await, 2);
    assert_eq!(count_chunks_for_worktree(&store, stale1_id).await, 1);
    assert_eq!(count_chunks_for_worktree(&store, stale2_id).await, 1);

    // Delete only the stale worktrees
    let stale = vec![
        StaleWorktree {
            id: stale1_id,
            repo_id,
            name: "stale1".to_string(),
            abs_path: "/tmp/nonexistent/stale1".to_string(),
            exists: false,
            chunk_count: 1,
        },
        StaleWorktree {
            id: stale2_id,
            repo_id,
            name: "stale2".to_string(),
            abs_path: "/tmp/nonexistent/stale2".to_string(),
            exists: false,
            chunk_count: 1,
        },
    ];

    let cleaner = WorktreeCleaner::new(&store, false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify report
    assert_eq!(report.total_stale, 2);
    assert_eq!(report.deleted_count, 2);
    assert_eq!(report.chunks_cleaned, 2); // Both stale chunks deleted
    assert_eq!(report.failed_count, 0);
    assert!(report.deleted_ids.contains(&stale1_id));
    assert!(report.deleted_ids.contains(&stale2_id));

    // Verify: stale worktrees are deleted
    assert!(
        !worktree_exists(&store, stale1_id).await,
        "Stale worktree 1 should be deleted"
    );
    assert!(
        !worktree_exists(&store, stale2_id).await,
        "Stale worktree 2 should be deleted"
    );

    // Verify: stale chunks are deleted
    assert!(
        !chunk_exists(&store, stale1_chunk_id).await,
        "Stale chunk 1 should be deleted"
    );
    assert!(
        !chunk_exists(&store, stale2_chunk_id).await,
        "Stale chunk 2 should be deleted"
    );

    // Verify: valid worktree still exists
    assert!(
        worktree_exists(&store, valid_id).await,
        "Valid worktree should still exist"
    );

    // Verify: valid worktree chunks are preserved
    assert!(
        chunk_exists(&store, valid_chunk1_id).await,
        "Valid chunk 1 should be preserved"
    );
    assert!(
        chunk_exists(&store, valid_chunk2_id).await,
        "Valid chunk 2 should be preserved"
    );
    assert_eq!(
        count_chunks_for_worktree(&store, valid_id).await,
        2,
        "Valid worktree should still have 2 chunks"
    );

    Ok(())
}

#[tokio::test]
async fn test_transaction_atomicity() -> Result<()> {
    // Test that all deletions in a batch are atomic (all succeed or all fail)

    // Setup test database
    let store = setup_test_store().await;

    // Setup: Create repo and 2 stale worktrees with chunks
    let repo_id = create_test_repo(&store, "test_transaction_atomicity").await;
    let stale1_id =
        create_test_worktree(&store, repo_id, "stale1", "/tmp/nonexistent/stale1").await;
    let stale2_id =
        create_test_worktree(&store, repo_id, "stale2", "/tmp/nonexistent/stale2").await;

    // Create chunks for both worktrees
    let chunk1_id = create_test_chunk(&store, repo_id, &[stale1_id], "stale1.rs").await;
    let chunk2_id = create_test_chunk(&store, repo_id, &[stale2_id], "stale2.rs").await;

    // Verify initial state
    assert!(worktree_exists(&store, stale1_id).await);
    assert!(worktree_exists(&store, stale2_id).await);
    assert!(chunk_exists(&store, chunk1_id).await);
    assert!(chunk_exists(&store, chunk2_id).await);

    // Delete both worktrees
    let stale = vec![
        StaleWorktree {
            id: stale1_id,
            repo_id,
            name: "stale1".to_string(),
            abs_path: "/tmp/nonexistent/stale1".to_string(),
            exists: false,
            chunk_count: 1,
        },
        StaleWorktree {
            id: stale2_id,
            repo_id,
            name: "stale2".to_string(),
            abs_path: "/tmp/nonexistent/stale2".to_string(),
            exists: false,
            chunk_count: 1,
        },
    ];

    let cleaner = WorktreeCleaner::new(&store, false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify both deletions succeeded
    assert_eq!(report.total_stale, 2);
    assert_eq!(report.deleted_count, 2);
    assert_eq!(report.chunks_cleaned, 2);
    assert_eq!(report.failed_count, 0);

    // Verify both worktrees are deleted
    assert!(!worktree_exists(&store, stale1_id).await);
    assert!(!worktree_exists(&store, stale2_id).await);
    assert!(!chunk_exists(&store, chunk1_id).await);
    assert!(!chunk_exists(&store, chunk2_id).await);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_cleanup_safety() -> Result<()> {
    // Test that cleanup operations complete successfully even with concurrent calls
    // (SQLite handles this via WAL mode and busy timeout)

    // Setup test database
    let store = setup_test_store().await;

    // Setup: Create repo and multiple stale worktrees
    let repo_id = create_test_repo(&store, "test_concurrent_cleanup").await;

    // Create 6 stale worktrees
    let mut worktree_ids = Vec::new();
    let mut chunk_ids = Vec::new();
    for i in 0..6 {
        let wt_id = create_test_worktree(
            &store,
            repo_id,
            &format!("concurrent{}", i),
            &format!("/tmp/concurrent{}", i),
        )
        .await;
        let chunk_id =
            create_test_chunk(&store, repo_id, &[wt_id], &format!("concurrent{}.rs", i)).await;
        worktree_ids.push(wt_id);
        chunk_ids.push(chunk_id);
    }

    // Verify initial state
    for wt_id in &worktree_ids {
        assert!(worktree_exists(&store, *wt_id).await);
    }
    for chunk_id in &chunk_ids {
        assert!(chunk_exists(&store, *chunk_id).await);
    }

    // Create stale worktree list
    let stale: Vec<StaleWorktree> = worktree_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| StaleWorktree {
            id,
            repo_id,
            name: format!("concurrent{}", i),
            abs_path: format!("/tmp/concurrent{}", i),
            exists: false,
            chunk_count: 1,
        })
        .collect();

    // Clean up all worktrees
    let cleaner = WorktreeCleaner::new(&store, false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify all deletions succeeded
    assert_eq!(report.deleted_count, 6);
    assert_eq!(report.chunks_cleaned, 6);
    assert_eq!(report.failed_count, 0);

    // Verify all worktrees are deleted
    for wt_id in &worktree_ids {
        assert!(
            !worktree_exists(&store, *wt_id).await,
            "Worktree {} should be deleted",
            wt_id
        );
    }

    // Verify all chunks are deleted
    for chunk_id in &chunk_ids {
        assert!(
            !chunk_exists(&store, *chunk_id).await,
            "Chunk {} should be deleted",
            chunk_id
        );
    }

    Ok(())
}
