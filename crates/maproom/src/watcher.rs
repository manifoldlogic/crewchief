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
use std::time::Instant;
use tokio_postgres::Client;
use tracing::{error, info};

use crate::db::{get_or_create_repo, get_or_create_worktree};
use crate::incremental::incremental_update;

/// Watches .git/HEAD for branch switches and triggers automatic indexing
pub struct BranchWatcher {
    /// Path to the git repository root
    repo_path: PathBuf,
    /// Database client for indexing operations
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
    /// Extracts the current branch name, gets or creates the worktree record,
    /// and triggers an incremental update to sync the index with the new branch.
    async fn handle_branch_switch(&self) -> Result<()> {
        // Extract current branch name
        let current_branch = get_current_branch(&self.repo_path)?;

        info!("Branch switch detected: {}", current_branch);

        // Extract repo name from path
        let repo_name = self
            .repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Get or create repo record
        let repo_id = get_or_create_repo(
            &self.client,
            repo_name,
            &self.repo_path.to_string_lossy(),
        )
        .await?;

        // Get or create worktree record
        let worktree_id = get_or_create_worktree(
            &self.client,
            repo_id,
            &current_branch,
            &self.repo_path.to_string_lossy(),
        )
        .await?;

        // Trigger incremental update with timing
        let start = Instant::now();
        let stats = incremental_update(&self.client, worktree_id, &self.repo_path).await?;
        let duration = start.elapsed();

        // Log results
        info!(
            "Index updated in {:.1}s: {} files, {} chunks, {} embeddings",
            duration.as_secs_f64(),
            stats.files_processed,
            stats.chunks_processed,
            stats.embeddings_generated
        );

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
