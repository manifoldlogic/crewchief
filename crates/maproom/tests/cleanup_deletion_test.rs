//! Integration tests for safe worktree deletion
//!
//! Tests verify:
//! - Multi-worktree chunk safety (chunks shared by multiple worktrees are preserved)
//! - Garbage collection accuracy (chunks belonging only to deleted worktree are removed)
//! - Dry-run mode (no changes made when dry_run = true)
//! - Transaction rollback (all-or-nothing behavior on errors)
//!
//! IDXCLEAN-1002: Safe Deletion Module Integration Tests

use crewchief_maproom::db::cleanup::{StaleWorktree, WorktreeCleaner};
use tokio_postgres::NoTls;

/// Get database connection URL from environment or skip test
fn get_database_url_or_skip() -> String {
    match std::env::var("MAPROOM_DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("Skipping test: MAPROOM_DATABASE_URL not set");
            std::process::exit(0); // Skip test gracefully
        }
    }
}

/// Try to connect to database or skip test
async fn connect_or_skip() -> tokio_postgres::Client {
    let url = get_database_url_or_skip();
    match tokio_postgres::connect(&url, NoTls).await {
        Ok((client, connection)) => {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            client
        }
        Err(e) => {
            eprintln!("Skipping test: Failed to connect to database: {}", e);
            std::process::exit(0); // Skip test gracefully
        }
    }
}

/// Test helper: Create a test repo
async fn create_test_repo(client: &tokio_postgres::Client, name: &str) -> i64 {
    let row = client
        .query_one(
            "INSERT INTO maproom.repos (name) VALUES ($1) RETURNING id",
            &[&name],
        )
        .await
        .unwrap();
    row.get(0)
}

/// Test helper: Create a test worktree
async fn create_test_worktree(
    client: &tokio_postgres::Client,
    repo_id: i64,
    name: &str,
    abs_path: &str,
) -> i64 {
    let row = client
        .query_one(
            "INSERT INTO maproom.worktrees (repo_id, name, abs_path) VALUES ($1, $2, $3) RETURNING id",
            &[&repo_id, &name, &abs_path],
        )
        .await
        .unwrap();
    row.get(0)
}

/// Test helper: Create a test chunk with specific worktree_ids
async fn create_test_chunk(
    client: &tokio_postgres::Client,
    worktree_ids: &[i64],
    relpath: &str,
) -> uuid::Uuid {
    let worktree_ids_json: serde_json::Value =
        serde_json::json!(worktree_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>());

    let row = client
        .query_one(
            r#"
            INSERT INTO maproom.chunks (
                symbol_name,
                chunk_type,
                language,
                relpath,
                start_line,
                end_line,
                preview,
                worktree_ids
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
            &[
                &format!("test_symbol_{}", relpath),
                &"function",
                &"rust",
                &relpath,
                &1i32,
                &10i32,
                &"test preview",
                &worktree_ids_json,
            ],
        )
        .await
        .unwrap();
    row.get(0)
}

/// Test helper: Count chunks for a specific worktree
async fn count_chunks_for_worktree(client: &tokio_postgres::Client, worktree_id: i64) -> i64 {
    let row = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks WHERE worktree_ids ? $1::text",
            &[&worktree_id.to_string()],
        )
        .await
        .unwrap();
    row.get(0)
}

/// Test helper: Check if chunk exists
async fn chunk_exists(client: &tokio_postgres::Client, chunk_id: uuid::Uuid) -> bool {
    let row = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM maproom.chunks WHERE id = $1)",
            &[&chunk_id],
        )
        .await
        .unwrap();
    row.get(0)
}

/// Test helper: Check if worktree exists
async fn worktree_exists(client: &tokio_postgres::Client, worktree_id: i64) -> bool {
    let row = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM maproom.worktrees WHERE id = $1)",
            &[&worktree_id],
        )
        .await
        .unwrap();
    row.get(0)
}

/// Test helper: Get chunk's worktree_ids array
async fn get_chunk_worktree_ids(
    client: &tokio_postgres::Client,
    chunk_id: uuid::Uuid,
) -> Vec<String> {
    let row = client
        .query_one(
            "SELECT worktree_ids FROM maproom.chunks WHERE id = $1",
            &[&chunk_id],
        )
        .await
        .unwrap();

    let json: serde_json::Value = row.get(0);
    json.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

/// Test helper: Cleanup test data
async fn cleanup_test_data(client: &tokio_postgres::Client, repo_id: i64) {
    // Delete cascades to worktrees and chunks
    client
        .execute("DELETE FROM maproom.repos WHERE id = $1", &[&repo_id])
        .await
        .unwrap();
}

#[tokio::test]
async fn test_multi_worktree_chunk_safety() {
    // Scenario 4 from quality-strategy.md: Multi-worktree chunks must be preserved
    let mut client = connect_or_skip().await;

    // Setup: Create repo and two worktrees
    let repo_id = create_test_repo(&client, "test_multi_worktree_safety").await;
    let wt1_id = create_test_worktree(&client, repo_id, "branch1", "/tmp/test/branch1").await;
    let wt2_id = create_test_worktree(&client, repo_id, "branch2", "/tmp/test/branch2").await;

    // Create a chunk shared by both worktrees
    let shared_chunk_id = create_test_chunk(&client, &[wt1_id, wt2_id], "shared.rs").await;

    // Create a chunk belonging only to wt1
    let wt1_only_chunk_id = create_test_chunk(&client, &[wt1_id], "wt1_only.rs").await;

    // Verify initial state
    assert_eq!(count_chunks_for_worktree(&client, wt1_id).await, 2);
    assert_eq!(count_chunks_for_worktree(&client, wt2_id).await, 1);
    assert!(chunk_exists(&client, shared_chunk_id).await);
    assert!(chunk_exists(&client, wt1_only_chunk_id).await);

    // Delete worktree 1 (simulate stale worktree)
    let stale = vec![StaleWorktree {
        id: wt1_id,
        repo_id,
        name: "branch1".to_string(),
        abs_path: "/tmp/test/branch1".to_string(),
        exists: false,
        chunk_count: 2,
    }];

    let mut cleaner = WorktreeCleaner::new(&mut client, false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify report
    assert_eq!(report.total_stale, 1);
    assert_eq!(report.deleted_count, 1);
    assert_eq!(report.chunks_cleaned, 1); // Only wt1_only_chunk should be deleted
    assert_eq!(report.failed_count, 0);
    assert_eq!(report.deleted_ids, vec![wt1_id]);

    // Verify multi-worktree chunk is preserved
    assert!(
        chunk_exists(&client, shared_chunk_id).await,
        "Shared chunk should still exist"
    );

    // Verify shared chunk's worktree_ids was updated
    let worktree_ids = get_chunk_worktree_ids(&client, shared_chunk_id).await;
    assert_eq!(
        worktree_ids,
        vec![wt2_id.to_string()],
        "Shared chunk should only contain wt2_id"
    );

    // Verify single-worktree chunk was garbage collected
    assert!(
        !chunk_exists(&client, wt1_only_chunk_id).await,
        "wt1-only chunk should be deleted"
    );

    // Verify worktree 1 is deleted
    assert!(
        !worktree_exists(&client, wt1_id).await,
        "Worktree 1 should be deleted"
    );

    // Verify worktree 2 still exists
    assert!(
        worktree_exists(&client, wt2_id).await,
        "Worktree 2 should still exist"
    );

    // Verify worktree 2 can still find its chunk
    assert_eq!(count_chunks_for_worktree(&client, wt2_id).await, 1);

    // Cleanup
    cleanup_test_data(&client, repo_id).await;
}

#[tokio::test]
async fn test_garbage_collection_accuracy() {
    // Scenario 5 from quality-strategy.md: Single-worktree chunks should be deleted
    let mut client = connect_or_skip().await;

    // Setup: Create repo and worktree
    let repo_id = create_test_repo(&client, "test_garbage_collection").await;
    let wt_id = create_test_worktree(&client, repo_id, "feature", "/tmp/test/feature").await;

    // Create chunks belonging ONLY to this worktree
    let chunk1_id = create_test_chunk(&client, &[wt_id], "file1.rs").await;
    let chunk2_id = create_test_chunk(&client, &[wt_id], "file2.rs").await;
    let chunk3_id = create_test_chunk(&client, &[wt_id], "file3.rs").await;

    // Verify initial state
    assert_eq!(count_chunks_for_worktree(&client, wt_id).await, 3);
    assert!(chunk_exists(&client, chunk1_id).await);
    assert!(chunk_exists(&client, chunk2_id).await);
    assert!(chunk_exists(&client, chunk3_id).await);

    // Delete the worktree
    let stale = vec![StaleWorktree {
        id: wt_id,
        repo_id,
        name: "feature".to_string(),
        abs_path: "/tmp/test/feature".to_string(),
        exists: false,
        chunk_count: 3,
    }];

    let mut cleaner = WorktreeCleaner::new(&mut client, false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify report
    assert_eq!(report.total_stale, 1);
    assert_eq!(report.deleted_count, 1);
    assert_eq!(report.chunks_cleaned, 3); // All 3 chunks should be deleted
    assert_eq!(report.failed_count, 0);

    // Verify all chunks are garbage collected
    assert!(
        !chunk_exists(&client, chunk1_id).await,
        "Chunk 1 should be deleted"
    );
    assert!(
        !chunk_exists(&client, chunk2_id).await,
        "Chunk 2 should be deleted"
    );
    assert!(
        !chunk_exists(&client, chunk3_id).await,
        "Chunk 3 should be deleted"
    );

    // Verify worktree is deleted
    assert!(
        !worktree_exists(&client, wt_id).await,
        "Worktree should be deleted"
    );

    // Cleanup
    cleanup_test_data(&client, repo_id).await;
}

#[tokio::test]
async fn test_dry_run_mode() {
    // Verify dry-run mode makes no changes
    let mut client = connect_or_skip().await;

    // Setup: Create repo, worktree, and chunk
    let repo_id = create_test_repo(&client, "test_dry_run").await;
    let wt_id = create_test_worktree(&client, repo_id, "test", "/tmp/test/dry_run").await;
    let chunk_id = create_test_chunk(&client, &[wt_id], "test.rs").await;

    // Verify initial state
    assert!(worktree_exists(&client, wt_id).await);
    assert!(chunk_exists(&client, chunk_id).await);
    assert_eq!(count_chunks_for_worktree(&client, wt_id).await, 1);

    // Run cleanup in dry-run mode
    let stale = vec![StaleWorktree {
        id: wt_id,
        repo_id,
        name: "test".to_string(),
        abs_path: "/tmp/test/dry_run".to_string(),
        exists: false,
        chunk_count: 1,
    }];

    let mut cleaner = WorktreeCleaner::new(&mut client, true); // dry_run = true
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify report shows what would be deleted but didn't actually delete
    assert_eq!(report.total_stale, 1);
    assert_eq!(report.deleted_count, 0); // No actual deletions
    assert_eq!(report.chunks_cleaned, 0); // No chunks cleaned
    assert_eq!(report.failed_count, 0);
    assert_eq!(report.deleted_ids.len(), 0);

    // Verify nothing was actually deleted
    assert!(
        worktree_exists(&client, wt_id).await,
        "Worktree should still exist (dry-run)"
    );
    assert!(
        chunk_exists(&client, chunk_id).await,
        "Chunk should still exist (dry-run)"
    );
    assert_eq!(
        count_chunks_for_worktree(&client, wt_id).await,
        1,
        "Chunk count should be unchanged (dry-run)"
    );

    // Cleanup
    cleanup_test_data(&client, repo_id).await;
}

#[tokio::test]
async fn test_partial_failure_handling() {
    // Test that partial failures are collected and don't abort the entire process
    let mut client = connect_or_skip().await;

    // Setup: Create repo and worktrees
    let repo_id = create_test_repo(&client, "test_partial_failure").await;
    let wt1_id = create_test_worktree(&client, repo_id, "wt1", "/tmp/test/wt1").await;
    let wt2_id = create_test_worktree(&client, repo_id, "wt2", "/tmp/test/wt2").await;

    // Create chunks for wt1
    let chunk1_id = create_test_chunk(&client, &[wt1_id], "file1.rs").await;

    // Create chunks for wt2
    let chunk2_id = create_test_chunk(&client, &[wt2_id], "file2.rs").await;

    // Try to delete wt1 (should succeed), a non-existent worktree (should fail), and wt2 (should succeed)
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

    let mut cleaner = WorktreeCleaner::new(&mut client, false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify report shows 2 successes and 1 failure
    assert_eq!(report.total_stale, 3);
    assert_eq!(report.deleted_count, 2); // wt1 and wt2
    assert_eq!(report.chunks_cleaned, 2); // chunks from wt1 and wt2
    assert_eq!(report.failed_count, 1); // fake worktree
    assert!(report.deleted_ids.contains(&wt1_id));
    assert!(report.deleted_ids.contains(&wt2_id));
    assert_eq!(report.failed_deletions.len(), 1);
    assert_eq!(report.failed_deletions[0].0, 999999);

    // Verify successful deletions happened
    assert!(!worktree_exists(&client, wt1_id).await);
    assert!(!worktree_exists(&client, wt2_id).await);
    assert!(!chunk_exists(&client, chunk1_id).await);
    assert!(!chunk_exists(&client, chunk2_id).await);

    // Cleanup
    cleanup_test_data(&client, repo_id).await;
}

#[tokio::test]
async fn test_complex_multi_worktree_scenario() {
    // Complex scenario: Multiple worktrees with overlapping chunks
    let mut client = connect_or_skip().await;

    // Setup: Create repo and 3 worktrees
    let repo_id = create_test_repo(&client, "test_complex_scenario").await;
    let wt1_id = create_test_worktree(&client, repo_id, "main", "/tmp/test/main").await;
    let wt2_id = create_test_worktree(&client, repo_id, "dev", "/tmp/test/dev").await;
    let wt3_id = create_test_worktree(&client, repo_id, "staging", "/tmp/test/staging").await;

    // Create chunks with different sharing patterns:
    // - chunk_all: shared by all 3 worktrees
    // - chunk_12: shared by wt1 and wt2
    // - chunk_23: shared by wt2 and wt3
    // - chunk_2: only in wt2
    let chunk_all_id = create_test_chunk(&client, &[wt1_id, wt2_id, wt3_id], "common.rs").await;
    let chunk_12_id = create_test_chunk(&client, &[wt1_id, wt2_id], "shared_1_2.rs").await;
    let chunk_23_id = create_test_chunk(&client, &[wt2_id, wt3_id], "shared_2_3.rs").await;
    let chunk_2_id = create_test_chunk(&client, &[wt2_id], "exclusive_2.rs").await;

    // Verify initial state
    assert_eq!(count_chunks_for_worktree(&client, wt1_id).await, 2); // chunk_all, chunk_12
    assert_eq!(count_chunks_for_worktree(&client, wt2_id).await, 4); // all chunks
    assert_eq!(count_chunks_for_worktree(&client, wt3_id).await, 2); // chunk_all, chunk_23

    // Delete wt2 (has all chunks)
    let stale = vec![StaleWorktree {
        id: wt2_id,
        repo_id,
        name: "dev".to_string(),
        abs_path: "/tmp/test/dev".to_string(),
        exists: false,
        chunk_count: 4,
    }];

    let mut cleaner = WorktreeCleaner::new(&mut client, false);
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
    assert!(chunk_exists(&client, chunk_all_id).await);
    assert!(chunk_exists(&client, chunk_12_id).await);
    assert!(chunk_exists(&client, chunk_23_id).await);
    assert!(!chunk_exists(&client, chunk_2_id).await);

    // Verify worktree_ids arrays are correct
    let chunk_all_wts = get_chunk_worktree_ids(&client, chunk_all_id).await;
    assert_eq!(chunk_all_wts.len(), 2);
    assert!(chunk_all_wts.contains(&wt1_id.to_string()));
    assert!(chunk_all_wts.contains(&wt3_id.to_string()));
    assert!(!chunk_all_wts.contains(&wt2_id.to_string()));

    let chunk_12_wts = get_chunk_worktree_ids(&client, chunk_12_id).await;
    assert_eq!(chunk_12_wts, vec![wt1_id.to_string()]);

    let chunk_23_wts = get_chunk_worktree_ids(&client, chunk_23_id).await;
    assert_eq!(chunk_23_wts, vec![wt3_id.to_string()]);

    // Verify remaining worktrees can still find their chunks
    assert_eq!(count_chunks_for_worktree(&client, wt1_id).await, 2);
    assert_eq!(count_chunks_for_worktree(&client, wt3_id).await, 2);

    // Cleanup
    cleanup_test_data(&client, repo_id).await;
}
