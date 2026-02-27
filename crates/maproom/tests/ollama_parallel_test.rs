//! Ollama Parallel Processing Integration Tests (EMBPERF-3001)
//!
//! Validates correctness and reliability of parallel embedding generation:
//! - Order preservation (embeddings[i] corresponds to texts[i])
//! - Embedding equivalence (same text → same embedding regardless of config)
//! - Dimension consistency (all embeddings are 768-dim for nomic-embed-text)
//! - Empty input handling
//! - Config validation
//! - Disabled parallel mode behavior
//!
//! # Running
//!
//! ```bash
//! # Prerequisites: Start Ollama with nomic-embed-text
//! ollama serve &
//! ollama pull nomic-embed-text
//!
//! # Run integration tests (requires Ollama)
//! cargo test --test ollama_parallel_test -- --ignored
//!
//! # Run specific test
//! cargo test --test ollama_parallel_test test_batch_preserves_order -- --ignored
//! ```
//!
//! # Requirements
//!
//! - Ollama running at localhost:11434
//! - nomic-embed-text model pulled
//! - Tests marked `#[ignore]` - run manually when Ollama available

use maproom::embedding::config::ParallelConfig;
use maproom::embedding::ollama::OllamaProvider;
use maproom::embedding::provider::EmbeddingProvider;

/// Helper: Generate test texts with identifiable content
fn generate_test_texts(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| format!("Test text number {} with unique content for embedding", i))
        .collect()
}

/// Helper: Calculate cosine similarity between two embeddings
///
/// Used to verify that embeddings are equivalent (same text → same embedding).
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Embeddings must have same dimension");

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

// ============================================================================
// Order Preservation Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires Ollama
async fn test_batch_preserves_order() {
    // Create provider with parallel processing enabled
    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 10,
        max_concurrency: 4,
    };
    let provider = OllamaProvider::new_with_config(
        OllamaProvider::DEFAULT_ENDPOINT.to_string(),
        OllamaProvider::DEFAULT_MODEL.to_string(),
        768,
        config,
    )
    .expect("Failed to create provider");

    // Generate texts with known order
    let texts = generate_test_texts(50);

    // Embed batch using parallel processing
    let embeddings = provider
        .embed_batch(texts.clone())
        .await
        .expect("Batch embedding failed");

    // Verify: Same number of embeddings as texts
    assert_eq!(
        embeddings.len(),
        texts.len(),
        "Number of embeddings must match number of texts"
    );

    // Verify: embeddings[i] corresponds to texts[i]
    // We do this by embedding each text individually and comparing
    for (i, text) in texts.iter().enumerate() {
        let single_embedding = provider
            .embed(text.clone())
            .await
            .expect("Single embedding failed");

        let similarity = cosine_similarity(&embeddings[i], &single_embedding);

        // Embeddings should be nearly identical (cosine similarity > 0.99)
        assert!(
            similarity > 0.99,
            "Embedding at index {} does not match text. Similarity: {:.4}",
            i,
            similarity
        );
    }
}

#[tokio::test]
#[ignore] // Requires Ollama
async fn test_parallel_preserves_order_large_batch() {
    // Test with larger batch that spans multiple sub-batches
    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 20,
        max_concurrency: 8,
    };
    let provider = OllamaProvider::new_with_config(
        OllamaProvider::DEFAULT_ENDPOINT.to_string(),
        OllamaProvider::DEFAULT_MODEL.to_string(),
        768,
        config,
    )
    .expect("Failed to create provider");

    let texts = generate_test_texts(100);
    let embeddings = provider
        .embed_batch(texts.clone())
        .await
        .expect("Batch embedding failed");

    assert_eq!(embeddings.len(), texts.len());

    // Spot check: verify first, middle, and last embeddings
    let check_indices = [0, 25, 50, 75, 99];

    for i in check_indices {
        let single_embedding = provider
            .embed(texts[i].clone())
            .await
            .expect("Single embedding failed");

        let similarity = cosine_similarity(&embeddings[i], &single_embedding);

        assert!(
            similarity > 0.99,
            "Embedding at index {} does not match. Similarity: {:.4}",
            i,
            similarity
        );
    }
}

// ============================================================================
// Embedding Equivalence Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires Ollama
async fn test_parallel_produces_same_embeddings() {
    // Same text should produce same embedding regardless of:
    // - Single vs batch API
    // - Parallel vs non-parallel processing
    // - Different batch/concurrency configs

    let test_text = "This is a test function for authentication".to_string();

    // Config 1: Single embedding
    let provider_single = OllamaProvider::default_config().expect("Failed to create provider");
    let embedding_single = provider_single
        .embed(test_text.clone())
        .await
        .expect("Single embedding failed");

    // Config 2: Batch without parallelism
    let config_batch = ParallelConfig {
        enabled: false,
        sub_batch_size: 1000,
        max_concurrency: 1,
    };
    let provider_batch = OllamaProvider::new_with_config(
        OllamaProvider::DEFAULT_ENDPOINT.to_string(),
        OllamaProvider::DEFAULT_MODEL.to_string(),
        768,
        config_batch,
    )
    .expect("Failed to create provider");
    let embedding_batch = provider_batch
        .embed_batch(vec![test_text.clone()])
        .await
        .expect("Batch embedding failed")[0]
        .clone();

    // Config 3: Parallel processing (small sub-batches)
    let config_parallel = ParallelConfig {
        enabled: true,
        sub_batch_size: 10,
        max_concurrency: 4,
    };
    let provider_parallel = OllamaProvider::new_with_config(
        OllamaProvider::DEFAULT_ENDPOINT.to_string(),
        OllamaProvider::DEFAULT_MODEL.to_string(),
        768,
        config_parallel,
    )
    .expect("Failed to create provider");
    let embedding_parallel = provider_parallel
        .embed_batch(vec![test_text.clone()])
        .await
        .expect("Parallel embedding failed")[0]
        .clone();

    // Verify all embeddings are equivalent (cosine similarity > 0.99)
    let sim_single_batch = cosine_similarity(&embedding_single, &embedding_batch);
    let sim_single_parallel = cosine_similarity(&embedding_single, &embedding_parallel);
    let sim_batch_parallel = cosine_similarity(&embedding_batch, &embedding_parallel);

    assert!(
        sim_single_batch > 0.99,
        "Single vs Batch: Similarity {:.4} < 0.99",
        sim_single_batch
    );
    assert!(
        sim_single_parallel > 0.99,
        "Single vs Parallel: Similarity {:.4} < 0.99",
        sim_single_parallel
    );
    assert!(
        sim_batch_parallel > 0.99,
        "Batch vs Parallel: Similarity {:.4} < 0.99",
        sim_batch_parallel
    );
}

// ============================================================================
// Dimension Consistency Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires Ollama
async fn test_all_embeddings_correct_dimension() {
    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 25,
        max_concurrency: 8,
    };
    let provider = OllamaProvider::new_with_config(
        OllamaProvider::DEFAULT_ENDPOINT.to_string(),
        OllamaProvider::DEFAULT_MODEL.to_string(),
        768,
        config,
    )
    .expect("Failed to create provider");

    // Generate variety of text lengths
    let texts = vec![
        "short".to_string(),
        "Medium length text for testing".to_string(),
        "Very long text with lots of content to test that embedding dimension remains consistent regardless of input length and complexity".to_string(),
        "Code snippet: fn main() { println!(\"hello\"); }".to_string(),
    ];

    let embeddings = provider
        .embed_batch(texts.clone())
        .await
        .expect("Batch embedding failed");

    // Verify all embeddings are 768-dimensional (nomic-embed-text)
    for (i, embedding) in embeddings.iter().enumerate() {
        assert_eq!(
            embedding.len(),
            768,
            "Embedding {} has dimension {} (expected 768)",
            i,
            embedding.len()
        );
    }
}

#[tokio::test]
#[ignore] // Requires Ollama
async fn test_dimension_matches_provider_spec() {
    let provider = OllamaProvider::default_config().expect("Failed to create provider");

    // Provider should report 768 dimensions
    assert_eq!(
        provider.dimension(),
        768,
        "Provider dimension() should return 768"
    );

    // Actual embeddings should match
    let embedding = provider
        .embed("test".to_string())
        .await
        .expect("Embedding failed");

    assert_eq!(
        embedding.len(),
        provider.dimension(),
        "Actual embedding dimension should match provider.dimension()"
    );
}

// ============================================================================
// Empty Input Handling Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires Ollama
async fn test_empty_batch_returns_empty() {
    let provider = OllamaProvider::default_config().expect("Failed to create provider");

    let empty_texts: Vec<String> = vec![];
    let embeddings = provider
        .embed_batch(empty_texts)
        .await
        .expect("Empty batch should succeed");

    assert_eq!(embeddings.len(), 0, "Empty batch should return empty vec");
}

#[tokio::test]
#[ignore] // Requires Ollama
async fn test_empty_batch_parallel_returns_empty() {
    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 50,
        max_concurrency: 8,
    };
    let provider = OllamaProvider::new_with_config(
        OllamaProvider::DEFAULT_ENDPOINT.to_string(),
        OllamaProvider::DEFAULT_MODEL.to_string(),
        768,
        config,
    )
    .expect("Failed to create provider");

    let empty_texts: Vec<String> = vec![];
    let embeddings = provider
        .embed_batch(empty_texts)
        .await
        .expect("Empty batch should succeed with parallel enabled");

    assert_eq!(
        embeddings.len(),
        0,
        "Empty batch should return empty vec (parallel)"
    );
}

// ============================================================================
// Config Validation Tests
// ============================================================================

#[test]
fn test_config_validation_zero_sub_batch_size() {
    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 0, // Invalid
        max_concurrency: 8,
    };

    let result = config.validate();
    assert!(
        result.is_err(),
        "Config with sub_batch_size=0 should be invalid"
    );

    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("sub_batch_size"),
            "Error should mention sub_batch_size"
        );
    }
}

#[test]
fn test_config_validation_zero_max_concurrency() {
    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 50,
        max_concurrency: 0, // Invalid
    };

    let result = config.validate();
    assert!(
        result.is_err(),
        "Config with max_concurrency=0 should be invalid"
    );

    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("max_concurrency"),
            "Error should mention max_concurrency"
        );
    }
}

#[test]
fn test_config_validation_valid() {
    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 50,
        max_concurrency: 8,
    };

    assert!(
        config.validate().is_ok(),
        "Valid config should pass validation"
    );
}

// ============================================================================
// Disabled Parallel Mode Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires Ollama
async fn test_disabled_parallel_uses_single_batch() {
    // When parallel is disabled, should use single batch request
    // even for large batches
    let config = ParallelConfig {
        enabled: false, // Disabled
        sub_batch_size: 10,
        max_concurrency: 1,
    };
    let provider = OllamaProvider::new_with_config(
        OllamaProvider::DEFAULT_ENDPOINT.to_string(),
        OllamaProvider::DEFAULT_MODEL.to_string(),
        768,
        config,
    )
    .expect("Failed to create provider");

    // Large batch (50 texts) that would trigger parallel if enabled
    let texts = generate_test_texts(50);
    let embeddings = provider
        .embed_batch(texts.clone())
        .await
        .expect("Batch embedding should succeed");

    // Verify correctness (should still produce valid embeddings)
    assert_eq!(embeddings.len(), texts.len());

    // Verify all embeddings are 768-dimensional
    for embedding in &embeddings {
        assert_eq!(embedding.len(), 768);
    }
}

#[tokio::test]
#[ignore] // Requires Ollama
async fn test_parallel_mode_switches_at_threshold() {
    // Provider should use:
    // - Single batch when texts.len() <= sub_batch_size
    // - Parallel processing when texts.len() > sub_batch_size

    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 30,
        max_concurrency: 4,
    };
    let provider = OllamaProvider::new_with_config(
        OllamaProvider::DEFAULT_ENDPOINT.to_string(),
        OllamaProvider::DEFAULT_MODEL.to_string(),
        768,
        config,
    )
    .expect("Failed to create provider");

    // Test below threshold (should use single batch)
    let small_texts = generate_test_texts(20);
    let small_embeddings = provider
        .embed_batch(small_texts.clone())
        .await
        .expect("Small batch failed");
    assert_eq!(small_embeddings.len(), small_texts.len());

    // Test above threshold (should use parallel)
    let large_texts = generate_test_texts(60);
    let large_embeddings = provider
        .embed_batch(large_texts.clone())
        .await
        .expect("Large batch failed");
    assert_eq!(large_embeddings.len(), large_texts.len());

    // Both should produce valid 768-dim embeddings
    for embedding in &small_embeddings {
        assert_eq!(embedding.len(), 768);
    }
    for embedding in &large_embeddings {
        assert_eq!(embedding.len(), 768);
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires Ollama
async fn test_single_text_batch() {
    // Edge case: batch of size 1
    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 50,
        max_concurrency: 8,
    };
    let provider = OllamaProvider::new_with_config(
        OllamaProvider::DEFAULT_ENDPOINT.to_string(),
        OllamaProvider::DEFAULT_MODEL.to_string(),
        768,
        config,
    )
    .expect("Failed to create provider");

    let texts = vec!["single text".to_string()];
    let embeddings = provider
        .embed_batch(texts.clone())
        .await
        .expect("Single text batch failed");

    assert_eq!(embeddings.len(), 1);
    assert_eq!(embeddings[0].len(), 768);
}

#[tokio::test]
#[ignore] // Requires Ollama
async fn test_exact_sub_batch_boundary() {
    // Edge case: batch size exactly equals sub_batch_size
    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 50,
        max_concurrency: 4,
    };
    let provider = OllamaProvider::new_with_config(
        OllamaProvider::DEFAULT_ENDPOINT.to_string(),
        OllamaProvider::DEFAULT_MODEL.to_string(),
        768,
        config,
    )
    .expect("Failed to create provider");

    let texts = generate_test_texts(50); // Exactly sub_batch_size
    let embeddings = provider
        .embed_batch(texts.clone())
        .await
        .expect("Exact boundary batch failed");

    assert_eq!(embeddings.len(), texts.len());
}

#[tokio::test]
#[ignore] // Requires Ollama
async fn test_uneven_sub_batch_split() {
    // Edge case: batch doesn't divide evenly into sub-batches
    // e.g., 105 texts with sub_batch_size=50 → [50, 50, 5]
    let config = ParallelConfig {
        enabled: true,
        sub_batch_size: 50,
        max_concurrency: 4,
    };
    let provider = OllamaProvider::new_with_config(
        OllamaProvider::DEFAULT_ENDPOINT.to_string(),
        OllamaProvider::DEFAULT_MODEL.to_string(),
        768,
        config,
    )
    .expect("Failed to create provider");

    let texts = generate_test_texts(105);
    let embeddings = provider
        .embed_batch(texts.clone())
        .await
        .expect("Uneven split batch failed");

    assert_eq!(embeddings.len(), texts.len());

    // Verify correctness of last few embeddings (from the small final sub-batch)
    for i in 100..105 {
        let single_embedding = provider
            .embed(texts[i].clone())
            .await
            .expect("Single embedding failed");

        let similarity = cosine_similarity(&embeddings[i], &single_embedding);
        assert!(
            similarity > 0.99,
            "Embedding {} from small sub-batch incorrect. Similarity: {:.4}",
            i,
            similarity
        );
    }
}
