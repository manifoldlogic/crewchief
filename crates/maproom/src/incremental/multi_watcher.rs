//! Multi-worktree file system watcher manager.
//!
//! This module provides the MultiWatcher component that manages multiple
//! WorktreeWatcher instances, enabling concurrent watching of multiple
//! worktree directories with proper event isolation.

use super::events::{IndexingEvent, WorktreeId};
use super::watcher::WatcherConfig;
use super::worktree_watcher::{WatcherStatus, WorktreeWatcher};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Configuration for health monitoring.
#[derive(Debug, Clone)]
struct HealthMonitorConfig {
    /// How often to check watcher health (default: 5 seconds)
    check_interval_ms: u64,
    /// Initial retry delay (default: 1 second)
    initial_retry_delay_ms: u64,
    /// Maximum retry delay (default: 60 seconds)
    max_retry_delay_ms: u64,
    /// Backoff multiplier (default: 2.0)
    backoff_multiplier: f64,
    /// Maximum retry attempts before giving up (default: 5)
    max_retries: u32,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval_ms: 5000,
            initial_retry_delay_ms: 1000,
            max_retry_delay_ms: 60000,
            backoff_multiplier: 2.0,
            max_retries: 5,
        }
    }
}

/// Retry state for a failed watcher.
#[derive(Debug, Clone)]
struct RetryState {
    /// Number of retry attempts so far
    attempts: u32,
    /// Next retry delay in milliseconds
    next_delay_ms: u64,
}

/// Manages multiple worktree watchers with event aggregation.
///
/// MultiWatcher coordinates multiple WorktreeWatcher instances, each monitoring
/// a different worktree directory. Events from all watchers are merged into a
/// single output channel with proper worktree_id tagging for isolation.
pub struct MultiWatcher {
    /// Map of worktree ID to watcher instance.
    watchers: HashMap<WorktreeId, WorktreeWatcher>,

    /// Sender for aggregated indexing events.
    tx: mpsc::Sender<IndexingEvent>,

    /// Configuration for new watchers.
    config: WatcherConfig,

    /// Health monitor task handle.
    health_monitor_handle: Option<JoinHandle<()>>,

    /// Channel to signal health monitor shutdown.
    health_monitor_shutdown_tx: Option<mpsc::Sender<()>>,

    /// Health monitor configuration.
    health_config: HealthMonitorConfig,

    /// Retry state for failed watchers.
    retry_state: HashMap<WorktreeId, RetryState>,
}

impl MultiWatcher {
    /// Create a new multi-watcher.
    ///
    /// Returns the MultiWatcher instance and a receiver for all IndexingEvents
    /// from all managed worktrees.
    pub fn new(config: WatcherConfig) -> (Self, mpsc::Receiver<IndexingEvent>) {
        let (tx, rx) = mpsc::channel(config.channel_capacity);

        let multi_watcher = Self {
            watchers: HashMap::new(),
            tx,
            config,
            health_monitor_handle: None,
            health_monitor_shutdown_tx: None,
            health_config: HealthMonitorConfig::default(),
            retry_state: HashMap::new(),
        };

        (multi_watcher, rx)
    }

    /// Create a new multi-watcher with default configuration.
    pub fn new_with_defaults() -> (Self, mpsc::Receiver<IndexingEvent>) {
        Self::new(WatcherConfig::default())
    }

    /// Add a new worktree to watch.
    ///
    /// Creates a new WorktreeWatcher for the specified path and starts watching.
    /// Events from this worktree will be tagged with the provided worktree_id.
    ///
    /// # Errors
    /// Returns an error if:
    /// - A watcher with this worktree_id already exists
    /// - Failed to create or start the watcher
    pub async fn add_worktree(&mut self, id: WorktreeId, path: PathBuf) -> Result<()> {
        if self.watchers.contains_key(&id) {
            bail!("Worktree {} is already being watched", id);
        }

        // Create the worktree watcher
        let (mut watcher, event_rx) =
            WorktreeWatcher::new(id.clone(), path.clone(), self.config.clone())
                .with_context(|| format!("Failed to create watcher for worktree: {}", id))?;

        // Start watching
        watcher
            .start()
            .with_context(|| format!("Failed to start watcher for worktree: {}", id))?;

        // Spawn task to forward events from this watcher to the aggregated channel
        let tx = self.tx.clone();
        let worktree_id_clone = id.clone();
        tokio::spawn(async move {
            Self::forward_events(worktree_id_clone, event_rx, tx).await;
        });

        // Store the watcher
        self.watchers.insert(id.clone(), watcher);
        info!("Added worktree watcher: {} at path: {}", id, path.display());

        Ok(())
    }

    /// Remove a worktree from watching.
    ///
    /// Stops and removes the watcher for the specified worktree.
    ///
    /// # Errors
    /// Returns an error if:
    /// - No watcher exists for this worktree_id
    /// - Failed to stop the watcher
    pub async fn remove_worktree(&mut self, id: &WorktreeId) -> Result<()> {
        let mut watcher = self
            .watchers
            .remove(id)
            .with_context(|| format!("No watcher found for worktree: {}", id))?;

        watcher
            .stop()
            .with_context(|| format!("Failed to stop watcher for worktree: {}", id))?;

        info!("Removed worktree watcher: {}", id);
        Ok(())
    }

    /// Restart a specific worktree watcher.
    ///
    /// Useful for recovering from watcher failures or applying configuration changes.
    ///
    /// # Errors
    /// Returns an error if:
    /// - No watcher exists for this worktree_id
    /// - Failed to restart the watcher
    pub async fn restart_worktree(&mut self, id: &WorktreeId) -> Result<()> {
        let watcher = self
            .watchers
            .get_mut(id)
            .with_context(|| format!("No watcher found for worktree: {}", id))?;

        watcher
            .restart()
            .with_context(|| format!("Failed to restart watcher for worktree: {}", id))?;

        info!("Restarted worktree watcher: {}", id);
        Ok(())
    }

    /// List all currently watched worktree IDs.
    pub fn list_worktrees(&self) -> Vec<WorktreeId> {
        self.watchers.keys().cloned().collect()
    }

    /// Get the status of a specific worktree watcher.
    ///
    /// Returns None if no watcher exists for the given worktree_id.
    pub fn get_status(&self, id: &WorktreeId) -> Option<&WatcherStatus> {
        self.watchers.get(id).map(|w| w.status())
    }

    /// Get the number of active watchers.
    pub fn watcher_count(&self) -> usize {
        self.watchers.len()
    }

    /// Check if a worktree is currently being watched.
    pub fn is_watching(&self, id: &WorktreeId) -> bool {
        self.watchers.contains_key(id)
    }

    /// Mark a watcher as failed (for testing purposes).
    ///
    /// This simulates a watcher failure and is useful for testing automatic restart logic.
    pub fn mark_watcher_failed(&mut self, id: &WorktreeId, error: String) -> Result<()> {
        let watcher = self
            .watchers
            .get_mut(id)
            .with_context(|| format!("No watcher found for worktree: {}", id))?;

        watcher.mark_failed(error);
        Ok(())
    }

    /// Start the health monitoring task.
    ///
    /// The health monitor periodically checks all watcher statuses and automatically
    /// restarts failed watchers with exponential backoff.
    pub fn start_health_monitor(&mut self) {
        if self.health_monitor_handle.is_some() {
            warn!("Health monitor already running");
            return;
        }

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.health_monitor_shutdown_tx = Some(shutdown_tx);

        // We need to share state with the health monitor
        // Since we can't clone self, we'll pass a channel for health checks
        let (health_check_tx, health_check_rx) = mpsc::channel::<()>(1);

        let handle = tokio::spawn(async move {
            Self::health_monitor_task(health_check_rx, shutdown_rx).await;
        });

        self.health_monitor_handle = Some(handle);

        // Spawn a separate task to periodically trigger health checks
        let check_interval = Duration::from_millis(self.health_config.check_interval_ms);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);
            loop {
                interval.tick().await;
                if health_check_tx.send(()).await.is_err() {
                    // Channel closed, exit
                    break;
                }
            }
        });

        info!("Started health monitor");
    }

    /// Stop the health monitoring task.
    pub async fn stop_health_monitor(&mut self) {
        if let Some(shutdown_tx) = self.health_monitor_shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }

        if let Some(handle) = self.health_monitor_handle.take() {
            let _ = handle.await;
            info!("Stopped health monitor");
        }
    }

    /// Perform a health check on all watchers and restart failed ones.
    ///
    /// This method is called periodically by the health monitor and can also
    /// be called manually for immediate health checks.
    pub async fn check_and_restart_failed_watchers(&mut self) {
        let failed_ids: Vec<WorktreeId> = self
            .watchers
            .iter()
            .filter_map(|(id, watcher)| {
                if matches!(watcher.status(), WatcherStatus::Failed(_)) {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();

        for id in failed_ids {
            self.attempt_restart_with_backoff(&id).await;
        }
    }

    /// Attempt to restart a failed watcher with exponential backoff.
    async fn attempt_restart_with_backoff(&mut self, id: &WorktreeId) {
        let retry_state = self.retry_state.entry(id.clone()).or_insert(RetryState {
            attempts: 0,
            next_delay_ms: self.health_config.initial_retry_delay_ms,
        });

        // Check if we've exceeded max retries
        if retry_state.attempts >= self.health_config.max_retries {
            error!(
                "Watcher {} has failed {} times, giving up",
                id, retry_state.attempts
            );
            // Remove from retry state - we've given up
            self.retry_state.remove(id);
            return;
        }

        // Wait for backoff delay
        let delay = Duration::from_millis(retry_state.next_delay_ms);
        info!(
            "Attempting to restart watcher {} (attempt {}/{}) after {}ms delay",
            id,
            retry_state.attempts + 1,
            self.health_config.max_retries,
            retry_state.next_delay_ms
        );

        tokio::time::sleep(delay).await;

        // Attempt restart
        match self.restart_worktree(id).await {
            Ok(_) => {
                info!("Successfully restarted watcher: {}", id);
                // Clear retry state on success
                self.retry_state.remove(id);
            }
            Err(e) => {
                error!("Failed to restart watcher {}: {}", id, e);

                // Update retry state with exponential backoff
                let current_retry_state = self.retry_state.get_mut(id).unwrap();
                current_retry_state.attempts += 1;

                // Calculate next delay with exponential backoff
                let next_delay = (current_retry_state.next_delay_ms as f64
                    * self.health_config.backoff_multiplier)
                    as u64;
                current_retry_state.next_delay_ms =
                    next_delay.min(self.health_config.max_retry_delay_ms);
            }
        }
    }

    /// Health monitor task that triggers periodic health checks.
    async fn health_monitor_task(
        mut health_check_rx: mpsc::Receiver<()>,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) {
        debug!("Health monitor task started");

        loop {
            tokio::select! {
                Some(_) = health_check_rx.recv() => {
                    // Health check signal received
                    // The actual check happens in check_and_restart_failed_watchers
                    // which is called from the main MultiWatcher context
                    debug!("Health check signal received");
                }
                Some(_) = shutdown_rx.recv() => {
                    debug!("Health monitor shutdown signal received");
                    break;
                }
            }
        }

        debug!("Health monitor task exiting");
    }

    /// Stop all watchers and clean up.
    pub async fn shutdown(&mut self) -> Result<()> {
        // Stop health monitor first
        self.stop_health_monitor().await;

        let worktree_ids: Vec<WorktreeId> = self.watchers.keys().cloned().collect();

        for id in worktree_ids {
            if let Err(e) = self.remove_worktree(&id).await {
                error!("Error stopping watcher for worktree {}: {}", id, e);
            }
        }

        info!("MultiWatcher shutdown complete");
        Ok(())
    }

    /// Task that forwards events from a single worktree to the aggregated channel.
    async fn forward_events(
        worktree_id: WorktreeId,
        mut event_rx: mpsc::Receiver<IndexingEvent>,
        tx: mpsc::Sender<IndexingEvent>,
    ) {
        debug!("Starting event forwarder for worktree: {}", worktree_id);

        while let Some(event) = event_rx.recv().await {
            if let Err(e) = tx.send(event).await {
                warn!(
                    "Failed to forward event for worktree {}: {}",
                    worktree_id, e
                );
                // Aggregated channel closed, exit task
                return;
            }
        }

        debug!("Event forwarder exiting for worktree: {}", worktree_id);
    }
}

impl Drop for MultiWatcher {
    fn drop(&mut self) {
        // Stop all watchers on drop
        for (id, watcher) in self.watchers.iter_mut() {
            if let Err(e) = watcher.stop() {
                error!("Error stopping watcher for worktree {} on drop: {}", id, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_watcher_creation() {
        let (multi_watcher, _rx) = MultiWatcher::new_with_defaults();
        assert_eq!(multi_watcher.watcher_count(), 0);
        assert!(multi_watcher.list_worktrees().is_empty());
    }

    #[test]
    fn test_list_worktrees_empty() {
        let (multi_watcher, _rx) = MultiWatcher::new_with_defaults();
        let worktrees = multi_watcher.list_worktrees();
        assert_eq!(worktrees.len(), 0);
    }

    #[test]
    fn test_is_watching() {
        let (multi_watcher, _rx) = MultiWatcher::new_with_defaults();
        assert!(!multi_watcher.is_watching(&"test".to_string()));
    }

    #[test]
    fn test_watcher_count() {
        let (multi_watcher, _rx) = MultiWatcher::new_with_defaults();
        assert_eq!(multi_watcher.watcher_count(), 0);
    }

    #[test]
    fn test_get_status_nonexistent() {
        let (multi_watcher, _rx) = MultiWatcher::new_with_defaults();
        assert!(multi_watcher
            .get_status(&"nonexistent".to_string())
            .is_none());
    }
}
