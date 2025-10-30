//! Integration tests for cache management features (PERF_OPT-4002).
//!
//! Tests:
//! - TTL configuration per cache layer
//! - Eviction policies (LRU, TTL-based, size-based)
//! - Cache warming strategies
//! - Invalidation logic
//! - No stale data served
//! - Cache effectiveness monitoring

use crewchief_maproom::cache::{
    CacheConfig, CacheInvalidator, CacheLayer, CacheSystem, CacheWarmer, EvictionPolicy,
    EvictionStrategy, LayerConfig, MaintenanceConfig, WarmingStrategy,
};
use crewchief_maproom::context::types::ContextBundle;
use crewchief_maproom::search::results::{
    FinalSearchResults, QueryProcessingDetails, SearchMetadata, SearchTiming,
};
use crewchief_maproom::search::types::SearchMode;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// Helper to create test search results
fn create_test_results(query: &str) -> FinalSearchResults {
    let query_processing = QueryProcessingDetails::new(
        query.to_string(),
        SearchMode::Auto,
        1,
        0,
        query.to_string(),
        false,
    );
    let result_counts = HashMap::new();
    let timing = SearchTiming::new(1.0, 1.0, 1.0, 1.0);
    let metadata = SearchMetadata::new(query_processing, result_counts, timing, 0, 0);
    FinalSearchResults::new(query.to_string(), vec![], metadata)
}

#[tokio::test]
async fn test_ttl_configuration_per_layer() {
    // Create custom TTL configuration for each layer
    let config = CacheConfig {
        l1_query: LayerConfig {
            max_entries: 100,
            ttl_seconds: 10, // 10 seconds
            enabled: true,
        },
        l2_embedding: LayerConfig {
            max_entries: 1000,
            ttl_seconds: 20, // 20 seconds
            enabled: true,
        },
        l3_context: LayerConfig {
            max_entries: 500,
            ttl_seconds: 5, // 5 seconds
            enabled: true,
        },
        parse_tree: LayerConfig {
            max_entries: 200,
            ttl_seconds: 0, // Never expire
            enabled: true,
        },
    };

    let cache = CacheSystem::new(config);

    // Add entries to each layer
    cache.put_query("test_query", create_test_results("test_query")).await;
    cache.put_embedding("test_text", vec![0.1, 0.2, 0.3]).await;
    cache.put_context(&[1, 2, 3], ContextBundle::new()).await;
    cache.put_parse_tree("test.rs", "hash123", vec![1, 2, 3]).await;

    // All should be available immediately
    assert!(cache.get_query("test_query").await.is_some());
    assert!(cache.get_embedding("test_text").await.is_some());
    assert!(cache.get_context(&[1, 2, 3]).await.is_some());
    assert!(cache.get_parse_tree("test.rs", "hash123").await.is_some());

    // Wait for L3 to expire (5 seconds)
    tokio::time::sleep(Duration::from_secs(6)).await;

    // L1, L2, parse tree should still be available
    assert!(cache.get_query("test_query").await.is_some());
    assert!(cache.get_embedding("test_text").await.is_some());
    assert!(cache.get_parse_tree("test.rs", "hash123").await.is_some());

    // L3 should be expired
    assert!(cache.get_context(&[1, 2, 3]).await.is_none());
}

#[tokio::test]
async fn test_lru_eviction_policy() {
    let config = CacheConfig {
        l1_query: LayerConfig {
            max_entries: 3, // Small cache to trigger eviction
            ttl_seconds: 3600,
            enabled: true,
        },
        ..Default::default()
    };

    let cache = CacheSystem::new(config);

    // Add 3 entries (at capacity)
    cache.put_query("query1", create_test_results("query1")).await;
    cache.put_query("query2", create_test_results("query2")).await;
    cache.put_query("query3", create_test_results("query3")).await;

    // All should be present
    assert!(cache.get_query("query1").await.is_some());
    assert!(cache.get_query("query2").await.is_some());
    assert!(cache.get_query("query3").await.is_some());

    // Add a 4th entry, should evict the LRU entry
    cache.put_query("query4", create_test_results("query4")).await;

    // Check statistics show an eviction occurred
    let stats = cache.stats().await;
    assert!(stats.l1_query.evictions > 0);
}

#[tokio::test]
async fn test_ttl_based_eviction() {
    let strategy = EvictionStrategy::new(EvictionPolicy::Ttl(Duration::from_secs(1)));

    // Create an entry and wait for it to expire
    let entry = crewchief_maproom::cache::CacheEntry::new("test_value");

    // Should not be expired immediately
    assert!(!strategy.should_evict_by_ttl(&entry, Duration::from_secs(1)));

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should be expired now
    assert!(strategy.should_evict_by_ttl(&entry, Duration::from_secs(1)));
}

#[tokio::test]
async fn test_size_based_eviction() {
    let strategy = EvictionStrategy::with_memory_limit(
        EvictionPolicy::Size(10 * 1024 * 1024), // 10MB
        10 * 1024 * 1024,
    );

    // Under limit
    assert!(!strategy.should_evict_by_memory(5 * 1024 * 1024));

    // At limit
    assert!(strategy.should_evict_by_memory(10 * 1024 * 1024));

    // Over limit
    assert!(strategy.should_evict_by_memory(15 * 1024 * 1024));
}

#[tokio::test]
async fn test_access_count_eviction() {
    let strategy = EvictionStrategy::new(EvictionPolicy::AccessCount(5));
    let mut entry = crewchief_maproom::cache::CacheEntry::new("test");

    // New entry with 0 accesses should be evicted
    assert!(strategy.should_evict_by_access(&entry, 5));

    // Access it 5 times
    for _ in 0..5 {
        entry.touch();
    }

    // Should not be evicted anymore
    assert!(!strategy.should_evict_by_access(&entry, 5));
}

#[tokio::test]
async fn test_cache_warming_manual() {
    let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
    let queries = vec!["query1".to_string(), "query2".to_string()];

    let warmer = CacheWarmer::with_queries(
        Arc::clone(&cache),
        WarmingStrategy::Manual,
        queries.clone(),
    );

    assert_eq!(warmer.query_count(), 2);

    // Warm the cache
    let stats = warmer.warm().await.unwrap();

    // Both queries should be marked as "would be warmed"
    // (actual warming requires search execution which is out of scope for cache management)
    assert!(stats.total_processed() > 0);
}

#[tokio::test]
async fn test_cache_warming_from_file() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
    let mut warmer = CacheWarmer::new(Arc::clone(&cache), WarmingStrategy::Startup);

    // Create a temporary file with queries
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "query1").unwrap();
    writeln!(file, "# comment").unwrap();
    writeln!(file, "").unwrap();
    writeln!(file, "query2").unwrap();
    file.flush().unwrap();

    // Load queries from file
    warmer.load_queries_from_file(file.path()).await.unwrap();

    assert_eq!(warmer.query_count(), 2);
    assert_eq!(warmer.queries(), &["query1", "query2"]);
}

#[tokio::test]
async fn test_cache_invalidation_file_change() {
    let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
    let invalidator = CacheInvalidator::new(Arc::clone(&cache));

    // Add some cache entries
    cache.put_query("test", create_test_results("test")).await;
    cache.put_parse_tree("test.rs", "hash", vec![1, 2, 3]).await;

    // Invalidate on file change
    let stats = invalidator
        .on_file_changed(Path::new("test.rs"))
        .await
        .unwrap();

    assert!(stats.has_invalidations());
    assert!(stats.parse_tree_invalidated > 0);

    // Parse tree should be invalidated
    assert!(cache.get_parse_tree("test.rs", "hash").await.is_none());
}

#[tokio::test]
async fn test_cache_invalidation_reindex() {
    let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
    let invalidator = CacheInvalidator::new(Arc::clone(&cache));

    // Add entries to all layers
    cache.put_query("test", create_test_results("test")).await;
    cache.put_embedding("text", vec![0.1]).await;
    cache.put_context(&[1], ContextBundle::new()).await;
    cache.put_parse_tree("test.rs", "hash", vec![1]).await;

    // Invalidate on re-index
    let stats = invalidator.on_reindex(123).await.unwrap();

    // All caches should be invalidated
    assert_eq!(stats.query_invalidated, 1);
    assert_eq!(stats.embedding_invalidated, 1);
    assert_eq!(stats.context_invalidated, 1);
    assert_eq!(stats.parse_tree_invalidated, 1);

    // All caches should be empty
    assert!(cache.get_query("test").await.is_none());
    assert!(cache.get_embedding("text").await.is_none());
    assert!(cache.get_context(&[1]).await.is_none());
    assert!(cache.get_parse_tree("test.rs", "hash").await.is_none());
}

#[tokio::test]
async fn test_cache_invalidation_pattern() {
    let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
    let invalidator = CacheInvalidator::new(Arc::clone(&cache));

    // Add cache entry
    cache.put_query("search term", create_test_results("search term")).await;

    // Invalidate by pattern
    let stats = invalidator.on_pattern("search").await.unwrap();

    assert!(stats.query_invalidated > 0);

    // Query cache should be cleared
    assert!(cache.get_query("search term").await.is_none());
}

#[tokio::test]
async fn test_cache_invalidation_specific_layers() {
    let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
    let invalidator = CacheInvalidator::new(Arc::clone(&cache));

    // Add entries to all layers
    cache.put_query("test", create_test_results("test")).await;
    cache.put_embedding("text", vec![0.1]).await;

    // Invalidate only L1
    let stats = invalidator
        .invalidate_layers(&[CacheLayer::L1Query])
        .await
        .unwrap();

    assert_eq!(stats.query_invalidated, 1);
    assert_eq!(stats.embedding_invalidated, 0);

    // L1 should be empty, L2 should still have data
    assert!(cache.get_query("test").await.is_none());
    assert!(cache.get_embedding("text").await.is_some());
}

#[tokio::test]
async fn test_no_stale_data_with_ttl() {
    let config = CacheConfig {
        l1_query: LayerConfig {
            max_entries: 100,
            ttl_seconds: 1, // 1 second TTL
            enabled: true,
        },
        ..Default::default()
    };

    let cache = CacheSystem::new(config);

    // Add entry
    cache.put_query("test", create_test_results("test")).await;

    // Should be available immediately
    assert!(cache.get_query("test").await.is_some());

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should return None (no stale data)
    assert!(cache.get_query("test").await.is_none());

    // Check that expiration was recorded
    let stats = cache.stats().await;
    assert!(stats.l1_query.expirations > 0);
}

#[tokio::test]
async fn test_cache_effectiveness_monitoring() {
    let cache = CacheSystem::new(CacheConfig::default());

    // Generate cache activity to achieve >60% hit rate
    for i in 0..10 {
        let query = format!("query{}", i);
        cache.put_query(&query, create_test_results(&query)).await;
    }

    // Access some queries multiple times (hits)
    for i in 0..7 {
        let query = format!("query{}", i);
        cache.get_query(&query).await;
    }

    // Try to access non-existent queries (misses)
    for i in 10..13 {
        let query = format!("query{}", i);
        cache.get_query(&query).await;
    }

    // Check statistics
    let stats = cache.stats().await;

    // We should have 7 hits and 3 misses = 70% hit rate
    assert!(stats.l1_query.hits >= 7);
    assert!(stats.l1_query.misses >= 3);

    let hit_rate = stats.l1_query.hit_rate();
    assert!(hit_rate >= 0.6, "Hit rate should be >60%, got {:.1}%", hit_rate * 100.0);

    // Check if cache is effective
    assert!(stats.l1_query.is_effective());
}

#[tokio::test]
async fn test_cache_layer_parsing() {
    assert_eq!(CacheLayer::from_str("l1"), Some(CacheLayer::L1Query));
    assert_eq!(CacheLayer::from_str("query"), Some(CacheLayer::L1Query));
    assert_eq!(CacheLayer::from_str("l2"), Some(CacheLayer::L2Embedding));
    assert_eq!(CacheLayer::from_str("embedding"), Some(CacheLayer::L2Embedding));
    assert_eq!(CacheLayer::from_str("l3"), Some(CacheLayer::L3Context));
    assert_eq!(CacheLayer::from_str("context"), Some(CacheLayer::L3Context));
    assert_eq!(CacheLayer::from_str("parse"), Some(CacheLayer::ParseTree));
    assert_eq!(CacheLayer::from_str("all"), Some(CacheLayer::All));
    assert_eq!(CacheLayer::from_str("invalid"), None);
}

#[tokio::test]
async fn test_maintenance_config_defaults() {
    let config = MaintenanceConfig::default();

    assert_eq!(config.interval_secs, 60);
    assert_eq!(config.min_hit_rate, 0.4);
    assert_eq!(config.max_memory_bytes, 500 * 1024 * 1024);
    assert!(config.enable_stats_logging);
    assert!(config.enable_cleanup);
}

#[tokio::test]
async fn test_cache_memory_tracking() {
    let cache = CacheSystem::new(CacheConfig::default());

    // Add some entries
    for i in 0..10 {
        cache.put_query(&format!("q{}", i), create_test_results(&format!("q{}", i))).await;
    }

    let stats = cache.stats().await;

    // Memory tracking should show some usage
    // Note: Size tracking depends on implementation details
    // This test just verifies the API works
    let _ = stats.total_size_mb();
    let _ = stats.is_within_memory_target();
}

#[tokio::test]
async fn test_eviction_stats_tracking() {
    use crewchief_maproom::cache::EvictionStats;

    let mut stats = EvictionStats::new();

    stats.record_ttl_eviction();
    stats.record_size_eviction();
    stats.record_access_eviction();
    stats.record_lru_eviction();

    assert_eq!(stats.total_evictions(), 4);

    stats.reset();
    assert_eq!(stats.total_evictions(), 0);
}
