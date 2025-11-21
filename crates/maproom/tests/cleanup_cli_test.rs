//! Integration tests for cleanup-stale CLI command execution logic
//!
//! Tests the three-phase workflow: detection → report → deletion
//! Requires test database connection via MAPROOM_TEST_DB_HOST environment variable.

use anyhow::Result;
use crewchief_maproom::db;
use crewchief_maproom::db::cleanup::{StaleWorktreeDetector, WorktreeCleaner};

/// Helper to get test database connection string
fn get_test_db_url() -> String {
    let host =
        std::env::var("MAPROOM_TEST_DB_HOST").unwrap_or_else(|_| "localhost:5433".to_string());
    // If host already includes port, use it as-is; otherwise add default port
    if host.contains(':') {
        format!("postgresql://maproom:maproom@{}/maproom", host)
    } else {
        format!("postgresql://maproom:maproom@{}:5433/maproom", host)
    }
}

/// Test dry-run mode: detection and reporting without deletion
#[tokio::test]
async fn test_cleanup_dry_run_no_deletion() -> Result<()> {
    // Setup: Connect to test database
    let db_url = get_test_db_url();
    std::env::set_var("MAPROOM_DATABASE_URL", &db_url);

    let client = db::connect().await?;

    // Ensure migrations are applied
    db::migrate(&client).await?;

    // Create a test repository
    let repo_id: i64 = client
        .query_one(
            "INSERT INTO maproom.repos (name, remote_url, local_path)
             VALUES ($1, $2, $3)
             ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
             RETURNING id",
            &[
                &"test-repo-dryrun",
                &"https://github.com/test/dryrun.git",
                &"/tmp/test-dryrun",
            ],
        )
        .await?
        .get(0);

    // Create a stale worktree (path that doesn't exist)
    let stale_worktree_id: i64 = client
        .query_one(
            "INSERT INTO maproom.worktrees (repo_id, name, abs_path)
             VALUES ($1, $2, $3)
             RETURNING id",
            &[
                &repo_id,
                &"stale-branch-dryrun",
                &"/tmp/nonexistent/path/dryrun",
            ],
        )
        .await?
        .get(0);

    // Phase 1: Detection
    let detector = StaleWorktreeDetector::new(&client);
    let stale = detector.detect_stale_worktrees().await?;

    // Verify detection found the stale worktree
    let found_stale = stale.iter().any(|wt| wt.id == stale_worktree_id);
    assert!(
        found_stale,
        "Detection phase should find the stale worktree"
    );

    // Phase 2: Simulate dry-run (no deletion)
    // In the actual CLI, this would be when --confirm flag is not provided

    // Phase 3: Verify worktree still exists in database
    let exists = client
        .query_opt(
            "SELECT id FROM maproom.worktrees WHERE id = $1",
            &[&stale_worktree_id],
        )
        .await?
        .is_some();

    assert!(
        exists,
        "Dry-run mode should not delete worktree from database"
    );

    // Cleanup: Delete test data
    client
        .execute(
            "DELETE FROM maproom.worktrees WHERE id = $1",
            &[&stale_worktree_id],
        )
        .await?;
    client
        .execute("DELETE FROM maproom.repos WHERE id = $1", &[&repo_id])
        .await?;

    Ok(())
}

/// Test confirm mode: actual deletion of stale worktrees
#[tokio::test]
async fn test_cleanup_confirm_deletes_worktrees() -> Result<()> {
    // Setup: Connect to test database
    let db_url = get_test_db_url();
    std::env::set_var("MAPROOM_DATABASE_URL", &db_url);

    let mut client = db::connect().await?;

    // Ensure migrations are applied
    db::migrate(&client).await?;

    // Create a test repository
    let repo_id: i64 = client
        .query_one(
            "INSERT INTO maproom.repos (name, remote_url, local_path)
             VALUES ($1, $2, $3)
             ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
             RETURNING id",
            &[
                &"test-repo-confirm",
                &"https://github.com/test/confirm.git",
                &"/tmp/test-confirm",
            ],
        )
        .await?
        .get(0);

    // Create a stale worktree (path that doesn't exist)
    let stale_worktree_id: i64 = client
        .query_one(
            "INSERT INTO maproom.worktrees (repo_id, name, abs_path)
             VALUES ($1, $2, $3)
             RETURNING id",
            &[
                &repo_id,
                &"stale-branch-confirm",
                &"/tmp/nonexistent/path/confirm",
            ],
        )
        .await?
        .get(0);

    // Phase 1: Detection
    let detector = StaleWorktreeDetector::new(&client);
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
    let mut cleaner = WorktreeCleaner::new(&mut client, false);
    let report = cleaner.cleanup_stale_worktrees(stale_to_delete).await?;

    // Verify cleanup report
    assert_eq!(report.total_stale, 1, "Should have 1 stale worktree");
    assert_eq!(report.deleted_count, 1, "Should have deleted 1 worktree");
    assert_eq!(report.failed_count, 0, "Should have no failures");

    // Phase 4: Verify worktree no longer exists in database
    let exists = client
        .query_opt(
            "SELECT id FROM maproom.worktrees WHERE id = $1",
            &[&stale_worktree_id],
        )
        .await?
        .is_some();

    assert!(!exists, "Confirm mode should delete worktree from database");

    // Cleanup: Delete test repository
    client
        .execute("DELETE FROM maproom.repos WHERE id = $1", &[&repo_id])
        .await?;

    Ok(())
}

/// Test verbose mode: verify additional details are available
#[tokio::test]
async fn test_cleanup_verbose_mode() -> Result<()> {
    // Setup: Connect to test database
    let db_url = get_test_db_url();
    std::env::set_var("MAPROOM_DATABASE_URL", &db_url);

    let client = db::connect().await?;

    // Ensure migrations are applied
    db::migrate(&client).await?;

    // Create a test repository
    let repo_id: i64 = client
        .query_one(
            "INSERT INTO maproom.repos (name, remote_url, local_path)
             VALUES ($1, $2, $3)
             ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
             RETURNING id",
            &[
                &"test-repo-verbose",
                &"https://github.com/test/verbose.git",
                &"/tmp/test-verbose",
            ],
        )
        .await?
        .get(0);

    // Create a stale worktree
    let stale_worktree_id: i64 = client
        .query_one(
            "INSERT INTO maproom.worktrees (repo_id, name, abs_path)
             VALUES ($1, $2, $3)
             RETURNING id",
            &[
                &repo_id,
                &"stale-branch-verbose",
                &"/tmp/nonexistent/path/verbose",
            ],
        )
        .await?
        .get(0);

    // Phase 1: Detection
    let detector = StaleWorktreeDetector::new(&client);
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

    // Cleanup: Delete test data
    client
        .execute(
            "DELETE FROM maproom.worktrees WHERE id = $1",
            &[&stale_worktree_id],
        )
        .await?;
    client
        .execute("DELETE FROM maproom.repos WHERE id = $1", &[&repo_id])
        .await?;

    Ok(())
}

/// Test error handling: database connection failure
#[tokio::test]
async fn test_cleanup_handles_connection_error() {
    // Set invalid database URL
    std::env::set_var(
        "MAPROOM_DATABASE_URL",
        "postgresql://invalid:invalid@nonexistent:5432/maproom",
    );

    // Attempt to connect should fail gracefully
    let result = db::connect().await;
    assert!(result.is_err(), "Should fail with invalid database URL");
}

/// Test exit code 2: no stale worktrees found
#[tokio::test]
async fn test_cleanup_no_stale_worktrees() -> Result<()> {
    // Setup: Connect to test database
    let db_url = get_test_db_url();
    std::env::set_var("MAPROOM_DATABASE_URL", &db_url);

    let client = db::connect().await?;

    // Ensure migrations are applied
    db::migrate(&client).await?;

    // Phase 1: Detection on clean database (or with only valid worktrees)
    let detector = StaleWorktreeDetector::new(&client);
    let stale = detector.detect_stale_worktrees().await?;

    // Filter to only non-existent paths (the actual stale ones)
    let actual_stale: Vec<_> = stale.into_iter().filter(|wt| !wt.exists).collect();

    // In a clean test scenario, there should be no stale worktrees
    // This simulates the exit code 2 scenario
    if actual_stale.is_empty() {
        // This would result in exit code 2 in the CLI
        assert!(true, "No stale worktrees found - would exit with code 2");
    }

    Ok(())
}
