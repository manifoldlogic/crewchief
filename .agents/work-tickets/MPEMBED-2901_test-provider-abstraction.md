# Ticket: MPEMBED-2901: Test provider abstraction with all providers

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- contract-test-engineer
- integration-tester
- test-runner
- verify-ticket
- commit-ticket

## Summary
Write unit and integration tests verifying all providers (Ollama, OpenAI) correctly implement the trait and can be swapped via factory.

## Background
It's critical to verify the trait contract is enforced across all providers. Tests must verify factory correctly selects providers based on configuration, that provider swap doesn't break embedding service, and dimension mismatch handling works correctly.

This ticket provides comprehensive test coverage for Phase 2: Provider Abstraction from the MPEMBED multi-provider embedding support plan.

## Acceptance Criteria
- [ ] Unit tests for each provider: `test_ollama_provider_embed()`, `test_openai_provider_embed()`
- [ ] Contract tests verify trait invariants (dimension matches output length)
- [ ] Factory test: auto-detection selects Ollama when available
- [ ] Factory test: explicit config overrides auto-detection
- [ ] Integration test: swap providers via env var, embeddings work
- [ ] Test error handling: provider unavailable, dimension mismatch
- [ ] All tests use mock HTTP server (no real API calls in unit tests)

## Technical Requirements
- File location: `crates/maproom/tests/provider_abstraction_test.rs` (NEW FILE)
- Use `wiremock` or similar for mocking Ollama HTTP API
- Use trait objects in tests: `Box<dyn EmbeddingProvider>`
- Test both success and failure paths
- Verify trait object Send + Sync (compile-time check)
- Mock server for Ollama API responses
- Integration tests marked with `#[ignore]` for real provider testing

## Implementation Notes

```rust
// crates/maproom/tests/provider_abstraction_test.rs (NEW FILE)

use crewchief_maproom::embedding::provider::EmbeddingProvider;
use crewchief_maproom::embedding::ollama::OllamaProvider;
use crewchief_maproom::embedding::openai::OpenAIClient;
use crewchief_maproom::embedding::factory::create_provider_from_env;

#[tokio::test]
async fn test_ollama_provider_implements_trait() {
    let provider: Box<dyn EmbeddingProvider> = Box::new(
        OllamaProvider::new(
            "http://localhost:11434/api/embed".to_string(),
            "nomic-embed-text".to_string()
        ).unwrap()
    );

    assert_eq!(provider.dimension(), 768);
    assert_eq!(provider.provider_name(), "ollama");
}

#[tokio::test]
async fn test_openai_provider_implements_trait() {
    // Mock OpenAIClient for testing
    let provider: Box<dyn EmbeddingProvider> = Box::new(
        OpenAIClient::new(config).unwrap()
    );

    assert_eq!(provider.dimension(), 1536);
    assert_eq!(provider.provider_name(), "openai");
}

#[tokio::test]
async fn test_trait_contract_dimension_matches_output() {
    // Test that embed() output length equals dimension()
    let provider: Box<dyn EmbeddingProvider> = create_test_provider();

    let embedding = provider.embed("test".to_string()).await.unwrap();
    assert_eq!(embedding.len(), provider.dimension());
}

#[tokio::test]
async fn test_trait_contract_batch_length_matches_input() {
    let provider: Box<dyn EmbeddingProvider> = create_test_provider();

    let texts = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let embeddings = provider.embed_batch(texts.clone()).await.unwrap();
    assert_eq!(embeddings.len(), texts.len());
}

#[tokio::test]
async fn test_factory_auto_detects_ollama() {
    // Start mock Ollama server on :11434
    let mock_server = setup_mock_ollama_server().await;
    std::env::remove_var("EMBEDDING_PROVIDER");

    let provider = create_provider_from_env().await.unwrap();
    assert_eq!(provider.provider_name(), "ollama");
}

#[tokio::test]
async fn test_factory_explicit_config_overrides_auto_detection() {
    std::env::set_var("EMBEDDING_PROVIDER", "openai");
    std::env::set_var("OPENAI_API_KEY", "test-key");

    let provider = create_provider_from_env().await.unwrap();
    assert_eq!(provider.provider_name(), "openai");
}

#[tokio::test]
async fn test_provider_swap() {
    // Test that swapping providers via env var works
    std::env::set_var("EMBEDDING_PROVIDER", "openai");
    let provider1 = create_provider_from_env().await.unwrap();
    assert_eq!(provider1.dimension(), 1536);

    std::env::set_var("EMBEDDING_PROVIDER", "ollama");
    let provider2 = create_provider_from_env().await.unwrap();
    assert_eq!(provider2.dimension(), 768);
}

#[tokio::test]
async fn test_error_handling_provider_unavailable() {
    std::env::set_var("EMBEDDING_PROVIDER", "ollama");
    std::env::set_var("EMBEDDING_API_ENDPOINT", "http://localhost:99999/api/embed");

    let result = create_provider_from_env().await;
    assert!(result.is_err());
}

// Helper functions
async fn setup_mock_ollama_server() -> MockServer {
    // Setup wiremock server
    todo!()
}

fn create_test_provider() -> Box<dyn EmbeddingProvider> {
    // Create provider for testing
    todo!()
}

// Integration tests with real providers (marked #[ignore])
#[tokio::test]
#[ignore = "requires real Ollama instance"]
async fn integration_test_ollama_real() {
    let provider = OllamaProvider::new(
        "http://localhost:11434/api/embed".to_string(),
        "nomic-embed-text".to_string()
    ).unwrap();

    let embedding = provider.embed("test".to_string()).await.unwrap();
    assert_eq!(embedding.len(), 768);
}
```

**Test Categories**:
1. **Unit Tests**: Mock all HTTP calls, fast execution
2. **Contract Tests**: Verify trait invariants hold
3. **Factory Tests**: Auto-detection and configuration logic
4. **Integration Tests**: Real providers (marked `#[ignore]`)

**Mock Server Strategy**:
- Use wiremock to mock Ollama API
- Mock successful responses with 768-dim vectors
- Mock error responses (timeout, 500, etc.)
- Verify request payloads

## Dependencies
- MPEMBED-2001 (trait definition)
- MPEMBED-2002 (Ollama provider)
- MPEMBED-2003 (OpenAI provider)
- MPEMBED-2004 (factory)
- MPEMBED-2005 (service refactor)

## Risk Assessment
- **Risk**: Tests pass but real providers fail (mocks don't match reality)
  - **Mitigation**: Also run integration tests with real providers (marked as `#[ignore]`)
- **Risk**: Mock server behavior diverges from real Ollama API
  - **Mitigation**: Regularly run integration tests against real Ollama instance
- **Risk**: Trait object testing doesn't catch compile-time errors
  - **Mitigation**: Include compile-time checks for Send + Sync bounds

## Files/Packages Affected
- crates/maproom/tests/provider_abstraction_test.rs (create)
- crates/maproom/tests/helpers/mock_server.rs (create if needed)
- crates/maproom/Cargo.toml (add wiremock dev-dependency if needed)
