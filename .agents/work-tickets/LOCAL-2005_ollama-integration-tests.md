# Ticket: LOCAL-2005: Add integration tests for Ollama provider

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive integration tests that validate Ollama connectivity, embedding generation, and error handling with a real Ollama server running nomic-embed-text. These tests provide end-to-end confidence in the Ollama provider implementation.

## Background
With the Ollama provider implementation complete (LOCAL-2004), we need integration tests to verify the provider works correctly against a real Ollama server. Unlike unit tests that mock external dependencies, these integration tests will validate:

1. Actual HTTP communication with Ollama
2. Real embedding generation with nomic-embed-text model
3. Error handling with real network failures
4. Performance characteristics under realistic conditions

These tests are critical for verifying the integration works end-to-end before moving to the indexing pipeline (LOCAL-2006). They provide confidence that the provider behaves correctly in production-like conditions.

## Acceptance Criteria
- [ ] Integration test file created at `tests/integration/ollama_test.rs`
- [ ] Test: Single embedding generation succeeds with 768-dimension vector
- [ ] Test: Batch embedding (3+ items) returns correct number of embeddings
- [ ] Test: All embeddings have correct dimensions (768) and non-zero values
- [ ] Test: Error handling for invalid model name shows appropriate error message
- [ ] Test: Error handling for unreachable endpoint shows appropriate error message
- [ ] All tests pass when Ollama container is running with nomic-embed-text
- [ ] Tests skip gracefully with warning if Ollama is unavailable

## Technical Requirements

### Test Environment Setup
- Ollama container must be running from docker-compose stack
- nomic-embed-text model must be pulled and available
- Tests can use `localhost:11434` or `http://ollama:11434` endpoints
- Use conditional test execution to skip if Ollama unavailable

### Test Cases Required

1. **Basic Embedding Generation** (`test_single_embedding_generation`):
   - Create `EmbeddingService` with Ollama config
   - Generate embedding for single code snippet
   - Verify 768-dimension vector returned
   - Verify non-zero values in vector

2. **Batch Processing** (`test_batch_embedding_generation`):
   - Generate embeddings for 3+ text samples
   - Verify batch size handling
   - Verify correct number of embeddings returned
   - Verify all embeddings have correct dimensions

3. **Error Handling - Invalid Model** (`test_invalid_model_error`):
   - Configure provider with non-existent model name
   - Attempt embedding generation
   - Verify appropriate error message

4. **Error Handling - Unreachable Endpoint** (`test_unreachable_endpoint_error`):
   - Configure provider with invalid endpoint URL
   - Attempt embedding generation
   - Verify appropriate error message

5. **Performance Baseline** (`test_batch_performance`):
   - Measure time for 50-chunk batch
   - Compare to expected throughput (500-1000 chunks/min)
   - Log performance metrics for monitoring

### Test Framework
- Use tokio test framework: `#[tokio::test]`
- Follow Rust integration testing conventions
- Place tests in `tests/integration/ollama_test.rs`
- Use helper functions for common setup (creating test config, etc.)

## Implementation Notes

### Test File Structure
```rust
// tests/integration/ollama_test.rs
use maproom::embedding::service::EmbeddingService;
use maproom::embedding::config::EmbeddingConfig;

// Helper to check if Ollama is available
async fn ollama_available() -> bool {
    // Check http://localhost:11434/api/tags
}

// Helper to create test config
fn test_config() -> EmbeddingConfig {
    // Return Ollama config with localhost:11434
}

#[tokio::test]
async fn test_single_embedding_generation() {
    if !ollama_available().await {
        eprintln!("WARNING: Skipping test - Ollama not available");
        return;
    }
    // Test implementation
}

// Additional tests...
```

### Environment Detection
Tests should detect Ollama availability by:
1. Attempting connection to `localhost:11434/api/tags`
2. Verifying nomic-embed-text model is present
3. Gracefully skipping tests if unavailable with clear warning

### Performance Expectations
- Single embedding: <100ms
- 50-chunk batch: 3-6 seconds (500-1000 chunks/min)
- Log actual metrics for baseline establishment

### Error Message Validation
- Invalid model: Should contain "model not found" or similar
- Unreachable endpoint: Should contain connection/network error
- Use pattern matching on error types, not exact string matching

## Dependencies
- **LOCAL-2004**: Ollama provider request formatting must be complete
- Ollama container running from docker-compose (LOCAL-1003)
- nomic-embed-text model provisioned (LOCAL-1004)

## Risk Assessment
- **Risk**: Tests may be flaky if Ollama server is slow or unavailable
  - **Mitigation**: Implement timeout handling and graceful skipping with warnings

- **Risk**: Performance baselines may vary by hardware
  - **Mitigation**: Use wide tolerance ranges (500-1000 chunks/min) and log actual metrics

- **Risk**: Integration tests may be slower than unit tests
  - **Mitigation**: Keep test suite focused (5 tests), use `cargo test --test ollama_test` to run individually

- **Risk**: CI/CD environment may not have Ollama available
  - **Mitigation**: Tests skip gracefully; consider separate integration test job in CI

## Files/Packages Affected
- `tests/integration/ollama_test.rs` (new file)
- `Cargo.toml` (may need test dependencies like `tokio-test`)
- `tests/integration/mod.rs` (if module organization needed)

## Reference Documentation
- Testing strategy defined in LOCAL_ARCHITECTURE.md lines 897-939
- Rust integration testing: https://doc.rust-lang.org/book/ch11-03-test-organization.html
- tokio test framework: https://docs.rs/tokio/latest/tokio/attr.test.html
- Planning doc: /workspace/crewchief_context/maproom/LOCAL/LOCAL_PLAN.md
- Architecture doc: /workspace/crewchief_context/maproom/LOCAL/LOCAL_ARCHITECTURE.md
