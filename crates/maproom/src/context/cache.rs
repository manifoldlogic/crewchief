//! Context bundle caching system.
//!
//! This module provides a SQLite-backed cache for assembled context bundles,
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
//! use crewchief_maproom::context::{ContextCache, ExpandOptions, CacheConfig};
//! use crewchief_maproom::db::SqliteStore;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let store = Arc::new(SqliteStore::connect(":memory:")?);
//!     let config = CacheConfig::default();
//!     let cache = ContextCache::new(store, config);
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

use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::debug;

use super::types::{ContextBundle, ExpandOptions};
use crate::db::SqliteStore;

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

/// Context bundle cache backed by SQLite.
///
/// Provides high-performance caching with TTL and LRU eviction strategies.
pub struct ContextCache {
    store: Arc<SqliteStore>,
    config: CacheConfig,
    stats: Arc<CacheStats>,
}

impl ContextCache {
    /// Create a new context cache with the specified configuration.
    pub fn new(store: Arc<SqliteStore>, config: CacheConfig) -> Self {
        Self {
            store,
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
        let cache_key = format!("{}:{}", key.chunk_id, key.options_hash);
        let cache_key_clone = cache_key.clone();

        let result = self
            .store
            .run(move |conn| {
                // Query for non-expired entry
                let bundle_json: Option<String> = conn
                    .query_row(
                        "SELECT bundle_json FROM context_cache
                     WHERE cache_key = ?1 AND expires_at > datetime('now')",
                        params![cache_key],
                        |row| row.get(0),
                    )
                    .optional()?;

                Ok(bundle_json)
            })
            .await?;

        match result {
            Some(json) => {
                // Update accessed_at for LRU tracking
                let cache_key_for_update = cache_key_clone.clone();
                self.store.run(move |conn| {
                    conn.execute(
                        "UPDATE context_cache SET accessed_at = datetime('now') WHERE cache_key = ?1",
                        params![cache_key_for_update],
                    )?;
                    Ok(())
                }).await?;

                // Deserialize the bundle
                let bundle: ContextBundle = serde_json::from_str(&json)?;
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                debug!("Cache hit for chunk_id={}", chunk_id);
                Ok(Some(bundle))
            }
            None => {
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                debug!("Cache miss for chunk_id={}", chunk_id);
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

        // Run LRU eviction if needed before inserting
        self.evict_lru_if_needed().await?;

        let key = CacheKey::new(chunk_id, options);
        let cache_key = format!("{}:{}", key.chunk_id, key.options_hash);
        let bundle_json = serde_json::to_string(bundle)?;
        let ttl_seconds = self.config.ttl_seconds;

        self.store.run(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO context_cache
                 (cache_key, bundle_json, created_at, expires_at, accessed_at)
                 VALUES (?1, ?2, datetime('now'), datetime('now', '+' || ?3 || ' seconds'), datetime('now'))",
                params![cache_key, bundle_json, ttl_seconds],
            )?;
            Ok(())
        }).await?;

        self.stats.puts.fetch_add(1, Ordering::Relaxed);
        debug!("Cached bundle for chunk_id={}", chunk_id);
        Ok(())
    }

    /// Invalidate all cache entries for a specific chunk.
    ///
    /// This should be called when a chunk is updated to ensure cache consistency.
    pub async fn invalidate(&self, chunk_id: i64) -> Result<u64> {
        if !self.config.enabled {
            return Ok(0);
        }

        // Delete all entries with cache_key starting with "chunk_id:"
        let prefix = format!("{}:", chunk_id);
        let count = self
            .store
            .run(move |conn| {
                let deleted = conn.execute(
                    "DELETE FROM context_cache WHERE cache_key LIKE ?1 || '%'",
                    params![prefix],
                )?;
                Ok(deleted as u64)
            })
            .await?;

        if count > 0 {
            self.stats.invalidations.fetch_add(count, Ordering::Relaxed);
            debug!(
                "Invalidated {} cache entries for chunk_id={}",
                count, chunk_id
            );
        }
        Ok(count)
    }

    /// Invalidate multiple chunks at once.
    ///
    /// More efficient than calling `invalidate()` multiple times.
    pub async fn invalidate_many(&self, chunk_ids: &[i64]) -> Result<u64> {
        if !self.config.enabled || chunk_ids.is_empty() {
            return Ok(0);
        }

        // Build a list of prefixes to match
        let prefixes: Vec<String> = chunk_ids.iter().map(|id| format!("{}:", id)).collect();

        let count = self
            .store
            .run(move |conn| {
                let mut total_deleted = 0u64;
                for prefix in prefixes {
                    let deleted = conn.execute(
                        "DELETE FROM context_cache WHERE cache_key LIKE ?1 || '%'",
                        params![prefix],
                    )?;
                    total_deleted += deleted as u64;
                }
                Ok(total_deleted)
            })
            .await?;

        if count > 0 {
            self.stats.invalidations.fetch_add(count, Ordering::Relaxed);
            debug!(
                "Invalidated {} cache entries for {} chunks",
                count,
                chunk_ids.len()
            );
        }
        Ok(count)
    }

    /// Clear all cache entries.
    ///
    /// Useful for manual cache clearing or testing.
    pub async fn clear(&self) -> Result<u64> {
        let count = self
            .store
            .run(|conn| {
                let deleted = conn.execute("DELETE FROM context_cache", [])?;
                Ok(deleted as u64)
            })
            .await?;

        self.stats.reset();
        debug!("Cleared {} cache entries", count);
        Ok(count)
    }

    /// Evict expired cache entries based on TTL.
    ///
    /// Returns the number of entries evicted.
    pub async fn evict_expired(&self) -> Result<u64> {
        if !self.config.enabled {
            return Ok(0);
        }

        let count = self
            .store
            .run(|conn| {
                let deleted = conn.execute(
                    "DELETE FROM context_cache WHERE expires_at < datetime('now')",
                    [],
                )?;
                Ok(deleted as u64)
            })
            .await?;

        if count > 0 {
            self.stats.ttl_evictions.fetch_add(count, Ordering::Relaxed);
            debug!("Evicted {} expired cache entries", count);
        }
        Ok(count)
    }

    /// Evict LRU entries if cache size exceeds max_entries.
    ///
    /// Returns the number of entries evicted.
    async fn evict_lru_if_needed(&self) -> Result<u64> {
        if !self.config.enabled {
            return Ok(0);
        }

        let max_entries = self.config.max_entries;
        let evict_batch_size = self.config.evict_batch_size;

        let count = self
            .store
            .run(move |conn| {
                // Check current entry count
                let current_count: i32 =
                    conn.query_row("SELECT COUNT(*) FROM context_cache", [], |row| row.get(0))?;

                if current_count <= max_entries {
                    return Ok(0u64);
                }

                // Delete oldest entries by accessed_at (LRU)
                let to_evict = std::cmp::min(
                    evict_batch_size,
                    current_count - max_entries + evict_batch_size, // Evict enough to make room
                );

                let deleted = conn.execute(
                    "DELETE FROM context_cache WHERE cache_key IN (
                    SELECT cache_key FROM context_cache
                    ORDER BY accessed_at ASC
                    LIMIT ?1
                )",
                    params![to_evict],
                )?;

                Ok(deleted as u64)
            })
            .await?;

        if count > 0 {
            self.stats.lru_evictions.fetch_add(count, Ordering::Relaxed);
            debug!("LRU evicted {} cache entries", count);
        }
        Ok(count)
    }

    /// Get database-level cache statistics.
    ///
    /// Returns aggregate statistics from the database, including:
    /// - Total entries
    /// - Total size in bytes
    /// - Average access count
    /// - Entry age distribution
    pub async fn get_db_stats(&self) -> Result<DbCacheStats> {
        self.store.run(|conn| {
            // Get total entries and total size
            let (total_entries, total_size_bytes): (i64, i64) = conn
                .query_row(
                    "SELECT COUNT(*), COALESCE(SUM(LENGTH(bundle_json)), 0) FROM context_cache",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap_or((0, 0));

            // Get entries by time period
            let entries_last_hour: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM context_cache WHERE created_at > datetime('now', '-1 hour')",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            let entries_last_day: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM context_cache WHERE created_at > datetime('now', '-1 day')",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            let entries_last_week: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM context_cache WHERE created_at > datetime('now', '-7 day')",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            Ok(DbCacheStats {
                total_entries,
                total_size_bytes,
                avg_access_count: 0.0, // Not tracked in current schema
                max_access_count: 0,   // Not tracked in current schema
                entries_last_hour,
                entries_last_day,
                entries_last_week,
            })
        }).await
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
    use crate::db::traits::StoreMigration;

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

    // Async integration tests
    use crate::context::types::{ContextItem, LineRange};
    use std::sync::atomic::AtomicUsize;

    // Counter for unique test database names
    static TEST_DB_COUNTER: AtomicUsize = AtomicUsize::new(0);

    async fn setup_test_store() -> Arc<crate::db::SqliteStore> {
        // Use file:memdb?mode=memory&cache=shared for shared in-memory database
        // Each test gets a unique name to avoid interference
        let counter = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_name = format!("file:memdb_cache_test_{}?mode=memory&cache=shared", counter);
        let store = crate::db::SqliteStore::connect(&db_name)
            .await
            .expect("Failed to create test store");
        store.migrate().await.expect("Failed to run migrations");
        Arc::new(store)
    }

    fn create_test_bundle() -> ContextBundle {
        let mut bundle = ContextBundle::new();
        bundle.add_item(ContextItem {
            relpath: "test.rs".to_string(),
            range: LineRange::new(1, 10),
            role: "primary".to_string(),
            reason: "Test item".to_string(),
            content: "fn test() {}".to_string(),
            tokens: 5,
        });
        bundle
    }

    #[tokio::test]
    async fn test_cache_put_and_get() {
        let store = setup_test_store().await;
        let config = CacheConfig::default();
        let cache = ContextCache::new(store, config);

        let options = ExpandOptions::with_common();
        let bundle = create_test_bundle();

        // Initially should be a miss
        let result = cache.get(123, &options).await.unwrap();
        assert!(result.is_none());
        assert_eq!(cache.stats().misses.load(Ordering::Relaxed), 1);

        // Put the bundle
        cache.put(123, &options, &bundle).await.unwrap();
        assert_eq!(cache.stats().puts.load(Ordering::Relaxed), 1);

        // Now should be a hit
        let result = cache.get(123, &options).await.unwrap();
        assert!(result.is_some());
        assert_eq!(cache.stats().hits.load(Ordering::Relaxed), 1);

        // Verify content
        let retrieved = result.unwrap();
        assert_eq!(retrieved.items.len(), 1);
        assert_eq!(retrieved.items[0].relpath, "test.rs");
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let store = setup_test_store().await;
        let config = CacheConfig::default();
        let cache = ContextCache::new(store, config);

        let options = ExpandOptions::with_common();
        let bundle = create_test_bundle();

        // Put and verify
        cache.put(123, &options, &bundle).await.unwrap();
        let result = cache.get(123, &options).await.unwrap();
        assert!(result.is_some());

        // Invalidate
        let count = cache.invalidate(123).await.unwrap();
        assert_eq!(count, 1);

        // Should be a miss now
        let result = cache.get(123, &options).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let store = setup_test_store().await;
        let config = CacheConfig::default();
        let cache = ContextCache::new(store, config);

        let options = ExpandOptions::with_common();
        let bundle = create_test_bundle();

        // Put multiple entries
        cache.put(100, &options, &bundle).await.unwrap();
        cache.put(200, &options, &bundle).await.unwrap();
        cache.put(300, &options, &bundle).await.unwrap();

        // Clear all
        let count = cache.clear().await.unwrap();
        assert_eq!(count, 3);

        // All should be misses
        assert!(cache.get(100, &options).await.unwrap().is_none());
        assert!(cache.get(200, &options).await.unwrap().is_none());
        assert!(cache.get(300, &options).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cache_db_stats() {
        let store = setup_test_store().await;
        let config = CacheConfig::default();
        let cache = ContextCache::new(store, config);

        let options = ExpandOptions::with_common();
        let bundle = create_test_bundle();

        // Initially empty
        let stats = cache.get_db_stats().await.unwrap();
        assert_eq!(stats.total_entries, 0);

        // Add some entries
        cache.put(100, &options, &bundle).await.unwrap();
        cache.put(200, &options, &bundle).await.unwrap();

        let stats = cache.get_db_stats().await.unwrap();
        assert_eq!(stats.total_entries, 2);
        assert!(stats.total_size_bytes > 0);
        assert_eq!(stats.entries_last_hour, 2);
    }

    #[tokio::test]
    async fn test_cache_disabled() {
        let store = setup_test_store().await;
        let mut config = CacheConfig::default();
        config.enabled = false;
        let cache = ContextCache::new(store, config);

        let options = ExpandOptions::with_common();
        let bundle = create_test_bundle();

        // Put should be no-op
        cache.put(123, &options, &bundle).await.unwrap();

        // Get should always return None
        let result = cache.get(123, &options).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_different_options() {
        let store = setup_test_store().await;
        let config = CacheConfig::default();
        let cache = ContextCache::new(store, config);

        let options1 = ExpandOptions::with_common();
        let options2 = ExpandOptions::with_all();
        let bundle = create_test_bundle();

        // Put with options1
        cache.put(123, &options1, &bundle).await.unwrap();

        // Get with options1 - hit
        let result = cache.get(123, &options1).await.unwrap();
        assert!(result.is_some());

        // Get with options2 - miss (different options hash)
        let result = cache.get(123, &options2).await.unwrap();
        assert!(result.is_none());
    }
}
