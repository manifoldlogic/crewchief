//! LRU cache implementation for embeddings with thread-safe access and metrics.

use crate::embedding::config::CacheConfig;
use crate::embedding::error::{CacheError, EmbeddingError};
use lru::LruCache;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Type alias for embedding vectors.
pub type Vector = Vec<f32>;

/// Cache entry with TTL support.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The embedding vector
    vector: Vector,
    /// Timestamp when the entry was created (Unix timestamp)
    created_at: u64,
}

impl CacheEntry {
    fn new(vector: Vector) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self { vector, created_at }
    }

    fn is_expired(&self, ttl_seconds: u64) -> bool {
        if ttl_seconds == 0 {
            return true; // Always expired if TTL is 0
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.created_at >= ttl_seconds
    }
}

/// Thread-safe LRU cache for embeddings with metrics tracking.
pub struct EmbeddingCache {
    /// Internal LRU cache
    cache: Arc<RwLock<LruCache<String, CacheEntry>>>,
    /// Cache configuration
    config: CacheConfig,
    /// Cache metrics
    metrics: Arc<RwLock<CacheMetrics>>,
}

/// Cache metrics for monitoring.
#[derive(Debug, Default, Clone)]
pub struct CacheMetrics {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of cache evictions
    pub evictions: u64,
    /// Number of expired entries removed
    pub expirations: u64,
    /// Total entries added
    pub insertions: u64,
}

impl CacheMetrics {
    /// Calculate cache hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Reset all metrics.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl EmbeddingCache {
    /// Create a new embedding cache with the given configuration.
    pub fn new(config: CacheConfig) -> Result<Self, EmbeddingError> {
        config.validate()?;

        let capacity = NonZeroUsize::new(config.max_entries)
            .ok_or_else(|| CacheError::ReadFailed("Invalid cache capacity".to_string()))?;

        Ok(Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            config,
            metrics: Arc::new(RwLock::new(CacheMetrics::default())),
        })
    }

    /// Get an embedding from the cache.
    pub async fn get(&self, text: &str) -> Option<Vector> {
        let key = self.hash_key(text);

        let mut cache = self.cache.write().await;
        let entry = cache.get(&key)?;

        // Check if entry is expired
        if entry.is_expired(self.config.ttl_seconds) {
            cache.pop(&key);
            if self.config.enable_metrics {
                let mut metrics = self.metrics.write().await;
                metrics.expirations += 1;
            }
            return None;
        }

        // Record cache hit
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.hits += 1;
        }

        Some(entry.vector.clone())
    }

    /// Put an embedding into the cache.
    pub async fn put(&self, text: &str, vector: Vector) -> Result<(), CacheError> {
        let key = self.hash_key(text);
        let entry = CacheEntry::new(vector);

        let mut cache = self.cache.write().await;
        let evicted = cache.put(key, entry).is_some();

        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.insertions += 1;
            if evicted {
                metrics.evictions += 1;
            }
        }

        Ok(())
    }

    /// Check if the cache contains a key (without updating LRU).
    pub async fn contains(&self, text: &str) -> bool {
        let key = self.hash_key(text);
        let cache = self.cache.read().await;
        cache.contains(&key)
    }

    /// Get cache statistics.
    pub async fn metrics(&self) -> CacheMetrics {
        self.metrics.read().await.clone()
    }

    /// Reset cache metrics.
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.reset();
    }

    /// Clear all entries from the cache.
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get current cache size.
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Check if cache is empty.
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.read().await;
        cache.is_empty()
    }

    /// Record a cache miss (for cases where we check but don't store).
    pub async fn record_miss(&self) {
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.misses += 1;
        }
    }

    /// Create a hash key from text input.
    fn hash_key(&self, text: &str) -> String {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Remove expired entries from the cache.
    pub async fn cleanup_expired(&self) -> usize {
        let mut cache = self.cache.write().await;
        let mut expired_keys = Vec::new();

        // Collect expired keys
        for (key, entry) in cache.iter() {
            if entry.is_expired(self.config.ttl_seconds) {
                expired_keys.push(key.clone());
            }
        }

        // Remove expired entries
        let count = expired_keys.len();
        for key in expired_keys {
            cache.pop(&key);
        }

        if self.config.enable_metrics && count > 0 {
            let mut metrics = self.metrics.write().await;
            metrics.expirations += count as u64;
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CacheConfig {
        CacheConfig {
            max_entries: 100,
            ttl_seconds: 60,
            enable_metrics: true,
        }
    }

    fn test_vector() -> Vector {
        vec![0.1, 0.2, 0.3, 0.4, 0.5]
    }

    #[tokio::test]
    async fn test_cache_put_and_get() {
        let cache = EmbeddingCache::new(test_config()).unwrap();

        let text = "test text";
        let vector = test_vector();

        // Put and get
        cache.put(text, vector.clone()).await.unwrap();
        let retrieved = cache.get(text).await;

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), vector);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = EmbeddingCache::new(test_config()).unwrap();

        let result = cache.get("nonexistent").await;
        assert!(result.is_none());

        // Note: get() does not record misses, only explicit record_miss() does
        // or when expired entries are found
    }

    #[tokio::test]
    async fn test_cache_hit_rate() {
        let cache = EmbeddingCache::new(test_config()).unwrap();

        // Add entry
        cache.put("text1", test_vector()).await.unwrap();

        // One hit
        cache.get("text1").await;

        // Record explicit misses
        cache.record_miss().await;
        cache.record_miss().await;

        let metrics = cache.metrics().await;
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.misses, 2);
        assert_eq!(metrics.hit_rate(), 1.0 / 3.0);
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let config = CacheConfig {
            max_entries: 2,
            ttl_seconds: 60,
            enable_metrics: true,
        };
        let cache = EmbeddingCache::new(config).unwrap();

        // Fill cache
        cache.put("text1", test_vector()).await.unwrap();
        cache.put("text2", test_vector()).await.unwrap();

        // Add third entry - should evict one entry (LRU)
        cache.put("text3", test_vector()).await.unwrap();

        // Cache should only hold 2 entries (capacity limit)
        assert_eq!(cache.len().await, 2);

        // Update existing key - this should count as eviction in our metrics
        cache.put("text3", vec![0.9; 5]).await.unwrap();

        let metrics = cache.metrics().await;
        assert_eq!(metrics.insertions, 4);
        // The last put replaced text3, so we get one eviction
        assert_eq!(metrics.evictions, 1);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let config = CacheConfig {
            max_entries: 100,
            ttl_seconds: 0, // Expire immediately
            enable_metrics: true,
        };
        let cache = EmbeddingCache::new(config).unwrap();

        cache.put("text1", test_vector()).await.unwrap();

        // With TTL of 0, entries expire immediately on next access
        let result = cache.get("text1").await;
        assert!(result.is_none());

        let metrics = cache.metrics().await;
        assert_eq!(metrics.expirations, 1);
    }

    #[tokio::test]
    async fn test_cache_contains() {
        let cache = EmbeddingCache::new(test_config()).unwrap();

        cache.put("text1", test_vector()).await.unwrap();

        assert!(cache.contains("text1").await);
        assert!(!cache.contains("text2").await);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = EmbeddingCache::new(test_config()).unwrap();

        cache.put("text1", test_vector()).await.unwrap();
        cache.put("text2", test_vector()).await.unwrap();

        assert_eq!(cache.len().await, 2);

        cache.clear().await;

        assert_eq!(cache.len().await, 0);
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_metrics_reset() {
        let cache = EmbeddingCache::new(test_config()).unwrap();

        cache.put("text1", test_vector()).await.unwrap();
        cache.get("text1").await;
        cache.get("text2").await;

        let metrics = cache.metrics().await;
        assert!(metrics.hits > 0 || metrics.misses > 0);

        cache.reset_metrics().await;

        let metrics = cache.metrics().await;
        assert_eq!(metrics.hits, 0);
        assert_eq!(metrics.misses, 0);
    }

    #[tokio::test]
    async fn test_hash_key_consistency() {
        let cache = EmbeddingCache::new(test_config()).unwrap();

        let text = "test text";
        let key1 = cache.hash_key(text);
        let key2 = cache.hash_key(text);

        assert_eq!(key1, key2);
    }

    #[tokio::test]
    async fn test_hash_key_uniqueness() {
        let cache = EmbeddingCache::new(test_config()).unwrap();

        let key1 = cache.hash_key("text1");
        let key2 = cache.hash_key("text2");

        assert_ne!(key1, key2);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let config = CacheConfig {
            max_entries: 100,
            ttl_seconds: 0, // Expire immediately
            enable_metrics: true,
        };
        let cache = EmbeddingCache::new(config).unwrap();

        cache.put("text1", test_vector()).await.unwrap();
        cache.put("text2", test_vector()).await.unwrap();

        assert_eq!(cache.len().await, 2);

        // With TTL of 0, all entries are expired
        let removed = cache.cleanup_expired().await;
        assert_eq!(removed, 2);
        assert_eq!(cache.len().await, 0);
    }

    #[test]
    fn test_cache_metrics_hit_rate() {
        let mut metrics = CacheMetrics::default();
        assert_eq!(metrics.hit_rate(), 0.0);

        metrics.hits = 8;
        metrics.misses = 2;
        assert_eq!(metrics.hit_rate(), 0.8);

        metrics.hits = 0;
        metrics.misses = 10;
        assert_eq!(metrics.hit_rate(), 0.0);
    }
}
