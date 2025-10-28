//! Contract tests for EmbeddingProvider trait abstraction.
//!
//! This test suite validates that all provider implementations correctly implement
//! the EmbeddingProvider trait and uphold its contract invariants. Tests include:
//!
//! - **Unit Tests**: Each provider implements the trait correctly
//! - **Contract Tests**: Trait invariants are enforced (dimensions, batch length)
//! - **Factory Tests**: Provider auto-detection and configuration work correctly
//! - **Integration Tests**: Provider swapping works seamlessly
//! - **Error Handling**: Providers handle errors consistently
//!
//! # Test Organization
//!
//! - Unit tests use mock HTTP servers (wiremock) - no real API calls
//! - Contract tests verify trait guarantees using trait objects
//! - Factory tests validate auto-detection and configuration logic
//! - Integration tests with real providers are marked `#[ignore]`
//!
//! # Running Tests
//!
//! ```bash
//! # Run all provider abstraction tests (uses mocks)
//! cargo test -p crewchief-maproom provider_abstraction
//!
//! # Run with real providers (requires Ollama running)
//! cargo test -p crewchief-maproom provider_abstraction -- --ignored --test-threads=1
//! ```

use crewchief_maproom::embedding::provider::EmbeddingProvider;
use crewchief_maproom::embedding::ollama::OllamaProvider;
use crewchief_maproom::embedding::client::OpenAIClient;
use crewchief_maproom::embedding::factory::create_provider_from_env;
use crewchief_maproom::embedding::config::{EmbeddingConfig, Provider, CacheConfig, RetryConfig, ParallelConfig};
use crewchief_maproom::embedding::error::{EmbeddingError, ApiError};
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};
use serde_json::json;
use serial_test::serial;

// ============================================================================
// Unit Tests: Provider Trait Implementation
// ============================================================================

/// Contract test: Ollama provider correctly implements EmbeddingProvider trait.
///
/// This test verifies that OllamaProvider can be used as a trait object and
/// provides correct metadata (dimension, provider_name).
#[tokio::test]
async fn test_ollama_provider_implements_trait() {
    // Create provider and verify it can be used as trait object
    let provider: Box<dyn EmbeddingProvider> = Box::new(
        OllamaProvider::new(
            "http://localhost:11434/api/embed".to_string(),
            "nomic-embed-text".to_string()
        ).expect("Failed to create Ollama provider")
    );

    // Verify trait methods return correct values
    assert_eq!(provider.dimension(), 768, "Ollama should use 768 dimensions");
    assert_eq!(provider.provider_name(), "ollama", "Provider name should be 'ollama'");

    // Verify metrics returns None (Ollama doesn't track metrics)
    assert!(provider.metrics().is_none(), "Ollama provider should not track metrics");
}

/// Contract test: OpenAI provider correctly implements EmbeddingProvider trait.
///
/// This test verifies that OpenAIClient can be used as a trait object and
/// provides correct metadata and metrics.
#[tokio::test]
async fn test_openai_provider_implements_trait() {
    let config = EmbeddingConfig {
        provider: Provider::OpenAI,
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        cache: CacheConfig::default(),
        batch_size: 100,
        retry: RetryConfig::default(),
        api_key: Some("test-key".to_string()),
        api_endpoint: None,
        parallel: ParallelConfig::default(),
    };

    // Create provider and verify it can be used as trait object
    let provider: Box<dyn EmbeddingProvider> = Box::new(
        OpenAIClient::new(config).expect("Failed to create OpenAI client")
    );

    // Verify trait methods return correct values
    assert_eq!(provider.dimension(), 1536, "OpenAI should use 1536 dimensions");
    assert_eq!(provider.provider_name(), "openai", "Provider name should be 'openai'");

    // Verify metrics returns Some (OpenAI tracks cost metrics)
    assert!(provider.metrics().is_some(), "OpenAI provider should track metrics");

    let metrics = provider.metrics().unwrap();
    assert_eq!(metrics.total_requests, 0, "Initial requests should be 0");
    assert_eq!(metrics.total_tokens, 0, "Initial tokens should be 0");
    assert_eq!(metrics.estimated_cost_usd, 0.0, "Initial cost should be 0");
}

// ============================================================================
// Contract Tests: Trait Invariants
// ============================================================================

/// Contract test: embed() output length must equal dimension().
///
/// This verifies the critical invariant that embedding vectors have the
/// expected dimension. Uses mock HTTP server to avoid real API calls.
#[tokio::test]
async fn test_trait_contract_dimension_matches_output() {
    // Setup mock Ollama server that returns 768-dimensional embeddings
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/embed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "embeddings": [vec![0.1_f32; 768]]
        })))
        .mount(&mock_server)
        .await;

    let provider: Box<dyn EmbeddingProvider> = Box::new(
        OllamaProvider::new(
            format!("{}/api/embed", mock_server.uri()),
            "nomic-embed-text".to_string()
        ).expect("Failed to create provider")
    );

    // Test the contract: output length must equal dimension
    let embedding = provider.embed("test text".to_string()).await
        .expect("Embedding request should succeed");

    assert_eq!(
        embedding.len(),
        provider.dimension(),
        "Contract violation: embedding length ({}) must equal dimension ({})",
        embedding.len(),
        provider.dimension()
    );
}

/// Contract test: embed_batch() output length must equal input length.
///
/// This verifies that batching preserves the number of texts - every input
/// text must have a corresponding output embedding.
#[tokio::test]
async fn test_trait_contract_batch_length_matches_input() {
    // Setup mock Ollama server that returns embeddings for each input
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/embed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "embeddings": [
                vec![0.1_f32; 768],
                vec![0.2_f32; 768],
                vec![0.3_f32; 768]
            ]
        })))
        .mount(&mock_server)
        .await;

    let provider: Box<dyn EmbeddingProvider> = Box::new(
        OllamaProvider::new(
            format!("{}/api/embed", mock_server.uri()),
            "nomic-embed-text".to_string()
        ).expect("Failed to create provider")
    );

    let texts = vec![
        "first text".to_string(),
        "second text".to_string(),
        "third text".to_string()
    ];
    let input_len = texts.len();

    // Test the contract: output length must equal input length
    let embeddings = provider.embed_batch(texts).await
        .expect("Batch embedding should succeed");

    assert_eq!(
        embeddings.len(),
        input_len,
        "Contract violation: batch output length ({}) must equal input length ({})",
        embeddings.len(),
        input_len
    );

    // Also verify each embedding has correct dimension
    for (idx, embedding) in embeddings.iter().enumerate() {
        assert_eq!(
            embedding.len(),
            provider.dimension(),
            "Embedding {} has wrong dimension: {} != {}",
            idx,
            embedding.len(),
            provider.dimension()
        );
    }
}

/// Contract test: dimension() must return consistent value.
///
/// The dimension must not change during the provider's lifetime.
#[tokio::test]
async fn test_trait_contract_dimension_is_consistent() {
    let provider = OllamaProvider::new(
        "http://localhost:11434/api/embed".to_string(),
        "nomic-embed-text".to_string()
    ).expect("Failed to create provider");

    let dim1 = provider.dimension();
    let dim2 = provider.dimension();
    let dim3 = provider.dimension();

    assert_eq!(dim1, dim2, "Dimension must be consistent across calls");
    assert_eq!(dim2, dim3, "Dimension must be consistent across calls");
    assert_eq!(dim1, 768, "Ollama dimension should be 768");
}

// ============================================================================
// Factory Tests: Auto-detection and Configuration
// ============================================================================

/// Factory test: Auto-detection selects Ollama when available.
///
/// When no explicit provider is configured and Ollama is available,
/// the factory should auto-detect and select it.
#[tokio::test]
#[serial]
async fn test_factory_auto_detects_ollama() {
    // Setup mock Ollama server for auto-detection
    let mock_server = MockServer::start().await;

    // Mock the /api/tags endpoint for Ollama detection
    Mock::given(method("GET"))
        .and(path("/api/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "models": [{"name": "nomic-embed-text"}]
        })))
        .mount(&mock_server)
        .await;

    // Note: This test can't fully test auto-detection because it checks localhost:11434
    // In real scenarios, if Ollama is running, it will be detected

    // Instead, test explicit Ollama configuration
    std::env::set_var("EMBEDDING_PROVIDER", "ollama");
    std::env::set_var("EMBEDDING_API_ENDPOINT", format!("{}/api/embed", mock_server.uri()));
    std::env::set_var("EMBEDDING_MODEL", "nomic-embed-text");

    let provider = create_provider_from_env().await
        .expect("Factory should create Ollama provider");

    assert_eq!(provider.provider_name(), "ollama", "Should select Ollama provider");
    assert_eq!(provider.dimension(), 768, "Ollama should have 768 dimensions");

    // Cleanup
    std::env::remove_var("EMBEDDING_PROVIDER");
    std::env::remove_var("EMBEDDING_API_ENDPOINT");
    std::env::remove_var("EMBEDDING_MODEL");
}

/// Factory test: Explicit config overrides auto-detection.
///
/// When EMBEDDING_PROVIDER is explicitly set, it should be used even
/// if Ollama is available for auto-detection.
#[tokio::test]
#[serial]
async fn test_factory_explicit_config_overrides_auto_detection() {
    // Set explicit OpenAI configuration
    std::env::set_var("EMBEDDING_PROVIDER", "openai");
    std::env::set_var("OPENAI_API_KEY", "test-key-12345");

    let provider = create_provider_from_env().await
        .expect("Factory should create OpenAI provider");

    assert_eq!(provider.provider_name(), "openai", "Should use explicit OpenAI config");
    assert_eq!(provider.dimension(), 1536, "OpenAI should have 1536 dimensions");

    // Cleanup
    std::env::remove_var("EMBEDDING_PROVIDER");
    std::env::remove_var("OPENAI_API_KEY");
}

/// Factory test: Case-insensitive provider names.
///
/// Provider names should be normalized to lowercase, allowing users
/// to specify "Ollama", "OLLAMA", or "ollama" interchangeably.
#[tokio::test]
#[serial]
async fn test_factory_case_insensitive_provider_names() {
    // Ensure clean environment (remove any stray API keys from previous tests)
    std::env::remove_var("OPENAI_API_KEY");

    // Test uppercase
    std::env::set_var("EMBEDDING_PROVIDER", "OLLAMA");
    let provider = create_provider_from_env().await
        .expect("Uppercase OLLAMA should work");
    assert_eq!(provider.provider_name(), "ollama");
    std::env::remove_var("EMBEDDING_PROVIDER");

    // Test mixed case
    std::env::set_var("EMBEDDING_PROVIDER", "Ollama");
    let provider = create_provider_from_env().await
        .expect("Mixed case Ollama should work");
    assert_eq!(provider.provider_name(), "ollama");
    std::env::remove_var("EMBEDDING_PROVIDER");
}

// ============================================================================
// Integration Tests: Provider Swapping
// ============================================================================

/// Integration test: Swap providers via environment variable.
///
/// This test verifies that provider selection works correctly when
/// switching between providers via environment configuration, and that
/// embeddings work correctly after swapping.
#[tokio::test]
#[serial]
async fn test_provider_swap() {
    // Test swapping from OpenAI to Ollama
    std::env::set_var("EMBEDDING_PROVIDER", "openai");
    std::env::set_var("OPENAI_API_KEY", "test-key");

    let provider1 = create_provider_from_env().await
        .expect("Should create OpenAI provider");
    assert_eq!(provider1.provider_name(), "openai");
    assert_eq!(provider1.dimension(), 1536);

    // Swap to Ollama with mock server
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/embed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "embeddings": [vec![0.1_f32; 768]]
        })))
        .mount(&mock_server)
        .await;

    std::env::set_var("EMBEDDING_PROVIDER", "ollama");
    std::env::set_var("EMBEDDING_API_ENDPOINT", format!("{}/api/embed", mock_server.uri()));
    std::env::set_var("EMBEDDING_MODEL", "nomic-embed-text");
    std::env::remove_var("OPENAI_API_KEY");

    let provider2 = create_provider_from_env().await
        .expect("Should create Ollama provider");
    assert_eq!(provider2.provider_name(), "ollama");
    assert_eq!(provider2.dimension(), 768);

    // Verify embeddings work after swapping
    let embedding = provider2.embed("test text".to_string()).await
        .expect("Embedding should work after provider swap");

    assert_eq!(
        embedding.len(),
        768,
        "Swapped Ollama provider should return 768-dimensional embeddings"
    );

    // Cleanup
    std::env::remove_var("EMBEDDING_PROVIDER");
    std::env::remove_var("EMBEDDING_API_ENDPOINT");
    std::env::remove_var("EMBEDDING_MODEL");
}

/// Integration test: Different providers can be used interchangeably via trait object.
///
/// This verifies that code using `Box<dyn EmbeddingProvider>` can seamlessly
/// work with any provider implementation.
#[tokio::test]
async fn test_providers_interchangeable_via_trait_object() {
    // Create a function that works with any provider
    async fn process_with_provider(provider: Box<dyn EmbeddingProvider>) -> (String, usize) {
        (provider.provider_name().to_string(), provider.dimension())
    }

    // Test with Ollama
    let ollama: Box<dyn EmbeddingProvider> = Box::new(
        OllamaProvider::new(
            "http://localhost:11434/api/embed".to_string(),
            "nomic-embed-text".to_string()
        ).expect("Create Ollama provider")
    );
    let (name1, dim1) = process_with_provider(ollama).await;
    assert_eq!(name1, "ollama");
    assert_eq!(dim1, 768);

    // Test with OpenAI
    let config = EmbeddingConfig {
        provider: Provider::OpenAI,
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        cache: CacheConfig::default(),
        batch_size: 100,
        retry: RetryConfig::default(),
        api_key: Some("test-key".to_string()),
        api_endpoint: None,
        parallel: ParallelConfig::default(),
    };
    let openai: Box<dyn EmbeddingProvider> = Box::new(
        OpenAIClient::new(config).expect("Create OpenAI client")
    );
    let (name2, dim2) = process_with_provider(openai).await;
    assert_eq!(name2, "openai");
    assert_eq!(dim2, 1536);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

/// Error handling test: Provider unavailable returns appropriate error.
///
/// When a provider cannot be reached, the error should be clear and retryable.
#[tokio::test]
async fn test_error_handling_provider_unavailable() {
    // Create provider pointing to non-existent endpoint
    let provider = OllamaProvider::new(
        "http://localhost:99999/api/embed".to_string(),
        "nomic-embed-text".to_string()
    ).expect("Create provider");

    let result = provider.embed("test".to_string()).await;

    assert!(result.is_err(), "Should return error for unavailable provider");

    match result.unwrap_err() {
        EmbeddingError::Network(_) => {
            // Expected: network error for connection refused
        }
        other => panic!("Expected Network error, got: {:?}", other),
    }
}

/// Error handling test: Invalid API key returns authentication error.
///
/// Authentication failures should be clearly identified and marked as non-retryable.
#[tokio::test]
async fn test_error_handling_authentication_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/embed"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "Invalid API key"
        })))
        .mount(&mock_server)
        .await;

    let provider = OllamaProvider::new(
        format!("{}/api/embed", mock_server.uri()),
        "nomic-embed-text".to_string()
    ).expect("Create provider");

    let result = provider.embed("test".to_string()).await;

    assert!(result.is_err(), "Should return error for invalid API key");

    match result.unwrap_err() {
        EmbeddingError::Api(api_err) => {
            match api_err {
                ApiError::Authentication(_) => {
                    // Expected
                    assert!(!api_err.is_retryable(), "Auth errors should not be retryable");
                }
                other => panic!("Expected Authentication error, got: {:?}", other),
            }
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

/// Error handling test: Server errors are marked as retryable.
///
/// Transient server errors (5xx) should be retryable with appropriate delays.
#[tokio::test]
async fn test_error_handling_server_error_retryable() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/embed"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": "Internal server error"
        })))
        .mount(&mock_server)
        .await;

    let provider = OllamaProvider::new(
        format!("{}/api/embed", mock_server.uri()),
        "nomic-embed-text".to_string()
    ).expect("Create provider");

    let result = provider.embed("test".to_string()).await;

    assert!(result.is_err(), "Should return error for server error");

    match result.unwrap_err() {
        EmbeddingError::Api(api_err) => {
            match api_err {
                ApiError::ServerError { status, .. } => {
                    assert_eq!(status, 500, "Should capture 500 status");
                    assert!(api_err.is_retryable(), "Server errors should be retryable");
                    assert_eq!(api_err.retry_delay_ms(), Some(1000), "Should suggest 1s retry delay");
                }
                other => panic!("Expected ServerError, got: {:?}", other),
            }
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

/// Error handling test: Rate limit errors include retry delay.
///
/// Rate limit errors (429) should be retryable with delay information.
#[tokio::test]
async fn test_error_handling_rate_limit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/embed"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({
            "error": "Rate limit exceeded"
        })))
        .mount(&mock_server)
        .await;

    let provider = OllamaProvider::new(
        format!("{}/api/embed", mock_server.uri()),
        "nomic-embed-text".to_string()
    ).expect("Create provider");

    let result = provider.embed("test".to_string()).await;

    assert!(result.is_err(), "Should return error for rate limit");

    match result.unwrap_err() {
        EmbeddingError::Api(api_err) => {
            match api_err {
                ApiError::RateLimit { retry_after_ms } => {
                    assert_eq!(retry_after_ms, 1000, "Should suggest retry delay");
                    assert!(api_err.is_retryable(), "Rate limits should be retryable");
                }
                other => panic!("Expected RateLimit error, got: {:?}", other),
            }
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

/// Error handling test: Empty embeddings response is invalid.
///
/// API responses with empty embeddings arrays should be rejected.
#[tokio::test]
async fn test_error_handling_empty_embeddings_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/embed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "embeddings": []
        })))
        .mount(&mock_server)
        .await;

    let provider = OllamaProvider::new(
        format!("{}/api/embed", mock_server.uri()),
        "nomic-embed-text".to_string()
    ).expect("Create provider");

    let result = provider.embed("test".to_string()).await;

    assert!(result.is_err(), "Should return error for empty embeddings");

    match result.unwrap_err() {
        EmbeddingError::Api(ApiError::InvalidResponse(msg)) => {
            assert!(msg.contains("Empty"), "Error should mention empty embeddings");
        }
        other => panic!("Expected InvalidResponse error, got: {:?}", other),
    }
}

/// Error handling test: Dimension mismatch is detected and returns error.
///
/// When API returns embeddings with wrong dimensions (e.g., 512 instead of 768),
/// the provider must detect this and return an appropriate error.
///
/// **Contract Requirement**: Providers MUST validate that returned embeddings
/// match the expected dimension and return `EmbeddingError::Api(InvalidResponse)`
/// if there's a mismatch. This upholds the critical trait guarantee that
/// `embed()` output length equals `dimension()`.
#[tokio::test]
async fn test_error_handling_dimension_mismatch() {
    let mock_server = MockServer::start().await;

    // Mock Ollama server that returns wrong dimension (512 instead of 768)
    Mock::given(method("POST"))
        .and(path("/api/embed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "embeddings": [vec![0.1_f32; 512]]  // Wrong dimension!
        })))
        .mount(&mock_server)
        .await;

    let provider = OllamaProvider::new(
        format!("{}/api/embed", mock_server.uri()),
        "nomic-embed-text".to_string()
    ).expect("Create provider");

    let result = provider.embed("test".to_string()).await;

    assert!(result.is_err(), "Should return error for dimension mismatch");

    match result.unwrap_err() {
        EmbeddingError::Api(ApiError::InvalidResponse(msg)) => {
            // Error message should mention dimension or length mismatch
            let msg_lower = msg.to_lowercase();
            assert!(
                msg_lower.contains("dimension") || msg_lower.contains("length"),
                "Error should mention dimension/length mismatch, got: {}",
                msg
            );
        }
        other => panic!("Expected InvalidResponse error for dimension mismatch, got: {:?}", other),
    }
}

// ============================================================================
// Compile-time Tests: Send + Sync Bounds
// ============================================================================

/// Compile-time test: Providers are Send + Sync.
///
/// This test verifies at compile-time that providers can be safely shared
/// across async tasks and threads. The trait requires Send + Sync bounds.
#[test]
fn test_providers_are_send_sync() {
    // This function only compiles if T: Send + Sync
    fn assert_send_sync<T: Send + Sync>() {}

    // These will fail to compile if providers don't implement Send + Sync
    assert_send_sync::<OllamaProvider>();
    assert_send_sync::<OpenAIClient>();

    // Also verify trait objects are Send + Sync
    assert_send_sync::<Box<dyn EmbeddingProvider>>();
}

/// Compile-time test: Trait objects can be stored in Arc.
///
/// Verifies that providers can be shared across threads using Arc,
/// which is a common pattern for async services.
#[test]
fn test_providers_can_be_stored_in_arc() {
    use std::sync::Arc;

    // This only compiles if the provider is Send + Sync
    fn use_in_arc(provider: Arc<dyn EmbeddingProvider>) -> Arc<dyn EmbeddingProvider> {
        provider
    }

    let provider = OllamaProvider::new(
        "http://localhost:11434/api/embed".to_string(),
        "nomic-embed-text".to_string()
    ).expect("Create provider");

    let arc: Arc<dyn EmbeddingProvider> = Arc::new(provider);
    let _returned = use_in_arc(arc);
}

// ============================================================================
// Integration Tests with Real Providers (marked #[ignore])
// ============================================================================

/// Integration test with real Ollama instance.
///
/// This test requires a real Ollama instance running on localhost:11434.
/// It verifies the complete request/response cycle with actual embeddings.
///
/// Run with: cargo test integration_test_ollama_real -- --ignored
#[tokio::test]
#[ignore = "requires real Ollama instance at localhost:11434"]
async fn integration_test_ollama_real() {
    let provider = OllamaProvider::new(
        "http://localhost:11434/api/embed".to_string(),
        "nomic-embed-text".to_string()
    ).expect("Create Ollama provider");

    // Test single embedding
    let embedding = provider.embed("test text for embedding".to_string()).await
        .expect("Should generate embedding with real Ollama");

    assert_eq!(embedding.len(), 768, "Real Ollama should return 768-dim vectors");

    // Verify embedding has non-zero values
    let has_nonzero = embedding.iter().any(|&v| v != 0.0);
    assert!(has_nonzero, "Embedding should contain non-zero values");

    // Test batch embedding
    let texts = vec![
        "first text".to_string(),
        "second text".to_string(),
        "third text".to_string()
    ];
    let embeddings = provider.embed_batch(texts.clone()).await
        .expect("Should generate batch embeddings");

    assert_eq!(embeddings.len(), texts.len(), "Batch should return all embeddings");
    for embedding in embeddings {
        assert_eq!(embedding.len(), 768, "Each embedding should be 768-dim");
    }
}

/// Integration test with real OpenAI API.
///
/// This test requires a valid OPENAI_API_KEY environment variable.
/// It verifies the complete request/response cycle with actual OpenAI embeddings.
///
/// Run with: cargo test integration_test_openai_real -- --ignored
#[tokio::test]
#[ignore = "requires valid OPENAI_API_KEY environment variable"]
async fn integration_test_openai_real() {
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set for this test");

    let config = EmbeddingConfig {
        provider: Provider::OpenAI,
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        cache: CacheConfig::default(),
        batch_size: 100,
        retry: RetryConfig::default(),
        api_key: Some(api_key),
        api_endpoint: None,
        parallel: ParallelConfig::default(),
    };

    let provider = OpenAIClient::new(config)
        .expect("Create OpenAI client");

    // Test single embedding
    let embedding = provider.embed("test text for embedding".to_string()).await
        .expect("Should generate embedding with real OpenAI");

    assert_eq!(embedding.len(), 1536, "Real OpenAI should return 1536-dim vectors");

    // Verify embedding has non-zero values
    let has_nonzero = embedding.iter().any(|&v| v != 0.0);
    assert!(has_nonzero, "Embedding should contain non-zero values");

    // Test batch embedding
    let texts = vec![
        "first text".to_string(),
        "second text".to_string()
    ];
    let embeddings = provider.embed_batch(texts.clone()).await
        .expect("Should generate batch embeddings");

    assert_eq!(embeddings.len(), texts.len(), "Batch should return all embeddings");
    for embedding in embeddings {
        assert_eq!(embedding.len(), 1536, "Each embedding should be 1536-dim");
    }
}

/// Integration test: Switching providers mid-session.
///
/// This test verifies that an application can switch between providers
/// dynamically without issues. Requires both Ollama and OpenAI to be available.
///
/// Run with: cargo test integration_test_provider_switching -- --ignored
#[tokio::test]
#[serial]
#[ignore = "requires both Ollama and OpenAI available"]
async fn integration_test_provider_switching() {
    // Start with Ollama
    std::env::set_var("EMBEDDING_PROVIDER", "ollama");
    let provider1 = create_provider_from_env().await
        .expect("Should create Ollama provider");

    let embedding1 = provider1.embed("test".to_string()).await
        .expect("Should embed with Ollama");
    assert_eq!(embedding1.len(), 768);

    // Switch to OpenAI
    std::env::set_var("EMBEDDING_PROVIDER", "openai");
    std::env::set_var("OPENAI_API_KEY",
        std::env::var("OPENAI_API_KEY").expect("Need OPENAI_API_KEY"));

    let provider2 = create_provider_from_env().await
        .expect("Should create OpenAI provider");

    let embedding2 = provider2.embed("test".to_string()).await
        .expect("Should embed with OpenAI");
    assert_eq!(embedding2.len(), 1536);

    // Cleanup
    std::env::remove_var("EMBEDDING_PROVIDER");
    std::env::remove_var("OPENAI_API_KEY");
}
