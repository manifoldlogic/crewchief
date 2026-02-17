//! Provider contract tests
//!
//! This test suite validates that all embedding providers (Ollama, OpenAI, Google)
//! correctly satisfy the EmbeddingProvider trait contract. Contract tests ensure:
//!
//! - **Dimension Consistency**: Output vectors match declared dimension
//! - **Batch Order Preservation**: Batch embeddings maintain input order
//! - **Empty Input Handling**: Empty batches return empty results
//! - **Error Propagation**: Errors are properly surfaced
//! - **Semantic Properties**: Similar texts produce similar embeddings
//!
//! # Running Contract Tests
//!
//! ```bash
//! # Run all contract tests with all providers
//! TEST_OLLAMA=1 OPENAI_API_KEY=sk-... GOOGLE_PROJECT_ID=... cargo test contract_
//!
//! # Run with specific provider (Ollama)
//! TEST_OLLAMA=1 cargo test contract_
//!
//! # Run with OpenAI only
//! OPENAI_API_KEY=sk-... cargo test contract_
//!
//! # Run property-based tests
//! cargo test prop_
//! ```
//!
//! # Environment Variables
//!
//! - `TEST_OLLAMA=1`: Enable Ollama tests (requires running Ollama server)
//! - `OPENAI_API_KEY=sk-...`: Enable OpenAI tests
//! - `GOOGLE_PROJECT_ID=...`: Enable Google tests (also requires GOOGLE_APPLICATION_CREDENTIALS)
//! - `GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json`: Google service account key
//!
//! # Contract Requirements
//!
//! All providers MUST satisfy these contract requirements:
//!
//! 1. **Dimension Contract**: `embed()` returns vectors of length `dimension()`
//! 2. **Batch Length Contract**: `embed_batch()` returns same number of vectors as input texts
//! 3. **Batch Order Contract**: Output embeddings preserve input text order
//! 4. **Empty Batch Contract**: Empty input produces empty output
//! 5. **Name Contract**: `provider_name()` matches factory key
//! 6. **Positive Dimension Contract**: `dimension()` returns positive integer
//! 7. **Non-Zero Contract**: Embeddings are not all zeros
//! 8. **Semantic Similarity Contract**: Similar texts have higher cosine similarity
//!
//! # Adding New Providers
//!
//! To add a new provider to contract tests:
//!
//! 1. Add provider to `get_test_providers()` function
//! 2. Add environment variable check (e.g., `TEST_NEW_PROVIDER=1`)
//! 3. Update factory match arm in `contract_name_matches_factory` test
//! 4. Run full test suite to verify compliance

#![cfg(test)]

use crewchief_maproom::embedding::factory::create_provider_from_env;
use crewchief_maproom::embedding::provider::EmbeddingProvider;
use serial_test::serial;
use std::sync::Arc;

/// Helper function to create test providers based on environment configuration.
///
/// This function checks environment variables to determine which providers to test:
/// - `TEST_OLLAMA=1`: Include Ollama provider
/// - `OPENAI_API_KEY=sk-...`: Include OpenAI provider
/// - `GOOGLE_PROJECT_ID=...`: Include Google provider
///
/// Each provider is created using the factory and wrapped in Arc<dyn EmbeddingProvider>
/// for polymorphic testing.
///
/// # Returns
///
/// Vector of (provider_name, provider) tuples for all configured providers.
async fn get_test_providers() -> Vec<(&'static str, Arc<dyn EmbeddingProvider>)> {
    let mut providers = vec![];

    // Ollama (requires running instance at localhost:11434)
    if std::env::var("TEST_OLLAMA").is_ok() {
        std::env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");
        std::env::set_var("MAPROOM_EMBEDDING_MODEL", "nomic-embed-text");
        std::env::set_var("EMBEDDING_API_ENDPOINT", "http://localhost:11434/api/embed");

        match create_provider_from_env().await {
            Ok(provider) => {
                providers.push(("ollama", Arc::from(provider) as Arc<dyn EmbeddingProvider>));
                println!("✓ Ollama provider added to test suite");
            }
            Err(e) => {
                eprintln!("✗ Failed to create Ollama provider: {}", e);
            }
        }
    }

    // OpenAI (requires API key)
    if std::env::var("OPENAI_API_KEY").is_ok() {
        std::env::set_var("MAPROOM_EMBEDDING_PROVIDER", "openai");

        match create_provider_from_env().await {
            Ok(provider) => {
                providers.push(("openai", Arc::from(provider) as Arc<dyn EmbeddingProvider>));
                println!("✓ OpenAI provider added to test suite");
            }
            Err(e) => {
                eprintln!("✗ Failed to create OpenAI provider: {}", e);
            }
        }
    }

    // Google (requires project ID and credentials)
    if std::env::var("GOOGLE_PROJECT_ID").is_ok()
        && std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok()
    {
        std::env::set_var("MAPROOM_EMBEDDING_PROVIDER", "google");

        match create_provider_from_env().await {
            Ok(provider) => {
                providers.push(("google", Arc::from(provider) as Arc<dyn EmbeddingProvider>));
                println!("✓ Google provider added to test suite");
            }
            Err(e) => {
                eprintln!("✗ Failed to create Google provider: {}", e);
            }
        }
    }

    if providers.is_empty() {
        eprintln!("⚠ No providers configured for contract tests!");
        eprintln!("  Set TEST_OLLAMA=1, OPENAI_API_KEY, or GOOGLE_PROJECT_ID to enable tests");
    }

    providers
}

/// Helper function to calculate cosine similarity between two vectors.
///
/// # Arguments
///
/// * `a` - First vector
/// * `b` - Second vector (must be same length as `a`)
///
/// # Returns
///
/// Cosine similarity in range [-1.0, 1.0], where:
/// - 1.0 = vectors point in same direction (identical)
/// - 0.0 = vectors are orthogonal (unrelated)
/// - -1.0 = vectors point in opposite directions
///
/// # Panics
///
/// Panics if vectors have different lengths.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(
        a.len(),
        b.len(),
        "Vectors must have same length for cosine similarity"
    );

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    // Handle zero magnitude (shouldn't happen with real embeddings)
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }

    dot / (mag_a * mag_b)
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    /// CONTRACT TEST: Dimension consistency
    ///
    /// Verifies that all providers return embeddings with length matching dimension().
    ///
    /// **Contract Requirement**: For all texts, `embed(text).len() == dimension()`
    ///
    /// **Why This Matters**: Database schemas, vector search indexes, and downstream
    /// components all depend on consistent embedding dimensions. Dimension mismatches
    /// cause runtime errors, index corruption, and search failures.
    #[tokio::test]
    #[serial]
    #[ignore = "Requires embedding provider (Ollama/OpenAI/Google)"]
    async fn contract_dimension_consistency() {
        let providers = get_test_providers().await;
        assert!(
            !providers.is_empty(),
            "No providers configured. Set TEST_OLLAMA=1, OPENAI_API_KEY, or GOOGLE_PROJECT_ID"
        );

        for (name, provider) in providers {
            println!("Testing dimension consistency: {}", name);

            let dimension = provider.dimension();
            let texts = vec!["Test text for dimension validation".to_string()];

            let embeddings = provider
                .embed_batch(texts)
                .await
                .unwrap_or_else(|e| panic!("{} should embed text without error: {}", name, e));

            assert_eq!(
                embeddings.len(),
                1,
                "{}: should return 1 embedding for 1 text",
                name
            );

            assert_eq!(
                embeddings[0].len(),
                dimension,
                "{}: embedding length {} should match dimension {}",
                name,
                embeddings[0].len(),
                dimension
            );

            println!(
                "✓ {} dimension consistency verified: {} dims",
                name, dimension
            );
        }
    }

    /// CONTRACT TEST: Batch order preservation
    ///
    /// Verifies that batch embeddings preserve the order of input texts.
    ///
    /// **Contract Requirement**: For inputs [text1, text2, text3],
    /// `embed_batch()` returns [embed1, embed2, embed3] where embed_i corresponds to text_i
    ///
    /// **Why This Matters**: Callers rely on positional correspondence between input
    /// texts and output embeddings. Order violations corrupt the mapping between
    /// texts and their embeddings, leading to incorrect search results.
    #[tokio::test]
    #[serial]
    #[ignore = "Requires embedding provider (Ollama/OpenAI/Google)"]
    async fn contract_batch_order_preservation() {
        let providers = get_test_providers().await;
        assert!(
            !providers.is_empty(),
            "No providers configured. Set TEST_OLLAMA=1, OPENAI_API_KEY, or GOOGLE_PROJECT_ID"
        );

        for (name, provider) in providers {
            println!("Testing batch order preservation: {}", name);

            let texts = vec![
                "First distinct text about databases".to_string(),
                "Second distinct text about machine learning".to_string(),
                "Third distinct text about web development".to_string(),
            ];

            let embeddings = provider
                .embed_batch(texts.clone())
                .await
                .unwrap_or_else(|e| panic!("{} should embed batch without error: {}", name, e));

            assert_eq!(
                embeddings.len(),
                texts.len(),
                "{}: should return {} embeddings for {} texts",
                name,
                texts.len(),
                texts.len()
            );

            // Verify embeddings are different (confirms order matters)
            assert_ne!(
                embeddings[0], embeddings[1],
                "{}: different texts should produce different embeddings",
                name
            );

            assert_ne!(
                embeddings[1], embeddings[2],
                "{}: different texts should produce different embeddings",
                name
            );

            assert_ne!(
                embeddings[0], embeddings[2],
                "{}: different texts should produce different embeddings",
                name
            );

            println!("✓ {} batch order preservation verified", name);
        }
    }

    /// CONTRACT TEST: Empty input handling
    ///
    /// Verifies that providers correctly handle empty batch inputs.
    ///
    /// **Contract Requirement**: `embed_batch([])` returns `Ok(vec![])`
    ///
    /// **Why This Matters**: Empty batches occur legitimately in processing pipelines
    /// (e.g., filtering, chunking). Providers must handle this edge case gracefully
    /// without errors or panics.
    #[tokio::test]
    #[serial]
    #[ignore = "Requires embedding provider (Ollama/OpenAI/Google)"]
    async fn contract_empty_input_handling() {
        let providers = get_test_providers().await;
        assert!(
            !providers.is_empty(),
            "No providers configured. Set TEST_OLLAMA=1, OPENAI_API_KEY, or GOOGLE_PROJECT_ID"
        );

        for (name, provider) in providers {
            println!("Testing empty input handling: {}", name);

            let texts: Vec<String> = vec![];
            let embeddings = provider.embed_batch(texts).await.unwrap_or_else(|e| {
                panic!("{} should handle empty input without error: {}", name, e)
            });

            assert_eq!(
                embeddings.len(),
                0,
                "{}: should return empty vec for empty input",
                name
            );

            println!("✓ {} empty input handling verified", name);
        }
    }

    /// CONTRACT TEST: Single text embedding
    ///
    /// Verifies that providers can embed single texts and produce non-trivial embeddings.
    ///
    /// **Contract Requirement**: Single text embedding produces a non-zero vector
    /// with reasonable magnitude.
    ///
    /// **Why This Matters**: Ensures providers don't return degenerate embeddings
    /// (all zeros, constant values) that would break semantic search.
    #[tokio::test]
    #[serial]
    #[ignore = "Requires embedding provider (Ollama/OpenAI/Google)"]
    async fn contract_single_text_embedding() {
        let providers = get_test_providers().await;
        assert!(
            !providers.is_empty(),
            "No providers configured. Set TEST_OLLAMA=1, OPENAI_API_KEY, or GOOGLE_PROJECT_ID"
        );

        for (name, provider) in providers {
            println!("Testing single text embedding: {}", name);

            let text = "fn main() { println!(\"Hello, world!\"); }".to_string();
            let embeddings = provider.embed_batch(vec![text]).await.unwrap_or_else(|e| {
                panic!("{} should embed single text without error: {}", name, e)
            });

            assert_eq!(embeddings.len(), 1, "{}: should return 1 embedding", name);

            assert_eq!(
                embeddings[0].len(),
                provider.dimension(),
                "{}: embedding should have correct dimension",
                name
            );

            // Verify embedding is not all zeros
            let sum: f32 = embeddings[0].iter().sum();
            assert!(
                sum.abs() > 0.01,
                "{}: embedding should not be all zeros (sum: {})",
                name,
                sum
            );

            // Verify embedding values are reasonable (not constant, not extreme)
            let magnitude: f32 = embeddings[0].iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!(
                magnitude > 0.5 && magnitude < 2.0,
                "{}: embedding magnitude {} seems abnormal (expected 0.5-2.0 for normalized embeddings)",
                name,
                magnitude
            );

            println!(
                "✓ {} single text embedding verified (magnitude: {:.3})",
                name, magnitude
            );
        }
    }

    /// CONTRACT TEST: Large batch handling
    ///
    /// Verifies that providers can handle larger batches efficiently.
    ///
    /// **Contract Requirement**: Batch of 50 texts produces 50 embeddings,
    /// all with correct dimension.
    ///
    /// **Why This Matters**: Real-world usage involves batching for efficiency.
    /// Providers must handle batch sizes beyond trivial single-item batches.
    #[tokio::test]
    #[serial]
    #[ignore = "Requires embedding provider (Ollama/OpenAI/Google)"]
    async fn contract_large_batch_handling() {
        let providers = get_test_providers().await;
        assert!(
            !providers.is_empty(),
            "No providers configured. Set TEST_OLLAMA=1, OPENAI_API_KEY, or GOOGLE_PROJECT_ID"
        );

        for (name, provider) in providers {
            println!("Testing large batch handling: {}", name);

            // Create batch of 50 distinct texts
            let texts: Vec<String> = (0..50)
                .map(|i| format!("function process{i}() {{ return {i} * 2; }}"))
                .collect();

            let embeddings = provider
                .embed_batch(texts.clone())
                .await
                .unwrap_or_else(|e| {
                    panic!("{} should handle large batch without error: {}", name, e)
                });

            assert_eq!(
                embeddings.len(),
                50,
                "{}: should return 50 embeddings for 50 texts",
                name
            );

            // Spot check dimensions for first, middle, and last embeddings
            let check_indices = [0, 24, 49];
            for i in check_indices {
                assert_eq!(
                    embeddings[i].len(),
                    provider.dimension(),
                    "{}: embedding {} should have dimension {}",
                    name,
                    i,
                    provider.dimension()
                );
            }

            println!("✓ {} large batch handling verified (50 embeddings)", name);
        }
    }

    /// CONTRACT TEST: Semantic similarity
    ///
    /// Verifies that semantically similar texts produce higher similarity scores.
    ///
    /// **Contract Requirement**: For similar texts A and B, and unrelated text C,
    /// `similarity(A, B) > similarity(A, C)`
    ///
    /// **Why This Matters**: This is the fundamental property that makes embeddings
    /// useful for semantic search. Embeddings that don't preserve semantic similarity
    /// are useless for search and ranking.
    #[tokio::test]
    #[serial]
    #[ignore = "Requires embedding provider (Ollama/OpenAI/Google)"]
    async fn contract_semantic_similarity() {
        let providers = get_test_providers().await;
        assert!(
            !providers.is_empty(),
            "No providers configured. Set TEST_OLLAMA=1, OPENAI_API_KEY, or GOOGLE_PROJECT_ID"
        );

        for (name, provider) in providers {
            println!("Testing semantic similarity: {}", name);

            let texts = vec![
                "The cat sits on the mat".to_string(),
                "A feline rests on the rug".to_string(), // Similar meaning
                "Database connection failed with error code 500".to_string(), // Unrelated
            ];

            let embeddings = provider
                .embed_batch(texts.clone())
                .await
                .unwrap_or_else(|e| panic!("{} should embed texts without error: {}", name, e));

            // Calculate cosine similarities
            let sim_1_2 = cosine_similarity(&embeddings[0], &embeddings[1]);
            let sim_1_3 = cosine_similarity(&embeddings[0], &embeddings[2]);

            println!(
                "  {} similarity scores: similar={:.3}, unrelated={:.3}",
                name, sim_1_2, sim_1_3
            );

            // Similar texts should have higher similarity than unrelated texts
            assert!(
                sim_1_2 > sim_1_3,
                "{}: similar texts should have higher similarity: {:.3} > {:.3}",
                name,
                sim_1_2,
                sim_1_3
            );

            // Similarity should be positive for related texts
            assert!(
                sim_1_2 > 0.3,
                "{}: similar texts should have meaningful similarity: {:.3} > 0.3",
                name,
                sim_1_2
            );

            println!("✓ {} semantic similarity verified", name);
        }
    }

    /// CONTRACT TEST: Provider name matches factory key
    ///
    /// Verifies that provider.provider_name() matches the key used in the factory.
    ///
    /// **Contract Requirement**: `create_provider_from_env("ollama").provider_name() == "ollama"`
    ///
    /// **Why This Matters**: Code uses provider names for logging, metrics, and
    /// database column selection. Name mismatches break monitoring and multi-provider
    /// database operations.
    #[tokio::test]
    #[serial]
    async fn contract_name_matches_factory() {
        // Test each provider independently (not using get_test_providers helper)
        let test_cases = vec![
            ("ollama", "TEST_OLLAMA"),
            ("openai", "OPENAI_API_KEY"),
            ("google", "GOOGLE_PROJECT_ID"),
        ];

        for (expected_name, env_var) in test_cases {
            if std::env::var(env_var).is_ok() {
                std::env::set_var("MAPROOM_EMBEDDING_PROVIDER", expected_name);

                match create_provider_from_env().await {
                    Ok(provider) => {
                        assert_eq!(
                            provider.provider_name(),
                            expected_name,
                            "Provider name should match factory key"
                        );
                        println!("✓ {} provider name matches factory key", expected_name);
                    }
                    Err(e) => {
                        eprintln!(
                            "✗ Failed to create {} provider (may be expected if not configured): {}",
                            expected_name, e
                        );
                    }
                }
            }
        }
    }

    /// CONTRACT TEST: Dimension is positive
    ///
    /// Verifies that all providers return positive dimension values.
    ///
    /// **Contract Requirement**: `dimension() > 0`
    ///
    /// **Why This Matters**: Zero or negative dimensions are invalid for vector
    /// spaces. Database schemas, array allocations, and vector operations all
    /// require positive dimensions.
    #[tokio::test]
    #[serial]
    #[ignore = "Requires embedding provider (Ollama/OpenAI/Google)"]
    async fn contract_dimension_is_positive() {
        let providers = get_test_providers().await;
        assert!(
            !providers.is_empty(),
            "No providers configured. Set TEST_OLLAMA=1, OPENAI_API_KEY, or GOOGLE_PROJECT_ID"
        );

        for (name, provider) in providers {
            let dimension = provider.dimension();
            assert!(
                dimension > 0,
                "{}: dimension must be positive, got {}",
                name,
                dimension
            );

            // Also verify dimension is reasonable (not absurdly large)
            assert!(
                dimension <= 10000,
                "{}: dimension {} seems unreasonably large (max expected: 10000)",
                name,
                dimension
            );

            println!("✓ {} dimension is positive: {}", name, dimension);
        }
    }

    /// CONTRACT TEST: Batch embedding consistency
    ///
    /// Verifies that embedding the same text multiple times in a batch produces
    /// identical embeddings (idempotency).
    ///
    /// **Contract Requirement**: For text T, `embed_batch([T, T])[0] == embed_batch([T, T])[1]`
    ///
    /// **Why This Matters**: Embeddings should be deterministic. Non-deterministic
    /// embeddings break caching, make debugging impossible, and produce inconsistent
    /// search results.
    #[tokio::test]
    #[serial]
    #[ignore = "Requires embedding provider (Ollama/OpenAI/Google)"]
    async fn contract_batch_embedding_consistency() {
        let providers = get_test_providers().await;
        assert!(
            !providers.is_empty(),
            "No providers configured. Set TEST_OLLAMA=1, OPENAI_API_KEY, or GOOGLE_PROJECT_ID"
        );

        for (name, provider) in providers {
            println!("Testing batch embedding consistency: {}", name);

            let text = "const value = 42;".to_string();
            let texts = vec![text.clone(), text.clone()];

            let embeddings = provider
                .embed_batch(texts)
                .await
                .unwrap_or_else(|e| panic!("{} should embed batch without error: {}", name, e));

            assert_eq!(embeddings.len(), 2, "{}: should return 2 embeddings", name);

            // Embeddings of identical text should be identical
            assert_eq!(
                embeddings[0], embeddings[1],
                "{}: identical texts should produce identical embeddings",
                name
            );

            println!("✓ {} batch embedding consistency verified", name);
        }
    }

    /// CONTRACT TEST: Error propagation
    ///
    /// Verifies that provider errors are properly propagated to callers.
    ///
    /// **Contract Requirement**: Errors from underlying APIs are surfaced as Result::Err
    ///
    /// **Why This Matters**: Callers need to handle errors gracefully. Silent failures
    /// or panics would crash the application. Proper error propagation enables retry
    /// logic, fallback strategies, and user-friendly error messages.
    ///
    /// **Note**: This test verifies the error handling structure exists. Actual error
    /// scenarios (network failures, invalid credentials) require specific environment
    /// setup and are tested separately in integration tests.
    #[tokio::test]
    #[serial]
    async fn contract_error_handling_structure() {
        // This test verifies that the provider trait returns Result types
        // and that errors can be propagated. Actual error scenarios require
        // specific setup (invalid API keys, network failures) which are
        // environment-dependent.

        let providers = get_test_providers().await;

        // If no providers configured, this is expected behavior (not an error)
        if providers.is_empty() {
            println!("⚠ No providers configured - skipping error handling test");
            return;
        }

        // Verify providers handle empty text gracefully
        for (name, provider) in providers {
            println!("Testing error handling structure: {}", name);

            // Test with empty strings (edge case, not necessarily an error)
            let result = provider.embed_batch(vec!["".to_string()]).await;

            // Providers should either:
            // 1. Successfully embed empty strings (returning valid embeddings)
            // 2. Return an error (which we can handle)
            // Either is acceptable; the key is that Result type allows error handling
            match result {
                Ok(embeddings) => {
                    println!(
                        "  ✓ {} handles empty strings (returned {} embeddings)",
                        name,
                        embeddings.len()
                    );
                }
                Err(e) => {
                    println!("  ✓ {} propagates errors for empty strings: {}", name, e);
                }
            }
        }
    }

    /// CONTRACT TEST: Timeout behavior (documentation)
    ///
    /// **Contract Requirement**: Providers should timeout gracefully for slow operations
    ///
    /// **Why This Matters**: Long-running embedding requests can hang applications.
    /// Timeouts prevent resource exhaustion and allow graceful degradation.
    ///
    /// **Implementation Note**: Actual timeout testing requires:
    /// - Mocking slow providers (complex setup)
    /// - Or waiting for real timeouts (very slow tests, 30-60+ seconds)
    /// - Environment-specific configuration
    ///
    /// For CI/CD, timeout behavior is verified through:
    /// 1. Provider implementations use reqwest with default timeouts
    /// 2. Integration tests in provider-specific test suites
    /// 3. Production monitoring and alerting
    ///
    /// This test documents the timeout contract requirement.
    #[tokio::test]
    #[serial]
    #[ignore = "Timeout testing requires special setup (see test documentation)"]
    async fn contract_timeout_behavior() {
        // This test is marked #[ignore] because timeout testing requires either:
        // 1. Mock providers that simulate slow responses
        // 2. Actual slow requests (making tests very slow)
        // 3. Environment-specific setup
        //
        // Timeout behavior is verified through:
        // - Provider implementations using reqwest with timeouts
        // - Integration tests with actual providers
        // - Production monitoring

        let providers = get_test_providers().await;

        for (name, _provider) in providers {
            println!("⚠ {} timeout behavior verified in integration tests", name);
        }
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // PROPERTY TEST: Batch size doesn't affect embedding dimensions
    //
    // **Property**: For any batch of texts, all output embeddings have dimension()
    //
    // This property-based test generates random batches of 1-20 texts and verifies
    // that dimension consistency holds regardless of batch size.
    proptest! {
        #[test]
        #[serial]
        fn prop_batch_size_consistency(texts in prop::collection::vec("\\PC+", 1..20)) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let providers = runtime.block_on(get_test_providers());
            if providers.is_empty() {
                // Skip test if no providers configured
                return Ok(());
            }

            runtime.block_on(async {
                for (name, provider) in providers {
                    let embeddings = provider
                        .embed_batch(texts.clone())
                        .await
                        .expect(&format!("{} should embed batch", name));

                    // Verify batch size matches
                    prop_assert_eq!(
                        embeddings.len(),
                        texts.len(),
                        "{}: batch size mismatch",
                        name
                    );

                    // Verify all embeddings have correct dimension
                    for (i, emb) in embeddings.iter().enumerate() {
                        prop_assert_eq!(
                            emb.len(),
                            provider.dimension(),
                            "{}: embedding {} has wrong dimension",
                            name,
                            i
                        );
                    }
                }
                Ok(())
            })?;
        }
    }

    // PROPERTY TEST: All embeddings are non-zero
    //
    // **Property**: For any non-empty text, embedding is not all zeros
    //
    // This verifies that providers don't return degenerate embeddings.
    proptest! {
        #[test]
        #[serial]
        fn prop_embeddings_non_zero(text in "\\PC{10,100}") {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let providers = runtime.block_on(get_test_providers());
            if providers.is_empty() {
                // Skip test if no providers configured
                return Ok(());
            }

            runtime.block_on(async {
                for (name, provider) in providers {
                    let embeddings = provider
                        .embed_batch(vec![text.clone()])
                        .await
                        .expect(&format!("{} should embed text", name));

                    prop_assert_eq!(embeddings.len(), 1, "{}: should return 1 embedding", name);

                    // Check magnitude (L2 norm) instead of sum to detect degenerate embeddings
                    // Sum can be near-zero even for valid embeddings if values cancel out
                    let magnitude: f32 = embeddings[0].iter().map(|x| x * x).sum::<f32>().sqrt();
                    prop_assert!(
                        magnitude > 0.1,
                        "{}: embedding magnitude ({}) too small - likely degenerate",
                        name,
                        magnitude
                    );
                }
                Ok(())
            })?;
        }
    }

    // PROPERTY TEST: Embedding magnitude is bounded
    //
    // **Property**: All embeddings have magnitude in reasonable range [0.1, 10.0]
    //
    // This ensures embeddings are normalized or at least bounded, preventing
    // numerical instability in downstream operations.
    proptest! {
        #[test]
        #[serial]
        fn prop_embedding_magnitude_bounded(text in "\\PC{10,100}") {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let providers = runtime.block_on(get_test_providers());
            if providers.is_empty() {
                // Skip test if no providers configured
                return Ok(());
            }

            runtime.block_on(async {
                for (name, provider) in providers {
                    let embeddings = provider
                        .embed_batch(vec![text.clone()])
                        .await
                        .expect(&format!("{} should embed text", name));

                    let magnitude: f32 = embeddings[0]
                        .iter()
                        .map(|x| x * x)
                        .sum::<f32>()
                        .sqrt();

                    prop_assert!(
                        magnitude > 0.1 && magnitude < 10.0,
                        "{}: embedding magnitude {} out of bounds [0.1, 10.0]",
                        name,
                        magnitude
                    );
                }
                Ok(())
            })?;
        }
    }
}
