//! Cache statistics monitoring and reporting.
//!
//! This module provides utilities for tracking, monitoring, and reporting on
//! context cache performance. It combines in-memory statistics with database
//! statistics to provide comprehensive cache insights.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::cache::{CacheStats, ContextCache, DbCacheStats};

/// Comprehensive cache statistics combining in-memory and database metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    /// In-memory statistics (resets when cache instance is dropped)
    pub memory: MemoryStats,
    /// Database statistics (persistent)
    pub database: DbCacheStats,
    /// Calculated metrics
    pub metrics: CacheMetrics,
}

/// In-memory cache statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of cache puts
    pub puts: u64,
    /// Number of invalidations
    pub invalidations: u64,
    /// Number of TTL-based evictions
    pub ttl_evictions: u64,
    /// Number of LRU evictions
    pub lru_evictions: u64,
    /// Hit rate as percentage
    pub hit_rate: f64,
    /// Total cache operations (hits + misses)
    pub total_operations: u64,
}

impl MemoryStats {
    /// Create memory stats from CacheStats.
    pub fn from_cache_stats(stats: &Arc<CacheStats>) -> Self {
        Self {
            hits: stats.hits.load(std::sync::atomic::Ordering::Relaxed),
            misses: stats.misses.load(std::sync::atomic::Ordering::Relaxed),
            puts: stats.puts.load(std::sync::atomic::Ordering::Relaxed),
            invalidations: stats
                .invalidations
                .load(std::sync::atomic::Ordering::Relaxed),
            ttl_evictions: stats
                .ttl_evictions
                .load(std::sync::atomic::Ordering::Relaxed),
            lru_evictions: stats
                .lru_evictions
                .load(std::sync::atomic::Ordering::Relaxed),
            hit_rate: stats.hit_rate(),
            total_operations: stats.total_operations(),
        }
    }
}

/// Calculated cache metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    /// Average bundle size in bytes
    pub avg_bundle_size_bytes: f64,
    /// Cache utilization percentage (entries / max_entries)
    pub utilization_percentage: f64,
    /// Whether cache hit rate meets target (>60%)
    pub meets_hit_rate_target: bool,
    /// Whether cache is healthy overall
    pub is_healthy: bool,
    /// Health status message
    pub health_message: String,
}

impl CacheMetrics {
    /// Calculate metrics from memory and database statistics.
    pub fn calculate(memory: &MemoryStats, database: &DbCacheStats, max_entries: i32) -> Self {
        let avg_bundle_size_bytes = if database.total_entries > 0 {
            database.total_size_bytes as f64 / database.total_entries as f64
        } else {
            0.0
        };

        let utilization_percentage = if max_entries > 0 {
            (database.total_entries as f64 / max_entries as f64) * 100.0
        } else {
            0.0
        };

        let meets_hit_rate_target = memory.hit_rate >= 60.0;

        // Determine overall health
        let (is_healthy, health_message) = if memory.total_operations == 0 {
            (true, "Cache not yet warmed up".to_string())
        } else if !meets_hit_rate_target {
            (
                false,
                format!("Hit rate {:.1}% below target (60%)", memory.hit_rate),
            )
        } else if utilization_percentage > 95.0 {
            (
                false,
                format!(
                    "Cache utilization at {:.1}%, near capacity",
                    utilization_percentage
                ),
            )
        } else {
            (
                true,
                format!(
                    "Hit rate {:.1}%, utilization {:.1}%",
                    memory.hit_rate, utilization_percentage
                ),
            )
        };

        Self {
            avg_bundle_size_bytes,
            utilization_percentage,
            meets_hit_rate_target,
            is_healthy,
            health_message,
        }
    }
}

/// Cache statistics monitor for tracking and reporting cache performance.
pub struct CacheStatsMonitor {
    cache: Arc<ContextCache>,
}

impl CacheStatsMonitor {
    /// Create a new cache statistics monitor.
    pub fn new(cache: Arc<ContextCache>) -> Self {
        Self { cache }
    }

    /// Get comprehensive cache statistics.
    pub async fn get_statistics(&self) -> Result<CacheStatistics> {
        let memory_stats = MemoryStats::from_cache_stats(&self.cache.stats());
        let db_stats = self.cache.get_db_stats().await?;
        let metrics =
            CacheMetrics::calculate(&memory_stats, &db_stats, self.cache.config().max_entries);

        Ok(CacheStatistics {
            memory: memory_stats,
            database: db_stats,
            metrics,
        })
    }

    /// Get a human-readable summary of cache statistics.
    pub async fn get_summary(&self) -> Result<String> {
        let stats = self.get_statistics().await?;

        let mut summary = String::new();
        summary.push_str("=== Cache Statistics Summary ===\n\n");

        // Memory stats
        summary.push_str("Memory (Current Session):\n");
        summary.push_str(&format!("  Hits: {}\n", stats.memory.hits));
        summary.push_str(&format!("  Misses: {}\n", stats.memory.misses));
        summary.push_str(&format!("  Hit Rate: {:.2}%\n", stats.memory.hit_rate));
        summary.push_str(&format!("  Puts: {}\n", stats.memory.puts));
        summary.push_str(&format!(
            "  Invalidations: {}\n",
            stats.memory.invalidations
        ));
        summary.push_str(&format!(
            "  TTL Evictions: {}\n",
            stats.memory.ttl_evictions
        ));
        summary.push_str(&format!(
            "  LRU Evictions: {}\n\n",
            stats.memory.lru_evictions
        ));

        // Database stats
        summary.push_str("Database (Persistent):\n");
        summary.push_str(&format!(
            "  Total Entries: {}\n",
            stats.database.total_entries
        ));
        summary.push_str(&format!(
            "  Total Size: {:.2} MB\n",
            stats.database.total_size_bytes as f64 / (1024.0 * 1024.0)
        ));
        summary.push_str(&format!(
            "  Avg Access Count: {:.2}\n",
            stats.database.avg_access_count
        ));
        summary.push_str(&format!(
            "  Max Access Count: {}\n",
            stats.database.max_access_count
        ));
        summary.push_str(&format!(
            "  Entries (Last Hour): {}\n",
            stats.database.entries_last_hour
        ));
        summary.push_str(&format!(
            "  Entries (Last Day): {}\n",
            stats.database.entries_last_day
        ));
        summary.push_str(&format!(
            "  Entries (Last Week): {}\n\n",
            stats.database.entries_last_week
        ));

        // Metrics
        summary.push_str("Metrics:\n");
        summary.push_str(&format!(
            "  Avg Bundle Size: {:.2} KB\n",
            stats.metrics.avg_bundle_size_bytes / 1024.0
        ));
        summary.push_str(&format!(
            "  Utilization: {:.2}%\n",
            stats.metrics.utilization_percentage
        ));
        summary.push_str(&format!(
            "  Meets Hit Rate Target (>60%): {}\n",
            if stats.metrics.meets_hit_rate_target {
                "Yes"
            } else {
                "No"
            }
        ));
        summary.push_str(&format!(
            "  Health: {}\n",
            if stats.metrics.is_healthy {
                "Healthy"
            } else {
                "Unhealthy"
            }
        ));
        summary.push_str(&format!("  Status: {}\n", stats.metrics.health_message));

        Ok(summary)
    }

    /// Check if the cache is healthy and meeting performance targets.
    pub async fn is_healthy(&self) -> Result<bool> {
        let stats = self.get_statistics().await?;
        Ok(stats.metrics.is_healthy)
    }

    /// Get a brief health status message.
    pub async fn health_status(&self) -> Result<String> {
        let stats = self.get_statistics().await?;
        Ok(stats.metrics.health_message)
    }

    /// Perform cache maintenance (evict expired entries).
    ///
    /// Returns the number of entries evicted.
    pub async fn perform_maintenance(&self) -> Result<u64> {
        self.cache.evict_expired().await
    }

    /// Get cache efficiency score (0-100).
    ///
    /// Combines hit rate, utilization, and other factors into a single score.
    pub async fn efficiency_score(&self) -> Result<f64> {
        let stats = self.get_statistics().await?;

        // If no operations yet, return 100 (neutral)
        if stats.memory.total_operations == 0 {
            return Ok(100.0);
        }

        // Hit rate contributes 60% of score
        let hit_rate_score = stats.memory.hit_rate * 0.6;

        // Utilization contributes 20% (penalty if too high or too low)
        let utilization = stats.metrics.utilization_percentage;
        let utilization_score = if utilization < 10.0 {
            // Too low: cache is underutilized
            utilization
        } else if utilization > 95.0 {
            // Too high: cache is near capacity
            100.0 - (utilization - 95.0) * 4.0
        } else {
            // Ideal range: 10-95%
            20.0
        } * 0.2;

        // Avg access count contributes 20% (higher is better, capped at 20)
        let access_score = (stats.database.avg_access_count.min(10.0) / 10.0) * 20.0;

        let total_score = hit_rate_score + utilization_score + access_score;
        Ok(total_score.min(100.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_memory_stats_from_cache_stats() {
        let cache_stats = Arc::new(CacheStats::default());
        cache_stats.hits.store(100, Ordering::Relaxed);
        cache_stats.misses.store(50, Ordering::Relaxed);

        let memory = MemoryStats::from_cache_stats(&cache_stats);
        assert_eq!(memory.hits, 100);
        assert_eq!(memory.misses, 50);
        assert_eq!(memory.hit_rate, 66.66666666666666); // 100 / (100 + 50) * 100
        assert_eq!(memory.total_operations, 150);
    }

    #[test]
    fn test_cache_metrics_calculation() {
        let memory = MemoryStats {
            hits: 80,
            misses: 20,
            puts: 90,
            invalidations: 5,
            ttl_evictions: 2,
            lru_evictions: 1,
            hit_rate: 80.0,
            total_operations: 100,
        };

        let database = DbCacheStats {
            total_entries: 500,
            total_size_bytes: 1_000_000,
            avg_access_count: 2.5,
            max_access_count: 10,
            entries_last_hour: 100,
            entries_last_day: 300,
            entries_last_week: 500,
        };

        let metrics = CacheMetrics::calculate(&memory, &database, 1000);

        assert_eq!(metrics.avg_bundle_size_bytes, 2000.0); // 1_000_000 / 500
        assert_eq!(metrics.utilization_percentage, 50.0); // 500 / 1000 * 100
        assert!(metrics.meets_hit_rate_target); // 80% > 60%
        assert!(metrics.is_healthy);
    }

    #[test]
    fn test_cache_metrics_unhealthy_hit_rate() {
        let memory = MemoryStats {
            hits: 40,
            misses: 60,
            puts: 90,
            invalidations: 5,
            ttl_evictions: 2,
            lru_evictions: 1,
            hit_rate: 40.0,
            total_operations: 100,
        };

        let database = DbCacheStats {
            total_entries: 100,
            total_size_bytes: 100_000,
            avg_access_count: 1.5,
            max_access_count: 5,
            entries_last_hour: 50,
            entries_last_day: 80,
            entries_last_week: 100,
        };

        let metrics = CacheMetrics::calculate(&memory, &database, 1000);

        assert!(!metrics.meets_hit_rate_target); // 40% < 60%
        assert!(!metrics.is_healthy);
        assert!(metrics.health_message.contains("below target"));
    }

    #[test]
    fn test_cache_metrics_unhealthy_utilization() {
        let memory = MemoryStats {
            hits: 80,
            misses: 20,
            puts: 90,
            invalidations: 5,
            ttl_evictions: 2,
            lru_evictions: 1,
            hit_rate: 80.0,
            total_operations: 100,
        };

        let database = DbCacheStats {
            total_entries: 980,
            total_size_bytes: 1_000_000,
            avg_access_count: 2.5,
            max_access_count: 10,
            entries_last_hour: 100,
            entries_last_day: 300,
            entries_last_week: 980,
        };

        let metrics = CacheMetrics::calculate(&memory, &database, 1000);

        assert_eq!(metrics.utilization_percentage, 98.0); // 980 / 1000 * 100
        assert!(!metrics.is_healthy);
        assert!(metrics.health_message.contains("near capacity"));
    }

    #[test]
    fn test_cache_metrics_not_warmed_up() {
        let memory = MemoryStats {
            hits: 0,
            misses: 0,
            puts: 0,
            invalidations: 0,
            ttl_evictions: 0,
            lru_evictions: 0,
            hit_rate: 0.0,
            total_operations: 0,
        };

        let database = DbCacheStats {
            total_entries: 0,
            total_size_bytes: 0,
            avg_access_count: 0.0,
            max_access_count: 0,
            entries_last_hour: 0,
            entries_last_day: 0,
            entries_last_week: 0,
        };

        let metrics = CacheMetrics::calculate(&memory, &database, 1000);

        assert!(metrics.is_healthy);
        assert!(metrics.health_message.contains("not yet warmed up"));
    }
}
