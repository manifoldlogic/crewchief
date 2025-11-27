//! Hot reload mechanism for configuration updates.

use crate::config::SearchConfig;
use anyhow::{Context, Result};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Errors that can occur during hot reload.
#[derive(Error, Debug)]
pub enum HotReloadError {
    #[error("Failed to setup file watcher: {0}")]
    WatcherSetupError(String),

    #[error("File watch error: {0}")]
    WatchError(String),

    #[error("Configuration reload failed: {0}")]
    ReloadError(String),
}

/// Configuration hot reloader.
///
/// Watches the configuration file for changes and reloads it automatically.
/// Only fusion weights are hot-reloadable; other configuration changes require
/// a service restart.
///
/// # Thread Safety
///
/// The reloader uses `Arc<RwLock<SearchConfig>>` for thread-safe configuration
/// access. Multiple readers can access the configuration concurrently, but updates
/// block all access briefly.
///
/// # Validation
///
/// All configuration updates are validated before being applied. Invalid updates
/// are logged and rejected, keeping the existing configuration intact.
///
/// # Examples
///
/// ```no_run
/// use crewchief_maproom::config::{SearchConfig, ConfigReloader};
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let config = Arc::new(RwLock::new(SearchConfig::load_default().await?));
///
///     let mut reloader = ConfigReloader::new(
///         config.clone(),
///         "config/maproom-search.yml"
///     )?;
///
///     // Spawn watcher in background
///     tokio::spawn(async move {
///         if let Err(e) = reloader.watch().await {
///             eprintln!("Hot reload error: {}", e);
///         }
///     });
///
///     // Use config
///     {
///         let cfg = config.read().await;
///         println!("FTS weight: {}", cfg.fusion.weights.fts);
///     }
///
///     tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
///     Ok(())
/// }
/// ```
pub struct ConfigReloader {
    /// Shared configuration
    config: Arc<RwLock<SearchConfig>>,

    /// Path to configuration file
    config_path: PathBuf,

    /// File watcher
    watcher: Option<RecommendedWatcher>,
}

impl ConfigReloader {
    /// Create a new ConfigReloader.
    ///
    /// # Arguments
    /// * `config` - Shared configuration to update
    /// * `config_path` - Path to configuration file to watch
    ///
    /// # Errors
    /// Returns error if file watcher cannot be created.
    pub fn new<P: AsRef<Path>>(config: Arc<RwLock<SearchConfig>>, config_path: P) -> Result<Self> {
        let config_path = config_path.as_ref().to_path_buf();

        if !config_path.exists() {
            warn!(
                "Configuration file does not exist: {}",
                config_path.display()
            );
        }

        Ok(Self {
            config,
            config_path,
            watcher: None,
        })
    }

    /// Start watching the configuration file for changes.
    ///
    /// This method blocks and should be run in a background task.
    ///
    /// # Errors
    /// Returns error if file watching fails.
    pub async fn watch(&mut self) -> Result<()> {
        info!(
            "Starting hot reload watcher for: {}",
            self.config_path.display()
        );

        // Create channel for file system events
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        // Create watcher
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                let _ = tx.blocking_send(result);
            },
            Config::default(),
        )
        .map_err(|e| HotReloadError::WatcherSetupError(e.to_string()))?;

        // Watch the configuration file
        watcher
            .watch(&self.config_path, RecursiveMode::NonRecursive)
            .map_err(|e| HotReloadError::WatchError(e.to_string()))?;

        self.watcher = Some(watcher);

        // Process file system events
        while let Some(result) = rx.recv().await {
            match result {
                Ok(event) => {
                    if self.should_reload(&event) {
                        debug!("Configuration file changed: {:?}", event);
                        if let Err(e) = self.reload_config().await {
                            error!("Failed to reload configuration: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("File watch error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Check if event should trigger a reload.
    fn should_reload(&self, event: &Event) -> bool {
        match event.kind {
            EventKind::Modify(_) | EventKind::Create(_) => {
                // Check if the event is for our config file
                event.paths.iter().any(|p| p.ends_with(&self.config_path))
            }
            _ => false,
        }
    }

    /// Reload configuration from file.
    async fn reload_config(&self) -> Result<()> {
        info!(
            "Reloading configuration from: {}",
            self.config_path.display()
        );

        // Load new configuration
        let new_config = SearchConfig::load_from_file(&self.config_path).await?;

        // Validate new configuration
        new_config
            .validate()
            .context("New configuration validation failed")?;

        // Get current configuration for comparison
        let old_weights = {
            let cfg = self.config.read().await;
            cfg.fusion.weights.clone()
        };

        // Check if hot-reloadable fields changed
        let weights_changed = old_weights.fts != new_config.fusion.weights.fts
            || old_weights.vector != new_config.fusion.weights.vector
            || old_weights.graph != new_config.fusion.weights.graph
            || old_weights.recency != new_config.fusion.weights.recency
            || old_weights.churn != new_config.fusion.weights.churn;

        if weights_changed {
            info!("Fusion weights changed:");
            info!(
                "  Old weights: fts={:.3}, vector={:.3}, graph={:.3}, recency={:.3}, churn={:.3}",
                old_weights.fts,
                old_weights.vector,
                old_weights.graph,
                old_weights.recency,
                old_weights.churn
            );
            info!(
                "  New weights: fts={:.3}, vector={:.3}, graph={:.3}, recency={:.3}, churn={:.3}",
                new_config.fusion.weights.fts,
                new_config.fusion.weights.vector,
                new_config.fusion.weights.graph,
                new_config.fusion.weights.recency,
                new_config.fusion.weights.churn
            );
        }

        // Update configuration
        // For now, we update the entire config. In production, you might want to
        // only update specific hot-reloadable fields to avoid breaking changes.
        {
            let mut cfg = self.config.write().await;

            // Only update hot-reloadable fields
            cfg.fusion.weights = new_config.fusion.weights;
            cfg.fusion.rrf_k = new_config.fusion.rrf_k;
            cfg.fusion.method = new_config.fusion.method;

            info!("Configuration reloaded successfully");
        }

        Ok(())
    }

    /// Manually trigger a configuration reload.
    ///
    /// This is useful for testing or manual updates.
    pub async fn reload(&self) -> Result<()> {
        self.reload_config().await
    }

    /// Get the path to the configuration file being watched.
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }
}

impl Drop for ConfigReloader {
    fn drop(&mut self) {
        if let Some(mut watcher) = self.watcher.take() {
            if let Err(e) = watcher.unwatch(&self.config_path) {
                warn!("Failed to unwatch configuration file: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::io::Write;
    use std::sync::Mutex;
    use tempfile::NamedTempFile;

    // Mutex to serialize tests that load configuration
    static CONFIG_MUTEX: Mutex<()> = Mutex::new(());

    #[tokio::test]
    #[serial]
    async fn test_config_reloader_creation() {
        let config = Arc::new(RwLock::new(SearchConfig::default()));
        let temp_file = NamedTempFile::new().unwrap();

        let reloader = ConfigReloader::new(config, temp_file.path());
        assert!(reloader.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_manual_reload() {
        let _guard = CONFIG_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        // Clean up environment variables that could override file config
        std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_FTS");
        std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_VECTOR");
        std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_GRAPH");
        std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_RECENCY");
        std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_CHURN");

        let config = Arc::new(RwLock::new(SearchConfig::default()));
        let mut temp_file = NamedTempFile::new().unwrap();

        // Write initial config
        let yaml = r#"
embedding:
  provider: openai
  model_name: test-model
  dimension: 1536
  cache_size: 1000
  cache_ttl_seconds: 3600

fusion:
  method: rrf
  rrf_k: 60
  weights:
    fts: 0.5
    vector: 0.3
    graph: 0.1
    recency: 0.08
    churn: 0.02

performance:
  max_candidates_per_method: 100
  final_result_limit: 20
  timeout_ms: 1000
  parallel_execution: true

index:
  ivfflat_lists: 100
  ivfflat_probes: 10
  refresh_interval_seconds: 3600

feature_flags:
  enable_vector_search: true
  enable_hybrid_fusion: true
  enable_graph_signals: true
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
"#;
        temp_file.write_all(yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let reloader = ConfigReloader::new(config.clone(), temp_file.path()).unwrap();

        // Reload configuration
        reloader.reload().await.unwrap();

        // Check that weights were updated
        let cfg = config.read().await;
        assert_eq!(cfg.fusion.weights.fts, 0.5);
        assert_eq!(cfg.fusion.weights.vector, 0.3);
    }

    #[tokio::test]
    #[serial]
    async fn test_invalid_config_rejected() {
        let _guard = CONFIG_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        // Clean up environment variables that could override file config
        std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_FTS");
        std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_VECTOR");
        std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_GRAPH");
        std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_RECENCY");
        std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_CHURN");

        let config = Arc::new(RwLock::new(SearchConfig::default()));
        let mut temp_file = NamedTempFile::new().unwrap();

        // Write invalid config (negative weight)
        let yaml = r#"
embedding:
  provider: openai
  model_name: test-model
  dimension: 1536
  cache_size: 1000
  cache_ttl_seconds: 3600

fusion:
  method: rrf
  rrf_k: 60
  weights:
    fts: -0.5
    vector: 0.3
    graph: 0.1
    recency: 0.08
    churn: 0.02

performance:
  max_candidates_per_method: 100
  final_result_limit: 20
  timeout_ms: 1000
  parallel_execution: true

index:
  ivfflat_lists: 100
  ivfflat_probes: 10
  refresh_interval_seconds: 3600

feature_flags:
  enable_vector_search: true
  enable_hybrid_fusion: true
  enable_graph_signals: true
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
"#;
        temp_file.write_all(yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let reloader = ConfigReloader::new(config.clone(), temp_file.path()).unwrap();

        // Reload should fail
        let result = reloader.reload().await;
        assert!(result.is_err());

        // Original config should be unchanged
        let cfg = config.read().await;
        assert_eq!(cfg.fusion.weights.fts, 0.4); // Default value
    }

    #[test]
    fn test_should_reload() {
        let config = Arc::new(RwLock::new(SearchConfig::default()));
        let temp_file = NamedTempFile::new().unwrap();
        let reloader = ConfigReloader::new(config, temp_file.path()).unwrap();

        // Modify event should trigger reload
        let event = Event::new(EventKind::Modify(notify::event::ModifyKind::Any))
            .add_path(temp_file.path().to_path_buf());
        assert!(reloader.should_reload(&event));

        // Create event should trigger reload
        let event = Event::new(EventKind::Create(notify::event::CreateKind::Any))
            .add_path(temp_file.path().to_path_buf());
        assert!(reloader.should_reload(&event));

        // Remove event should not trigger reload
        let event = Event::new(EventKind::Remove(notify::event::RemoveKind::Any))
            .add_path(temp_file.path().to_path_buf());
        assert!(!reloader.should_reload(&event));
    }
}
