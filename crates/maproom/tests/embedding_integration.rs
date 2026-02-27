//! Integration tests for the embedding service.
//!
//! These tests validate end-to-end embedding generation with real API calls.
//! Set OPENAI_API_KEY environment variable to run these tests.

use maproom::embedding::{
    cache::EmbeddingCache, CacheConfig, EmbeddingConfig, EmbeddingService, Provider, RetryConfig,
};

/// Create a test configuration.
///
/// This uses environment variables if available, or returns None if no API key is set.
fn test_config() -> Option<EmbeddingConfig> {
    use maproom::embedding::ParallelConfig;
    let api_key = std::env::var("OPENAI_API_KEY").ok()?;

    Some(EmbeddingConfig {
        provider: Provider::OpenAI,
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        cache: CacheConfig {
            max_entries: 1000,
            ttl_seconds: 3600,
            enable_metrics: true,
        },
        batch_size: 10,
        retry: RetryConfig::default(),
        api_key: Some(api_key),
        api_endpoint: None,
        parallel: ParallelConfig::default(),
    })
}

/// Skip test if no API key is configured.
async fn skip_if_no_api_key() -> Option<EmbeddingService> {
    // Check if API key is available
    test_config()?;

    // Use from_env() to create service (will use environment variables)
    EmbeddingService::from_env().await.ok()
}

#[tokio::test]
async fn test_single_embedding_generation() {
    let Some(service) = skip_if_no_api_key().await else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    let text = "This is a test sentence for embedding generation.";
    let embedding = service.embed_text(text).await;

    assert!(
        embedding.is_ok(),
        "Embedding generation failed: {:?}",
        embedding.err()
    );

    let embedding = embedding.unwrap();
    assert_eq!(embedding.len(), 1536, "Expected 1536-dimensional embedding");

    // Check that embedding values are reasonable
    assert!(
        embedding.iter().all(|&v| v.is_finite()),
        "Embedding contains non-finite values"
    );
}

#[tokio::test]
async fn test_batch_embedding_generation() {
    let Some(service) = skip_if_no_api_key().await else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    let texts = vec![
        "First test sentence.".to_string(),
        "Second test sentence.".to_string(),
        "Third test sentence.".to_string(),
    ];

    let embeddings = service.embed_batch(texts.clone()).await;

    assert!(
        embeddings.is_ok(),
        "Batch embedding failed: {:?}",
        embeddings.err()
    );

    let embeddings = embeddings.unwrap();
    assert_eq!(embeddings.len(), 3, "Expected 3 embeddings");

    for embedding in &embeddings {
        assert_eq!(embedding.len(), 1536);
        assert!(embedding.iter().all(|&v| v.is_finite()));
    }

    // Verify different texts produce different embeddings
    assert_ne!(embeddings[0], embeddings[1]);
    assert_ne!(embeddings[1], embeddings[2]);
}

#[tokio::test]
async fn test_caching_behavior() {
    let Some(service) = skip_if_no_api_key().await else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    let text = "This text will be cached.";

    // First call - should miss cache
    let initial_metrics = service.cache_metrics().await;
    let initial_misses = initial_metrics.misses;
    let embedding1 = service.embed_text(text).await.unwrap();
    let after_first = service.cache_metrics().await;

    assert!(
        after_first.misses > initial_misses,
        "Expected cache miss for first embedding"
    );

    // Second call - should hit cache
    let embedding2 = service.embed_text(text).await.unwrap();
    let after_second = service.cache_metrics().await;

    assert!(
        after_second.hits > after_first.hits,
        "Expected cache hit for second embedding"
    );
    assert_eq!(
        embedding1, embedding2,
        "Cached embedding should match original"
    );

    // Check cache metrics
    let cache_metrics = service.cache_metrics().await;
    assert!(cache_metrics.hits >= 1, "Expected at least 1 cache hit");
}

#[tokio::test]
async fn test_cache_hit_rate() {
    let Some(service) = skip_if_no_api_key().await else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    // Generate embeddings for 5 unique texts
    let texts: Vec<String> = (0..5).map(|i| format!("Unique text {}", i)).collect();
    service.embed_batch(texts.clone()).await.unwrap();

    // Embed the same texts again - should all hit cache
    let (embeddings, stats) = service.embed_batch_with_stats(texts).await.unwrap();

    assert_eq!(embeddings.len(), 5);
    assert_eq!(stats.total, 5);
    assert_eq!(stats.cached, 5, "All embeddings should be cached");
    assert_eq!(stats.from_api, 0, "No API calls should be needed");
    assert_eq!(stats.cache_hit_rate, 1.0, "Cache hit rate should be 100%");

    let cache_metrics = service.cache_metrics().await;
    assert!(
        cache_metrics.hit_rate() > 0.5,
        "Overall cache hit rate should be > 50%"
    );
}

#[tokio::test]
async fn test_large_batch_processing() {
    let Some(service) = skip_if_no_api_key().await else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    // Create a batch larger than the configured batch size (10)
    let texts: Vec<String> = (0..25).map(|i| format!("Large batch text {}", i)).collect();

    let embeddings = service.embed_large_batch(texts.clone()).await;

    assert!(
        embeddings.is_ok(),
        "Large batch processing failed: {:?}",
        embeddings.err()
    );

    let embeddings = embeddings.unwrap();
    assert_eq!(embeddings.len(), 25, "Expected all 25 embeddings");

    for embedding in &embeddings {
        assert_eq!(embedding.len(), 1536);
        assert!(embedding.iter().all(|&v| v.is_finite()));
    }
}

#[tokio::test]
async fn test_cache_insertions() {
    let Some(service) = skip_if_no_api_key().await else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    let initial_metrics = service.cache_metrics().await;
    let initial_insertions = initial_metrics.insertions;

    // Generate some embeddings
    let texts = vec![
        "Cache insertion test 1".to_string(),
        "Cache insertion test 2".to_string(),
    ];
    service.embed_batch(texts).await.unwrap();

    let final_metrics = service.cache_metrics().await;

    // Verify cache insertions increased
    assert!(
        final_metrics.insertions > initial_insertions,
        "Cache insertions should increase"
    );
    assert!(
        final_metrics.insertions >= initial_insertions + 2,
        "Should have inserted at least 2 new entries"
    );
}

#[tokio::test]
async fn test_error_handling_invalid_config() {
    let mut cache_config = CacheConfig::default();
    cache_config.max_entries = 0; // Invalid

    let result = EmbeddingCache::new(cache_config);
    assert!(result.is_err(), "Should reject invalid cache configuration");
}

#[tokio::test]
async fn test_empty_batch() {
    let Some(service) = skip_if_no_api_key().await else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    let embeddings = service.embed_batch(vec![]).await;
    assert!(embeddings.is_ok());
    assert_eq!(embeddings.unwrap().len(), 0);
}

#[tokio::test]
async fn test_service_from_env() {
    // This test only runs if OPENAI_API_KEY is set
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    }

    let service = EmbeddingService::from_env().await;
    assert!(
        service.is_ok(),
        "Failed to create service from env: {:?}",
        service.as_ref().err()
    );

    let service = service.unwrap();
    assert_eq!(service.dimension(), 1536);
}

#[tokio::test]
async fn test_cache_cleanup() {
    let Some(service) = skip_if_no_api_key().await else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    // Add some embeddings
    let texts = vec!["test1".to_string(), "test2".to_string()];
    service.embed_batch(texts).await.unwrap();

    let size_before = service.cache_size().await;
    assert!(size_before > 0, "Cache should contain entries");

    service.clear_cache().await;
    let size_after = service.cache_size().await;
    assert_eq!(size_after, 0, "Cache should be empty after clear");
}

#[tokio::test]
async fn test_dimension_retrieval() {
    let Some(service) = skip_if_no_api_key().await else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    assert_eq!(service.dimension(), 1536);
}

#[tokio::test]
async fn test_retry_logic_simulation() {
    // This test verifies the retry configuration is properly set up
    let config = test_config();
    if config.is_none() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    }

    let config = config.unwrap();
    assert_eq!(config.retry.max_attempts, 3);
    assert_eq!(config.retry.initial_delay_ms, 1000);

    // Verify delay calculation
    assert_eq!(config.retry.delay_for_attempt(0), 0);
    assert_eq!(config.retry.delay_for_attempt(1), 1000);
    assert_eq!(config.retry.delay_for_attempt(2), 2000);
    assert_eq!(config.retry.delay_for_attempt(3), 4000);
}
