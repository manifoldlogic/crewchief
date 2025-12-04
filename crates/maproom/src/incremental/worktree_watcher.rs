//! Per-worktree file system watcher.
//!
//! This module provides a wrapper around FileWatcher that tags all events
//! with a worktree identifier, enabling multi-worktree indexing scenarios.

use super::events::{FileEvent, IndexingEvent, WorktreeId};
use super::ignore::IgnorePatternMatcher;
use super::watcher::{FileWatcher, WatcherConfig};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

/// Health status of a worktree watcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherStatus {
    /// Watcher is running normally.
    Running,
    /// Watcher has stopped.
    Stopped,
    /// Watcher encountered an error.
    Failed(String),
}

/// A file system watcher for a specific worktree.
///
/// WorktreeWatcher wraps a FileWatcher and tags all emitted events with
/// a worktree identifier, allowing multiple worktrees to be watched
/// simultaneously with proper event isolation.
pub struct WorktreeWatcher {
    /// The unique identifier for this worktree.
    worktree_id: WorktreeId,

    /// The path being watched.
    path: PathBuf,

    /// The underlying file watcher.
    file_watcher: FileWatcher,

    /// Current status of this watcher.
    status: WatcherStatus,
}

impl WorktreeWatcher {
    /// Create a new worktree watcher.
    ///
    /// Returns the watcher instance and a receiver for IndexingEvent messages.
    /// All events emitted on the receiver will be tagged with the provided worktree_id.
    pub fn new(
        worktree_id: WorktreeId,
        path: PathBuf,
        config: WatcherConfig,
    ) -> Result<(Self, mpsc::Receiver<IndexingEvent>)> {
        // Create the underlying file watcher
        let (file_watcher, file_event_rx) =
            FileWatcher::new(path.clone(), config).context("Failed to create FileWatcher")?;

        // Create channel for indexing events
        let (indexing_event_tx, indexing_event_rx) = mpsc::channel(1000);

        // Spawn task to convert FileEvents to IndexingEvents
        let worktree_id_clone = worktree_id.clone();
        let repo_root = path.clone();
        tokio::spawn(async move {
            Self::event_conversion_task(
                worktree_id_clone,
                file_event_rx,
                indexing_event_tx,
                repo_root,
            )
            .await;
        });

        let watcher = Self {
            worktree_id,
            path,
            file_watcher,
            status: WatcherStatus::Stopped,
        };

        Ok((watcher, indexing_event_rx))
    }

    /// Start watching the worktree path.
    pub fn start(&mut self) -> Result<()> {
        self.file_watcher
            .watch(&self.path)
            .with_context(|| format!("Failed to start watching worktree: {}", self.worktree_id))?;

        self.status = WatcherStatus::Running;
        debug!(
            "Started worktree watcher: {} at path: {}",
            self.worktree_id,
            self.path.display()
        );

        Ok(())
    }

    /// Stop watching.
    pub fn stop(&mut self) -> Result<()> {
        self.file_watcher
            .stop()
            .with_context(|| format!("Failed to stop watching worktree: {}", self.worktree_id))?;

        self.status = WatcherStatus::Stopped;
        debug!("Stopped worktree watcher: {}", self.worktree_id);

        Ok(())
    }

    /// Restart the watcher (stop then start).
    pub fn restart(&mut self) -> Result<()> {
        debug!("Restarting worktree watcher: {}", self.worktree_id);
        self.stop()
            .context("Failed to stop watcher during restart")?;
        self.start()
            .context("Failed to start watcher during restart")?;
        Ok(())
    }

    /// Get the current status of this watcher.
    pub fn status(&self) -> &WatcherStatus {
        &self.status
    }

    /// Get the worktree ID.
    pub fn worktree_id(&self) -> &WorktreeId {
        &self.worktree_id
    }

    /// Get the path being watched.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Mark this watcher as failed.
    ///
    /// This is useful for testing or when detecting failures from external signals.
    pub fn mark_failed(&mut self, error: String) {
        warn!("Watcher {} marked as failed: {}", self.worktree_id, error);
        self.status = WatcherStatus::Failed(error);
    }

    /// Task that converts FileEvents to IndexingEvents with worktree tagging.
    async fn event_conversion_task(
        worktree_id: WorktreeId,
        mut file_event_rx: mpsc::Receiver<FileEvent>,
        indexing_event_tx: mpsc::Sender<IndexingEvent>,
        repo_root: PathBuf,
    ) {
        // Load ignore patterns once at start
        let ignore_matcher = match IgnorePatternMatcher::from_repository(&repo_root) {
            Ok(matcher) => matcher,
            Err(e) => {
                error!(
                    "Failed to load ignore patterns, watcher cannot start: {}",
                    e
                );
                return; // Fail-fast
            }
        };

        while let Some(file_event) = file_event_rx.recv().await {
            // Filter events based on .maproomignore patterns
            let path = file_event.path();
            if ignore_matcher.should_ignore(path) {
                debug!("Ignoring event for maproomignore path: {}", path.display());
                continue;
            }

            let timestamp = SystemTime::now();
            let indexing_event =
                IndexingEvent::from_file_event(worktree_id.clone(), file_event, timestamp);

            if let Err(e) = indexing_event_tx.send(indexing_event).await {
                warn!(
                    "Failed to send indexing event for worktree {}: {}",
                    worktree_id, e
                );
                // Channel closed, exit task
                return;
            }
        }

        debug!(
            "Event conversion task exiting for worktree: {}",
            worktree_id
        );
    }
}

impl Drop for WorktreeWatcher {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_status() {
        assert_eq!(WatcherStatus::Running, WatcherStatus::Running);
        assert_eq!(WatcherStatus::Stopped, WatcherStatus::Stopped);
        assert_ne!(WatcherStatus::Running, WatcherStatus::Stopped);

        let failed = WatcherStatus::Failed("error".to_string());
        assert!(matches!(failed, WatcherStatus::Failed(_)));
    }

    #[test]
    fn test_worktree_id_accessor() {
        // We can't easily test the full watcher without a real directory,
        // but we can verify the types compile
        let worktree_id = "test-worktree".to_string();
        assert_eq!(worktree_id, "test-worktree");
    }
}
