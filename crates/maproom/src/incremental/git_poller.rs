//! Git-based file change polling.
//!
//! This module implements file change detection using `git status --porcelain`
//! instead of native file watchers. This approach eliminates "too many open files"
//! errors on large repositories while providing consistent cross-platform behavior.
//!
//! # Architecture
//!
//! The GitPoller runs an async loop that:
//! 1. Executes `git status --porcelain` at configurable intervals
//! 2. Parses output into GitState using the git_state module
//! 3. Compares with previous state to detect changes
//! 4. Emits FileEvents for detected changes
//!
//! # Trade-offs
//!
//! - **Latency**: 2-5 second detection latency (acceptable for dev workflows)
//! - **Resources**: Zero file descriptors used (vs thousands with notify)
//! - **Git-aware**: Automatically respects .gitignore patterns
//! - **Requirements**: Must be in a git repository, git must be in PATH
//!
//! # Initial Behavior
//!
//! On first poll, previous_state is empty. This means all dirty files in the
//! first poll emit Modified events, which triggers initial indexing of changed
//! files. This is intentional and acceptable behavior.

use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;
use tokio::process::Command;
use tokio::sync::{mpsc, watch};
use tracing::{debug, info, warn};

use super::events::FileEvent;
use super::git_state::{GitState, GitStateError};

/// Configuration for the git poller.
#[derive(Debug, Clone)]
pub struct GitPollerConfig {
    /// Polling interval (default: 3 seconds).
    pub poll_interval: Duration,

    /// Include untracked files in detection (default: true).
    /// When true, uses `git status --porcelain` which includes untracked.
    pub include_untracked: bool,

    /// Enable rename detection (default: true).
    /// When true, adds `-M` flag to git status.
    pub detect_renames: bool,

    /// Timeout for git command (default: 10 seconds).
    pub git_timeout: Duration,

    /// Number of retries on transient git failures (default: 3).
    pub retry_count: u32,

    /// Delay between retries (default: 1 second).
    pub retry_delay: Duration,

    /// Channel capacity for events (default: 1000).
    pub channel_capacity: usize,
}

impl Default for GitPollerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(3),
            include_untracked: true,
            detect_renames: true,
            git_timeout: Duration::from_secs(10),
            retry_count: 3,
            retry_delay: Duration::from_secs(1),
            channel_capacity: 1000,
        }
    }
}

/// Errors that can occur during git polling.
#[derive(Debug, Error)]
pub enum GitPollerError {
    /// Git command failed with an error.
    #[error("git command failed: {stderr}")]
    GitExecutionError { stderr: String },

    /// Git command timed out.
    #[error("git command timed out after {timeout:?}")]
    GitTimeout { timeout: Duration },

    /// Path is not a git repository.
    #[error("not a git repository: {path}")]
    NotGitRepository { path: PathBuf },

    /// A git operation (rebase, merge, etc.) is in progress.
    #[error("git operation in progress (index.lock exists)")]
    GitOperationInProgress,

    /// Failed to parse git status output.
    #[error("failed to parse git status: {reason}")]
    ParseError { line: String, reason: String },

    /// Event channel was closed.
    #[error("event channel closed")]
    ChannelClosed,

    /// IO error during git execution.
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<GitStateError> for GitPollerError {
    fn from(err: GitStateError) -> Self {
        match err {
            GitStateError::InvalidPath { path, reason } => GitPollerError::ParseError {
                line: path.display().to_string(),
                reason,
            },
            GitStateError::ParseError { line, reason } => GitPollerError::ParseError { line, reason },
        }
    }
}

/// Git-based file change poller.
///
/// Monitors a git repository for file changes by periodically running
/// `git status --porcelain` and comparing state between polls.
pub struct GitPoller {
    /// Root directory of the git repository.
    root: PathBuf,

    /// Configuration for polling behavior.
    config: GitPollerConfig,

    /// Previous git state for comparison.
    previous_state: GitState,

    /// Channel sender for file events.
    event_tx: mpsc::Sender<FileEvent>,

    /// Receiver for shutdown signal.
    shutdown_rx: watch::Receiver<bool>,
}

impl GitPoller {
    /// Create a new GitPoller for the given repository root.
    ///
    /// Returns the poller instance, an event receiver, and a shutdown sender.
    ///
    /// # Errors
    ///
    /// Returns an error if the path is not a git repository.
    pub fn new(
        root: PathBuf,
        config: GitPollerConfig,
    ) -> Result<(Self, mpsc::Receiver<FileEvent>, watch::Sender<bool>), GitPollerError> {
        // Verify it's a git repository synchronously
        let output = std::process::Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(&root)
            .output()
            .map_err(|e| GitPollerError::GitExecutionError {
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(GitPollerError::NotGitRepository { path: root });
        }

        let (event_tx, event_rx) = mpsc::channel(config.channel_capacity);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let poller = Self {
            root,
            config,
            previous_state: GitState::default(),
            event_tx,
            shutdown_rx,
        };

        Ok((poller, event_rx, shutdown_tx))
    }

    /// Run the polling loop until shutdown is signaled.
    ///
    /// This method runs indefinitely, polling at the configured interval
    /// and emitting FileEvents for detected changes.
    pub async fn run(&mut self) -> Result<(), GitPollerError> {
        let mut interval = tokio::time::interval(self.config.poll_interval);

        // First tick is immediate, which we want for initial state capture
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    match self.poll_with_retry().await {
                        Ok(events) => {
                            for event in events {
                                if self.event_tx.send(event).await.is_err() {
                                    // Receiver dropped, stop polling
                                    return Err(GitPollerError::ChannelClosed);
                                }
                            }
                        }
                        Err(GitPollerError::GitOperationInProgress) => {
                            // Skip this cycle, git is busy
                            debug!("git operation in progress, skipping poll cycle");
                        }
                        Err(e) => {
                            // Log but continue - may recover on next poll
                            warn!("git poll error: {}", e);
                        }
                    }
                }
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        info!("git poller shutting down");
                        return Ok(());
                    }
                }
            }
        }
    }

    /// Execute a single poll cycle without retry logic.
    ///
    /// This is useful for testing and one-off polling.
    pub async fn poll_once(&mut self) -> Result<Vec<FileEvent>, GitPollerError> {
        let output = self.run_git_status().await?;
        let new_state = GitState::from_git_status(&output)?;

        let events = self.previous_state.diff(&new_state);
        self.previous_state = new_state;

        debug!("poll_once: {} events detected", events.len());
        Ok(events)
    }

    /// Execute a poll cycle with retry logic for transient failures.
    async fn poll_with_retry(&mut self) -> Result<Vec<FileEvent>, GitPollerError> {
        let mut last_error = None;

        for attempt in 0..self.config.retry_count {
            match self.poll_once_inner().await {
                Ok(events) => return Ok(events),
                Err(GitPollerError::GitOperationInProgress) => {
                    // Don't retry for lock errors, just skip this cycle
                    return Err(GitPollerError::GitOperationInProgress);
                }
                Err(e) => {
                    debug!("poll attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                    if attempt < self.config.retry_count - 1 {
                        tokio::time::sleep(self.config.retry_delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or(GitPollerError::GitExecutionError {
            stderr: "unknown error after retries".to_string(),
        }))
    }

    /// Inner poll logic without public exposure.
    async fn poll_once_inner(&mut self) -> Result<Vec<FileEvent>, GitPollerError> {
        let output = self.run_git_status().await?;
        let new_state = GitState::from_git_status(&output)?;

        let events = self.previous_state.diff(&new_state);
        self.previous_state = new_state;

        Ok(events)
    }

    /// Execute git status command with timeout.
    async fn run_git_status(&self) -> Result<String, GitPollerError> {
        let mut cmd = Command::new("git");
        cmd.args(["status", "--porcelain"]);

        if self.config.detect_renames {
            cmd.arg("-M");
        }

        cmd.current_dir(&self.root);

        let output = tokio::time::timeout(self.config.git_timeout, cmd.output())
            .await
            .map_err(|_| GitPollerError::GitTimeout {
                timeout: self.config.git_timeout,
            })?
            .map_err(|e| GitPollerError::GitExecutionError {
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // Check for specific error conditions
            if stderr.contains("not a git repository") {
                return Err(GitPollerError::NotGitRepository {
                    path: self.root.clone(),
                });
            }
            if stderr.contains(".git/index.lock") || stderr.contains("index.lock") {
                return Err(GitPollerError::GitOperationInProgress);
            }

            return Err(GitPollerError::GitExecutionError { stderr });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get the root directory being watched.
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Get the current configuration.
    pub fn config(&self) -> &GitPollerConfig {
        &self.config
    }

    /// Get statistics about the current state.
    pub fn stats(&self) -> GitPollerStats {
        GitPollerStats {
            tracked_files: self.previous_state.len(),
        }
    }
}

/// Statistics about the git poller state.
#[derive(Debug, Clone)]
pub struct GitPollerStats {
    /// Number of files currently tracked as dirty.
    pub tracked_files: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper to create a temporary git repository.
    fn create_temp_git_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        dir
    }

    #[test]
    fn test_config_default() {
        let config = GitPollerConfig::default();
        assert_eq!(config.poll_interval, Duration::from_secs(3));
        assert!(config.include_untracked);
        assert!(config.detect_renames);
        assert_eq!(config.git_timeout, Duration::from_secs(10));
        assert_eq!(config.retry_count, 3);
        assert_eq!(config.retry_delay, Duration::from_secs(1));
        assert_eq!(config.channel_capacity, 1000);
    }

    #[test]
    fn test_new_valid_repo() {
        let dir = create_temp_git_repo();
        let config = GitPollerConfig::default();
        let result = GitPoller::new(dir.path().to_path_buf(), config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_invalid_repo() {
        let dir = TempDir::new().unwrap();
        let config = GitPollerConfig::default();
        let result = GitPoller::new(dir.path().to_path_buf(), config);
        assert!(matches!(result, Err(GitPollerError::NotGitRepository { .. })));
    }

    #[tokio::test]
    async fn test_poll_once_empty_repo() {
        let dir = create_temp_git_repo();
        let config = GitPollerConfig::default();
        let (mut poller, _rx, _shutdown) = GitPoller::new(dir.path().to_path_buf(), config).unwrap();

        let events = poller.poll_once().await.unwrap();
        // Empty repo should have no dirty files
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_poll_once_detects_new_file() {
        let dir = create_temp_git_repo();

        // Initial poll to capture baseline
        let config = GitPollerConfig::default();
        let (mut poller, _rx, _shutdown) = GitPoller::new(dir.path().to_path_buf(), config).unwrap();
        let _ = poller.poll_once().await.unwrap();

        // Create a new file
        std::fs::write(dir.path().join("test.txt"), "hello").unwrap();

        // Second poll should detect the new file
        let events = poller.poll_once().await.unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            FileEvent::Modified(path) => {
                assert_eq!(path.file_name().unwrap(), "test.txt");
            }
            _ => panic!("Expected Modified event"),
        }
    }

    #[tokio::test]
    async fn test_poll_once_detects_modification() {
        let dir = create_temp_git_repo();

        // Create and commit a file
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "initial").unwrap();
        std::process::Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Initial poll
        let config = GitPollerConfig::default();
        let (mut poller, _rx, _shutdown) = GitPoller::new(dir.path().to_path_buf(), config).unwrap();
        let _ = poller.poll_once().await.unwrap();

        // Modify the file
        std::fs::write(&file_path, "modified").unwrap();

        // Second poll should detect modification
        let events = poller.poll_once().await.unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            FileEvent::Modified(path) => {
                assert_eq!(path.file_name().unwrap(), "test.txt");
            }
            _ => panic!("Expected Modified event"),
        }
    }

    #[tokio::test]
    async fn test_poll_once_detects_deletion() {
        let dir = create_temp_git_repo();

        // Create an untracked file
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content").unwrap();

        // Initial poll captures the untracked file
        let config = GitPollerConfig::default();
        let (mut poller, _rx, _shutdown) = GitPoller::new(dir.path().to_path_buf(), config).unwrap();
        let _ = poller.poll_once().await.unwrap();

        // Delete the file
        std::fs::remove_file(&file_path).unwrap();

        // Second poll should detect deletion
        let events = poller.poll_once().await.unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            FileEvent::Deleted(path) => {
                assert_eq!(path.file_name().unwrap(), "test.txt");
            }
            _ => panic!("Expected Deleted event"),
        }
    }

    #[tokio::test]
    async fn test_shutdown_signal() {
        let dir = create_temp_git_repo();
        let config = GitPollerConfig {
            poll_interval: Duration::from_millis(50),
            ..Default::default()
        };

        let (mut poller, _rx, shutdown_tx) = GitPoller::new(dir.path().to_path_buf(), config).unwrap();

        // Spawn the poller
        let handle = tokio::spawn(async move { poller.run().await });

        // Give it time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Send shutdown signal
        shutdown_tx.send(true).unwrap();

        // Should complete without error
        let result = tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("poller should stop within timeout");

        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }

    #[test]
    fn test_stats() {
        let dir = create_temp_git_repo();
        let config = GitPollerConfig::default();
        let (poller, _rx, _shutdown) = GitPoller::new(dir.path().to_path_buf(), config).unwrap();

        let stats = poller.stats();
        assert_eq!(stats.tracked_files, 0);
    }

    #[test]
    fn test_error_conversion_from_git_state_error() {
        let git_state_err = GitStateError::InvalidPath {
            path: PathBuf::from("/test"),
            reason: "test reason".to_string(),
        };
        let poller_err: GitPollerError = git_state_err.into();
        assert!(matches!(poller_err, GitPollerError::ParseError { .. }));
    }
}
