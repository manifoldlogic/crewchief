//! Multi-layer cache system for Maproom.
//!
//! This module provides a unified cache system with multiple layers:
//! - **L1 Query Cache**: Caches search results (100 entries, 1 hour TTL)
//! - **L2 Embedding Cache**: Caches embedding vectors (1000 entries, 24 hour TTL)
//! - **L3 Context Cache**: Caches context bundles (500 entries, 30 min TTL)
//! - **Parse Tree Cache**: Caches parsed ASTs (memory-bounded)
//!
//! # Architecture
//!
//! The cache system uses LRU eviction and TTL-based expiration:
//! - Each cache layer is independently sized and configured
//! - Thread-safe access using Arc<RwLock<>>
//! - Atomic counters for statistics tracking
//! - Configurable TTL per cache layer
//!
//! # Performance Goals
//!
//! - Cache hit rate: >60% (target from PERF_OPT-4001)
//! - Memory usage: <500MB with all caches
//! - Thread-safe concurrent access
//! - Minimal contention with RwLock
//!
//! # Cache Management (PERF_OPT-4002)
//!
//! Cache management features include:
//! - **Eviction policies**: LRU, TTL, size-based, access-count
//! - **Warming strategies**: Startup, predictive, scheduled, manual
//! - **Invalidation logic**: File changes, re-indexing, pattern-based
//! - **Background maintenance**: Periodic cleanup, monitoring, alerts
//!
//! # Example
//!
//! ```no_run
//! use crewchief_maproom::cache::{CacheSystem, CacheConfig};
//! use crewchief_maproom::cache::maintenance::{CacheMaintenance, MaintenanceConfig};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = CacheConfig::default();
//!     let cache_system = Arc::new(CacheSystem::new(config));
//!
//!     // Use L1 query cache
//!     let query_key = "search term".to_string();
//!     if let Some(results) = cache_system.get_query(&query_key).await {
//!         println!("Cache hit!");
//!     }
//!
//!     // Get overall cache statistics
//!     let stats = cache_system.stats().await;
//!     println!("Overall hit rate: {:.1}%", stats.overall_hit_rate() * 100.0);
//!
//!     // Spawn background maintenance
//!     let maintenance_config = MaintenanceConfig::default();
//!     let maintenance = CacheMaintenance::new(Arc::clone(&cache_system), maintenance_config);
//!     tokio::spawn(async move {
//!         maintenance.run().await.ok();
//!     });
//!
//!     Ok(())
//! }
//! ```

pub mod entry;
pub mod eviction;
pub mod invalidation;
pub mod maintenance;
pub mod stats;
pub mod system;
pub mod warming;

pub use entry::CacheEntry;
pub use eviction::{EvictionPolicy, EvictionStats, EvictionStrategy};
pub use invalidation::{CacheInvalidator, CacheLayer, InvalidationStats, InvalidationTrigger};
pub use maintenance::{CacheMaintenance, MaintenanceConfig};
pub use stats::{CacheStats, CacheStatsSnapshot, MultiLayerStats};
pub use system::{CacheConfig, CacheSystem, LayerConfig};
pub use warming::{CacheWarmer, WarmingStats, WarmingStrategy};
