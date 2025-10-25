//! Cache statistics tracking and reporting.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Cache performance statistics.
///
/// Tracks cache operations using atomic counters for lock-free updates.
/// These statistics provide insight into cache effectiveness.
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: AtomicU64,
    /// Number of cache misses
    pub misses: AtomicU64,
    /// Number of cache evictions (LRU)
    pub evictions: AtomicU64,
    /// Number of expired entries removed
    pub expirations: AtomicU64,
    /// Total cache size in bytes (approximate)
    pub total_size: AtomicUsize,
    /// Number of insertions
    pub insertions: AtomicU64,
}

impl CacheStats {
    /// Create new cache statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate cache hit rate (0.0 to 1.0).
    ///
    /// Returns 0.0 if no operations have occurred.
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let total = hits + self.misses.load(Ordering::Relaxed) as f64;
        if total > 0.0 {
            hits / total
        } else {
            0.0
        }
    }

    /// Get total number of cache operations.
    pub fn total_operations(&self) -> u64 {
        self.hits.load(Ordering::Relaxed) + self.misses.load(Ordering::Relaxed)
    }

    /// Get total cache size in bytes.
    pub fn size_bytes(&self) -> usize {
        self.total_size.load(Ordering::Relaxed)
    }

    /// Get total cache size in megabytes.
    pub fn size_mb(&self) -> f64 {
        self.size_bytes() as f64 / 1_048_576.0
    }

    /// Record a cache hit.
    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss.
    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache eviction.
    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an expiration.
    pub fn record_expiration(&self) {
        self.expirations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an insertion.
    pub fn record_insertion(&self) {
        self.insertions.fetch_add(1, Ordering::Relaxed);
    }

    /// Update cache size.
    pub fn update_size(&self, delta: isize) {
        if delta >= 0 {
            self.total_size.fetch_add(delta as usize, Ordering::Relaxed);
        } else {
            self.total_size.fetch_sub((-delta) as usize, Ordering::Relaxed);
        }
    }

    /// Reset all statistics to zero.
    pub fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.expirations.store(0, Ordering::Relaxed);
        self.insertions.store(0, Ordering::Relaxed);
        self.total_size.store(0, Ordering::Relaxed);
    }

    /// Get a snapshot of current statistics.
    pub fn snapshot(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            expirations: self.expirations.load(Ordering::Relaxed),
            insertions: self.insertions.load(Ordering::Relaxed),
            total_size_bytes: self.total_size.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of cache statistics at a point in time.
///
/// This is a serializable copy of the atomic statistics,
/// useful for logging, metrics export, and testing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStatsSnapshot {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of cache evictions
    pub evictions: u64,
    /// Number of expirations
    pub expirations: u64,
    /// Number of insertions
    pub insertions: u64,
    /// Total cache size in bytes
    pub total_size_bytes: usize,
}

impl CacheStatsSnapshot {
    /// Calculate cache hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Get total operations.
    pub fn total_operations(&self) -> u64 {
        self.hits + self.misses
    }

    /// Get cache size in megabytes.
    pub fn size_mb(&self) -> f64 {
        self.total_size_bytes as f64 / 1_048_576.0
    }

    /// Check if cache is performing well (hit rate > 60%).
    pub fn is_effective(&self) -> bool {
        self.hit_rate() > 0.6
    }

    /// Get eviction rate (evictions per operation).
    pub fn eviction_rate(&self) -> f64 {
        let total = self.total_operations();
        if total > 0 {
            self.evictions as f64 / total as f64
        } else {
            0.0
        }
    }
}

/// Multi-layer cache statistics aggregator.
///
/// Combines statistics from multiple cache layers (L1, L2, L3, ParseTree)
/// to provide overall system performance metrics.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MultiLayerStats {
    /// L1 query cache statistics
    pub l1_query: CacheStatsSnapshot,
    /// L2 embedding cache statistics
    pub l2_embedding: CacheStatsSnapshot,
    /// L3 context cache statistics
    pub l3_context: CacheStatsSnapshot,
    /// Parse tree cache statistics
    pub parse_tree: CacheStatsSnapshot,
}

impl MultiLayerStats {
    /// Calculate overall hit rate across all cache layers.
    pub fn overall_hit_rate(&self) -> f64 {
        let total_hits = self.l1_query.hits
            + self.l2_embedding.hits
            + self.l3_context.hits
            + self.parse_tree.hits;
        let total_ops = self.l1_query.total_operations()
            + self.l2_embedding.total_operations()
            + self.l3_context.total_operations()
            + self.parse_tree.total_operations();

        if total_ops > 0 {
            total_hits as f64 / total_ops as f64
        } else {
            0.0
        }
    }

    /// Calculate total memory usage across all caches.
    pub fn total_size_bytes(&self) -> usize {
        self.l1_query.total_size_bytes
            + self.l2_embedding.total_size_bytes
            + self.l3_context.total_size_bytes
            + self.parse_tree.total_size_bytes
    }

    /// Calculate total memory usage in megabytes.
    pub fn total_size_mb(&self) -> f64 {
        self.total_size_bytes() as f64 / 1_048_576.0
    }

    /// Check if overall cache system is effective (>60% hit rate).
    pub fn is_effective(&self) -> bool {
        self.overall_hit_rate() > 0.6
    }

    /// Check if memory usage is within target (<500MB).
    pub fn is_within_memory_target(&self) -> bool {
        self.total_size_mb() < 500.0
    }

    /// Get total operations across all caches.
    pub fn total_operations(&self) -> u64 {
        self.l1_query.total_operations()
            + self.l2_embedding.total_operations()
            + self.l3_context.total_operations()
            + self.parse_tree.total_operations()
    }

    /// Get total evictions across all caches.
    pub fn total_evictions(&self) -> u64 {
        self.l1_query.evictions
            + self.l2_embedding.evictions
            + self.l3_context.evictions
            + self.parse_tree.evictions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats::new();

        // No operations yet
        assert_eq!(stats.hit_rate(), 0.0);

        // 60% hit rate
        for _ in 0..60 {
            stats.record_hit();
        }
        for _ in 0..40 {
            stats.record_miss();
        }

        assert!((stats.hit_rate() - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_cache_stats_operations() {
        let stats = CacheStats::new();

        stats.record_hit();
        stats.record_hit();
        stats.record_miss();

        assert_eq!(stats.total_operations(), 3);
        assert_eq!(stats.hits.load(Ordering::Relaxed), 2);
        assert_eq!(stats.misses.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_cache_stats_size() {
        let stats = CacheStats::new();

        stats.update_size(1024);
        assert_eq!(stats.size_bytes(), 1024);

        stats.update_size(1024);
        assert_eq!(stats.size_bytes(), 2048);

        stats.update_size(-1024);
        assert_eq!(stats.size_bytes(), 1024);
    }

    #[test]
    fn test_cache_stats_reset() {
        let stats = CacheStats::new();

        stats.record_hit();
        stats.record_miss();
        stats.record_eviction();
        stats.update_size(1024);

        stats.reset();

        assert_eq!(stats.hits.load(Ordering::Relaxed), 0);
        assert_eq!(stats.misses.load(Ordering::Relaxed), 0);
        assert_eq!(stats.evictions.load(Ordering::Relaxed), 0);
        assert_eq!(stats.size_bytes(), 0);
    }

    #[test]
    fn test_cache_stats_snapshot() {
        let stats = CacheStats::new();

        stats.record_hit();
        stats.record_hit();
        stats.record_miss();
        stats.update_size(1024);

        let snapshot = stats.snapshot();

        assert_eq!(snapshot.hits, 2);
        assert_eq!(snapshot.misses, 1);
        assert_eq!(snapshot.total_size_bytes, 1024);
        assert_eq!(snapshot.hit_rate(), 2.0 / 3.0);
    }

    #[test]
    fn test_snapshot_is_effective() {
        let mut snapshot = CacheStatsSnapshot {
            hits: 70,
            misses: 30,
            evictions: 0,
            expirations: 0,
            insertions: 100,
            total_size_bytes: 0,
        };

        assert!(snapshot.is_effective()); // 70% hit rate

        snapshot.hits = 50;
        snapshot.misses = 50;
        assert!(!snapshot.is_effective()); // 50% hit rate
    }

    #[test]
    fn test_multi_layer_stats() {
        let stats = MultiLayerStats {
            l1_query: CacheStatsSnapshot {
                hits: 60,
                misses: 40,
                evictions: 5,
                expirations: 2,
                insertions: 100,
                total_size_bytes: 10_000_000, // 10 MB
            },
            l2_embedding: CacheStatsSnapshot {
                hits: 80,
                misses: 20,
                evictions: 3,
                expirations: 1,
                insertions: 100,
                total_size_bytes: 50_000_000, // 50 MB
            },
            l3_context: CacheStatsSnapshot {
                hits: 70,
                misses: 30,
                evictions: 4,
                expirations: 3,
                insertions: 100,
                total_size_bytes: 30_000_000, // 30 MB
            },
            parse_tree: CacheStatsSnapshot {
                hits: 90,
                misses: 10,
                evictions: 2,
                expirations: 0,
                insertions: 100,
                total_size_bytes: 20_000_000, // 20 MB
            },
        };

        // Overall hit rate: (60+80+70+90) / (60+40+80+20+70+30+90+10) = 300/400 = 0.75
        assert!((stats.overall_hit_rate() - 0.75).abs() < 0.01);

        // Total size: ~104.9 MiB (110 million bytes / 1,048,576 bytes per MiB)
        let actual_mb = stats.total_size_mb();
        assert!(
            (actual_mb - 104.9).abs() < 0.2,
            "Expected ~104.9 MiB, got {} MiB (total bytes: {})",
            actual_mb,
            stats.total_size_bytes()
        );

        // Is effective (>60% hit rate)
        assert!(stats.is_effective());

        // Within memory target (<500MB)
        assert!(stats.is_within_memory_target());

        // Total operations
        assert_eq!(stats.total_operations(), 400);

        // Total evictions
        assert_eq!(stats.total_evictions(), 14);
    }
}
