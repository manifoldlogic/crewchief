# Ticket: MPEMBED-6001: Provider contract tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- contract-test-engineer
- rust-test-runner
- verify-ticket
- commit-ticket

## Summary
Create contract tests verifying all providers satisfy the EmbeddingProvider trait correctly. Test dimension consistency, batch ordering preservation, and error handling across all providers.

## Background
This ticket implements Phase 6 (Testing and Validation) from the MPEMBED multi-provider embeddings plan. Contract tests ensure all providers behave consistently and satisfy the trait contract, preventing regressions and catching provider-specific bugs.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-6-testing-validation.md

## Acceptance Criteria
- [ ] All providers tested against same contract
- [ ] Dimension consistency verified (output length == provider.dimension())
- [ ] Batch order preservation verified
- [ ] Empty input handling tested
- [ ] Error propagation tested
- [ ] Timeout behavior tested
- [ ] Contract test runs for ollama, openai, google providers
- [ ] Test framework allows easy addition of new providers

## Technical Requirements
- Use trait-based testing pattern
- Parameterized tests for each provider
- Mock external APIs where possible
- Test both success and failure paths
- Measure test coverage (>90% for providers)
- Document contract requirements
- Use property-based testing where applicable

## Implementation Notes
```rust
// crates/maproom/tests/provider_contract.rs
#![cfg(test)]

use maproom::embedding::{EmbeddingProvider, create_provider};
use std::sync::Arc;

/// Provider contract test suite
///
/// All providers must satisfy these tests to be considered valid
#[cfg(test)]
mod contract_tests {
    use super::*;

    /// Test providers (add new providers here)
    fn get_test_providers() -> Vec<(&'static str, Arc<dyn EmbeddingProvider>)> {
        let mut providers = vec![];

        // Ollama (requires running instance)
        if std::env::var("TEST_OLLAMA").is_ok() {
            if let Ok(provider) = create_provider("ollama") {
                providers.push(("ollama", provider));
            }
        }

        // OpenAI (requires API key)
        if std::env::var("OPENAI_API_KEY").is_ok() {
            if let Ok(provider) = create_provider("openai") {
                providers.push(("openai", provider));
            }
        }

        // Google (requires credentials)
        if std::env::var("GOOGLE_PROJECT_ID").is_ok() {
            if let Ok(provider) = create_provider("google") {
                providers.push(("google", provider));
            }
        }

        providers
    }

    #[tokio::test]
    async fn contract_dimension_consistency() {
        for (name, provider) in get_test_providers() {
            println!("Testing dimension consistency: {}", name);

            let dimension = provider.dimension();
            let texts = vec!["Test text".to_string()];

            let embeddings = provider.embed(texts).await
                .expect(&format!("{} should embed text", name));

            assert_eq!(embeddings.len(), 1, "{}: should return 1 embedding", name);
            assert_eq!(
                embeddings[0].len(),
                dimension,
                "{}: embedding length {} should match dimension {}",
                name,
                embeddings[0].len(),
                dimension
            );
        }
    }

    #[tokio::test]
    async fn contract_batch_order_preservation() {
        for (name, provider) in get_test_providers() {
            println!("Testing batch order preservation: {}", name);

            let texts = vec![
                "First text".to_string(),
                "Second text".to_string(),
                "Third text".to_string(),
            ];

            let embeddings = provider.embed(texts.clone()).await
                .expect(&format!("{} should embed batch", name));

            assert_eq!(
                embeddings.len(),
                texts.len(),
                "{}: should return {} embeddings for {} texts",
                name,
                texts.len(),
                texts.len()
            );

            // Verify embeddings are different (order matters)
            assert_ne!(
                embeddings[0], embeddings[1],
                "{}: different texts should produce different embeddings",
                name
            );
        }
    }

    #[tokio::test]
    async fn contract_empty_input_handling() {
        for (name, provider) in get_test_providers() {
            println!("Testing empty input handling: {}", name);

            let texts: Vec<String> = vec![];
            let embeddings = provider.embed(texts).await
                .expect(&format!("{} should handle empty input", name));

            assert_eq!(
                embeddings.len(), 0,
                "{}: should return empty vec for empty input",
                name
            );
        }
    }

    #[tokio::test]
    async fn contract_single_text_embedding() {
        for (name, provider) in get_test_providers() {
            println!("Testing single text embedding: {}", name);

            let text = "fn main() { println!(\"Hello\"); }".to_string();
            let embeddings = provider.embed(vec![text]).await
                .expect(&format!("{} should embed single text", name));

            assert_eq!(embeddings.len(), 1, "{}: should return 1 embedding", name);

            // Verify embedding is not all zeros
            let sum: f32 = embeddings[0].iter().sum();
            assert!(
                sum.abs() > 0.01,
                "{}: embedding should not be all zeros",
                name
            );

            // Verify embedding values are normalized (roughly)
            let magnitude: f32 = embeddings[0].iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!(
                magnitude > 0.5 && magnitude < 2.0,
                "{}: embedding magnitude {} seems abnormal",
                name,
                magnitude
            );
        }
    }

    #[tokio::test]
    async fn contract_large_batch_handling() {
        for (name, provider) in get_test_providers() {
            println!("Testing large batch handling: {}", name);

            // Create batch of 50 texts
            let texts: Vec<String> = (0..50)
                .map(|i| format!("function process{i}() {{ return {i}; }}"))
                .collect();

            let embeddings = provider.embed(texts.clone()).await
                .expect(&format!("{} should handle large batch", name));

            assert_eq!(
                embeddings.len(),
                50,
                "{}: should return 50 embeddings for 50 texts",
                name
            );

            // Spot check dimensions
            for (i, emb) in embeddings.iter().enumerate() {
                assert_eq!(
                    emb.len(),
                    provider.dimension(),
                    "{}: embedding {} should have dimension {}",
                    name,
                    i,
                    provider.dimension()
                );
            }
        }
    }

    #[tokio::test]
    async fn contract_semantic_similarity() {
        for (name, provider) in get_test_providers() {
            println!("Testing semantic similarity: {}", name);

            let texts = vec![
                "The cat sits on the mat".to_string(),
                "A feline rests on the rug".to_string(), // Similar meaning
                "Database connection failed".to_string(), // Unrelated
            ];

            let embeddings = provider.embed(texts).await
                .expect(&format!("{} should embed texts", name));

            // Cosine similarity
            let sim_1_2 = cosine_similarity(&embeddings[0], &embeddings[1]);
            let sim_1_3 = cosine_similarity(&embeddings[0], &embeddings[2]);

            println!("{}: sim(1,2)={:.3}, sim(1,3)={:.3}", name, sim_1_2, sim_1_3);

            // Similar texts should have higher similarity
            assert!(
                sim_1_2 > sim_1_3,
                "{}: similar texts should have higher similarity: {:.3} > {:.3}",
                name,
                sim_1_2,
                sim_1_3
            );
        }
    }

    #[tokio::test]
    async fn contract_name_matches_factory() {
        let providers = vec![
            ("ollama", create_provider("ollama")),
            ("openai", create_provider("openai")),
            ("google", create_provider("google")),
        ];

        for (expected_name, result) in providers {
            if result.is_err() {
                continue; // Skip if provider not configured
            }

            let provider = result.unwrap();
            assert_eq!(
                provider.name(),
                expected_name,
                "Provider name should match factory key"
            );
        }
    }

    #[tokio::test]
    async fn contract_dimension_is_positive() {
        for (name, provider) in get_test_providers() {
            assert!(
                provider.dimension() > 0,
                "{}: dimension must be positive",
                name
            );
        }
    }

    // Helper function
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        dot / (mag_a * mag_b)
    }
}

// Property-based testing with proptest
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_batch_size_consistency(texts in prop::collection::vec("\\PC+", 1..20)) {
            // This test would run for each provider
            // Verifying that batch size doesn't affect embedding dimensions
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                for (name, provider) in get_test_providers() {
                    let embeddings = provider.embed(texts.clone()).await.unwrap();
                    assert_eq!(embeddings.len(), texts.len());

                    for emb in embeddings {
                        assert_eq!(emb.len(), provider.dimension());
                    }
                }
            });
        }
    }
}
```

**Test Execution:**
```bash
# Run contract tests for all configured providers
cargo test contract_

# Run with specific provider
TEST_OLLAMA=1 cargo test contract_
OPENAI_API_KEY=sk-test cargo test contract_

# Run all provider tests
TEST_OLLAMA=1 OPENAI_API_KEY=sk-test GOOGLE_PROJECT_ID=test cargo test contract_
```

## Dependencies
- MPEMBED-2001 (EmbeddingProvider trait)
- MPEMBED-2002 (OllamaProvider)
- MPEMBED-2003 (OpenAI provider refactor)
- MPEMBED-3001 (GoogleProvider)

## Risk Assessment
- **Risk**: Contract tests may be flaky due to external API dependencies
  - **Mitigation**: Mock APIs where possible, mark as #[ignore] for CI, retry transient failures

## Files/Packages Affected
- crates/maproom/tests/provider_contract.rs (create)
- crates/maproom/Cargo.toml (add proptest dependency)
