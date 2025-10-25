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
//! # Example
//!
//! ```no_run
//! use crewchief_maproom::cache::{CacheSystem, CacheConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = CacheConfig::default();
//!     let cache_system = CacheSystem::new(config);
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
//!     Ok(())
//! }
//! ```

pub mod entry;
pub mod stats;
pub mod system;

pub use entry::CacheEntry;
pub use stats::{CacheStats, CacheStatsSnapshot, MultiLayerStats};
pub use system::{CacheConfig, CacheSystem, LayerConfig};
