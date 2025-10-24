//! Memory monitoring for cache systems.
//!
//! This module provides functionality to track memory usage across
//! multiple cache layers and enforce memory limits.
//!
//! # Architecture
//!
//! Memory monitoring tracks:
//! 1. **Per-Cache Usage**: Estimated memory per cache layer
//! 2. **Total Usage**: Aggregate across all caches
//! 3. **Memory Limits**: Enforce 500MB total limit
//!
//! # Performance Target
//!
//! Memory tracking should have minimal overhead (<1% CPU).
//!
//! # Example
//!
//! ```no_run
//! use crewchief_maproom::search::memory::MemoryMonitor;
//! use crewchief_maproom::search::SearchCache;
//! use std::sync::Arc;
//!
//! let cache = Arc::new(SearchCache::new(1000));
//! let monitor = MemoryMonitor::new();
//!
//! monitor.register_cache("query_cache", cache);
//!
//! let stats = monitor.stats();
//! println!("Total memory: {}MB", stats.total_mb);
//! ```

use crate::search::cache::SearchCache;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, warn};

/// Default memory limit in megabytes.
const DEFAULT_MEMORY_LIMIT_MB: usize = 500;

/// Estimated memory per search result entry (in bytes).
///
/// This is a rough estimate based on typical search results:
/// - Query string: ~100 bytes
/// - Results array: ~10 results * 500 bytes each = 5KB
/// - Metadata: ~1KB
/// - Total: ~6KB per entry (rounded to 8KB for safety)
const ESTIMATED_BYTES_PER_SEARCH_ENTRY: usize = 8 * 1024; // 8KB

/// Estimated memory per embedding entry (in bytes).
///
/// Based on typical embedding dimensions:
/// - Vector (1536 dimensions * 4 bytes): ~6KB
/// - Cache overhead: ~1KB
/// - Total: ~7KB per entry (rounded to 8KB for consistency)
#[allow(dead_code)]
const ESTIMATED_BYTES_PER_EMBEDDING_ENTRY: usize = 8 * 1024; // 8KB

/// Memory monitor for tracking cache memory usage.
pub struct MemoryMonitor {
    /// Registered caches with their names
    caches: Arc<RwLock<HashMap<String, Arc<SearchCache>>>>,

    /// Memory limit in megabytes
    memory_limit_mb: usize,
}

impl MemoryMonitor {
    /// Create a new MemoryMonitor with default limit.
    pub fn new() -> Self {
        Self::with_limit(DEFAULT_MEMORY_LIMIT_MB)
    }

    /// Create a new MemoryMonitor with custom limit.
    pub fn with_limit(memory_limit_mb: usize) -> Self {
        Self {
            caches: Arc::new(RwLock::new(HashMap::new())),
            memory_limit_mb,
        }
    }

    /// Register a cache for monitoring.
    pub fn register_cache(&self, name: &str, cache: Arc<SearchCache>) {
        let mut caches = self.caches.write().unwrap();
        caches.insert(name.to_string(), cache);
        debug!("Registered cache '{}' for memory monitoring", name);
    }

    /// Unregister a cache from monitoring.
    pub fn unregister_cache(&self, name: &str) -> bool {
        let mut caches = self.caches.write().unwrap();
        caches.remove(name).is_some()
    }

    /// Get current memory statistics.
    pub fn stats(&self) -> MemoryStats {
        let caches = self.caches.read().unwrap();
        let mut cache_stats = HashMap::new();
        let mut total_bytes = 0;

        for (name, cache) in caches.iter() {
            let stats = cache.stats();
            let estimated_bytes = stats.size * ESTIMATED_BYTES_PER_SEARCH_ENTRY;

            cache_stats.insert(
                name.clone(),
                CacheMemoryStats {
                    size: stats.size,
                    capacity: stats.capacity,
                    estimated_mb: estimated_bytes / (1024 * 1024),
                },
            );

            total_bytes += estimated_bytes;
        }

        let total_mb = total_bytes / (1024 * 1024);
        let utilization = if self.memory_limit_mb > 0 {
            (total_mb as f64 / self.memory_limit_mb as f64) * 100.0
        } else {
            0.0
        };

        MemoryStats {
            total_mb,
            limit_mb: self.memory_limit_mb,
            utilization_percent: utilization,
            cache_stats,
        }
    }

    /// Check if memory usage is within limits.
    pub fn is_within_limit(&self) -> bool {
        let stats = self.stats();
        stats.total_mb <= self.memory_limit_mb
    }

    /// Check if memory usage is approaching limit (>80%).
    pub fn is_approaching_limit(&self) -> bool {
        let stats = self.stats();
        stats.utilization_percent > 80.0
    }

    /// Log current memory stats if approaching or exceeding limit.
    pub fn check_and_log(&self) {
        let stats = self.stats();

        if stats.total_mb > self.memory_limit_mb {
            warn!(
                "Cache memory usage EXCEEDS limit: {}MB / {}MB ({:.1}%)",
                stats.total_mb, stats.limit_mb, stats.utilization_percent
            );
        } else if stats.utilization_percent > 80.0 {
            warn!(
                "Cache memory usage approaching limit: {}MB / {}MB ({:.1}%)",
                stats.total_mb, stats.limit_mb, stats.utilization_percent
            );
        } else {
            debug!(
                "Cache memory usage: {}MB / {}MB ({:.1}%)",
                stats.total_mb, stats.limit_mb, stats.utilization_percent
            );
        }
    }

    /// Clear all registered caches if memory limit is exceeded.
    ///
    /// Returns the number of caches cleared.
    pub fn emergency_clear_if_needed(&self) -> usize {
        if !self.is_within_limit() {
            warn!("Emergency cache clear triggered due to memory limit");
            let caches = self.caches.read().unwrap();
            let count = caches.len();

            for (name, cache) in caches.iter() {
                cache.clear();
                debug!("Cleared cache '{}'", name);
            }

            count
        } else {
            0
        }
    }
}

impl Default for MemoryMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory statistics for all monitored caches.
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total memory usage in MB
    pub total_mb: usize,
    /// Memory limit in MB
    pub limit_mb: usize,
    /// Memory utilization percentage (0-100+)
    pub utilization_percent: f64,
    /// Per-cache memory statistics
    pub cache_stats: HashMap<String, CacheMemoryStats>,
}

impl MemoryStats {
    /// Check if memory usage is safe (<80% utilization).
    pub fn is_safe(&self) -> bool {
        self.utilization_percent < 80.0
    }

    /// Check if memory usage is critical (>100% utilization).
    pub fn is_critical(&self) -> bool {
        self.total_mb > self.limit_mb
    }
}

/// Memory statistics for a single cache.
#[derive(Debug, Clone)]
pub struct CacheMemoryStats {
    /// Current number of entries
    pub size: usize,
    /// Maximum capacity
    pub capacity: usize,
    /// Estimated memory usage in MB
    pub estimated_mb: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_monitor_creation() {
        let monitor = MemoryMonitor::new();
        let stats = monitor.stats();

        assert_eq!(stats.total_mb, 0);
        assert_eq!(stats.limit_mb, DEFAULT_MEMORY_LIMIT_MB);
        assert_eq!(stats.utilization_percent, 0.0);
    }

    #[test]
    fn test_memory_monitor_custom_limit() {
        let monitor = MemoryMonitor::with_limit(1000);
        let stats = monitor.stats();

        assert_eq!(stats.limit_mb, 1000);
    }

    #[test]
    fn test_memory_stats_safety_checks() {
        let stats = MemoryStats {
            total_mb: 300,
            limit_mb: 500,
            utilization_percent: 60.0,
            cache_stats: HashMap::new(),
        };

        assert!(stats.is_safe());
        assert!(!stats.is_critical());
    }

    #[test]
    fn test_memory_stats_critical() {
        let stats = MemoryStats {
            total_mb: 600,
            limit_mb: 500,
            utilization_percent: 120.0,
            cache_stats: HashMap::new(),
        };

        assert!(!stats.is_safe());
        assert!(stats.is_critical());
    }

    #[test]
    fn test_cache_registration() {
        let monitor = MemoryMonitor::new();
        let cache = Arc::new(SearchCache::new(100));

        monitor.register_cache("test_cache", cache);

        let stats = monitor.stats();
        assert!(stats.cache_stats.contains_key("test_cache"));
    }

    #[test]
    fn test_cache_unregistration() {
        let monitor = MemoryMonitor::new();
        let cache = Arc::new(SearchCache::new(100));

        monitor.register_cache("test_cache", cache);
        assert!(monitor.unregister_cache("test_cache"));
        assert!(!monitor.unregister_cache("test_cache")); // Already removed

        let stats = monitor.stats();
        assert!(!stats.cache_stats.contains_key("test_cache"));
    }
}
