//! Branch switch detection via .git/HEAD file watching
//!
//! This module implements automatic detection of git branch switches by monitoring
//! the `.git/HEAD` file for changes. When a branch switch is detected, it triggers
//! incremental indexing to keep the search index synchronized with the current branch.

use anyhow::{bail, Result};
use notify::{Event, RecursiveMode, Watcher};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use tokio_postgres::Client;
use tracing::{error, info};

/// Watches .git/HEAD for branch switches and triggers automatic indexing
pub struct BranchWatcher {
    /// Path to the git repository root
    repo_path: PathBuf,
    /// Database client for indexing operations
    #[allow(dead_code)] // Will be used in BRWATCH-2001 (handle_branch_switch implementation)
    client: Client,
    /// File system watcher
    watcher: notify::RecommendedWatcher,
    /// Event receiver channel
    rx: Receiver<Result<Event, notify::Error>>,
}

impl BranchWatcher {
    /// Creates a new branch watcher for the specified repository
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the git repository root (containing `.git/`)
    /// * `client` - PostgreSQL client for database operations
    ///
    /// # Errors
    ///
    /// Returns error if file watcher initialization fails
    pub fn new(repo_path: PathBuf, client: Client) -> Result<Self> {
        let (tx, rx) = channel();
        let watcher = notify::recommended_watcher(tx)?;

        Ok(Self {
            repo_path,
            client,
            watcher,
            rx,
        })
    }

    /// Starts watching for branch switches and indexes the current branch
    ///
    /// This method:
    /// 1. Validates that `.git/HEAD` exists
    /// 2. Begins watching the file for changes
    /// 3. Indexes the current branch initially
    /// 4. Enters the watch loop (blocks until error/shutdown)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Not a git repository (no `.git/HEAD`)
    /// - File watcher fails to start
    /// - Initial indexing fails
    pub async fn start(&mut self) -> Result<()> {
        let git_head = self.repo_path.join(".git/HEAD");

        if !git_head.exists() {
            bail!("Not a git repository: {}", self.repo_path.display());
        }

        info!("Watching {} for branch switches", git_head.display());
        self.watcher
            .watch(&git_head, RecursiveMode::NonRecursive)?;

        // Initial index of current branch
        self.index_current_branch().await?;

        // Watch loop
        self.watch_loop().await?;

        Ok(())
    }

    /// Main event loop processing file system changes
    async fn watch_loop(&mut self) -> Result<()> {
        loop {
            match self.rx.recv() {
                Ok(event_result) => match event_result {
                    Ok(event) => {
                        // Process modify events (branch switch changes .git/HEAD content)
                        if event.kind.is_modify() || event.kind.is_create() {
                            if let Err(e) = self.handle_branch_switch().await {
                                error!("Failed to handle branch switch: {}", e);
                                // Continue watching despite error
                            }
                        }
                    }
                    Err(e) => {
                        error!("Watcher error: {}", e);
                        // Continue watching despite error
                    }
                },
                Err(e) => {
                    error!("Channel error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Indexes the current branch on watcher startup
    async fn index_current_branch(&self) -> Result<()> {
        info!("Indexing current branch...");
        self.handle_branch_switch().await
    }

    /// Handles a detected branch switch by triggering incremental update
    ///
    /// This method will be implemented in BRWATCH-2001 to integrate with
    /// the incremental_update function from BRANCHX.
    async fn handle_branch_switch(&self) -> Result<()> {
        // TODO: Implement in BRWATCH-2001
        // Will extract branch name, get/create worktree, and trigger incremental update
        Ok(())
    }
}

/// Extracts the current branch name from a git repository
///
/// Reads `.git/HEAD` and parses either a branch reference or detached HEAD.
///
/// # Branch Formats
///
/// - Standard branch: `ref: refs/heads/main` → `"main"`
/// - Feature branch: `ref: refs/heads/feature/auth` → `"feature/auth"`
/// - Detached HEAD: `abc123def...` → `"abc123de"` (first 8 chars)
///
/// # Arguments
///
/// * `repo_path` - Path to repository root
///
/// # Errors
///
/// Returns error if:
/// - `.git/HEAD` doesn't exist
/// - Cannot read file
/// - Content has invalid format
/// - SHA is too short (<8 characters)
///
/// # Example
///
/// ```rust,no_run
/// use std::path::Path;
/// use crewchief_maproom::watcher::get_current_branch;
///
/// let branch = get_current_branch(Path::new("/workspace/myproject")).unwrap();
/// assert_eq!(branch, "main");
/// ```
pub fn get_current_branch(repo_path: &Path) -> Result<String> {
    let head_path = repo_path.join(".git/HEAD");
    let content = fs::read_to_string(&head_path)?;

    // Parse "ref: refs/heads/main" or commit SHA
    if let Some(branch_ref) = content.strip_prefix("ref: refs/heads/") {
        Ok(branch_ref.trim().to_string())
    } else {
        // Detached HEAD (commit SHA)
        let sha = content.trim();
        // Validate it's a hex string (valid SHA)
        if sha.len() >= 8 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(sha[..8].to_string()) // Short SHA
        } else {
            bail!("Invalid HEAD content: expected branch ref or commit SHA")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to parse HEAD content without file I/O (for testing)
    fn parse_head_content(content: &str) -> Result<String> {
        if let Some(branch_ref) = content.strip_prefix("ref: refs/heads/") {
            Ok(branch_ref.trim().to_string())
        } else {
            let sha = content.trim();
            // Validate it's a hex string (valid SHA)
            if sha.len() >= 8 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
                Ok(sha[..8].to_string())
            } else {
                bail!("Invalid HEAD content: expected branch ref or commit SHA")
            }
        }
    }

    #[test]
    fn test_parse_branch_ref() {
        let branch = parse_head_content("ref: refs/heads/main\n").unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_parse_feature_branch() {
        let branch = parse_head_content("ref: refs/heads/feature/auth-system\n").unwrap();
        assert_eq!(branch, "feature/auth-system");
    }

    #[test]
    fn test_parse_detached_head() {
        let branch = parse_head_content("abc123def456789012345678901234567890abcd\n").unwrap();
        assert_eq!(branch, "abc123de"); // Short SHA
    }

    #[test]
    fn test_parse_invalid_format() {
        let result = parse_head_content("invalid format");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_sha() {
        let result = parse_head_content("abc\n");
        assert!(result.is_err()); // SHA too short
    }

    #[test]
    fn test_parse_branch_with_no_newline() {
        let branch = parse_head_content("ref: refs/heads/develop").unwrap();
        assert_eq!(branch, "develop");
    }

    #[test]
    fn test_parse_long_sha() {
        let sha = "a".repeat(40);
        let branch = parse_head_content(&sha).unwrap();
        assert_eq!(branch, "aaaaaaaa");
    }
}
