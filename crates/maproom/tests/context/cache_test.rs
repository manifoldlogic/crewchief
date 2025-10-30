//! Comprehensive unit tests for the context cache system.

use crewchief_maproom::context::{
    CacheConfig, CacheKey, ContextBundle, ContextCache, ContextItem, ExpandOptions, LineRange,
    hash_options,
};
use crewchief_maproom::db::create_pool;
use serial_test::serial;
use std::sync::Arc;

/// Helper to create a test context bundle.
fn create_test_bundle(tokens: usize) -> ContextBundle {
    let mut bundle = ContextBundle::new();
    bundle.add_item(ContextItem {
        relpath: "test.rs".to_string(),
        range: LineRange::new(1, 10),
        role: "primary".to_string(),
        reason: "Test chunk".to_string(),
        content: "fn test() {}".to_string(),
        tokens,
    });
    bundle
}

#[tokio::test]
#[serial]
async fn test_cache_key_creation() {
    let options = ExpandOptions::primary_only();
    let key = CacheKey::new(123, &options);

    assert_eq!(key.chunk_id, 123);
    assert!(!key.options_hash.is_empty());
    assert_eq!(key.options_hash.len(), 64); // SHA-256 hex is 64 chars
}

#[tokio::test]
#[serial]
async fn test_hash_options_deterministic() {
    let options1 = ExpandOptions::with_common();
    let options2 = ExpandOptions::with_common();

    let hash1 = hash_options(&options1);
    let hash2 = hash_options(&options2);

    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64); // SHA-256 hex
}

#[tokio::test]
#[serial]
async fn test_hash_options_different() {
    let options1 = ExpandOptions::primary_only();
    let options2 = ExpandOptions::with_common();
    let options3 = ExpandOptions::with_all();

    let hash1 = hash_options(&options1);
    let hash2 = hash_options(&options2);
    let hash3 = hash_options(&options3);

    assert_ne!(hash1, hash2);
    assert_ne!(hash2, hash3);
    assert_ne!(hash1, hash3);
}

#[tokio::test]
#[serial]
async fn test_cache_disabled() {
    let pool = create_pool().await.expect("Failed to create pool");
    let config = CacheConfig {
        enabled: false,
        ..Default::default()
    };
    let cache = ContextCache::new(pool, config);

    let options = ExpandOptions::primary_only();
    let bundle = create_test_bundle(100);

    // Put should succeed but be a no-op
    cache
        .put(123, &options, &bundle)
        .await
        .expect("Put should succeed");

    // Get should always return None when disabled
    let result = cache.get(123, &options).await.expect("Get should succeed");
    assert!(result.is_none());
}

#[tokio::test]
#[serial]
async fn test_cache_put_and_get() {
    let pool = create_pool().await.expect("Failed to create pool");
    let cache = ContextCache::new(pool, CacheConfig::default());

    // Clear cache first
    cache.clear().await.expect("Clear should succeed");

    let options = ExpandOptions::primary_only();
    let original_bundle = create_test_bundle(100);

    // Put bundle in cache
    cache
        .put(999, &options, &original_bundle)
        .await
        .expect("Put should succeed");

    // Get bundle from cache
    let cached_bundle = cache
        .get(999, &options)
        .await
        .expect("Get should succeed")
        .expect("Bundle should be cached");

    // Verify bundle content matches
    assert_eq!(cached_bundle.items.len(), original_bundle.items.len());
    assert_eq!(cached_bundle.total_tokens, original_bundle.total_tokens);
    assert_eq!(cached_bundle.truncated, original_bundle.truncated);
    assert_eq!(
        cached_bundle.items[0].relpath,
        original_bundle.items[0].relpath
    );
}

#[tokio::test]
#[serial]
async fn test_cache_miss() {
    let pool = create_pool().await.expect("Failed to create pool");
    let cache = ContextCache::new(pool, CacheConfig::default());

    // Clear cache first
    cache.clear().await.expect("Clear should succeed");

    let options = ExpandOptions::primary_only();

    // Get for non-existent chunk should return None
    let result = cache
        .get(99999, &options)
        .await
        .expect("Get should succeed");
    assert!(result.is_none());
}

#[tokio::test]
#[serial]
async fn test_cache_different_options() {
    let pool = create_pool().await.expect("Failed to create pool");
    let cache = ContextCache::new(pool, CacheConfig::default());

    // Clear cache first
    cache.clear().await.expect("Clear should succeed");

    let options1 = ExpandOptions::primary_only();
    let options2 = ExpandOptions::with_common();

    let bundle1 = create_test_bundle(100);
    let bundle2 = create_test_bundle(200);

    // Put bundles with different options
    cache
        .put(777, &options1, &bundle1)
        .await
        .expect("Put should succeed");
    cache
        .put(777, &options2, &bundle2)
        .await
        .expect("Put should succeed");

    // Get with options1 should return bundle1
    let cached1 = cache
        .get(777, &options1)
        .await
        .expect("Get should succeed")
        .expect("Bundle should be cached");
    assert_eq!(cached1.total_tokens, 100);

    // Get with options2 should return bundle2
    let cached2 = cache
        .get(777, &options2)
        .await
        .expect("Get should succeed")
        .expect("Bundle should be cached");
    assert_eq!(cached2.total_tokens, 200);
}

#[tokio::test]
#[serial]
async fn test_cache_update_existing() {
    let pool = create_pool().await.expect("Failed to create pool");
    let cache = ContextCache::new(pool, CacheConfig::default());

    // Clear cache first
    cache.clear().await.expect("Clear should succeed");

    let options = ExpandOptions::primary_only();
    let bundle1 = create_test_bundle(100);
    let bundle2 = create_test_bundle(200);

    // Put first bundle
    cache
        .put(888, &options, &bundle1)
        .await
        .expect("Put should succeed");

    // Put second bundle (should update)
    cache
        .put(888, &options, &bundle2)
        .await
        .expect("Put should succeed");

    // Get should return updated bundle
    let cached = cache
        .get(888, &options)
        .await
        .expect("Get should succeed")
        .expect("Bundle should be cached");
    assert_eq!(cached.total_tokens, 200);
}

#[tokio::test]
#[serial]
async fn test_cache_invalidate() {
    let pool = create_pool().await.expect("Failed to create pool");
    let cache = ContextCache::new(pool, CacheConfig::default());

    // Clear cache first
    cache.clear().await.expect("Clear should succeed");

    let options1 = ExpandOptions::primary_only();
    let options2 = ExpandOptions::with_common();
    let bundle = create_test_bundle(100);

    // Put bundles with different options
    cache
        .put(555, &options1, &bundle)
        .await
        .expect("Put should succeed");
    cache
        .put(555, &options2, &bundle)
        .await
        .expect("Put should succeed");

    // Invalidate chunk
    let count = cache.invalidate(555).await.expect("Invalidate should succeed");
    assert_eq!(count, 2); // Both entries should be invalidated

    // Get should return None for both options
    let result1 = cache.get(555, &options1).await.expect("Get should succeed");
    assert!(result1.is_none());

    let result2 = cache.get(555, &options2).await.expect("Get should succeed");
    assert!(result2.is_none());
}

#[tokio::test]
#[serial]
async fn test_cache_invalidate_many() {
    let pool = create_pool().await.expect("Failed to create pool");
    let cache = ContextCache::new(pool, CacheConfig::default());

    // Clear cache first
    cache.clear().await.expect("Clear should succeed");

    let options = ExpandOptions::primary_only();
    let bundle = create_test_bundle(100);

    // Put bundles for multiple chunks
    for chunk_id in 1000..1010 {
        cache
            .put(chunk_id, &options, &bundle)
            .await
            .expect("Put should succeed");
    }

    // Invalidate subset of chunks
    let chunks_to_invalidate: Vec<i64> = vec![1000, 1002, 1004, 1006, 1008];
    let count = cache
        .invalidate_many(&chunks_to_invalidate)
        .await
        .expect("Invalidate many should succeed");
    assert_eq!(count, 5);

    // Invalidated chunks should be gone
    for chunk_id in &chunks_to_invalidate {
        let result = cache.get(*chunk_id, &options).await.expect("Get should succeed");
        assert!(result.is_none());
    }

    // Non-invalidated chunks should still be cached
    let result = cache.get(1001, &options).await.expect("Get should succeed");
    assert!(result.is_some());
}

#[tokio::test]
#[serial]
async fn test_cache_clear() {
    let pool = create_pool().await.expect("Failed to create pool");
    let cache = ContextCache::new(pool, CacheConfig::default());

    // Clear cache first
    cache.clear().await.expect("Clear should succeed");

    let options = ExpandOptions::primary_only();
    let bundle = create_test_bundle(100);

    // Put multiple bundles
    for chunk_id in 2000..2010 {
        cache
            .put(chunk_id, &options, &bundle)
            .await
            .expect("Put should succeed");
    }

    // Clear cache
    let count = cache.clear().await.expect("Clear should succeed");
    assert_eq!(count, 10);

    // All bundles should be gone
    for chunk_id in 2000..2010 {
        let result = cache.get(chunk_id, &options).await.expect("Get should succeed");
        assert!(result.is_none());
    }
}

#[tokio::test]
#[serial]
async fn test_cache_stats_tracking() {
    let pool = create_pool().await.expect("Failed to create pool");
    let cache = Arc::new(ContextCache::new(pool, CacheConfig::default()));

    // Clear cache and reset stats
    cache.clear().await.expect("Clear should succeed");

    let stats = cache.stats();
    stats.reset();

    let options = ExpandOptions::primary_only();
    let bundle = create_test_bundle(100);

    // Initial stats should be zero
    assert_eq!(stats.hits.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert_eq!(stats.misses.load(std::sync::atomic::Ordering::Relaxed), 0);

    // Cache miss
    let _ = cache.get(3000, &options).await;
    assert_eq!(stats.misses.load(std::sync::atomic::Ordering::Relaxed), 1);

    // Put
    cache
        .put(3000, &options, &bundle)
        .await
        .expect("Put should succeed");
    assert_eq!(stats.puts.load(std::sync::atomic::Ordering::Relaxed), 1);

    // Cache hit
    let _ = cache.get(3000, &options).await;
    assert_eq!(stats.hits.load(std::sync::atomic::Ordering::Relaxed), 1);

    // Hit rate should be 50% (1 hit, 1 miss)
    assert_eq!(stats.hit_rate(), 50.0);

    // Invalidate
    cache.invalidate(3000).await.expect("Invalidate should succeed");
    assert_eq!(
        stats.invalidations.load(std::sync::atomic::Ordering::Relaxed),
        1
    );
}

#[tokio::test]
#[serial]
async fn test_cache_db_stats() {
    let pool = create_pool().await.expect("Failed to create pool");
    let cache = ContextCache::new(pool, CacheConfig::default());

    // Clear cache first
    cache.clear().await.expect("Clear should succeed");

    let options = ExpandOptions::primary_only();
    let bundle = create_test_bundle(100);

    // Put some bundles
    for chunk_id in 4000..4005 {
        cache
            .put(chunk_id, &options, &bundle)
            .await
            .expect("Put should succeed");
    }

    // Get database stats
    let db_stats = cache.get_db_stats().await.expect("Get stats should succeed");

    assert_eq!(db_stats.total_entries, 5);
    assert!(db_stats.total_size_bytes > 0);
    assert!(db_stats.entries_last_hour >= 5);
}

#[tokio::test]
#[serial]
async fn test_cache_ttl_expiration() {
    let pool = create_pool().await.expect("Failed to create pool");
    let config = CacheConfig {
        enabled: true,
        ttl_seconds: 1, // 1 second TTL
        max_entries: 1000,
        evict_batch_size: 100,
    };
    let cache = ContextCache::new(pool, config);

    // Clear cache first
    cache.clear().await.expect("Clear should succeed");

    let options = ExpandOptions::primary_only();
    let bundle = create_test_bundle(100);

    // Put bundle
    cache
        .put(5000, &options, &bundle)
        .await
        .expect("Put should succeed");

    // Should be cached immediately
    let result = cache.get(5000, &options).await.expect("Get should succeed");
    assert!(result.is_some());

    // Wait for TTL to expire
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Should be expired now
    let result = cache.get(5000, &options).await.expect("Get should succeed");
    assert!(result.is_none());
}

#[tokio::test]
#[serial]
async fn test_cache_evict_expired() {
    let pool = create_pool().await.expect("Failed to create pool");
    let config = CacheConfig {
        enabled: true,
        ttl_seconds: 1, // 1 second TTL
        max_entries: 1000,
        evict_batch_size: 100,
    };
    let cache = ContextCache::new(pool, config);

    // Clear cache first
    cache.clear().await.expect("Clear should succeed");

    let options = ExpandOptions::primary_only();
    let bundle = create_test_bundle(100);

    // Put some bundles
    for chunk_id in 6000..6005 {
        cache
            .put(chunk_id, &options, &bundle)
            .await
            .expect("Put should succeed");
    }

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Evict expired entries
    let evicted = cache
        .evict_expired()
        .await
        .expect("Evict expired should succeed");
    assert_eq!(evicted, 5);

    // All should be gone
    for chunk_id in 6000..6005 {
        let result = cache.get(chunk_id, &options).await.expect("Get should succeed");
        assert!(result.is_none());
    }
}

#[tokio::test]
#[serial]
async fn test_cache_config_default() {
    let config = CacheConfig::default();

    assert!(config.enabled);
    assert_eq!(config.ttl_seconds, 3600);
    assert_eq!(config.max_entries, 1000);
    assert_eq!(config.evict_batch_size, 100);
}

#[tokio::test]
#[serial]
async fn test_cache_hit_rate_calculation() {
    let pool = create_pool().await.expect("Failed to create pool");
    let cache = Arc::new(ContextCache::new(pool, CacheConfig::default()));

    // Clear and reset
    cache.clear().await.expect("Clear should succeed");
    cache.stats().reset();

    let options = ExpandOptions::primary_only();
    let bundle = create_test_bundle(100);

    // Put bundle
    cache
        .put(7000, &options, &bundle)
        .await
        .expect("Put should succeed");

    // Generate hits and misses
    // 8 hits
    for _ in 0..8 {
        let _ = cache.get(7000, &options).await;
    }

    // 2 misses
    let _ = cache.get(7001, &options).await;
    let _ = cache.get(7002, &options).await;

    // Hit rate should be 80% (8 hits out of 10 total operations)
    let hit_rate = cache.stats().hit_rate();
    assert_eq!(hit_rate, 80.0);
}

#[tokio::test]
#[serial]
async fn test_cache_access_tracking() {
    let pool = create_pool().await.expect("Failed to create pool");
    let cache = ContextCache::new(pool, CacheConfig::default());

    // Clear cache first
    cache.clear().await.expect("Clear should succeed");

    let options = ExpandOptions::primary_only();
    let bundle = create_test_bundle(100);

    // Put bundle
    cache
        .put(8000, &options, &bundle)
        .await
        .expect("Put should succeed");

    // Access multiple times
    for _ in 0..5 {
        let _ = cache.get(8000, &options).await;
    }

    // Get database stats to verify access count
    let db_stats = cache.get_db_stats().await.expect("Get stats should succeed");

    // Average access count should be > 1 due to multiple accesses
    assert!(db_stats.avg_access_count > 1.0);
}
