//! Branch switch detection via .git/HEAD file watching
//!
//! This module implements automatic detection of git branch switches by monitoring
//! the `.git/HEAD` file for changes. When a branch switch is detected, it triggers
//! incremental indexing to keep the search index synchronized with the current branch.

use anyhow::{bail, Result};
use notify::{Event, RecursiveMode, Watcher};
use std::path::PathBuf;
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
