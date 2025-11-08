//! Git integration functions for tree SHA comparison and file change detection.
//!
//! This module provides core functionality for incremental updates by:
//! - Getting git tree SHA to detect repository changes
//! - Finding changed files between two tree states
//! - Retrieving current branch information
//!
//! These functions enable the optimization: check tree SHA (instant), and if different,
//! find changed files (fast) instead of scanning the entire repository.

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Gets the git tree SHA for the current HEAD.
///
/// The tree SHA represents the complete state of all files in the repository.
/// It's content-addressed and immutable—if two commits have the same tree SHA,
/// their file contents are identical.
///
/// # Arguments
/// * `repo_path` - Path to the git repository
///
/// # Returns
/// * `Ok(String)` - The 40-character SHA-1 tree hash
/// * `Err` - If git command fails or repository is invalid
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use maproom::git::get_git_tree_sha;
///
/// let tree_sha = get_git_tree_sha(Path::new("/path/to/repo"))?;
/// println!("Tree SHA: {}", tree_sha);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_git_tree_sha(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD^{tree}"])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        bail!(
            "Failed to get git tree SHA: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Gets the current branch name.
///
/// # Arguments
/// * `repo_path` - Path to the git repository
///
/// # Returns
/// * `Ok(String)` - Branch name (or "HEAD" if in detached HEAD state)
/// * `Err` - If git command fails
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use maproom::git::get_current_branch;
///
/// let branch = get_current_branch(Path::new("/path/to/repo"))?;
/// println!("Current branch: {}", branch);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_current_branch(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        bail!(
            "Failed to get current branch: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// File status in a git diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    /// File was added
    Added,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
}

/// Represents a file change detected by git diff-tree.
#[derive(Debug, Clone)]
pub struct FileChange {
    /// The type of change (Added, Modified, or Deleted)
    pub status: FileStatus,
    /// Path to the changed file
    pub path: PathBuf,
}

/// Finds files that changed between two git tree SHAs.
///
/// Uses `git diff-tree` to efficiently identify which files were added, modified,
/// or deleted between two tree states. This is much faster than scanning all files.
///
/// # Arguments
/// * `old_tree` - The old tree SHA (or "init" for initial scan)
/// * `new_tree` - The new tree SHA
/// * `repo_path` - Path to the git repository
///
/// # Returns
/// * `Ok(Vec<FileChange>)` - List of changed files with their status
/// * `Err` - If git command fails or output parsing fails
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use maproom::git::{get_git_tree_sha, git_diff_tree};
///
/// let repo_path = Path::new("/path/to/repo");
/// let old_tree = "abc123...";  // Previous tree SHA
/// let new_tree = get_git_tree_sha(repo_path)?;
///
/// let changes = git_diff_tree(old_tree, &new_tree, repo_path)?;
/// println!("Found {} changed files", changes.len());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn git_diff_tree(old_tree: &str, new_tree: &str, repo_path: &Path) -> Result<Vec<FileChange>> {
    let output = Command::new("git")
        .args([
            "diff-tree",
            "-r",              // Recursive
            "--no-commit-id",  // Don't show commit hash
            "--name-status",   // Show status (A/M/D) and filename
            "--diff-filter=AMD", // Only Added, Modified, Deleted
            old_tree,
            new_tree,
        ])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        bail!(
            "git diff-tree failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    parse_diff_tree_output(&String::from_utf8(output.stdout)?)
}

/// Parses the output of `git diff-tree --name-status`.
///
/// Expected format: `<status><tab><filename>`
/// Example: `M\tsrc/main.rs`
fn parse_diff_tree_output(output: &str) -> Result<Vec<FileChange>> {
    let mut changes = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let status = match parts[0] {
            "A" => FileStatus::Added,
            "M" => FileStatus::Modified,
            "D" => FileStatus::Deleted,
            _ => continue, // Skip unknown status codes
        };

        changes.push(FileChange {
            status,
            path: PathBuf::from(parts[1]),
        });
    }

    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_get_git_tree_sha() {
        // Test in current repo
        let repo_path = Path::new(".");
        let result = get_git_tree_sha(repo_path);
        assert!(result.is_ok(), "Should get tree SHA from current repo");

        let tree_sha = result.unwrap();
        assert_eq!(tree_sha.len(), 40, "Git SHA-1 should be 40 characters");
        assert!(
            tree_sha.chars().all(|c| c.is_ascii_hexdigit()),
            "Tree SHA should be hexadecimal"
        );
    }

    #[test]
    fn test_get_current_branch() {
        let repo_path = Path::new(".");
        let result = get_current_branch(repo_path);
        assert!(result.is_ok(), "Should get branch name from current repo");

        let branch = result.unwrap();
        assert!(!branch.is_empty(), "Branch name should not be empty");
    }

    #[test]
    fn test_parse_diff_tree_output() {
        let output = "A\tsrc/new_file.rs\nM\tsrc/existing.rs\nD\tsrc/old.rs\n";
        let result = parse_diff_tree_output(output);
        assert!(result.is_ok(), "Should parse diff-tree output successfully");

        let changes = result.unwrap();
        assert_eq!(changes.len(), 3, "Should parse 3 file changes");

        assert_eq!(changes[0].status, FileStatus::Added);
        assert_eq!(changes[0].path, PathBuf::from("src/new_file.rs"));

        assert_eq!(changes[1].status, FileStatus::Modified);
        assert_eq!(changes[1].path, PathBuf::from("src/existing.rs"));

        assert_eq!(changes[2].status, FileStatus::Deleted);
        assert_eq!(changes[2].path, PathBuf::from("src/old.rs"));
    }

    #[test]
    fn test_parse_diff_tree_empty_output() {
        let output = "";
        let result = parse_diff_tree_output(output);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0, "Empty output should yield no changes");
    }

    #[test]
    fn test_parse_diff_tree_unknown_status() {
        // 'R' for renamed, 'C' for copied - should be skipped
        let output = "R100\told.rs\tnew.rs\nM\tvalid.rs\n";
        let result = parse_diff_tree_output(output);
        assert!(result.is_ok());

        let changes = result.unwrap();
        assert_eq!(changes.len(), 1, "Should only parse valid status codes");
        assert_eq!(changes[0].status, FileStatus::Modified);
        assert_eq!(changes[0].path, PathBuf::from("valid.rs"));
    }
}
