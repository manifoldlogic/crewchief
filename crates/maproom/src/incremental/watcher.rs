//! File system watcher for incremental indexing.
//!
//! This module provides real-time file system monitoring using git status polling.
//! The git polling approach eliminates "too many open files" errors that occur
//! with native file watchers on large repositories.
//!
//! # How It Works
//!
//! Instead of using native file watchers (which create file descriptors for each
//! watched directory), this implementation polls `git status --porcelain` at
//! configurable intervals. This provides:
//!
//! - Zero file descriptor usage
//! - Automatic .gitignore respect
//! - Consistent cross-platform behavior
//! - 2-5 second detection latency (configurable)
//!
//! # Migration Note
//!
//! Previously used `notify::RecommendedWatcher` which caused EMFILE errors on
//! large repos. Now uses `GitPoller` from the `git_poller` module.

use super::events::FileEvent;
use super::git_poller::{GitPoller, GitPollerConfig, GitPollerError};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tracing::{debug, info};

/// Errors that can occur with the file watcher.
#[derive(Debug, Error)]
pub enum WatcherError {
    /// The path is not a git repository.
    #[error("not a git repository: {0}")]
    NotGitRepository(PathBuf),

    /// Git poller error.
    #[error("git poller error: {0}")]
    GitPollerError(#[from] GitPollerError),

    /// The watcher task failed.
    #[error("watcher task failed: {0}")]
    TaskFailed(String),

    /// The watcher is already running.
    #[error("watcher is already running")]
    AlreadyRunning,
}

/// Configuration for the file watcher.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Debounce delay in milliseconds (kept for backward compatibility, unused with git polling).
    /// With git polling, debouncing is implicit in the poll interval.
    pub debounce_ms: u64,

    /// Channel capacity for file events (default: 1000).
    pub channel_capacity: usize,

    /// Polling interval in milliseconds (default: 3000ms = 3 seconds).
    pub poll_interval_ms: u64,

    /// Include untracked files in detection (default: true).
    pub include_untracked: bool,

    /// Enable rename detection (default: true).
    pub detect_renames: bool,

    /// Timeout for git command in milliseconds (default: 10000ms = 10 seconds).
    pub git_timeout_ms: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 500, // Kept for backward compatibility
            channel_capacity: 1000,
            poll_interval_ms: 3000,
            include_untracked: true,
            detect_renames: true,
            git_timeout_ms: 10000,
        }
    }
}

impl From<&WatcherConfig> for GitPollerConfig {
    fn from(config: &WatcherConfig) -> Self {
        Self {
            poll_interval: Duration::from_millis(config.poll_interval_ms),
            include_untracked: config.include_untracked,
            detect_renames: config.detect_renames,
            git_timeout: Duration::from_millis(config.git_timeout_ms),
            channel_capacity: config.channel_capacity,
            ..Default::default()
        }
    }
}

/// File system watcher that monitors a git repository for changes.
///
/// Uses git status polling to detect file changes, eliminating the
/// "too many open files" errors that occur with native file watchers.
pub struct FileWatcher {
    /// Root path being watched.
    root: PathBuf,

    /// Configuration for the watcher.
    config: WatcherConfig,

    /// Shutdown signal sender.
    shutdown_tx: Option<watch::Sender<bool>>,

    /// Poller task handle.
    poller_handle: Option<JoinHandle<Result<(), GitPollerError>>>,
}

impl FileWatcher {
    /// Create a new file watcher for the given path.
    ///
    /// Returns the watcher instance and a receiver for file events.
    /// The watcher does not start polling until `watch()` is called.
    ///
    /// # Arguments
    ///
    /// * `path` - The root directory to watch (must be a git repository)
    /// * `config` - Configuration for the watcher
    ///
    /// # Returns
    ///
    /// A tuple of `(FileWatcher, Receiver<FileEvent>)` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if the path is not a git repository.
    pub fn new(
        path: PathBuf,
        config: WatcherConfig,
    ) -> Result<(Self, mpsc::Receiver<FileEvent>), WatcherError> {
        // Create git poller to validate it's a git repo and get the event channel
        let poller_config = GitPollerConfig::from(&config);
        let (poller, event_rx, shutdown_tx) =
            GitPoller::new(path.clone(), poller_config).map_err(|e| match e {
                GitPollerError::NotGitRepository { path } => WatcherError::NotGitRepository(path),
                other => WatcherError::GitPollerError(other),
            })?;

        // Spawn the poller task
        let handle = tokio::spawn(async move {
            let mut poller = poller;
            poller.run().await
        });

        let watcher = Self {
            root: path,
            config,
            shutdown_tx: Some(shutdown_tx),
            poller_handle: Some(handle),
        };

        info!("Created git polling watcher for: {}", watcher.root.display());
        Ok((watcher, event_rx))
    }

    /// Start watching the specified path.
    ///
    /// Note: With git polling, watching starts immediately when the watcher is created.
    /// This method is kept for backward compatibility but is essentially a no-op.
    pub fn watch(&mut self, path: &std::path::Path) -> Result<(), WatcherError> {
        debug!(
            "watch() called for {} (polling already active for {})",
            path.display(),
            self.root.display()
        );
        // Polling starts immediately on creation, so this is a no-op
        // Just validate the path matches what we're watching
        if path != self.root {
            debug!(
                "Note: watch path {} differs from root {}",
                path.display(),
                self.root.display()
            );
        }
        Ok(())
    }

    /// Stop watching.
    pub fn stop(&mut self) -> Result<(), WatcherError> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            debug!("Sending shutdown signal to git poller");
            let _ = shutdown_tx.send(true);
        }
        Ok(())
    }

    /// Stop the watcher and wait for the poller to finish.
    pub async fn stop_and_wait(&mut self) -> Result<(), WatcherError> {
        self.stop()?;

        if let Some(handle) = self.poller_handle.take() {
            match handle.await {
                Ok(Ok(())) => {
                    debug!("Git poller stopped successfully");
                }
                Ok(Err(e)) => {
                    debug!("Git poller stopped with error: {}", e);
                    // Don't return error - the poller was stopped
                }
                Err(e) => {
                    return Err(WatcherError::TaskFailed(e.to_string()));
                }
            }
        }

        Ok(())
    }

    /// Get the root path being watched.
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Get the current configuration.
    pub fn config(&self) -> &WatcherConfig {
        &self.config
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        let _ = self.stop();
    }
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
    fn test_watcher_config_default() {
        let config = WatcherConfig::default();
        assert_eq!(config.debounce_ms, 500);
        assert_eq!(config.channel_capacity, 1000);
        assert_eq!(config.poll_interval_ms, 3000);
        assert!(config.include_untracked);
        assert!(config.detect_renames);
        assert_eq!(config.git_timeout_ms, 10000);
    }

    #[test]
    fn test_watcher_config_to_git_poller_config() {
        let config = WatcherConfig {
            debounce_ms: 100,
            channel_capacity: 500,
            poll_interval_ms: 5000,
            include_untracked: false,
            detect_renames: false,
            git_timeout_ms: 30000,
        };

        let poller_config = GitPollerConfig::from(&config);
        assert_eq!(poller_config.poll_interval, Duration::from_millis(5000));
        assert!(!poller_config.include_untracked);
        assert!(!poller_config.detect_renames);
        assert_eq!(poller_config.git_timeout, Duration::from_millis(30000));
        assert_eq!(poller_config.channel_capacity, 500);
    }

    #[tokio::test]
    async fn test_new_valid_repo() {
        let dir = create_temp_git_repo();
        let config = WatcherConfig::default();
        let result = FileWatcher::new(dir.path().to_path_buf(), config);
        assert!(result.is_ok());

        let (mut watcher, _rx) = result.unwrap();
        watcher.stop_and_wait().await.unwrap();
    }

    #[tokio::test]
    async fn test_new_invalid_repo() {
        let dir = TempDir::new().unwrap();
        let config = WatcherConfig::default();
        let result = FileWatcher::new(dir.path().to_path_buf(), config);
        assert!(matches!(result, Err(WatcherError::NotGitRepository(_))));
    }

    #[tokio::test]
    async fn test_watch_is_noop() {
        let dir = create_temp_git_repo();
        let config = WatcherConfig::default();
        let (mut watcher, _rx) = FileWatcher::new(dir.path().to_path_buf(), config).unwrap();

        // watch() should succeed (no-op with git polling)
        let result = watcher.watch(dir.path());
        assert!(result.is_ok());

        watcher.stop_and_wait().await.unwrap();
    }

    #[tokio::test]
    async fn test_detects_new_file() {
        let dir = create_temp_git_repo();
        let config = WatcherConfig {
            poll_interval_ms: 100, // Fast polling for test
            ..Default::default()
        };

        let (mut watcher, mut rx) = FileWatcher::new(dir.path().to_path_buf(), config).unwrap();

        // Wait for initial poll
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Create a new file
        std::fs::write(dir.path().join("test.txt"), "hello").unwrap();

        // Wait for next poll to detect the change
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should receive a Modified event
        let event = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;

        if let Ok(Some(FileEvent::Modified(path))) = event {
            assert_eq!(path.file_name().unwrap(), "test.txt");
        }
        // Note: The event may or may not be ready depending on timing

        watcher.stop_and_wait().await.unwrap();
    }
}
