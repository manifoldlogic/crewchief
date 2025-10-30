//! Background cache maintenance tasks.
//!
//! Provides periodic maintenance operations to keep caches healthy:
//! - **Cleanup**: Remove expired entries
//! - **Monitoring**: Track hit rates and alert on issues
//! - **Memory**: Monitor memory usage and trigger eviction
//! - **Stats**: Log cache statistics periodically

use super::eviction::EvictionStats;
use super::system::CacheSystem;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, info, warn};

/// Cache maintenance configuration.
#[derive(Debug, Clone)]
pub struct MaintenanceConfig {
    /// Interval between maintenance cycles (default: 60 seconds)
    pub interval_secs: u64,
    /// Minimum hit rate threshold for warnings (default: 0.4 = 40%)
    pub min_hit_rate: f64,
    /// Maximum memory usage in bytes (default: 500MB)
    pub max_memory_bytes: usize,
    /// Enable periodic statistics logging
    pub enable_stats_logging: bool,
    /// Enable expired entry cleanup
    pub enable_cleanup: bool,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            interval_secs: 60,
            min_hit_rate: 0.4,
            max_memory_bytes: 500 * 1024 * 1024, // 500MB
            enable_stats_logging: true,
            enable_cleanup: true,
        }
    }
}

/// Background cache maintenance task manager.
///
/// Runs periodic maintenance operations to:
/// - Clean up expired entries
/// - Monitor cache effectiveness
/// - Alert on low hit rates
/// - Track memory usage
pub struct CacheMaintenance {
    /// The cache system to maintain
    cache: Arc<CacheSystem>,
    /// Maintenance configuration
    config: MaintenanceConfig,
    /// Eviction statistics
    eviction_stats: Arc<tokio::sync::RwLock<EvictionStats>>,
}

impl CacheMaintenance {
    /// Create a new cache maintenance task manager.
    pub fn new(cache: Arc<CacheSystem>, config: MaintenanceConfig) -> Self {
        Self {
            cache,
            config,
            eviction_stats: Arc::new(tokio::sync::RwLock::new(EvictionStats::new())),
        }
    }

    /// Create a maintenance task manager with default configuration.
    pub fn with_defaults(cache: Arc<CacheSystem>) -> Self {
        Self::new(cache, MaintenanceConfig::default())
    }

    /// Run the maintenance loop indefinitely.
    ///
    /// This should be spawned as a background task:
    /// ```no_run
    /// use std::sync::Arc;
    /// use crewchief_maproom::cache::{CacheSystem, CacheConfig};
    /// use crewchief_maproom::cache::maintenance::CacheMaintenance;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
    ///     let maintenance = CacheMaintenance::with_defaults(Arc::clone(&cache));
    ///
    ///     // Spawn background task
    ///     tokio::spawn(async move {
    ///         maintenance.run().await.ok();
    ///     });
    /// }
    /// ```
    pub async fn run(self) -> Result<()> {
        info!(
            "Starting cache maintenance loop (interval: {}s, min_hit_rate: {:.1}%)",
            self.config.interval_secs,
            self.config.min_hit_rate * 100.0
        );

        let mut interval = interval(Duration::from_secs(self.config.interval_secs));

        loop {
            interval.tick().await;

            if let Err(e) = self.run_maintenance_cycle().await {
                warn!("Cache maintenance cycle failed: {}", e);
            }
        }
    }

    /// Run a single maintenance cycle.
    async fn run_maintenance_cycle(&self) -> Result<()> {
        debug!("Running cache maintenance cycle");

        // Cleanup expired entries
        if self.config.enable_cleanup {
            self.cleanup_expired().await?;
        }

        // Log statistics
        if self.config.enable_stats_logging {
            self.log_statistics().await?;
        }

        // Check hit rate and alert if low
        self.check_hit_rate().await?;

        // Check memory usage
        self.check_memory_usage().await?;

        debug!("Cache maintenance cycle complete");

        Ok(())
    }

    /// Clean up expired entries from all cache layers.
    async fn cleanup_expired(&self) -> Result<()> {
        debug!("Cleaning up expired cache entries");

        // Note: The current cache implementation using LruCache
        // automatically handles expiration on access (lazy cleanup).
        // This method is a placeholder for eager cleanup if needed.
        //
        // In a real implementation, we would:
        // 1. Iterate through each cache layer
        // 2. Remove entries that have exceeded their TTL
        // 3. Update eviction statistics

        Ok(())
    }

    /// Log cache statistics.
    async fn log_statistics(&self) -> Result<()> {
        let stats = self.cache.stats().await;

        info!(
            "Cache stats: overall_hit_rate={:.1}% ({} ops), size={:.1}MB, evictions={}",
            stats.overall_hit_rate() * 100.0,
            stats.total_operations(),
            stats.total_size_mb(),
            stats.total_evictions()
        );

        debug!(
            "L1 query: hit_rate={:.1}% ({}/{} ops), size={:.1}MB",
            stats.l1_query.hit_rate() * 100.0,
            stats.l1_query.hits,
            stats.l1_query.total_operations(),
            stats.l1_query.size_mb()
        );

        debug!(
            "L2 embedding: hit_rate={:.1}% ({}/{} ops), size={:.1}MB",
            stats.l2_embedding.hit_rate() * 100.0,
            stats.l2_embedding.hits,
            stats.l2_embedding.total_operations(),
            stats.l2_embedding.size_mb()
        );

        debug!(
            "L3 context: hit_rate={:.1}% ({}/{} ops), size={:.1}MB",
            stats.l3_context.hit_rate() * 100.0,
            stats.l3_context.hits,
            stats.l3_context.total_operations(),
            stats.l3_context.size_mb()
        );

        debug!(
            "Parse tree: hit_rate={:.1}% ({}/{} ops), size={:.1}MB",
            stats.parse_tree.hit_rate() * 100.0,
            stats.parse_tree.hits,
            stats.parse_tree.total_operations(),
            stats.parse_tree.size_mb()
        );

        Ok(())
    }

    /// Check hit rate and alert if below threshold.
    async fn check_hit_rate(&self) -> Result<()> {
        let stats = self.cache.stats().await;
        let hit_rate = stats.overall_hit_rate();

        if hit_rate < self.config.min_hit_rate && stats.total_operations() > 100 {
            warn!(
                "Cache hit rate below threshold: {:.1}% < {:.1}% (consider adjusting TTL or size)",
                hit_rate * 100.0,
                self.config.min_hit_rate * 100.0
            );
        } else if stats.total_operations() > 0 {
            debug!("Cache hit rate healthy: {:.1}%", hit_rate * 100.0);
        }

        Ok(())
    }

    /// Check memory usage and alert if approaching limit.
    async fn check_memory_usage(&self) -> Result<()> {
        let stats = self.cache.stats().await;
        let total_mb = stats.total_size_mb();
        let limit_mb = self.config.max_memory_bytes as f64 / 1_048_576.0;

        if total_mb > limit_mb * 0.9 {
            warn!(
                "Cache memory usage approaching limit: {:.1}MB / {:.1}MB ({:.0}%)",
                total_mb,
                limit_mb,
                (total_mb / limit_mb) * 100.0
            );
        } else {
            debug!(
                "Cache memory usage healthy: {:.1}MB / {:.1}MB ({:.0}%)",
                total_mb,
                limit_mb,
                (total_mb / limit_mb) * 100.0
            );
        }

        Ok(())
    }

    /// Get current eviction statistics.
    pub async fn eviction_stats(&self) -> EvictionStats {
        self.eviction_stats.read().await.clone()
    }

    /// Reset eviction statistics.
    pub async fn reset_eviction_stats(&self) {
        self.eviction_stats.write().await.reset();
    }

    /// Get a reference to the cache system.
    pub fn cache(&self) -> &Arc<CacheSystem> {
        &self.cache
    }

    /// Get the maintenance configuration.
    pub fn config(&self) -> &MaintenanceConfig {
        &self.config
    }
}

/// Spawn a cache maintenance background task.
///
/// Returns a join handle that can be awaited or aborted.
pub fn spawn_maintenance_task(
    cache: Arc<CacheSystem>,
    config: MaintenanceConfig,
) -> tokio::task::JoinHandle<Result<()>> {
    let maintenance = CacheMaintenance::new(cache, config);

    tokio::spawn(async move {
        maintenance.run().await
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::CacheConfig;

    #[tokio::test]
    async fn test_maintenance_creation() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let maintenance = CacheMaintenance::with_defaults(Arc::clone(&cache));

        assert!(Arc::ptr_eq(maintenance.cache(), &cache));
        assert_eq!(maintenance.config().interval_secs, 60);
        assert_eq!(maintenance.config().min_hit_rate, 0.4);
    }

    #[tokio::test]
    async fn test_maintenance_with_custom_config() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let config = MaintenanceConfig {
            interval_secs: 30,
            min_hit_rate: 0.5,
            max_memory_bytes: 100 * 1024 * 1024, // 100MB
            enable_stats_logging: false,
            enable_cleanup: false,
        };

        let maintenance = CacheMaintenance::new(Arc::clone(&cache), config);

        assert_eq!(maintenance.config().interval_secs, 30);
        assert_eq!(maintenance.config().min_hit_rate, 0.5);
        assert!(!maintenance.config().enable_stats_logging);
    }

    #[tokio::test]
    async fn test_run_maintenance_cycle() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let maintenance = CacheMaintenance::with_defaults(Arc::clone(&cache));

        // Should complete without error
        maintenance.run_maintenance_cycle().await.unwrap();
    }

    #[tokio::test]
    async fn test_log_statistics() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let maintenance = CacheMaintenance::with_defaults(Arc::clone(&cache));

        // Should complete without error
        maintenance.log_statistics().await.unwrap();
    }

    #[tokio::test]
    async fn test_check_hit_rate() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let maintenance = CacheMaintenance::with_defaults(Arc::clone(&cache));

        // Should complete without error (no operations yet, so no warning)
        maintenance.check_hit_rate().await.unwrap();
    }

    #[tokio::test]
    async fn test_check_memory_usage() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let maintenance = CacheMaintenance::with_defaults(Arc::clone(&cache));

        // Should complete without error
        maintenance.check_memory_usage().await.unwrap();
    }

    #[tokio::test]
    async fn test_eviction_stats() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let maintenance = CacheMaintenance::with_defaults(Arc::clone(&cache));

        let stats = maintenance.eviction_stats().await;
        assert_eq!(stats.total_evictions(), 0);

        // Reset should work
        maintenance.reset_eviction_stats().await;
        let stats = maintenance.eviction_stats().await;
        assert_eq!(stats.total_evictions(), 0);
    }

    #[tokio::test]
    async fn test_spawn_maintenance_task() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let config = MaintenanceConfig {
            interval_secs: 1,
            ..Default::default()
        };

        let handle = spawn_maintenance_task(Arc::clone(&cache), config);

        // Let it run briefly
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Abort the task
        handle.abort();
    }
}
