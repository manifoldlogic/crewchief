//! Configuration management for Maproom hybrid search.
//!
//! This module provides a comprehensive configuration system with:
//! - YAML configuration file loading
//! - Environment variable overrides
//! - Hot reload support for fusion weights
//! - Feature flags for toggling search capabilities
//! - Thread-safe configuration access
//!
//! # Configuration Loading
//!
//! Configuration is loaded in the following order (later sources override earlier ones):
//! 1. Default configuration file (`config/maproom-search.yml`)
//! 2. Environment-specific overrides via `MAPROOM_SEARCH_*` variables
//!
//! # Environment Variables
//!
//! Any configuration value can be overridden using environment variables with the pattern:
//! ```text
//! MAPROOM_SEARCH_<SECTION>_<KEY>=<value>
//! ```
//!
//! Examples:
//! - `MAPROOM_SEARCH_FUSION_WEIGHTS_FTS=0.5`
//! - `MAPROOM_SEARCH_PERFORMANCE_TIMEOUT_MS=200`
//! - `MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_VECTOR_SEARCH=false`
//!
//! # Hot Reload
//!
//! Fusion weights can be updated at runtime without restarting the service when
//! `feature_flags.enable_hot_reload=true`. Changes to the configuration file are
//! detected and applied automatically after validation.
//!
//! # Examples
//!
//! ## Load Configuration
//!
//! ```no_run
//! use crewchief_maproom::config::SearchConfig;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Load from default path (config/maproom-search.yml)
//!     let config = SearchConfig::load_default().await?;
//!
//!     println!("Fusion method: {:?}", config.fusion.method);
//!     println!("FTS weight: {}", config.fusion.weights.fts);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Access Configuration with Thread Safety
//!
//! ```no_run
//! use crewchief_maproom::config::SearchConfig;
//! use std::sync::Arc;
//! use tokio::sync::RwLock;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Arc::new(RwLock::new(SearchConfig::load_default().await?));
//!
//!     // Read configuration
//!     {
//!         let cfg = config.read().await;
//!         println!("Vector search enabled: {}", cfg.feature_flags.enable_vector_search);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Hot Reload
//!
//! ```no_run
//! use crewchief_maproom::config::{SearchConfig, ConfigReloader};
//! use std::sync::Arc;
//! use tokio::sync::RwLock;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Arc::new(RwLock::new(SearchConfig::load_default().await?));
//!
//!     // Start hot reload watcher
//!     let mut reloader = ConfigReloader::new(config.clone(), "config/maproom-search.yml")?;
//!
//!     // Spawn watcher in background
//!     tokio::spawn(async move {
//!         if let Err(e) = reloader.watch().await {
//!             eprintln!("Hot reload error: {}", e);
//!         }
//!     });
//!
//!     // Configuration updates automatically as file changes
//!     tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
//!
//!     Ok(())
//! }
//! ```

mod feature_flags;
mod hot_reload;
mod search_config;
mod sqlite_config;

#[cfg(test)]
mod tests;

// Re-export cache configuration from cache module
pub use crate::cache::{CacheConfig, LayerConfig};

pub use feature_flags::FeatureFlags;
pub use hot_reload::{ConfigReloader, HotReloadError};
pub use search_config::{
    BufferConfig, DatabaseConfig, EdgeQualityWeights, EmbeddingConfig, FusionConfig, FusionMethod,
    GraphImportanceConfig, IndexConfig, IndexingConfig, PerformanceConfig, RuntimeConfig,
    SearchConfig, SearchConfigError,
};
pub use sqlite_config::{PoolConfig, PragmaConfig, RetryConfig, SqliteConfig, SqliteConfigError};
