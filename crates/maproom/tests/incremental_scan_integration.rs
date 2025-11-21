//! Integration tests for incremental scan functionality (INCRSCAN-2001)
//!
//! These tests verify the tree SHA-based skip logic and state persistence
//! work correctly across different scan scenarios. Uses real database and
//! temporary git repositories.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use tokio_postgres::Client;

use crewchief_maproom::db;
use crewchief_maproom::indexer;
use crewchief_maproom::progress::{OutputMode, ProgressTracker};

// ============================================================================
// Test Helpers
// ============================================================================

/// Setup test database connection
async fn setup_test_db() -> Client {
    // Use test database via environment variable
    // Tests should be run with MAPROOM_DATABASE_URL set to test database
    let client = db::connect()
        .await
        .expect("Failed to connect to test database");

    // Apply migrations to ensure schema is up to date
    // Ignore errors if schema already exists (concurrent test setup)
    let _ = db::migrate(&client).await;

    client
}

/// Create a temporary git repository for testing
async fn create_test_repo() -> PathBuf {
    let temp_dir = std::env::temp_dir().join(format!("maproom_test_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to init git repo");

    // Configure git user
    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to config git user");

    Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to config git email");

    temp_dir
}

/// Add test files to repository
async fn add_test_files(repo: &Path, count: usize) {
    for i in 0..count {
        let file_path = repo.join(format!("test{}.ts", i));
        let content = format!(
            r#"// Test file {}
export function test{}() {{
    const message = "Hello from test {}";
    console.log(message);
    return message;
}}
"#,
            i, i, i
        );
        fs::write(&file_path, content).expect("Failed to write test file");
    }
}

/// Create a git commit and return the commit SHA
async fn git_commit(repo: &Path, message: &str) -> String {
    // Add all files
    Command::new("git")
        .args(&["add", "."])
        .current_dir(repo)
        .output()
        .expect("Failed to git add");

    // Commit
    Command::new("git")
        .args(&["commit", "-m", message])
        .current_dir(repo)
        .output()
        .expect("Failed to git commit");

    // Get commit SHA
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .expect("Failed to get commit SHA");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Get index state for a worktree
async fn get_index_state(
    db: &Client,
    repo: &str,
    worktree: &str,
) -> Result<Option<String>, tokio_postgres::Error> {
    // Get repo ID
    let repo_row = db
        .query_one("SELECT id FROM maproom.repos WHERE name = $1", &[&repo])
        .await?;
    let repo_id: i64 = repo_row.get(0);

    // Get worktree ID
    let worktree_row = db
        .query_one(
            "SELECT id FROM maproom.worktrees WHERE repo_id = $1 AND name = $2",
            &[&repo_id, &worktree],
        )
        .await?;
    let worktree_id: i64 = worktree_row.get(0);

    // Get last tree SHA
    let state_row = db
        .query_opt(
            "SELECT last_tree_sha FROM maproom.worktree_index_state WHERE worktree_id = $1",
            &[&worktree_id],
        )
        .await?;

    Ok(state_row.and_then(|row| row.get(0)))
}

/// Clean up test repository
async fn cleanup_test_repo(repo: &Path) {
    if repo.exists() {
        fs::remove_dir_all(repo).ok();
    }
}

/// Clean up test database records
async fn cleanup_test_db(db: &Client, repo: &str) {
    // Delete from worktree_index_state via worktrees -> repos cascade
    db.execute("DELETE FROM maproom.repos WHERE name = $1", &[&repo])
        .await
        .ok();
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires database - run with: cargo test --test incremental_scan_integration -- --ignored
async fn test_unchanged_tree_skip() {
    let db = setup_test_db().await;
    let temp_repo = create_test_repo().await;
    let repo_name = format!("test_unchanged_{}", uuid::Uuid::new_v4());

    // Add test files and commit
    add_test_files(&temp_repo, 10).await;
    let commit_sha = git_commit(&temp_repo, "Initial commit").await;

    // First scan: should process all files
    let progress1 = ProgressTracker::new(OutputMode::Minimal);
    indexer::scan_worktree(
        &db,
        &repo_name,
        "main",
        &temp_repo,
        &commit_sha,
        4,
        None,
        None,
        Some(&progress1),
    )
    .await
    .expect("First scan should succeed");

    // Verify files were processed
    assert!(
        progress1.files_processed() > 0,
        "First scan should process files"
    );

    // Verify state was saved
    let state = get_index_state(&db, &repo_name, "main")
        .await
        .expect("Should get state");
    assert!(state.is_some(), "State should be saved after first scan");

    // Second scan: should skip (no changes)
    let progress2 = ProgressTracker::new(OutputMode::Minimal);
    let start = Instant::now();
    indexer::scan_worktree(
        &db,
        &repo_name,
        "main",
        &temp_repo,
        &commit_sha,
        4,
        None,
        None,
        Some(&progress2),
    )
    .await
    .expect("Second scan should succeed");
    let duration = start.elapsed();

    // Verify skip happened
    assert_eq!(
        progress2.files_processed(),
        0,
        "Second scan should skip (no files processed)"
    );
    assert!(
        duration < Duration::from_millis(1000),
        "Skip should be fast (< 1 second), was {:?}",
        duration
    );

    // Cleanup
    cleanup_test_db(&db, &repo_name).await;
    cleanup_test_repo(&temp_repo).await;
}

#[tokio::test]
#[ignore] // Requires database - run with: cargo test --test incremental_scan_integration -- --ignored
async fn test_changed_tree_scan() {
    let db = setup_test_db().await;
    let temp_repo = create_test_repo().await;
    let repo_name = format!("test_changed_{}", uuid::Uuid::new_v4());

    // First scan
    add_test_files(&temp_repo, 5).await;
    let commit1 = git_commit(&temp_repo, "Initial commit").await;

    let progress1 = ProgressTracker::new(OutputMode::Minimal);
    indexer::scan_worktree(
        &db,
        &repo_name,
        "main",
        &temp_repo,
        &commit1,
        4,
        None,
        None,
        Some(&progress1),
    )
    .await
    .expect("First scan should succeed");

    let first_count = progress1.files_processed();
    assert!(first_count > 0, "First scan should process files");

    // Get first tree SHA
    let state1 = get_index_state(&db, &repo_name, "main")
        .await
        .expect("Should get state")
        .expect("State should exist");

    // Modify a file and commit (tree SHA changes)
    let file_path = temp_repo.join("test0.ts");
    fs::write(
        &file_path,
        "// Modified content\nexport function modified() {}",
    )
    .expect("Failed to modify file");
    let commit2 = git_commit(&temp_repo, "Modified file").await;

    // Second scan: should process files (tree changed)
    let progress2 = ProgressTracker::new(OutputMode::Minimal);
    indexer::scan_worktree(
        &db,
        &repo_name,
        "main",
        &temp_repo,
        &commit2,
        4,
        None,
        None,
        Some(&progress2),
    )
    .await
    .expect("Second scan should succeed");

    // Verify full scan executed
    assert!(
        progress2.files_processed() > 0,
        "Second scan should process files when tree changed"
    );

    // Verify state updated with new tree SHA
    let state2 = get_index_state(&db, &repo_name, "main")
        .await
        .expect("Should get state")
        .expect("State should exist");
    assert_ne!(state1, state2, "Tree SHA should have changed");

    // Cleanup
    cleanup_test_db(&db, &repo_name).await;
    cleanup_test_repo(&temp_repo).await;
}

#[tokio::test]
#[ignore] // Requires database - run with: cargo test --test incremental_scan_integration -- --ignored
async fn test_force_flag_override() {
    let db = setup_test_db().await;
    let temp_repo = create_test_repo().await;
    let repo_name = format!("test_force_{}", uuid::Uuid::new_v4());

    // First scan
    add_test_files(&temp_repo, 5).await;
    let commit_sha = git_commit(&temp_repo, "Initial commit").await;

    let progress1 = ProgressTracker::new(OutputMode::Minimal);
    indexer::scan_worktree(
        &db,
        &repo_name,
        "main",
        &temp_repo,
        &commit_sha,
        4,
        None,
        None,
        Some(&progress1),
    )
    .await
    .expect("First scan should succeed");

    assert!(
        progress1.files_processed() > 0,
        "First scan should process files"
    );

    // Note: The force flag is passed to the scan command, not to scan_worktree function
    // This test verifies that when force is NOT set, unchanged scans skip
    // The actual force flag testing requires CLI-level testing
    // For now, we verify the default behavior (no force = skip on unchanged)

    let progress2 = ProgressTracker::new(OutputMode::Minimal);
    indexer::scan_worktree(
        &db,
        &repo_name,
        "main",
        &temp_repo,
        &commit_sha,
        4,
        None,
        None,
        Some(&progress2),
    )
    .await
    .expect("Second scan should succeed");

    // Should skip since force is not set and tree unchanged
    assert_eq!(
        progress2.files_processed(),
        0,
        "Should skip when force not set and tree unchanged"
    );

    // Cleanup
    cleanup_test_db(&db, &repo_name).await;
    cleanup_test_repo(&temp_repo).await;
}

#[tokio::test]
#[ignore] // Requires database - run with: cargo test --test incremental_scan_integration -- --ignored
async fn test_first_scan_state_creation() {
    let db = setup_test_db().await;
    let temp_repo = create_test_repo().await;
    let repo_name = format!("test_first_{}", uuid::Uuid::new_v4());

    // Create files and commit
    add_test_files(&temp_repo, 3).await;
    let commit_sha = git_commit(&temp_repo, "Initial commit").await;

    // Verify no state exists yet
    let state_before = get_index_state(&db, &repo_name, "main").await;
    assert!(
        state_before.is_err() || state_before.unwrap().is_none(),
        "State should not exist before first scan"
    );

    // First scan
    let progress = ProgressTracker::new(OutputMode::Minimal);
    indexer::scan_worktree(
        &db,
        &repo_name,
        "main",
        &temp_repo,
        &commit_sha,
        4,
        None,
        None,
        Some(&progress),
    )
    .await
    .expect("First scan should succeed");

    // Verify scan executed
    assert!(
        progress.files_processed() > 0,
        "First scan should process files"
    );

    // Verify state was created
    let state_after = get_index_state(&db, &repo_name, "main")
        .await
        .expect("Should get state")
        .expect("State should exist after first scan");
    assert!(!state_after.is_empty(), "State should have tree SHA");

    // Cleanup
    cleanup_test_db(&db, &repo_name).await;
    cleanup_test_repo(&temp_repo).await;
}

#[tokio::test]
#[ignore] // Requires database - run with: cargo test --test incremental_scan_integration -- --ignored
async fn test_concurrent_scans() {
    let db = setup_test_db().await;
    let temp_repo = create_test_repo().await;
    let repo_name = format!("test_concurrent_{}", uuid::Uuid::new_v4());

    // Setup: create files and commit
    add_test_files(&temp_repo, 5).await;
    let commit_sha = git_commit(&temp_repo, "Initial commit").await;

    // Create two separate database connections for concurrent scans
    let db1 = setup_test_db().await;
    let db2 = setup_test_db().await;

    // Clone values for moving into async blocks
    let temp_repo_1 = temp_repo.clone();
    let temp_repo_2 = temp_repo.clone();
    let repo_name_1 = repo_name.clone();
    let repo_name_2 = repo_name.clone();
    let commit_sha_1 = commit_sha.clone();
    let commit_sha_2 = commit_sha.clone();

    // Spawn two concurrent scans
    let handle1 = tokio::spawn(async move {
        let progress = ProgressTracker::new(OutputMode::Minimal);
        indexer::scan_worktree(
            &db1,
            &repo_name_1,
            "main",
            &temp_repo_1,
            &commit_sha_1,
            4,
            None,
            None,
            Some(&progress),
        )
        .await
    });

    let handle2 = tokio::spawn(async move {
        let progress = ProgressTracker::new(OutputMode::Minimal);
        indexer::scan_worktree(
            &db2,
            &repo_name_2,
            "main",
            &temp_repo_2,
            &commit_sha_2,
            4,
            None,
            None,
            Some(&progress),
        )
        .await
    });

    // Wait for both to complete
    let result1 = handle1.await.expect("Task 1 should not panic");
    let result2 = handle2.await.expect("Task 2 should not panic");

    // Both should succeed (one does full scan, other might skip or also scan)
    assert!(result1.is_ok(), "First concurrent scan should succeed");
    assert!(result2.is_ok(), "Second concurrent scan should succeed");

    // Verify state is consistent (ON CONFLICT DO UPDATE handles race)
    let final_state = get_index_state(&db, &repo_name, "main")
        .await
        .expect("Should get state");
    assert!(
        final_state.is_some(),
        "State should exist after concurrent scans"
    );

    // Cleanup
    cleanup_test_db(&db, &repo_name).await;
    cleanup_test_repo(&temp_repo).await;
}
