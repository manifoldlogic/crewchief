//! Integration tests for git tree SHA detection and diff-tree operations.
//!
//! Tests verify:
//! - Git tree SHA format (40-char hex for SHA-1, 64-char for SHA-256)
//! - Tree SHA stability (unchanged when content unchanged)
//! - Tree SHA changes when files modified
//! - diff-tree detection of Added/Modified/Deleted files
//! - diff-tree output parsing
//!
//! Requirements:
//! - Git must be installed and available in PATH
//!
//! Run with: cargo test --test git_integration

use anyhow::Result;
use crewchief_maproom::git::{git_diff_tree, get_current_branch, get_git_tree_sha, FileStatus};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Helper to create a temporary git repository for testing.
///
/// Returns the path to the repository root.
fn create_test_repo() -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir().join(format!("maproom_test_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir)?;

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&temp_dir)
        .output()?;

    // Configure git user (required for commits)
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&temp_dir)
        .output()?;

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&temp_dir)
        .output()?;

    // Create initial file
    fs::write(temp_dir.join("file1.ts"), "console.log('hello');")?;

    // Initial commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(&temp_dir)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&temp_dir)
        .output()?;

    Ok(temp_dir)
}

/// Helper to commit changes in a test repo.
fn git_commit(repo: &Path, message: &str) -> Result<()> {
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(repo)
        .output()?;

    Ok(())
}

/// Helper to clean up test repository.
fn cleanup_test_repo(repo: &Path) {
    let _ = fs::remove_dir_all(repo);
}

// ============================================================================
// GIT TREE SHA TESTS
// ============================================================================

#[test]
fn test_get_git_tree_sha() {
    let repo = create_test_repo().expect("Failed to create test repo");

    let tree_sha = get_git_tree_sha(&repo).expect("Failed to get tree SHA");

    // Git tree SHA should be 40 hex chars (SHA-1) or 64 hex chars (SHA-256)
    // Most git installations still use SHA-1 (40 chars)
    assert!(
        tree_sha.len() == 40 || tree_sha.len() == 64,
        "Tree SHA should be 40 or 64 hex chars, got {}",
        tree_sha.len()
    );
    assert!(
        tree_sha.chars().all(|c| c.is_ascii_hexdigit()),
        "Tree SHA should be hex string"
    );

    cleanup_test_repo(&repo);
}

#[test]
fn test_tree_sha_changes_on_modification() {
    let repo = create_test_repo().expect("Failed to create test repo");

    let tree1 = get_git_tree_sha(&repo).expect("Failed to get initial tree SHA");

    // Modify a file
    fs::write(repo.join("file1.ts"), "console.log('modified');")
        .expect("Failed to modify file");
    git_commit(&repo, "Modify file").expect("Failed to commit");

    let tree2 = get_git_tree_sha(&repo).expect("Failed to get second tree SHA");

    assert_ne!(
        tree1, tree2,
        "Tree SHA must change when content changes"
    );

    cleanup_test_repo(&repo);
}

#[test]
fn test_tree_sha_unchanged_for_same_content() {
    let repo = create_test_repo().expect("Failed to create test repo");

    let tree1 = get_git_tree_sha(&repo).expect("Failed to get first tree SHA");
    let tree2 = get_git_tree_sha(&repo).expect("Failed to get second tree SHA");

    assert_eq!(
        tree1, tree2,
        "Tree SHA must be stable when content unchanged"
    );

    cleanup_test_repo(&repo);
}

#[test]
fn test_get_current_branch() {
    let repo = create_test_repo().expect("Failed to create test repo");

    let branch = get_current_branch(&repo).expect("Failed to get branch");

    // Default branch is usually "master" or "main"
    assert!(
        branch == "master" || branch == "main",
        "Expected 'master' or 'main', got '{}'",
        branch
    );

    cleanup_test_repo(&repo);
}

// ============================================================================
// GIT DIFF-TREE TESTS
// ============================================================================

#[test]
fn test_git_diff_tree_detects_changes() {
    let repo = create_test_repo().expect("Failed to create test repo");

    // Get initial tree SHA
    let tree1 = get_git_tree_sha(&repo).expect("Failed to get tree1");

    // Add a new file
    fs::write(repo.join("new.ts"), "console.log('new');").expect("Failed to create new file");

    // Modify existing file
    fs::write(repo.join("file1.ts"), "console.log('modified');")
        .expect("Failed to modify file");

    // Create a file to delete later
    fs::write(repo.join("old.ts"), "console.log('old');").expect("Failed to create old file");
    git_commit(&repo, "Add files").expect("Failed to commit");

    // Get second tree SHA
    let tree2 = get_git_tree_sha(&repo).expect("Failed to get tree2");

    // Delete file
    fs::remove_file(repo.join("old.ts")).expect("Failed to delete file");
    git_commit(&repo, "Delete old file").expect("Failed to commit");

    // Get third tree SHA
    let tree3 = get_git_tree_sha(&repo).expect("Failed to get tree3");

    // Test diff from tree1 to tree2 (should show Added and Modified)
    let changes_1_2 = git_diff_tree(&tree1, &tree2, &repo).expect("Failed to get diff 1->2");

    let added = changes_1_2
        .iter()
        .filter(|c| matches!(c.status, FileStatus::Added))
        .count();
    let modified = changes_1_2
        .iter()
        .filter(|c| matches!(c.status, FileStatus::Modified))
        .count();

    assert_eq!(added, 2, "Expected 2 added files (new.ts, old.ts)");
    assert_eq!(modified, 1, "Expected 1 modified file (file1.ts)");

    // Test diff from tree2 to tree3 (should show Deleted)
    let changes_2_3 = git_diff_tree(&tree2, &tree3, &repo).expect("Failed to get diff 2->3");

    let deleted = changes_2_3
        .iter()
        .filter(|c| matches!(c.status, FileStatus::Deleted))
        .count();

    assert_eq!(deleted, 1, "Expected 1 deleted file (old.ts)");

    cleanup_test_repo(&repo);
}

#[test]
fn test_diff_tree_parses_correctly() {
    let repo = create_test_repo().expect("Failed to create test repo");

    let tree1 = get_git_tree_sha(&repo).expect("Failed to get tree1");

    // Add a file with known name
    fs::write(repo.join("test_file.ts"), "content").expect("Failed to create file");
    git_commit(&repo, "Add test file").expect("Failed to commit");

    let tree2 = get_git_tree_sha(&repo).expect("Failed to get tree2");

    let changes = git_diff_tree(&tree1, &tree2, &repo).expect("Failed to get diff");

    // Find the change for our test file
    let test_change = changes
        .iter()
        .find(|c| c.path.ends_with("test_file.ts"))
        .expect("test_file.ts not found in changes");

    assert!(
        matches!(test_change.status, FileStatus::Added),
        "Expected Added status"
    );
    assert!(
        test_change.path.ends_with("test_file.ts"),
        "Path should end with test_file.ts"
    );

    cleanup_test_repo(&repo);
}

#[test]
fn test_diff_tree_empty_when_no_changes() {
    let repo = create_test_repo().expect("Failed to create test repo");

    let tree1 = get_git_tree_sha(&repo).expect("Failed to get tree SHA");
    let tree2 = get_git_tree_sha(&repo).expect("Failed to get tree SHA again");

    let changes = git_diff_tree(&tree1, &tree2, &repo).expect("Failed to get diff");

    assert!(
        changes.is_empty(),
        "No changes should be detected when tree SHAs are identical"
    );

    cleanup_test_repo(&repo);
}

#[test]
fn test_diff_tree_with_nested_paths() {
    let repo = create_test_repo().expect("Failed to create test repo");

    let tree1 = get_git_tree_sha(&repo).expect("Failed to get tree1");

    // Create nested directory structure
    fs::create_dir_all(repo.join("src/components")).expect("Failed to create directories");
    fs::write(
        repo.join("src/components/Button.tsx"),
        "export const Button = () => <button />;",
    )
    .expect("Failed to create nested file");

    git_commit(&repo, "Add nested file").expect("Failed to commit");

    let tree2 = get_git_tree_sha(&repo).expect("Failed to get tree2");

    let changes = git_diff_tree(&tree1, &tree2, &repo).expect("Failed to get diff");

    // Find the nested file change
    let nested_change = changes
        .iter()
        .find(|c| c.path.to_string_lossy().contains("Button.tsx"))
        .expect("Button.tsx not found in changes");

    assert!(
        matches!(nested_change.status, FileStatus::Added),
        "Nested file should be detected as Added"
    );

    cleanup_test_repo(&repo);
}
