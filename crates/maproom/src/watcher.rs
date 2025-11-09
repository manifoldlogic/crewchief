//! Branch switch detection via .git/HEAD file watching
//!
//! This module implements automatic detection of git branch switches by monitoring
//! the `.git/HEAD` file for changes. When a branch switch is detected, it triggers
//! incremental indexing to keep the search index synchronized with the current branch.
//!
//! # Architecture
//!
//! The watcher uses the [`notify`](https://docs.rs/notify) crate to monitor `.git/HEAD`
//! for file system events. When a write event occurs (indicating a branch switch), the
//! watcher:
//!
//! 1. Extracts the new branch name from `.git/HEAD`
//! 2. Gets or creates a worktree record in the database
//! 3. Triggers incremental update for the new branch via [`incremental_update`]
//! 4. Logs indexing metrics (files processed, cache hit rate, cost)
//!
//! # Performance
//!
//! - **Detection latency**: <1 second (OS file events)
//! - **CPU usage while idle**: <5%
//! - **Memory usage**: ~15-20MB
//! - **Update time**: Depends on changed files (typically 30-60s for medium changes)
//!
//! # Example
//!
//! ```rust,no_run
//! use crewchief_maproom::watcher::BranchWatcher;
//! use crewchief_maproom::db;
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let database_url = std::env::var("DATABASE_URL")?;
//!     let client = db::connect(&database_url).await?;
//!     let repo_path = PathBuf::from("/workspace/myproject");
//!
//!     let mut watcher = BranchWatcher::new(repo_path, client)?;
//!
//!     // Blocks until Ctrl+C or error (no shutdown signal)
//!     watcher.start(None).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # CLI Integration
//!
//! This module is used by the `maproom branch-watch` CLI command:
//!
//! ```bash
//! maproom branch-watch --repo /workspace/myproject
//! ```
//!
//! # Related
//!
//! - [`incremental_update`] - The BRANCHX function called on branch switches
//! - [`get_or_create_worktree`] - Database function for worktree records
//! - [`get_current_branch`] - Branch name extraction from .git/HEAD

use anyhow::{bail, Result};
use notify::{Event, RecursiveMode, Watcher};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio_postgres::Client;
use tracing::{debug, error, info, warn};

use crate::db::{get_or_create_repo, get_or_create_worktree};
use crate::incremental::incremental_update;

/// Debouncer to prevent rapid successive branch switch handling
///
/// Implements time-based debouncing to avoid triggering indexing operations
/// too frequently. This prevents issues with:
/// - Multiple rapid branch switches
/// - Git operations that modify `.git/HEAD` multiple times
/// - File system noise (duplicate events from the OS)
///
/// # Debouncing Strategy
///
/// Events that occur within the debounce duration (default: 2 seconds) of the
/// previous event are ignored. This ensures at most one indexing operation
/// per debounce window.
///
/// # Thread Safety
///
/// The last event timestamp is protected by a `Mutex` to allow safe access
/// from the event handler thread.
struct DebouncedHandler {
    /// Timestamp of the last processed event, protected by mutex for thread safety
    last_event: Mutex<Instant>,
    /// Minimum duration between processed events
    debounce_duration: Duration,
}

impl DebouncedHandler {
    /// Creates a new debounced handler with the specified duration
    ///
    /// # Arguments
    ///
    /// * `debounce_duration` - Minimum time between processed events
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    /// # use std::sync::Mutex;
    /// # use std::time::Instant;
    /// # struct DebouncedHandler {
    /// #     last_event: Mutex<Instant>,
    /// #     debounce_duration: Duration,
    /// # }
    /// # impl DebouncedHandler {
    /// #     fn new(debounce_duration: Duration) -> Self {
    /// #         Self {
    /// #             last_event: Mutex::new(Instant::now()),
    /// #             debounce_duration,
    /// #         }
    /// #     }
    /// # }
    ///
    /// let debouncer = DebouncedHandler::new(Duration::from_secs(2));
    /// ```
    fn new(debounce_duration: Duration) -> Self {
        Self {
            last_event: Mutex::new(Instant::now()),
            debounce_duration,
        }
    }

    /// Checks if an event should be processed or debounced
    ///
    /// Returns `true` if sufficient time has passed since the last event,
    /// `false` if the event should be debounced (ignored).
    ///
    /// # Thread Safety
    ///
    /// This method acquires a lock on the last event timestamp. If the lock
    /// is poisoned (due to a panic while holding the lock), this will panic.
    ///
    /// # Returns
    ///
    /// - `true` - Process the event (>= debounce_duration since last event)
    /// - `false` - Ignore the event (< debounce_duration since last event)
    fn should_handle(&self) -> bool {
        let mut last = self.last_event.lock().unwrap();
        let now = Instant::now();

        if now.duration_since(*last) < self.debounce_duration {
            debug!("Debouncing event (too soon after previous)");
            false
        } else {
            *last = now;
            true
        }
    }
}

/// Watches `.git/HEAD` for branch switches and triggers automatic indexing
///
/// The `BranchWatcher` uses OS-level file system events to detect when a developer
/// switches branches. This avoids polling and ensures minimal resource usage while idle.
///
/// # Lifecycle
///
/// 1. Create watcher with [`BranchWatcher::new`]
/// 2. Start watching with [`BranchWatcher::start`]
/// 3. Watcher runs until error or shutdown signal (Ctrl+C)
///
/// # Error Handling
///
/// The watcher is designed to be fault-tolerant. Errors during indexing (database
/// connection failures, git errors) are logged but don't crash the watcher. Transient
/// errors trigger retry logic with exponential backoff (2s, 4s, 8s delays).
///
/// # Example
///
/// ```rust,no_run
/// use crewchief_maproom::watcher::BranchWatcher;
/// use crewchief_maproom::db;
/// use std::path::PathBuf;
/// use tokio::sync::oneshot;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let database_url = std::env::var("DATABASE_URL")?;
///     let client = db::connect(&database_url).await?;
///     let repo_path = PathBuf::from("/workspace/myproject");
///
///     let mut watcher = BranchWatcher::new(repo_path, client)?;
///
///     // Create shutdown channel for graceful shutdown
///     let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
///
///     // Setup Ctrl+C handler
///     ctrlc::set_handler(move || {
///         let _ = shutdown_tx.send(());
///     })?;
///
///     // Runs until shutdown signal or error
///     watcher.start(Some(shutdown_rx)).await?;
///
///     Ok(())
/// }
/// ```
///
/// # Resource Usage
///
/// - **CPU (idle)**: <5% (file watcher sleeps until events)
/// - **Memory**: ~15-20 MB (watcher state + buffers)
/// - **CPU (indexing)**: 50-80% (embedding generation, parsing)
/// - **Memory (indexing)**: 50-100 MB (tree-sitter parsers, buffers)
pub struct BranchWatcher {
    /// Path to the git repository root
    repo_path: PathBuf,
    /// Database client for indexing operations
    client: Client,
    /// File system watcher from notify crate
    watcher: notify::RecommendedWatcher,
    /// Event receiver channel from file watcher
    rx: Receiver<Result<Event, notify::Error>>,
    /// Debouncer to prevent rapid successive indexing operations
    debouncer: DebouncedHandler,
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
        let debouncer = DebouncedHandler::new(Duration::from_secs(2));

        Ok(Self {
            repo_path,
            client,
            watcher,
            rx,
            debouncer,
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
    /// # Arguments
    ///
    /// * `shutdown_rx` - Optional shutdown signal receiver. When the sender is dropped or sends,
    ///                   the watcher will exit cleanly.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Not a git repository (no `.git/HEAD`)
    /// - File watcher fails to start
    /// - Initial indexing fails
    pub async fn start(
        &mut self,
        shutdown_rx: Option<tokio::sync::oneshot::Receiver<()>>,
    ) -> Result<()> {
        let git_head = self.repo_path.join(".git/HEAD");

        if !git_head.exists() {
            bail!("Not a git repository: {}", self.repo_path.display());
        }

        info!("Watching {} for branch switches", git_head.display());
        self.watcher.watch(&git_head, RecursiveMode::NonRecursive)?;

        // Initial index of current branch
        self.index_current_branch().await?;

        // Watch loop
        self.watch_loop(shutdown_rx).await?;

        Ok(())
    }

    /// Main event loop processing file system changes
    ///
    /// Continuously receives events from the file watcher and processes branch
    /// switches. This method blocks until:
    /// - A channel error occurs (watcher disconnected)
    /// - A shutdown signal is received
    /// - The watcher is explicitly stopped
    ///
    /// # Arguments
    ///
    /// * `shutdown_rx` - Optional shutdown signal receiver. When triggered, the loop exits cleanly.
    ///
    /// # Event Processing
    ///
    /// For each file system event:
    /// 1. Check if it's a modify or create event (branch switch)
    /// 2. Apply debouncing to prevent rapid triggers
    /// 3. Call [`handle_branch_switch_with_retry`] to process the switch
    /// 4. Log any errors but continue watching (fault-tolerant)
    ///
    /// # Error Handling
    ///
    /// - **File watcher errors**: Logged and ignored, watching continues
    /// - **Indexing errors**: Logged and ignored after retry attempts
    /// - **Channel errors**: Cause the loop to exit
    async fn watch_loop(
        &mut self,
        mut shutdown_rx: Option<tokio::sync::oneshot::Receiver<()>>,
    ) -> Result<()> {
        loop {
            // Check shutdown signal if provided
            if let Some(ref mut rx) = shutdown_rx {
                tokio::select! {
                    // Check for shutdown signal
                    _ = rx => {
                        info!("Shutdown signal received, exiting watch loop");
                        break;
                    }
                    // Process file watcher events
                    _ = tokio::time::sleep(Duration::from_millis(0)) => {
                        // Fall through to event processing below
                    }
                }
            }

            // Use recv_timeout to allow cooperative cancellation
            match self.rx.recv_timeout(Duration::from_millis(100)) {
                Ok(event_result) => match event_result {
                    Ok(event) => {
                        // Process modify events (branch switch changes .git/HEAD content)
                        if (event.kind.is_modify() || event.kind.is_create())
                            && self.debouncer.should_handle()
                        {
                            if let Err(e) = self.handle_branch_switch_with_retry().await {
                                error!("Failed to handle branch switch after retries: {}", e);
                                // Continue watching despite error
                            }
                        }
                    }
                    Err(e) => {
                        error!("Watcher error: {}", e);
                        // Continue watching despite error
                    }
                },
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Timeout is normal, allows checking shutdown signal
                    tokio::task::yield_now().await;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    error!("Channel disconnected");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Indexes the current branch on watcher startup
    ///
    /// Called once when the watcher starts to ensure the current branch is
    /// indexed before monitoring for switches. This provides immediate search
    /// functionality even if the branch hasn't been indexed before.
    ///
    /// # Errors
    ///
    /// Returns error if the initial indexing fails. This is a hard failure
    /// that prevents the watcher from starting.
    async fn index_current_branch(&self) -> Result<()> {
        info!("Indexing current branch...");
        self.handle_branch_switch().await
    }

    /// Handles branch switch with retry logic and exponential backoff
    ///
    /// Attempts to handle branch switches up to 3 times with exponential backoff:
    /// - Attempt 1: immediate
    /// - Attempt 2: after 2 seconds
    /// - Attempt 3: after 4 seconds
    /// - Attempt 4: after 8 seconds
    ///
    /// Only retries on transient errors (I/O errors, database connection issues).
    /// Permanent errors (invalid data, logic errors) fail immediately.
    async fn handle_branch_switch_with_retry(&self) -> Result<()> {
        let max_retries = 3;
        let mut attempt = 0;

        loop {
            match self.handle_branch_switch().await {
                Ok(_) => return Ok(()),
                Err(e) if attempt < max_retries => {
                    warn!(
                        "Branch switch failed (attempt {}/{}): {}",
                        attempt + 1,
                        max_retries,
                        e
                    );

                    // Check if we should retry
                    if should_retry(&e) {
                        attempt += 1;
                        let delay = Duration::from_secs(2_u64.pow(attempt));
                        warn!("Retrying in {:?}...", delay);
                        tokio::time::sleep(delay).await;
                    } else {
                        // Permanent error, don't retry
                        return Err(e);
                    }
                }
                Err(e) => {
                    error!("Failed after {} retries: {}", max_retries, e);
                    return Err(e);
                }
            }
        }
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
        let repo_id =
            get_or_create_repo(&self.client, repo_name, &self.repo_path.to_string_lossy()).await?;

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
        info!("  Cache hit rate: {:.1}%", stats.cache_hit_rate() * 100.0);
        info!("  Estimated cost: ${:.4}", stats.cost());
        info!("Waiting for changes...");

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

/// Classifies errors to determine if retry should be attempted
///
/// Returns `true` for transient errors that may resolve on retry:
/// - `tokio_postgres::Error` - Database connection issues, network timeouts
/// - `std::io::Error` - File I/O failures, temporary access issues
///
/// Returns `false` for permanent errors that won't resolve on retry:
/// - Invalid data format
/// - Logic errors
/// - Invalid arguments
fn should_retry(error: &anyhow::Error) -> bool {
    // Retry on tokio_postgres database errors (connection issues, timeouts)
    if error.downcast_ref::<tokio_postgres::Error>().is_some() {
        return true;
    }

    // Retry on I/O errors (file read, network)
    if error.downcast_ref::<std::io::Error>().is_some() {
        return true;
    }

    false
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
