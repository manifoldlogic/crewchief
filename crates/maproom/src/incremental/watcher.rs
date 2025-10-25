//! File system watcher for incremental indexing.
//!
//! This module provides real-time file system monitoring with debouncing,
//! ignore pattern filtering, and event emission for downstream processing.

use super::events::FileEvent;
use super::ignore::IgnorePatternMatcher;
use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

/// Configuration for the file watcher.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Debounce delay in milliseconds (default: 500ms)
    pub debounce_ms: u64,

    /// Channel capacity for file events (default: 1000)
    pub channel_capacity: usize,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 500,
            channel_capacity: 1000,
        }
    }
}

/// File system watcher that monitors a directory for changes.
pub struct FileWatcher {
    /// The notify watcher instance
    watcher: Option<RecommendedWatcher>,

    /// Raw event sender channel (before debouncing)
    raw_event_tx: mpsc::Sender<FileEvent>,

    /// Ignore pattern matcher
    ignore_matcher: Arc<IgnorePatternMatcher>,
}

/// Pending event for debouncing
struct PendingEvent {
    event: FileEvent,
    timestamp: Instant,
}

impl FileWatcher {
    /// Create a new file watcher for the given path.
    ///
    /// Returns the watcher instance and a receiver for file events.
    pub fn new(
        path: PathBuf,
        config: WatcherConfig,
    ) -> Result<(Self, mpsc::Receiver<FileEvent>)> {
        // Create two channels: raw events and debounced events
        let (raw_event_tx, raw_event_rx) = mpsc::channel(config.channel_capacity);
        let (debounced_event_tx, debounced_event_rx) = mpsc::channel(config.channel_capacity);

        // Try to read .gitignore from the watched path
        let gitignore_path = path.join(".gitignore");
        let ignore_matcher = if gitignore_path.exists() {
            IgnorePatternMatcher::from_gitignore(&gitignore_path)
                .context("Failed to load .gitignore patterns")?
        } else {
            IgnorePatternMatcher::new().context("Failed to create default ignore matcher")?
        };

        let watcher = Self {
            watcher: None,
            raw_event_tx,
            ignore_matcher: Arc::new(ignore_matcher),
        };

        // Spawn debounce task
        let debounce_duration = Duration::from_millis(config.debounce_ms);
        tokio::spawn(async move {
            Self::debounce_task(raw_event_rx, debounced_event_tx, debounce_duration).await;
        });

        Ok((watcher, debounced_event_rx))
    }

    /// Start watching the specified path.
    pub fn watch(&mut self, path: &Path) -> Result<()> {
        let raw_event_tx = self.raw_event_tx.clone();
        let ignore_matcher = self.ignore_matcher.clone();

        // Create the notify watcher with event handler
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                match result {
                    Ok(event) => {
                        // Process the event in a blocking context
                        let raw_event_tx = raw_event_tx.clone();
                        let ignore_matcher = ignore_matcher.clone();

                        // Use blocking_send to avoid needing tokio runtime
                        // This runs in notify's event thread, so we can't use async
                        if let Err(e) = Self::handle_notify_event_sync(
                            event,
                            raw_event_tx,
                            ignore_matcher,
                        ) {
                            error!("Error handling file event: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Watch error: {}", e);
                    }
                }
            },
            notify::Config::default(),
        )
        .context("Failed to create file watcher")?;

        // Start watching the path recursively
        watcher
            .watch(path, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch path: {}", path.display()))?;

        debug!("Started watching path: {}", path.display());
        self.watcher = Some(watcher);

        Ok(())
    }

    /// Stop watching.
    pub fn stop(&mut self) -> Result<()> {
        if let Some(watcher) = self.watcher.take() {
            drop(watcher);
            debug!("Stopped file watcher");
        }
        Ok(())
    }

    /// Handle a notify event synchronously (called from notify thread).
    fn handle_notify_event_sync(
        event: Event,
        raw_event_tx: mpsc::Sender<FileEvent>,
        ignore_matcher: Arc<IgnorePatternMatcher>,
    ) -> Result<()> {
        // Convert event to FileEvent and send to raw event channel
        // Debouncing will happen in the separate async task
        let mut file_events = Vec::new();

        match event.kind {
            EventKind::Modify(notify::event::ModifyKind::Name(rename_mode)) => {
                use notify::event::RenameMode;
                match rename_mode {
                    RenameMode::Both => {
                        if event.paths.len() == 2 {
                            let from = event.paths[0].clone();
                            let to = event.paths[1].clone();

                            if !ignore_matcher.should_ignore(&from)
                                || !ignore_matcher.should_ignore(&to)
                            {
                                file_events.push(FileEvent::Renamed(from, to));
                            }
                        }
                    }
                    _ => {
                        for path in event.paths {
                            if !ignore_matcher.should_ignore(&path) {
                                file_events.push(FileEvent::Modified(path));
                            }
                        }
                    }
                }
            }
            EventKind::Create(_) | EventKind::Modify(_) => {
                for path in event.paths {
                    if !ignore_matcher.should_ignore(&path) {
                        file_events.push(FileEvent::Modified(path));
                    }
                }
            }
            EventKind::Remove(_) => {
                for path in event.paths {
                    if !ignore_matcher.should_ignore(&path) {
                        file_events.push(FileEvent::Deleted(path));
                    }
                }
            }
            _ => {
                // Ignore other event types
                debug!("Ignoring event kind: {:?}", event.kind);
            }
        }

        // Send to raw event channel (debouncing happens later)
        for file_event in file_events {
            // Use blocking send - this is called from notify's thread
            if let Err(e) = raw_event_tx.blocking_send(file_event) {
                warn!("Failed to send event: {}", e);
            }
        }

        Ok(())
    }

    /// Debounce task that receives raw events and emits debounced events.
    async fn debounce_task(
        mut raw_event_rx: mpsc::Receiver<FileEvent>,
        debounced_event_tx: mpsc::Sender<FileEvent>,
        debounce_duration: Duration,
    ) {
        let mut pending_events: HashMap<PathBuf, PendingEvent> = HashMap::new();
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            tokio::select! {
                // Receive new events
                Some(event) = raw_event_rx.recv() => {
                    let path = event.path().clone();
                    pending_events.insert(path, PendingEvent {
                        event,
                        timestamp: Instant::now(),
                    });
                }
                // Check for events to emit
                _ = interval.tick() => {
                    let now = Instant::now();
                    let mut to_emit = Vec::new();

                    // Find events that have been quiet for the debounce duration
                    pending_events.retain(|path, pending| {
                        if now.duration_since(pending.timestamp) >= debounce_duration {
                            to_emit.push((path.clone(), pending.event.clone()));
                            false
                        } else {
                            true
                        }
                    });

                    // Send the debounced events
                    for (path, event) in to_emit {
                        if let Err(e) = debounced_event_tx.send(event).await {
                            warn!("Failed to send debounced event for {}: {}", path.display(), e);
                            // Channel closed, exit task
                            return;
                        }
                    }
                }
            }
        }
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

    #[test]
    fn test_watcher_config_default() {
        let config = WatcherConfig::default();
        assert_eq!(config.debounce_ms, 500);
        assert_eq!(config.channel_capacity, 1000);
    }

    // Note: Comprehensive debouncing tests are in the integration test suite
}
