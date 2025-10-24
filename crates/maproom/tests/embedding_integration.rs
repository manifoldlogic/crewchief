//! Integration tests for the embedding service.
//!
//! These tests validate end-to-end embedding generation with real API calls.
//! Set OPENAI_API_KEY environment variable to run these tests.

use crewchief_maproom::embedding::{
    CacheConfig, EmbeddingConfig, EmbeddingService, Provider, RetryConfig,
};

/// Create a test configuration.
///
/// This uses environment variables if available, or returns None if no API key is set.
fn test_config() -> Option<EmbeddingConfig> {
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
    })
}

/// Skip test if no API key is configured.
fn skip_if_no_api_key() -> Option<EmbeddingService> {
    let config = test_config()?;
    EmbeddingService::new(config).ok()
}

#[tokio::test]
async fn test_single_embedding_generation() {
    let Some(service) = skip_if_no_api_key() else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    let text = "This is a test sentence for embedding generation.";
    let embedding = service.embed_text(text).await;

    assert!(embedding.is_ok(), "Embedding generation failed: {:?}", embedding.err());

    let embedding = embedding.unwrap();
    assert_eq!(embedding.len(), 1536, "Expected 1536-dimensional embedding");

    // Check that embedding values are reasonable
    assert!(embedding.iter().all(|&v| v.is_finite()), "Embedding contains non-finite values");
}

#[tokio::test]
async fn test_batch_embedding_generation() {
    let Some(service) = skip_if_no_api_key() else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    let texts = vec![
        "First test sentence.".to_string(),
        "Second test sentence.".to_string(),
        "Third test sentence.".to_string(),
    ];

    let embeddings = service.embed_batch(texts.clone()).await;

    assert!(embeddings.is_ok(), "Batch embedding failed: {:?}", embeddings.err());

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
    let Some(service) = skip_if_no_api_key() else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    let text = "This text will be cached.";

    // First call - should hit API
    let initial_requests = service.cost_metrics().total_requests();
    let embedding1 = service.embed_text(text).await.unwrap();
    let after_first = service.cost_metrics().total_requests();

    assert!(after_first > initial_requests, "Expected API call for first embedding");

    // Second call - should use cache
    let embedding2 = service.embed_text(text).await.unwrap();
    let after_second = service.cost_metrics().total_requests();

    assert_eq!(after_second, after_first, "Expected cache hit for second embedding");
    assert_eq!(embedding1, embedding2, "Cached embedding should match original");

    // Check cache metrics
    let cache_metrics = service.cache_metrics().await;
    assert!(cache_metrics.hits >= 1, "Expected at least 1 cache hit");
}

#[tokio::test]
async fn test_cache_hit_rate() {
    let Some(service) = skip_if_no_api_key() else {
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
    assert!(cache_metrics.hit_rate() > 0.5, "Overall cache hit rate should be > 50%");
}

#[tokio::test]
async fn test_large_batch_processing() {
    let Some(service) = skip_if_no_api_key() else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    // Create a batch larger than the configured batch size (10)
    let texts: Vec<String> = (0..25).map(|i| format!("Large batch text {}", i)).collect();

    let embeddings = service.embed_large_batch(texts.clone()).await;

    assert!(embeddings.is_ok(), "Large batch processing failed: {:?}", embeddings.err());

    let embeddings = embeddings.unwrap();
    assert_eq!(embeddings.len(), 25, "Expected all 25 embeddings");

    for embedding in &embeddings {
        assert_eq!(embedding.len(), 1536);
        assert!(embedding.iter().all(|&v| v.is_finite()));
    }
}

#[tokio::test]
async fn test_cost_tracking() {
    let Some(service) = skip_if_no_api_key() else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    let initial_tokens = service.cost_metrics().total_tokens();
    let initial_cost = service.cost_metrics().estimated_cost_usd();

    // Generate some embeddings
    let texts = vec![
        "Cost tracking test 1".to_string(),
        "Cost tracking test 2".to_string(),
    ];
    service.embed_batch(texts).await.unwrap();

    let final_tokens = service.cost_metrics().total_tokens();
    let final_cost = service.cost_metrics().estimated_cost_usd();

    // Verify metrics increased
    assert!(final_tokens > initial_tokens, "Token count should increase");
    assert!(final_cost > initial_cost, "Estimated cost should increase");

    // Cost should be reasonable (text-embedding-3-small is $0.02 per 1M tokens)
    let tokens_used = final_tokens - initial_tokens;
    assert!(tokens_used > 0, "Should have used some tokens");
    assert!(tokens_used < 1000, "Should not use excessive tokens for 2 short texts");
}

#[tokio::test]
async fn test_error_handling_invalid_config() {
    let mut config = EmbeddingConfig::default();
    config.dimension = 0; // Invalid
    config.api_key = Some("test-key".to_string());

    let result = EmbeddingService::new(config);
    assert!(result.is_err(), "Should reject invalid configuration");
}

#[tokio::test]
async fn test_empty_batch() {
    let Some(service) = skip_if_no_api_key() else {
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

    let service = EmbeddingService::from_env();
    assert!(service.is_ok(), "Failed to create service from env: {:?}", service.err());

    let service = service.unwrap();
    assert_eq!(service.dimension(), 1536);
}

#[tokio::test]
async fn test_cache_cleanup() {
    let Some(service) = skip_if_no_api_key() else {
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
    let config = test_config();
    if config.is_none() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    }

    let service = EmbeddingService::new(config.unwrap()).unwrap();
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
