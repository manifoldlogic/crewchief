//! Context bundle caching system.
//!
//! This module provides a PostgreSQL-backed cache for assembled context bundles,
//! significantly improving performance by avoiding redundant graph traversals and
//! assembly operations.
//!
//! # Features
//!
//! - **Composite Keys**: Cache entries identified by (chunk_id, options_hash)
//! - **TTL Support**: Time-based cache expiration (default: 3600 seconds)
//! - **LRU Eviction**: Least-recently-used eviction when max entries exceeded
//! - **Statistics**: Hit rate, miss rate, and cache size tracking
//! - **Invalidation**: Automatic invalidation on chunk updates
//!
//! # Cache Key Design
//!
//! Cache keys combine:
//! - `chunk_id`: The primary chunk for which context was assembled
//! - `options_hash`: SHA-256 hash of the ExpandOptions used
//!
//! This ensures that different expansion options (e.g., with/without tests)
//! produce distinct cache entries.
//!
//! # Example
//!
//! ```no_run
//! use crewchief_maproom::context::{ContextCache, ExpandOptions};
//! use crewchief_maproom::db::create_pool;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let pool = create_pool().await?;
//!     let config = CacheConfig::default();
//!     let cache = ContextCache::new(pool, config);
//!
//!     let options = ExpandOptions::with_common();
//!
//!     // Try to get from cache
//!     if let Some(bundle) = cache.get(12345, &options).await? {
//!         println!("Cache hit!");
//!     } else {
//!         println!("Cache miss, assembling...");
//!         // ... assemble bundle ...
//!         // cache.put(12345, &options, &bundle).await?;
//!     }
//!
//!     Ok(())
//! }
//! ```

use anyhow::{Context as AnyhowContext, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::debug;

use super::types::{ContextBundle, ExpandOptions};
use crate::db::PgPool;

/// Configuration for the context cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled
    pub enabled: bool,
    /// Time-to-live for cache entries in seconds
    pub ttl_seconds: i32,
    /// Maximum number of cache entries before LRU eviction
    pub max_entries: i32,
    /// Number of entries to evict at once when max is exceeded
    pub evict_batch_size: i32,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_seconds: 3600, // 1 hour
            max_entries: 1000,
            evict_batch_size: 100,
        }
    }
}

/// Cache key combining chunk ID and options hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub chunk_id: i64,
    pub options_hash: String,
}

impl CacheKey {
    /// Create a new cache key from chunk ID and expand options.
    pub fn new(chunk_id: i64, options: &ExpandOptions) -> Self {
        Self {
            chunk_id,
            options_hash: hash_options(options),
        }
    }
}

/// Compute a deterministic SHA-256 hash of ExpandOptions.
///
/// This creates a stable hash that uniquely identifies a set of expand options,
/// ensuring that identical options always produce the same cache key.
///
/// # Example
///
/// ```
/// use crewchief_maproom::context::{ExpandOptions, hash_options};
///
/// let options1 = ExpandOptions::with_common();
/// let options2 = ExpandOptions::with_common();
/// assert_eq!(hash_options(&options1), hash_options(&options2));
/// ```
pub fn hash_options(options: &ExpandOptions) -> String {
    // Serialize options to JSON for consistent hashing
    let json = serde_json::to_string(options).expect("ExpandOptions should serialize");

    // Compute SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    let result = hasher.finalize();

    // Convert to hex string
    format!("{:x}", result)
}

/// In-memory statistics tracking for cache operations.
///
/// These statistics are tracked in memory and not persisted to the database.
/// They reset when the cache instance is dropped.
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Number of successful cache hits
    pub hits: AtomicU64,
    /// Number of cache misses
    pub misses: AtomicU64,
    /// Number of cache puts
    pub puts: AtomicU64,
    /// Number of cache invalidations
    pub invalidations: AtomicU64,
    /// Number of TTL-based evictions
    pub ttl_evictions: AtomicU64,
    /// Number of LRU evictions
    pub lru_evictions: AtomicU64,
}

impl CacheStats {
    /// Get the cache hit rate as a percentage.
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            (hits as f64 / total as f64) * 100.0
        }
    }

    /// Get the total number of cache operations (hits + misses).
    pub fn total_operations(&self) -> u64 {
        self.hits.load(Ordering::Relaxed) + self.misses.load(Ordering::Relaxed)
    }

    /// Reset all statistics to zero.
    pub fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.puts.store(0, Ordering::Relaxed);
        self.invalidations.store(0, Ordering::Relaxed);
        self.ttl_evictions.store(0, Ordering::Relaxed);
        self.lru_evictions.store(0, Ordering::Relaxed);
    }
}

/// Context bundle cache backed by PostgreSQL.
///
/// Provides high-performance caching with TTL and LRU eviction strategies.
pub struct ContextCache {
    pool: PgPool,
    config: CacheConfig,
    stats: Arc<CacheStats>,
}

impl ContextCache {
    /// Create a new context cache with the specified configuration.
    pub fn new(pool: PgPool, config: CacheConfig) -> Self {
        Self {
            pool,
            config,
            stats: Arc::new(CacheStats::default()),
        }
    }

    /// Get the cache configuration.
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Get a reference to the cache statistics.
    pub fn stats(&self) -> Arc<CacheStats> {
        Arc::clone(&self.stats)
    }

    /// Get a cached bundle if it exists and is not expired.
    ///
    /// Returns `None` if:
    /// - Caching is disabled
    /// - No cache entry exists for the given key
    /// - The cache entry has exceeded its TTL
    ///
    /// On cache hit, updates the `last_accessed_at` timestamp for LRU tracking.
    pub async fn get(
        &self,
        chunk_id: i64,
        options: &ExpandOptions,
    ) -> Result<Option<ContextBundle>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let key = CacheKey::new(chunk_id, options);

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get database connection")?;

        // Query cache with TTL check
        let row = client
            .query_opt(
                "SELECT bundle, created_at
                FROM maproom.context_cache
                WHERE chunk_id = $1 AND options_hash = $2
                  AND created_at > NOW() - ($3 || ' seconds')::INTERVAL",
                &[&key.chunk_id, &key.options_hash, &self.config.ttl_seconds],
            )
            .await
            .context("Failed to query cache")?;

        match row {
            Some(row) => {
                // Update access tracking for LRU
                let _ = client
                    .execute(
                        "UPDATE maproom.context_cache
                        SET last_accessed_at = NOW(),
                            access_count = access_count + 1
                        WHERE chunk_id = $1 AND options_hash = $2",
                        &[&key.chunk_id, &key.options_hash],
                    )
                    .await;

                // Deserialize bundle from JSONB
                let bundle_json: serde_json::Value = row.get(0);
                let bundle: ContextBundle = serde_json::from_value(bundle_json)
                    .context("Failed to deserialize cached bundle")?;

                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                debug!(
                    "Cache HIT for chunk {} with options hash {}",
                    chunk_id, key.options_hash
                );

                Ok(Some(bundle))
            }
            None => {
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                debug!(
                    "Cache MISS for chunk {} with options hash {}",
                    chunk_id, key.options_hash
                );
                Ok(None)
            }
        }
    }

    /// Store a bundle in the cache.
    ///
    /// If caching is disabled, this is a no-op.
    ///
    /// Before storing, checks if max entries would be exceeded and runs
    /// LRU eviction if necessary.
    pub async fn put(
        &self,
        chunk_id: i64,
        options: &ExpandOptions,
        bundle: &ContextBundle,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Check if we need to evict entries first
        self.evict_lru_if_needed().await?;

        let key = CacheKey::new(chunk_id, options);

        // Serialize bundle to JSON
        let bundle_json = serde_json::to_value(bundle).context("Failed to serialize bundle")?;
        let bundle_size = bundle_json.to_string().len() as i32;

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get database connection")?;

        // Insert or update cache entry
        client
            .execute(
                "INSERT INTO maproom.context_cache
                (chunk_id, options_hash, bundle, bundle_size_bytes, created_at, last_accessed_at, access_count)
                VALUES ($1, $2, $3, $4, NOW(), NOW(), 1)
                ON CONFLICT (chunk_id, options_hash)
                DO UPDATE SET
                    bundle = EXCLUDED.bundle,
                    bundle_size_bytes = EXCLUDED.bundle_size_bytes,
                    created_at = NOW(),
                    last_accessed_at = NOW(),
                    access_count = 1",
                &[&key.chunk_id, &key.options_hash, &bundle_json, &bundle_size],
            )
            .await
            .context("Failed to insert cache entry")?;

        self.stats.puts.fetch_add(1, Ordering::Relaxed);
        debug!(
            "Cache PUT for chunk {} with options hash {} ({} bytes)",
            chunk_id, key.options_hash, bundle_size
        );

        Ok(())
    }

    /// Invalidate all cache entries for a specific chunk.
    ///
    /// This should be called when a chunk is updated to ensure cache consistency.
    pub async fn invalidate(&self, chunk_id: i64) -> Result<u64> {
        if !self.config.enabled {
            return Ok(0);
        }

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get database connection")?;

        let row = client
            .query_one("SELECT maproom.invalidate_chunk_cache($1)", &[&chunk_id])
            .await
            .context("Failed to invalidate cache")?;

        let count: i64 = row.get(0);

        self.stats
            .invalidations
            .fetch_add(count as u64, Ordering::Relaxed);
        debug!("Invalidated {} cache entries for chunk {}", count, chunk_id);

        Ok(count as u64)
    }

    /// Invalidate multiple chunks at once.
    ///
    /// More efficient than calling `invalidate()` multiple times.
    pub async fn invalidate_many(&self, chunk_ids: &[i64]) -> Result<u64> {
        if !self.config.enabled || chunk_ids.is_empty() {
            return Ok(0);
        }

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get database connection")?;

        let count = client
            .execute(
                "DELETE FROM maproom.context_cache WHERE chunk_id = ANY($1)",
                &[&chunk_ids],
            )
            .await
            .context("Failed to invalidate cache entries")?;

        self.stats.invalidations.fetch_add(count, Ordering::Relaxed);
        debug!(
            "Invalidated {} cache entries for {} chunks",
            count,
            chunk_ids.len()
        );

        Ok(count)
    }

    /// Clear all cache entries.
    ///
    /// Useful for manual cache clearing or testing.
    pub async fn clear(&self) -> Result<u64> {
        let client = self
            .pool
            .get()
            .await
            .context("Failed to get database connection")?;

        let row = client
            .query_one("SELECT maproom.clear_context_cache()", &[])
            .await
            .context("Failed to clear cache")?;

        let count: i64 = row.get(0);

        debug!("Cleared {} cache entries", count);
        self.stats.reset();

        Ok(count as u64)
    }

    /// Evict expired cache entries based on TTL.
    ///
    /// Returns the number of entries evicted.
    pub async fn evict_expired(&self) -> Result<u64> {
        if !self.config.enabled {
            return Ok(0);
        }

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get database connection")?;

        let row = client
            .query_one(
                "SELECT maproom.evict_expired_cache_entries($1)",
                &[&self.config.ttl_seconds],
            )
            .await
            .context("Failed to evict expired entries")?;

        let count: i64 = row.get(0);

        if count > 0 {
            self.stats
                .ttl_evictions
                .fetch_add(count as u64, Ordering::Relaxed);
            debug!("Evicted {} expired cache entries", count);
        }

        Ok(count as u64)
    }

    /// Evict LRU entries if cache size exceeds max_entries.
    ///
    /// Returns the number of entries evicted.
    async fn evict_lru_if_needed(&self) -> Result<u64> {
        if !self.config.enabled {
            return Ok(0);
        }

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get database connection")?;

        let row = client
            .query_one(
                "SELECT maproom.evict_lru_cache_entries($1, $2)",
                &[&self.config.max_entries, &self.config.evict_batch_size],
            )
            .await
            .context("Failed to evict LRU entries")?;

        let count: i64 = row.get(0);

        if count > 0 {
            self.stats
                .lru_evictions
                .fetch_add(count as u64, Ordering::Relaxed);
            debug!("Evicted {} LRU cache entries", count);
        }

        Ok(count as u64)
    }

    /// Get database-level cache statistics.
    ///
    /// Returns aggregate statistics from the database, including:
    /// - Total entries
    /// - Total size in bytes
    /// - Average access count
    /// - Entry age distribution
    pub async fn get_db_stats(&self) -> Result<DbCacheStats> {
        let client = self
            .pool
            .get()
            .await
            .context("Failed to get database connection")?;

        let row = client
            .query_one("SELECT * FROM maproom.context_cache_stats", &[])
            .await
            .context("Failed to query cache stats")?;

        Ok(DbCacheStats {
            total_entries: row.get::<_, Option<i64>>(0).unwrap_or(0),
            total_size_bytes: row.get::<_, Option<i64>>(1).unwrap_or(0),
            avg_access_count: row.get::<_, Option<f64>>(2).unwrap_or(0.0),
            max_access_count: row.get::<_, Option<i32>>(3).unwrap_or(0),
            entries_last_hour: row.get::<_, Option<i64>>(8).unwrap_or(0),
            entries_last_day: row.get::<_, Option<i64>>(9).unwrap_or(0),
            entries_last_week: row.get::<_, Option<i64>>(10).unwrap_or(0),
        })
    }
}

/// Database-level cache statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbCacheStats {
    /// Total number of cache entries
    pub total_entries: i64,
    /// Total size of all cached bundles in bytes
    pub total_size_bytes: i64,
    /// Average access count across all entries
    pub avg_access_count: f64,
    /// Maximum access count for any entry
    pub max_access_count: i32,
    /// Number of entries created in the last hour
    pub entries_last_hour: i64,
    /// Number of entries created in the last day
    pub entries_last_day: i64,
    /// Number of entries created in the last week
    pub entries_last_week: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_options_deterministic() {
        let options1 = ExpandOptions::with_common();
        let options2 = ExpandOptions::with_common();
        assert_eq!(hash_options(&options1), hash_options(&options2));
    }

    #[test]
    fn test_hash_options_different() {
        let options1 = ExpandOptions::with_common();
        let options2 = ExpandOptions::with_all();
        assert_ne!(hash_options(&options1), hash_options(&options2));
    }

    #[test]
    fn test_cache_key_creation() {
        let options = ExpandOptions::primary_only();
        let key = CacheKey::new(123, &options);
        assert_eq!(key.chunk_id, 123);
        assert!(!key.options_hash.is_empty());
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);

        stats.hits.store(60, Ordering::Relaxed);
        stats.misses.store(40, Ordering::Relaxed);
        assert_eq!(stats.hit_rate(), 60.0);

        stats.hits.store(80, Ordering::Relaxed);
        stats.misses.store(20, Ordering::Relaxed);
        assert_eq!(stats.hit_rate(), 80.0);
    }

    #[test]
    fn test_cache_stats_total_operations() {
        let stats = CacheStats::default();
        assert_eq!(stats.total_operations(), 0);

        stats.hits.store(100, Ordering::Relaxed);
        stats.misses.store(50, Ordering::Relaxed);
        assert_eq!(stats.total_operations(), 150);
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(config.enabled);
        assert_eq!(config.ttl_seconds, 3600);
        assert_eq!(config.max_entries, 1000);
        assert_eq!(config.evict_batch_size, 100);
    }

    #[test]
    fn test_cache_stats_reset() {
        let stats = CacheStats::default();
        stats.hits.store(100, Ordering::Relaxed);
        stats.misses.store(50, Ordering::Relaxed);
        stats.puts.store(75, Ordering::Relaxed);

        stats.reset();

        assert_eq!(stats.hits.load(Ordering::Relaxed), 0);
        assert_eq!(stats.misses.load(Ordering::Relaxed), 0);
        assert_eq!(stats.puts.load(Ordering::Relaxed), 0);
    }
}
