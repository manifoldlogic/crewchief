//! Integration tests for the Ollama embedding provider.
//!
//! These tests validate end-to-end embedding generation with a real Ollama server.
//! Requires Ollama running on localhost:11434 with nomic-embed-text model.
//!
//! Run these tests with:
//! ```
//! cargo test --test ollama_test
//! ```

use crewchief_maproom::embedding::{
    CacheConfig, EmbeddingConfig, EmbeddingService, Provider, RetryConfig,
};
use std::time::Instant;

/// Helper function to check if Ollama is available at localhost:11434.
async fn ollama_available() -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();

    // Check if Ollama is running by hitting the tags endpoint
    let result = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await;

    if result.is_err() {
        return false;
    }

    // Check if nomic-embed-text model is available
    let response = result.unwrap();
    if !response.status().is_success() {
        return false;
    }

    // Parse response to check for nomic-embed-text
    let body = response.text().await.unwrap_or_default();
    body.contains("nomic-embed-text")
}

/// Helper function to create test configuration for Ollama.
fn test_config() -> EmbeddingConfig {
    EmbeddingConfig {
        provider: Provider::Ollama,
        model: "nomic-embed-text".to_string(),
        dimension: 768, // nomic-embed-text uses 768-dimensional embeddings
        cache: CacheConfig {
            max_entries: 1000,
            ttl_seconds: 3600,
            enable_metrics: true,
        },
        batch_size: 10,
        retry: RetryConfig::default(),
        api_key: None, // Ollama doesn't require API key
        api_endpoint: None, // Use default localhost:11434
    }
}

/// Helper function to skip test if Ollama is not available.
async fn skip_if_ollama_unavailable() -> Option<EmbeddingService> {
    if !ollama_available().await {
        eprintln!("WARNING: Skipping test - Ollama not available at localhost:11434 or nomic-embed-text model not found");
        eprintln!("To run these tests:");
        eprintln!("  1. Start Ollama: docker-compose up -d ollama");
        eprintln!("  2. Pull model: ollama pull nomic-embed-text");
        return None;
    }

    let config = test_config();
    match EmbeddingService::new(config) {
        Ok(service) => Some(service),
        Err(e) => {
            eprintln!("WARNING: Failed to create EmbeddingService: {:?}", e);
            None
        }
    }
}

#[tokio::test]
async fn test_single_embedding_generation() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    // Generate embedding for a single code snippet
    let text = "async fn parse_typescript_file(content: &str) -> Result<Vec<Chunk>, Error>";
    let start = Instant::now();
    let embedding = service.embed_text(text).await;
    let duration = start.elapsed();

    println!("Single embedding took: {:?}", duration);

    assert!(
        embedding.is_ok(),
        "Embedding generation failed: {:?}",
        embedding.err()
    );

    let embedding = embedding.unwrap();
    assert_eq!(
        embedding.len(),
        768,
        "Expected 768-dimensional embedding for nomic-embed-text"
    );

    // Verify embedding contains non-zero values
    let non_zero_count = embedding.iter().filter(|&&v| v != 0.0).count();
    assert!(
        non_zero_count > 700,
        "Embedding should have mostly non-zero values, got {} non-zero out of 768",
        non_zero_count
    );

    // Check that embedding values are finite
    assert!(
        embedding.iter().all(|&v| v.is_finite()),
        "Embedding contains non-finite values"
    );

    // Performance check: single embedding should be < 1 second
    assert!(
        duration.as_secs() < 2,
        "Single embedding took too long: {:?}",
        duration
    );
}

#[tokio::test]
async fn test_batch_embedding_generation() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    // Test with multiple code-like text samples
    let texts = vec![
        "function calculateSum(a: number, b: number): number { return a + b; }".to_string(),
        "class UserRepository { async findById(id: string): Promise<User> {} }".to_string(),
        "const config = { apiUrl: 'http://localhost:3000', timeout: 5000 };".to_string(),
        "interface RequestHandler { handle(req: Request): Promise<Response>; }".to_string(),
    ];

    let start = Instant::now();
    let embeddings = service.embed_batch(texts.clone()).await;
    let duration = start.elapsed();

    println!("Batch of {} embeddings took: {:?}", texts.len(), duration);

    assert!(
        embeddings.is_ok(),
        "Batch embedding failed: {:?}",
        embeddings.err()
    );

    let embeddings = embeddings.unwrap();
    assert_eq!(
        embeddings.len(),
        4,
        "Expected 4 embeddings for 4 input texts"
    );

    // Verify all embeddings have correct dimensions and properties
    for (i, embedding) in embeddings.iter().enumerate() {
        assert_eq!(
            embedding.len(),
            768,
            "Embedding {} should have 768 dimensions",
            i
        );
        assert!(
            embedding.iter().all(|&v| v.is_finite()),
            "Embedding {} contains non-finite values",
            i
        );

        let non_zero_count = embedding.iter().filter(|&&v| v != 0.0).count();
        assert!(
            non_zero_count > 700,
            "Embedding {} should have mostly non-zero values",
            i
        );
    }

    // Verify different texts produce different embeddings
    assert_ne!(
        embeddings[0], embeddings[1],
        "Different texts should produce different embeddings"
    );
    assert_ne!(
        embeddings[1], embeddings[2],
        "Different texts should produce different embeddings"
    );
    assert_ne!(
        embeddings[2], embeddings[3],
        "Different texts should produce different embeddings"
    );
}

#[tokio::test]
async fn test_invalid_model_error() {
    // Create config with non-existent model
    let config = EmbeddingConfig {
        provider: Provider::Ollama,
        model: "nonexistent-model-xyz".to_string(),
        dimension: 768,
        cache: CacheConfig::default(),
        batch_size: 10,
        retry: RetryConfig::default(),
        api_key: None,
        api_endpoint: None,
    };

    // Check if Ollama is available first
    if !ollama_available().await {
        eprintln!("WARNING: Skipping test - Ollama not available");
        return;
    }

    let service = EmbeddingService::new(config);
    assert!(
        service.is_ok(),
        "Service creation should succeed even with invalid model"
    );

    let service = service.unwrap();
    let text = "test text";
    let result = service.embed_text(text).await;

    assert!(
        result.is_err(),
        "Embedding with invalid model should fail"
    );

    // Check error message contains model-related information
    let error = result.err().unwrap();
    let error_msg = format!("{:?}", error);
    println!("Error message for invalid model: {}", error_msg);

    // The error should indicate a problem with the model or API
    assert!(
        error_msg.contains("Ollama") || error_msg.contains("model") || error_msg.contains("API"),
        "Error message should mention Ollama, model, or API issues"
    );
}

#[tokio::test]
async fn test_unreachable_endpoint_error() {
    // Create config with invalid endpoint
    let config = EmbeddingConfig {
        provider: Provider::Ollama,
        model: "nomic-embed-text".to_string(),
        dimension: 768,
        cache: CacheConfig::default(),
        batch_size: 10,
        retry: RetryConfig::default(),
        api_key: None,
        api_endpoint: Some("http://localhost:99999/api/embeddings".to_string()), // Invalid port
    };

    let service = EmbeddingService::new(config);
    assert!(service.is_ok(), "Service creation should succeed");

    let service = service.unwrap();
    let text = "test text";
    let result = service.embed_text(text).await;

    assert!(
        result.is_err(),
        "Embedding with unreachable endpoint should fail"
    );

    let error = result.err().unwrap();
    let error_msg = format!("{:?}", error);
    println!("Error message for unreachable endpoint: {}", error_msg);

    // The error should indicate a network/connection problem
    assert!(
        error_msg.contains("Network") || error_msg.contains("connection") || error_msg.contains("timeout"),
        "Error message should indicate network/connection issues"
    );
}

#[tokio::test]
async fn test_batch_performance() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    // Create 50 code-like chunks for performance testing
    let texts: Vec<String> = (0..50)
        .map(|i| {
            format!(
                "function processItem{}(data: Record<string, any>): Promise<Result> {{ \
                 const result = await transform(data); \
                 return validate(result); \
                 }}",
                i
            )
        })
        .collect();

    println!("Starting batch performance test with {} texts", texts.len());
    let start = Instant::now();
    let embeddings = service.embed_large_batch(texts.clone()).await;
    let duration = start.elapsed();

    assert!(
        embeddings.is_ok(),
        "Large batch embedding failed: {:?}",
        embeddings.err()
    );

    let embeddings = embeddings.unwrap();
    assert_eq!(
        embeddings.len(),
        50,
        "Expected 50 embeddings for 50 input texts"
    );

    // Calculate performance metrics
    let chunks_per_second = 50.0 / duration.as_secs_f64();
    let chunks_per_minute = chunks_per_second * 60.0;

    println!("Performance metrics for 50-chunk batch:");
    println!("  Total time: {:?}", duration);
    println!("  Chunks/second: {:.2}", chunks_per_second);
    println!("  Chunks/minute: {:.2}", chunks_per_minute);

    // Log cost metrics
    let metrics = service.cost_metrics();
    println!("  Total requests: {}", metrics.total_requests());
    println!("  Total tokens: {}", metrics.total_tokens());

    // Performance target: 500-1000 chunks/min (8-17 chunks/sec)
    // Allow some flexibility since performance varies by hardware
    // We'll just check it's not extremely slow (< 3 chunks/sec = 180/min)
    assert!(
        chunks_per_minute > 180.0,
        "Performance too slow: {:.2} chunks/min (expected > 180/min)",
        chunks_per_minute
    );

    // Log whether we meet the target range
    if chunks_per_minute >= 500.0 && chunks_per_minute <= 1000.0 {
        println!("✓ Performance within target range (500-1000 chunks/min)");
    } else if chunks_per_minute < 500.0 {
        println!(
            "⚠ Performance below target range: {:.2} chunks/min (target: 500-1000)",
            chunks_per_minute
        );
    } else {
        println!(
            "✓ Performance exceeds target range: {:.2} chunks/min (target: 500-1000)",
            chunks_per_minute
        );
    }

    // Verify all embeddings are valid
    for (i, embedding) in embeddings.iter().enumerate() {
        assert_eq!(embedding.len(), 768, "Embedding {} has wrong dimension", i);
        assert!(
            embedding.iter().all(|&v| v.is_finite()),
            "Embedding {} contains non-finite values",
            i
        );
    }
}

#[tokio::test]
async fn test_ollama_config_validation() {
    // Test that config validation works for Ollama
    let config = test_config();
    assert!(
        config.validate().is_ok(),
        "Valid Ollama config should pass validation"
    );

    // Test wrong dimension for nomic-embed-text
    let mut bad_config = test_config();
    bad_config.dimension = 512; // nomic-embed-text requires 768
    assert!(
        bad_config.validate().is_err(),
        "Wrong dimension should fail validation"
    );

    // Test that Ollama doesn't require API key
    let mut no_key_config = test_config();
    no_key_config.api_key = None;
    assert!(
        no_key_config.validate().is_ok(),
        "Ollama should not require API key"
    );
}

#[tokio::test]
async fn test_ollama_caching_behavior() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    let text = "const cache = new Map<string, CachedValue>();";

    // First call - should hit API
    let initial_requests = service.cost_metrics().total_requests();
    let embedding1 = service.embed_text(text).await.unwrap();
    let after_first = service.cost_metrics().total_requests();

    assert!(
        after_first > initial_requests,
        "Expected API call for first embedding"
    );

    // Second call - should use cache
    let embedding2 = service.embed_text(text).await.unwrap();
    let after_second = service.cost_metrics().total_requests();

    assert_eq!(
        after_second, after_first,
        "Expected cache hit for second embedding"
    );
    assert_eq!(
        embedding1, embedding2,
        "Cached embedding should match original"
    );

    // Check cache metrics
    let cache_metrics = service.cache_metrics().await;
    assert!(
        cache_metrics.hits >= 1,
        "Expected at least 1 cache hit, got {}",
        cache_metrics.hits
    );
}

#[tokio::test]
async fn test_empty_batch_handling() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    let embeddings = service.embed_batch(vec![]).await;
    assert!(embeddings.is_ok(), "Empty batch should succeed");
    assert_eq!(embeddings.unwrap().len(), 0, "Empty batch should return empty result");
}

#[tokio::test]
async fn test_ollama_dimension_retrieval() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    assert_eq!(
        service.dimension(),
        768,
        "Ollama nomic-embed-text service should report 768 dimensions"
    );
}

#[tokio::test]
async fn test_ollama_api_endpoint_default() {
    let config = test_config();
    assert_eq!(
        config.api_endpoint_url(),
        "http://localhost:11434/api/embeddings",
        "Default Ollama endpoint should be localhost:11434"
    );
}

#[tokio::test]
async fn test_ollama_custom_endpoint() {
    let mut config = test_config();
    config.api_endpoint = Some("http://custom-ollama:8080/api/embeddings".to_string());

    assert_eq!(
        config.api_endpoint_url(),
        "http://custom-ollama:8080/api/embeddings",
        "Custom endpoint should be used"
    );
}
