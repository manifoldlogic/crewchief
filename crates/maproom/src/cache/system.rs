//! Multi-layer cache system implementation.
//!
//! Provides unified cache management with L1/L2/L3 caches and parse tree cache.

use super::entry::CacheEntry;
use super::stats::{CacheStats, MultiLayerStats};
use crate::context::types::ContextBundle;
use crate::search::results::FinalSearchResults;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Type alias for embedding vectors.
pub type Vector = Vec<f32>;

/// Cache configuration for the multi-layer cache system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// L1 query cache configuration
    pub l1_query: LayerConfig,
    /// L2 embedding cache configuration
    pub l2_embedding: LayerConfig,
    /// L3 context cache configuration
    pub l3_context: LayerConfig,
    /// Parse tree cache configuration
    pub parse_tree: LayerConfig,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_query: LayerConfig {
                max_entries: 100,
                ttl_seconds: 3600, // 1 hour
                enabled: true,
            },
            l2_embedding: LayerConfig {
                max_entries: 1000,
                ttl_seconds: 86400, // 24 hours
                enabled: true,
            },
            l3_context: LayerConfig {
                max_entries: 500,
                ttl_seconds: 1800, // 30 minutes
                enabled: true,
            },
            parse_tree: LayerConfig {
                max_entries: 200,
                ttl_seconds: 0, // Never expire (until file changes)
                enabled: true,
            },
        }
    }
}

/// Configuration for a single cache layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConfig {
    /// Maximum number of entries
    pub max_entries: usize,
    /// Time-to-live in seconds (0 = never expire)
    pub ttl_seconds: u64,
    /// Whether this layer is enabled
    pub enabled: bool,
}

/// Multi-layer cache system.
///
/// Provides unified cache management with:
/// - L1: Query result cache (100 entries, 1 hour TTL)
/// - L2: Embedding cache (1000 entries, 24 hour TTL)
/// - L3: Context bundle cache (500 entries, 30 min TTL)
/// - Parse tree cache (200 entries, never expire until file changes)
pub struct CacheSystem {
    /// L1: Query result cache
    l1_query: Arc<RwLock<LruCache<String, CacheEntry<FinalSearchResults>>>>,
    l1_stats: Arc<CacheStats>,
    l1_ttl: Duration,
    l1_enabled: bool,

    /// L2: Embedding cache
    l2_embedding: Arc<RwLock<LruCache<String, CacheEntry<Vector>>>>,
    l2_stats: Arc<CacheStats>,
    l2_ttl: Duration,
    l2_enabled: bool,

    /// L3: Context bundle cache
    l3_context: Arc<RwLock<LruCache<u64, CacheEntry<ContextBundle>>>>,
    l3_stats: Arc<CacheStats>,
    l3_ttl: Duration,
    l3_enabled: bool,

    /// Parse tree cache (stores serialized tree-sitter trees)
    parse_tree: Arc<RwLock<LruCache<String, CacheEntry<Vec<u8>>>>>,
    parse_tree_stats: Arc<CacheStats>,
    parse_tree_ttl: Duration,
    parse_tree_enabled: bool,
}

impl CacheSystem {
    /// Create a new multi-layer cache system with the given configuration.
    pub fn new(config: CacheConfig) -> Self {
        info!(
            "Initializing multi-layer cache system (L1: {}, L2: {}, L3: {}, ParseTree: {})",
            config.l1_query.max_entries,
            config.l2_embedding.max_entries,
            config.l3_context.max_entries,
            config.parse_tree.max_entries
        );

        Self {
            // L1: Query cache
            l1_query: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(config.l1_query.max_entries).unwrap(),
            ))),
            l1_stats: Arc::new(CacheStats::new()),
            l1_ttl: Duration::from_secs(config.l1_query.ttl_seconds),
            l1_enabled: config.l1_query.enabled,

            // L2: Embedding cache
            l2_embedding: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(config.l2_embedding.max_entries).unwrap(),
            ))),
            l2_stats: Arc::new(CacheStats::new()),
            l2_ttl: Duration::from_secs(config.l2_embedding.ttl_seconds),
            l2_enabled: config.l2_embedding.enabled,

            // L3: Context cache
            l3_context: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(config.l3_context.max_entries).unwrap(),
            ))),
            l3_stats: Arc::new(CacheStats::new()),
            l3_ttl: Duration::from_secs(config.l3_context.ttl_seconds),
            l3_enabled: config.l3_context.enabled,

            // Parse tree cache
            parse_tree: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(config.parse_tree.max_entries).unwrap(),
            ))),
            parse_tree_stats: Arc::new(CacheStats::new()),
            parse_tree_ttl: Duration::from_secs(config.parse_tree.ttl_seconds),
            parse_tree_enabled: config.parse_tree.enabled,
        }
    }

    // === L1: Query Cache ===

    /// Get a cached query result.
    pub async fn get_query(&self, query: &str) -> Option<FinalSearchResults> {
        if !self.l1_enabled {
            return None;
        }

        let key = Self::hash_key(query);
        let mut cache = self.l1_query.write().await;

        if let Some(entry) = cache.get(&key) {
            if entry.is_expired(self.l1_ttl) {
                cache.pop(&key);
                self.l1_stats.record_expiration();
                self.l1_stats.record_miss();
                debug!("L1 cache EXPIRED: {}", query);
                None
            } else {
                self.l1_stats.record_hit();
                debug!("L1 cache HIT: {}", query);
                Some(entry.value.clone())
            }
        } else {
            self.l1_stats.record_miss();
            debug!("L1 cache MISS: {}", query);
            None
        }
    }

    /// Put a query result into the cache.
    pub async fn put_query(&self, query: &str, results: FinalSearchResults) {
        if !self.l1_enabled {
            return;
        }

        let key = Self::hash_key(query);
        let entry = CacheEntry::new(results);

        let mut cache = self.l1_query.write().await;
        if cache.len() >= cache.cap().get() && !cache.contains(&key) {
            self.l1_stats.record_eviction();
        }

        cache.put(key, entry);
        self.l1_stats.record_insertion();
        debug!("L1 cache PUT: {}", query);
    }

    /// Clear the L1 query cache.
    pub async fn clear_l1(&self) {
        let mut cache = self.l1_query.write().await;
        cache.clear();
        info!("L1 query cache cleared");
    }

    // === L2: Embedding Cache ===

    /// Get a cached embedding.
    pub async fn get_embedding(&self, text: &str) -> Option<Vector> {
        if !self.l2_enabled {
            return None;
        }

        let key = Self::hash_key(text);
        let mut cache = self.l2_embedding.write().await;

        if let Some(entry) = cache.get(&key) {
            if entry.is_expired(self.l2_ttl) {
                cache.pop(&key);
                self.l2_stats.record_expiration();
                self.l2_stats.record_miss();
                debug!("L2 cache EXPIRED: text hash {}", &key[..8]);
                None
            } else {
                self.l2_stats.record_hit();
                debug!("L2 cache HIT: text hash {}", &key[..8]);
                Some(entry.value.clone())
            }
        } else {
            self.l2_stats.record_miss();
            debug!("L2 cache MISS: text hash {}", &key[..8]);
            None
        }
    }

    /// Put an embedding into the cache.
    pub async fn put_embedding(&self, text: &str, vector: Vector) {
        if !self.l2_enabled {
            return;
        }

        let key = Self::hash_key(text);
        let entry = CacheEntry::new(vector);

        let mut cache = self.l2_embedding.write().await;
        if cache.len() >= cache.cap().get() && !cache.contains(&key) {
            self.l2_stats.record_eviction();
        }

        cache.put(key, entry);
        self.l2_stats.record_insertion();
        debug!("L2 cache PUT: text hash {}", &Self::hash_key(text)[..8]);
    }

    /// Clear the L2 embedding cache.
    pub async fn clear_l2(&self) {
        let mut cache = self.l2_embedding.write().await;
        cache.clear();
        info!("L2 embedding cache cleared");
    }

    // === L3: Context Cache ===

    /// Get a cached context bundle.
    pub async fn get_context(&self, chunk_ids: &[i64]) -> Option<ContextBundle> {
        if !self.l3_enabled {
            return None;
        }

        let key = Self::hash_chunk_ids(chunk_ids);
        let mut cache = self.l3_context.write().await;

        if let Some(entry) = cache.get(&key) {
            if entry.is_expired(self.l3_ttl) {
                cache.pop(&key);
                self.l3_stats.record_expiration();
                self.l3_stats.record_miss();
                debug!("L3 cache EXPIRED: key {}", key);
                None
            } else {
                self.l3_stats.record_hit();
                debug!("L3 cache HIT: key {}", key);
                Some(entry.value.clone())
            }
        } else {
            self.l3_stats.record_miss();
            debug!("L3 cache MISS: key {}", key);
            None
        }
    }

    /// Put a context bundle into the cache.
    pub async fn put_context(&self, chunk_ids: &[i64], bundle: ContextBundle) {
        if !self.l3_enabled {
            return;
        }

        let key = Self::hash_chunk_ids(chunk_ids);
        let entry = CacheEntry::new(bundle);

        let mut cache = self.l3_context.write().await;
        if cache.len() >= cache.cap().get() && !cache.contains(&key) {
            self.l3_stats.record_eviction();
        }

        cache.put(key, entry);
        self.l3_stats.record_insertion();
        debug!("L3 cache PUT: key {}", key);
    }

    /// Invalidate a context cache entry by chunk IDs.
    pub async fn invalidate_context(&self, chunk_ids: &[i64]) {
        let key = Self::hash_chunk_ids(chunk_ids);
        let mut cache = self.l3_context.write().await;
        cache.pop(&key);
        debug!("L3 cache INVALIDATE: key {}", key);
    }

    /// Clear the L3 context cache.
    pub async fn clear_l3(&self) {
        let mut cache = self.l3_context.write().await;
        cache.clear();
        info!("L3 context cache cleared");
    }

    // === Parse Tree Cache ===

    /// Get a cached parse tree.
    ///
    /// The key should be: `<file_path>:<content_hash>`
    pub async fn get_parse_tree(&self, file_path: &str, content_hash: &str) -> Option<Vec<u8>> {
        if !self.parse_tree_enabled {
            return None;
        }

        let key = format!("{}:{}", file_path, content_hash);
        let mut cache = self.parse_tree.write().await;

        if let Some(entry) = cache.get(&key) {
            if self.parse_tree_ttl.as_secs() > 0 && entry.is_expired(self.parse_tree_ttl) {
                cache.pop(&key);
                self.parse_tree_stats.record_expiration();
                self.parse_tree_stats.record_miss();
                debug!("ParseTree cache EXPIRED: {}", file_path);
                None
            } else {
                self.parse_tree_stats.record_hit();
                debug!("ParseTree cache HIT: {}", file_path);
                Some(entry.value.clone())
            }
        } else {
            self.parse_tree_stats.record_miss();
            debug!("ParseTree cache MISS: {}", file_path);
            None
        }
    }

    /// Put a parse tree into the cache.
    pub async fn put_parse_tree(&self, file_path: &str, content_hash: &str, tree_data: Vec<u8>) {
        if !self.parse_tree_enabled {
            return;
        }

        let key = format!("{}:{}", file_path, content_hash);
        let entry = CacheEntry::new(tree_data);

        let mut cache = self.parse_tree.write().await;
        if cache.len() >= cache.cap().get() && !cache.contains(&key) {
            self.parse_tree_stats.record_eviction();
        }

        cache.put(key, entry);
        self.parse_tree_stats.record_insertion();
        debug!("ParseTree cache PUT: {}", file_path);
    }

    /// Invalidate all parse tree entries for a given file path.
    pub async fn invalidate_parse_tree(&self, file_path: &str) {
        let mut cache = self.parse_tree.write().await;
        let prefix = format!("{}:", file_path);

        // Collect keys to remove
        let keys_to_remove: Vec<String> = cache
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .map(|(key, _)| key.clone())
            .collect();

        // Remove entries
        for key in keys_to_remove {
            cache.pop(&key);
        }

        debug!("ParseTree cache INVALIDATE: {}", file_path);
    }

    /// Clear the parse tree cache.
    pub async fn clear_parse_tree(&self) {
        let mut cache = self.parse_tree.write().await;
        cache.clear();
        info!("Parse tree cache cleared");
    }

    // === Statistics ===

    /// Get statistics for all cache layers.
    pub async fn stats(&self) -> MultiLayerStats {
        MultiLayerStats {
            l1_query: self.l1_stats.snapshot(),
            l2_embedding: self.l2_stats.snapshot(),
            l3_context: self.l3_stats.snapshot(),
            parse_tree: self.parse_tree_stats.snapshot(),
        }
    }

    /// Get L1 statistics.
    pub fn l1_stats(&self) -> Arc<CacheStats> {
        Arc::clone(&self.l1_stats)
    }

    /// Get L2 statistics.
    pub fn l2_stats(&self) -> Arc<CacheStats> {
        Arc::clone(&self.l2_stats)
    }

    /// Get L3 statistics.
    pub fn l3_stats(&self) -> Arc<CacheStats> {
        Arc::clone(&self.l3_stats)
    }

    /// Get parse tree cache statistics.
    pub fn parse_tree_stats(&self) -> Arc<CacheStats> {
        Arc::clone(&self.parse_tree_stats)
    }

    /// Reset all statistics.
    pub fn reset_stats(&self) {
        self.l1_stats.reset();
        self.l2_stats.reset();
        self.l3_stats.reset();
        self.parse_tree_stats.reset();
        info!("All cache statistics reset");
    }

    /// Clear all caches.
    pub async fn clear_all(&self) {
        self.clear_l1().await;
        self.clear_l2().await;
        self.clear_l3().await;
        self.clear_parse_tree().await;
        info!("All caches cleared");
    }

    // === Helper Methods ===

    /// Generate a hash key from a string.
    fn hash_key<T: Hash + ?Sized>(value: &T) -> String {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Generate a hash key from chunk IDs.
    fn hash_chunk_ids(chunk_ids: &[i64]) -> u64 {
        let mut hasher = DefaultHasher::new();
        chunk_ids.hash(&mut hasher);
        hasher.finish()
    }
}

impl Clone for CacheSystem {
    fn clone(&self) -> Self {
        Self {
            l1_query: Arc::clone(&self.l1_query),
            l1_stats: Arc::clone(&self.l1_stats),
            l1_ttl: self.l1_ttl,
            l1_enabled: self.l1_enabled,

            l2_embedding: Arc::clone(&self.l2_embedding),
            l2_stats: Arc::clone(&self.l2_stats),
            l2_ttl: self.l2_ttl,
            l2_enabled: self.l2_enabled,

            l3_context: Arc::clone(&self.l3_context),
            l3_stats: Arc::clone(&self.l3_stats),
            l3_ttl: self.l3_ttl,
            l3_enabled: self.l3_enabled,

            parse_tree: Arc::clone(&self.parse_tree),
            parse_tree_stats: Arc::clone(&self.parse_tree_stats),
            parse_tree_ttl: self.parse_tree_ttl,
            parse_tree_enabled: self.parse_tree_enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::results::{QueryProcessingDetails, SearchMetadata, SearchTiming};
    use crate::search::types::SearchMode;
    use std::collections::HashMap;

    fn create_test_results() -> FinalSearchResults {
        let query_processing = QueryProcessingDetails::new(
            "test".to_string(),
            SearchMode::Auto,
            1,
            0,
            "test".to_string(),
            false,
        );
        let result_counts = HashMap::new();
        let timing = SearchTiming::new(1.0, 1.0, 1.0, 1.0);
        let metadata = SearchMetadata::new(query_processing, result_counts, timing, 0, 0);
        FinalSearchResults::new("test".to_string(), vec![], metadata)
    }

    #[tokio::test]
    async fn test_l1_cache_operations() {
        let config = CacheConfig::default();
        let cache = CacheSystem::new(config);

        let query = "test query";
        let results = create_test_results();

        // Miss on first access
        assert!(cache.get_query(query).await.is_none());

        // Put and hit
        cache.put_query(query, results.clone()).await;
        assert!(cache.get_query(query).await.is_some());

        // Stats
        let stats = cache.stats().await;
        assert_eq!(stats.l1_query.hits, 1);
        assert_eq!(stats.l1_query.misses, 1);
    }

    #[tokio::test]
    async fn test_l2_cache_operations() {
        let config = CacheConfig::default();
        let cache = CacheSystem::new(config);

        let text = "test text";
        let vector = vec![0.1, 0.2, 0.3];

        // Miss on first access
        assert!(cache.get_embedding(text).await.is_none());

        // Put and hit
        cache.put_embedding(text, vector.clone()).await;
        assert_eq!(cache.get_embedding(text).await.unwrap(), vector);

        // Stats
        let stats = cache.stats().await;
        assert_eq!(stats.l2_embedding.hits, 1);
        assert_eq!(stats.l2_embedding.misses, 1);
    }

    #[tokio::test]
    async fn test_l3_cache_operations() {
        let config = CacheConfig::default();
        let cache = CacheSystem::new(config);

        let chunk_ids = vec![1, 2, 3];
        let bundle = ContextBundle::new();

        // Miss on first access
        assert!(cache.get_context(&chunk_ids).await.is_none());

        // Put and hit
        cache.put_context(&chunk_ids, bundle.clone()).await;
        assert!(cache.get_context(&chunk_ids).await.is_some());

        // Invalidate
        cache.invalidate_context(&chunk_ids).await;
        assert!(cache.get_context(&chunk_ids).await.is_none());

        // Stats
        let stats = cache.stats().await;
        assert_eq!(stats.l3_context.hits, 1);
        assert_eq!(stats.l3_context.misses, 2); // Initial miss + after invalidation
    }

    #[tokio::test]
    async fn test_parse_tree_cache_operations() {
        let config = CacheConfig::default();
        let cache = CacheSystem::new(config);

        let file_path = "test.rs";
        let content_hash = "abc123";
        let tree_data = vec![1, 2, 3, 4, 5];

        // Miss on first access
        assert!(cache.get_parse_tree(file_path, content_hash).await.is_none());

        // Put and hit
        cache.put_parse_tree(file_path, content_hash, tree_data.clone()).await;
        assert_eq!(
            cache.get_parse_tree(file_path, content_hash).await.unwrap(),
            tree_data
        );

        // Invalidate
        cache.invalidate_parse_tree(file_path).await;
        assert!(cache.get_parse_tree(file_path, content_hash).await.is_none());

        // Stats
        let stats = cache.stats().await;
        assert_eq!(stats.parse_tree.hits, 1);
        assert_eq!(stats.parse_tree.misses, 2);
    }

    #[tokio::test]
    async fn test_overall_statistics() {
        let config = CacheConfig::default();
        let cache = CacheSystem::new(config);

        // Generate some cache activity
        cache.put_query("q1", create_test_results()).await;
        cache.get_query("q1").await;
        cache.get_query("q2").await; // Miss

        cache.put_embedding("e1", vec![0.1]).await;
        cache.get_embedding("e1").await;

        let stats = cache.stats().await;

        // Overall hit rate: 2 hits / 3 operations = 0.666...
        // L1: 1 hit (q1), 1 miss (q2) = 2 operations
        // L2: 1 hit (e1), 0 misses = 1 operation
        // Total: 2 hits / 3 operations = 0.666...
        assert!((stats.overall_hit_rate() - 0.666).abs() < 0.02);

        // Total operations across all caches
        assert_eq!(stats.total_operations(), 3);
    }

    #[tokio::test]
    async fn test_clear_all() {
        let config = CacheConfig::default();
        let cache = CacheSystem::new(config);

        // Add entries to all caches
        cache.put_query("q1", create_test_results()).await;
        cache.put_embedding("e1", vec![0.1]).await;
        cache.put_context(&[1, 2, 3], ContextBundle::new()).await;
        cache.put_parse_tree("test.rs", "hash", vec![1, 2, 3]).await;

        // Clear all
        cache.clear_all().await;

        // All should be empty
        assert!(cache.get_query("q1").await.is_none());
        assert!(cache.get_embedding("e1").await.is_none());
        assert!(cache.get_context(&[1, 2, 3]).await.is_none());
        assert!(cache.get_parse_tree("test.rs", "hash").await.is_none());
    }

    #[tokio::test]
    async fn test_disabled_cache_layer() {
        let mut config = CacheConfig::default();
        config.l1_query.enabled = false;

        let cache = CacheSystem::new(config);

        let query = "test";
        let results = create_test_results();

        // Put should be no-op when disabled
        cache.put_query(query, results).await;
        assert!(cache.get_query(query).await.is_none());

        // Stats should remain at zero
        let stats = cache.stats().await;
        assert_eq!(stats.l1_query.hits, 0);
        assert_eq!(stats.l1_query.misses, 0);
    }
}
