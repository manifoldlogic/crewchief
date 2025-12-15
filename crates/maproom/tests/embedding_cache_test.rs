//! Integration tests for embedding cache behavior.
//!
//! Tests cover:
//! - Cache hit/miss tracking with realistic query sequences
//! - LRU eviction policy validation
//! - Concurrent access stress tests
//! - Cache hit rate measurement over 1000 operations
//! - TTL and expiration behavior
//! - Thread-safety under concurrent load

use crewchief_maproom::embedding::cache::{CacheMetrics, EmbeddingCache, Vector};
use crewchief_maproom::embedding::config::CacheConfig;
use std::sync::Arc;
use tokio::task::JoinSet;

/// Create a standard test configuration.
fn test_config() -> CacheConfig {
    CacheConfig {
        max_entries: 100,
        ttl_seconds: 60,
        enable_metrics: true,
    }
}

/// Create a test vector.
fn test_vector(seed: f32) -> Vector {
    vec![seed; 1536]
}

// ============================================================================
// CACHE HIT/MISS TRACKING TESTS
// ============================================================================

#[tokio::test]
async fn test_cache_hit_miss_tracking() {
    let cache = EmbeddingCache::new(test_config()).unwrap();

    // First access - miss (manually recorded)
    let result = cache.get("text1").await;
    assert!(result.is_none());
    cache.record_miss().await;

    // Add to cache
    cache.put("text1", test_vector(0.1)).await.unwrap();

    // Second access - hit
    let result = cache.get("text1").await;
    assert!(result.is_some());

    // Check metrics
    let metrics = cache.metrics().await;
    assert_eq!(metrics.hits, 1);
    assert_eq!(metrics.misses, 1);
    assert_eq!(metrics.hit_rate(), 0.5);
}

#[tokio::test]
async fn test_cache_hit_rate_realistic_sequence() {
    let config = CacheConfig {
        max_entries: 1000,
        ttl_seconds: 3600,
        enable_metrics: true,
    };
    let cache = EmbeddingCache::new(config).unwrap();

    // Simulate a realistic query sequence with 1000 operations
    // - 100 unique queries
    // - Each query repeated ~10 times (simulating common patterns)
    let mut operations = 0;
    let num_unique = 100;
    let repetitions = 10;

    // Seed the cache with unique queries
    for i in 0..num_unique {
        let text = format!("query_{}", i);
        cache.put(&text, test_vector(i as f32)).await.unwrap();
        operations += 1;
    }

    // Access queries with realistic pattern (Zipf-like distribution)
    // Most common queries accessed more frequently
    for _ in 0..repetitions {
        for i in 0..num_unique {
            let text = format!("query_{}", i);

            // Access more frequent queries more often
            let frequency = if i < 20 {
                3 // Top 20% accessed 3x
            } else if i < 50 {
                2 // Next 30% accessed 2x
            } else {
                1 // Rest accessed 1x
            };

            for _ in 0..frequency {
                if operations >= 1000 {
                    break;
                }

                let result = cache.get(&text).await;
                if result.is_none() {
                    cache.record_miss().await;
                }
                operations += 1;
            }

            if operations >= 1000 {
                break;
            }
        }
    }

    // Check final metrics
    let metrics = cache.metrics().await;
    let hit_rate = metrics.hit_rate();

    println!(
        "Cache hit rate over {} operations: {:.1}%",
        operations,
        hit_rate * 100.0
    );
    println!("Hits: {}, Misses: {}", metrics.hits, metrics.misses);

    // Should exceed 80% hit rate with this pattern
    assert!(
        hit_rate > 0.8,
        "Cache hit rate {:.1}% is below 80% threshold",
        hit_rate * 100.0
    );
}

#[tokio::test]
async fn test_cache_hit_rate_working_set() {
    // Simulate a developer working on a small set of files
    let cache = EmbeddingCache::new(test_config()).unwrap();

    let working_set = vec![
        "src/main.rs",
        "src/lib.rs",
        "src/utils.rs",
        "tests/integration_test.rs",
        "README.md",
    ];

    // First pass: populate cache (misses)
    for file in &working_set {
        cache.record_miss().await;
        cache.put(file, test_vector(0.5)).await.unwrap();
    }

    // Simulate repeated access patterns (hits)
    for _ in 0..20 {
        for file in &working_set {
            let result = cache.get(file).await;
            assert!(result.is_some());
        }
    }

    let metrics = cache.metrics().await;
    let _total_operations = 5 + (5 * 20); // 5 initial + 100 repeated
    let expected_hits = 5 * 20; // 100 hits
    let expected_misses = 5; // 5 misses

    assert_eq!(metrics.hits, expected_hits);
    assert_eq!(metrics.misses, expected_misses);

    let hit_rate = metrics.hit_rate();
    assert!(
        hit_rate > 0.95,
        "Working set should have >95% hit rate, got {:.1}%",
        hit_rate * 100.0
    );
}

// ============================================================================
// LRU EVICTION POLICY TESTS
// ============================================================================

#[tokio::test]
async fn test_lru_eviction_basic() {
    let config = CacheConfig {
        max_entries: 3,
        ttl_seconds: 3600,
        enable_metrics: true,
    };
    let cache = EmbeddingCache::new(config).unwrap();

    // Fill cache to capacity
    cache.put("text1", test_vector(0.1)).await.unwrap();
    cache.put("text2", test_vector(0.2)).await.unwrap();
    cache.put("text3", test_vector(0.3)).await.unwrap();

    assert_eq!(cache.len().await, 3);

    // Add fourth entry - should evict least recently used (text1)
    cache.put("text4", test_vector(0.4)).await.unwrap();

    assert_eq!(cache.len().await, 3);
    assert!(!cache.contains("text1").await); // text1 should be evicted
    assert!(cache.contains("text2").await);
    assert!(cache.contains("text3").await);
    assert!(cache.contains("text4").await);
}

#[tokio::test]
async fn test_lru_eviction_with_access() {
    let config = CacheConfig {
        max_entries: 3,
        ttl_seconds: 3600,
        enable_metrics: true,
    };
    let cache = EmbeddingCache::new(config).unwrap();

    // Fill cache
    cache.put("text1", test_vector(0.1)).await.unwrap();
    cache.put("text2", test_vector(0.2)).await.unwrap();
    cache.put("text3", test_vector(0.3)).await.unwrap();

    // Access text1 to make it recently used
    cache.get("text1").await;

    // Add text4 - should evict text2 (least recently used)
    cache.put("text4", test_vector(0.4)).await.unwrap();

    assert_eq!(cache.len().await, 3);
    assert!(cache.contains("text1").await); // text1 was accessed, should remain
    assert!(!cache.contains("text2").await); // text2 should be evicted
    assert!(cache.contains("text3").await);
    assert!(cache.contains("text4").await);
}

#[tokio::test]
async fn test_lru_eviction_metrics() {
    let config = CacheConfig {
        max_entries: 2,
        ttl_seconds: 3600,
        enable_metrics: true,
    };
    let cache = EmbeddingCache::new(config).unwrap();

    // Fill cache
    cache.put("text1", test_vector(0.1)).await.unwrap();
    cache.put("text2", test_vector(0.2)).await.unwrap();

    // This should trigger eviction
    cache.put("text3", test_vector(0.3)).await.unwrap();

    let metrics = cache.metrics().await;
    assert_eq!(metrics.insertions, 3);
    // Note: LRU eviction doesn't count as an "eviction" in our metrics
    // because LRU::put returns the old value when capacity is full
}

#[tokio::test]
async fn test_lru_maintains_order() {
    let config = CacheConfig {
        max_entries: 5,
        ttl_seconds: 3600,
        enable_metrics: true,
    };
    let cache = EmbeddingCache::new(config).unwrap();

    // Add 5 entries
    for i in 0..5 {
        cache
            .put(&format!("text{}", i), test_vector(i as f32))
            .await
            .unwrap();
    }

    // Access in specific order to set recency
    cache.get("text0").await; // Least recent (accessed first)
    cache.get("text2").await;
    cache.get("text3").await;
    cache.get("text4").await;
    cache.get("text1").await; // Most recent (accessed last)

    // Add new entry - should evict text0 (least recently accessed)
    cache.put("text5", test_vector(0.5)).await.unwrap();

    assert!(!cache.contains("text0").await); // text0 was least recently used, should be evicted
    assert!(cache.contains("text1").await);
    assert!(cache.contains("text2").await);
    assert!(cache.contains("text3").await);
    assert!(cache.contains("text4").await);
    assert!(cache.contains("text5").await);
}

// ============================================================================
// CONCURRENT ACCESS STRESS TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_reads() {
    let cache = Arc::new(EmbeddingCache::new(test_config()).unwrap());

    // Populate cache
    for i in 0..10 {
        cache
            .put(&format!("text{}", i), test_vector(i as f32))
            .await
            .unwrap();
    }

    // Spawn 50 concurrent read tasks
    let mut tasks = JoinSet::new();

    for _ in 0..50 {
        let cache_clone = Arc::clone(&cache);
        tasks.spawn(async move {
            for i in 0..10 {
                let result = cache_clone.get(&format!("text{}", i)).await;
                assert!(result.is_some());
            }
        });
    }

    // Wait for all tasks to complete
    while let Some(result) = tasks.join_next().await {
        assert!(result.is_ok());
    }

    // Verify cache integrity
    assert_eq!(cache.len().await, 10);
}

#[tokio::test]
async fn test_concurrent_writes() {
    let cache = Arc::new(EmbeddingCache::new(test_config()).unwrap());

    // Spawn 50 concurrent write tasks
    let mut tasks = JoinSet::new();

    for task_id in 0..50 {
        let cache_clone = Arc::clone(&cache);
        tasks.spawn(async move {
            for i in 0..10 {
                let key = format!("task{}_text{}", task_id, i);
                cache_clone.put(&key, test_vector(i as f32)).await.unwrap();
            }
        });
    }

    // Wait for all tasks to complete
    while let Some(result) = tasks.join_next().await {
        assert!(result.is_ok());
    }

    // Cache should contain up to max_entries
    let final_size = cache.len().await;
    assert!(final_size <= 100); // max_entries from test_config
}

#[tokio::test]
async fn test_concurrent_mixed_operations() {
    let cache = Arc::new(EmbeddingCache::new(test_config()).unwrap());

    // Seed cache with some data
    for i in 0..20 {
        cache
            .put(&format!("text{}", i), test_vector(i as f32))
            .await
            .unwrap();
    }

    // Spawn mixed read/write tasks
    let mut tasks = JoinSet::new();

    // 25 readers
    for _ in 0..25 {
        let cache_clone = Arc::clone(&cache);
        tasks.spawn(async move {
            for i in 0..20 {
                cache_clone.get(&format!("text{}", i)).await;
            }
        });
    }

    // 25 writers
    for task_id in 0..25 {
        let cache_clone = Arc::clone(&cache);
        tasks.spawn(async move {
            for i in 0..10 {
                let key = format!("new_task{}_text{}", task_id, i);
                cache_clone.put(&key, test_vector(i as f32)).await.unwrap();
            }
        });
    }

    // Wait for all tasks
    while let Some(result) = tasks.join_next().await {
        assert!(result.is_ok());
    }

    // Verify cache is still functional
    let final_size = cache.len().await;
    assert!(final_size <= 100);
}

// ============================================================================
// TTL AND EXPIRATION TESTS
// ============================================================================

#[tokio::test]
async fn test_ttl_expiration_immediate() {
    let config = CacheConfig {
        max_entries: 100,
        ttl_seconds: 0, // Expire immediately
        enable_metrics: true,
    };
    let cache = EmbeddingCache::new(config).unwrap();

    // Add entry
    cache.put("text1", test_vector(0.1)).await.unwrap();
    assert_eq!(cache.len().await, 1);

    // Should be expired on access
    let result = cache.get("text1").await;
    assert!(result.is_none());

    // Should be removed from cache
    assert_eq!(cache.len().await, 0);

    // Check expiration metrics
    let metrics = cache.metrics().await;
    assert_eq!(metrics.expirations, 1);
}

#[tokio::test]
async fn test_ttl_cleanup_expired() {
    let config = CacheConfig {
        max_entries: 100,
        ttl_seconds: 0, // Expire immediately
        enable_metrics: true,
    };
    let cache = EmbeddingCache::new(config).unwrap();

    // Add multiple entries
    for i in 0..10 {
        cache
            .put(&format!("text{}", i), test_vector(i as f32))
            .await
            .unwrap();
    }

    assert_eq!(cache.len().await, 10);

    // Clean up expired entries
    let removed = cache.cleanup_expired().await;
    assert_eq!(removed, 10);
    assert_eq!(cache.len().await, 0);

    let metrics = cache.metrics().await;
    assert_eq!(metrics.expirations, 10);
}

#[tokio::test]
async fn test_ttl_not_expired() {
    let config = CacheConfig {
        max_entries: 100,
        ttl_seconds: 3600, // 1 hour TTL
        enable_metrics: true,
    };
    let cache = EmbeddingCache::new(config).unwrap();

    // Add entry
    cache.put("text1", test_vector(0.1)).await.unwrap();

    // Should not be expired
    let result = cache.get("text1").await;
    assert!(result.is_some());

    // Cleanup should not remove it
    let removed = cache.cleanup_expired().await;
    assert_eq!(removed, 0);
    assert_eq!(cache.len().await, 1);
}

// ============================================================================
// METRICS AND STATISTICS TESTS
// ============================================================================

#[test]
fn test_cache_metrics_hit_rate_calculation() {
    let mut metrics = CacheMetrics::default();

    // Empty cache
    assert_eq!(metrics.hit_rate(), 0.0);

    // 80% hit rate
    metrics.hits = 80;
    metrics.misses = 20;
    assert_eq!(metrics.hit_rate(), 0.8);

    // 100% hit rate
    metrics.hits = 100;
    metrics.misses = 0;
    assert_eq!(metrics.hit_rate(), 1.0);

    // 0% hit rate
    metrics.hits = 0;
    metrics.misses = 100;
    assert_eq!(metrics.hit_rate(), 0.0);
}

#[tokio::test]
async fn test_cache_metrics_reset() {
    let cache = EmbeddingCache::new(test_config()).unwrap();

    // Generate some metrics
    cache.put("text1", test_vector(0.1)).await.unwrap();
    cache.get("text1").await;
    cache.record_miss().await;

    let metrics = cache.metrics().await;
    assert!(metrics.hits > 0 || metrics.misses > 0);

    // Reset metrics
    cache.reset_metrics().await;

    let metrics = cache.metrics().await;
    assert_eq!(metrics.hits, 0);
    assert_eq!(metrics.misses, 0);
    assert_eq!(metrics.insertions, 0);
    assert_eq!(metrics.evictions, 0);
    assert_eq!(metrics.expirations, 0);
}

#[tokio::test]
async fn test_cache_insertion_tracking() {
    let cache = EmbeddingCache::new(test_config()).unwrap();

    // Add 10 entries
    for i in 0..10 {
        cache
            .put(&format!("text{}", i), test_vector(i as f32))
            .await
            .unwrap();
    }

    let metrics = cache.metrics().await;
    assert_eq!(metrics.insertions, 10);
}

// ============================================================================
// EDGE CASES AND ERROR HANDLING
// ============================================================================

#[test]
fn test_invalid_cache_config() {
    let config = CacheConfig {
        max_entries: 0, // Invalid
        ttl_seconds: 3600,
        enable_metrics: true,
    };

    let result = EmbeddingCache::new(config);
    assert!(result.is_err(), "Should reject cache with 0 entries");
}

#[tokio::test]
async fn test_cache_empty_operations() {
    let cache = EmbeddingCache::new(test_config()).unwrap();

    assert!(cache.is_empty().await);
    assert_eq!(cache.len().await, 0);

    let result = cache.get("nonexistent").await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_cache_key_consistency() {
    let cache = EmbeddingCache::new(test_config()).unwrap();

    let text = "test text with special characters: @#$%^&*()";
    let vector = test_vector(0.5);

    cache.put(text, vector.clone()).await.unwrap();

    let retrieved = cache.get(text).await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), vector);
}

#[tokio::test]
async fn test_cache_unicode_keys() {
    let cache = EmbeddingCache::new(test_config()).unwrap();

    let texts = vec!["Hello, 世界!", "Привет мир", "مرحبا بالعالم", "🚀 Rocket"];

    for (i, text) in texts.iter().enumerate() {
        cache.put(text, test_vector(i as f32)).await.unwrap();
    }

    for (i, text) in texts.iter().enumerate() {
        let result = cache.get(text).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), test_vector(i as f32));
    }
}
