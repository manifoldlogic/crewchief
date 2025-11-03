# Ticket: PROVFIX-1002: Add Comprehensive Unit Tests for Endpoint Resolution

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose (or rust-specialist if available)
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add unit tests to verify the endpoint resolution bug fix from PROVFIX-1001. Tests must catch the original bug (OpenAI inheriting Ollama endpoint) and verify all provider-specific endpoint handling works correctly.

## Background
The endpoint resolution bug existed because there were no tests validating cross-provider endpoint behavior. This ticket adds the critical tests that would have caught the bug before production.

From `.agents/projects/PROVFIX_maproom-provider-fixes/planning/quality-strategy.md`:
- `test_openai_ignores_ollama_endpoint` - The bug test, prevents regression
- `test_openai_accepts_custom_openai_endpoint` - Proves override still works
- `test_ollama_uses_custom_endpoint` - Verifies Ollama not broken
- Provider-specific default tests for all providers

These tests provide confidence the fix works and prevent future regressions.

This is Phase 1, Ticket 2 of the PROVFIX implementation plan.

## Acceptance Criteria
- [ ] Test: OpenAI uses default endpoint when no env var set
- [ ] Test: OpenAI ignores Ollama endpoint in environment (THE BUG TEST)
- [ ] Test: OpenAI accepts custom OpenAI endpoint (domain matches)
- [ ] Test: Ollama uses custom endpoint from environment
- [ ] Test: Ollama uses default when no override
- [ ] Test: Google ignores EMBEDDING_API_ENDPOINT (uses region)
- [ ] Test: Cohere provider endpoint handling
- [ ] All tests pass with `cargo test config_tests`

## Technical Requirements

### 1. Add Test Module to config.rs

Add test module at end of `/workspace/crates/maproom/src/embedding/config.rs`:

```rust
#[cfg(test)]
mod config_tests {
    use super::*;
    use std::env;

    // Tests here
}
```

### 2. Test Structure

Each test must:
- Set up environment variables using `env::set_var()` and `env::remove_var()`
- Create config via `EmbeddingConfig::from_env()`
- Assert `config.api_endpoint_url()` returns expected URL
- Clean up environment after test

### 3. Critical Test (The Bug)

This is the most important test - it must catch the original bug:

```rust
#[test]
fn test_openai_ignores_ollama_endpoint() {
    // Set up: Ollama endpoint in environment (like Docker Compose default)
    env::set_var("EMBEDDING_API_ENDPOINT", "http://localhost:11434/api/embed");
    env::set_var("EMBEDDING_PROVIDER", "openai");
    env::remove_var("OPENAI_API_KEY"); // Use default or mock

    let config = EmbeddingConfig::from_env().unwrap();

    // Assert: OpenAI should use its default, NOT the Ollama endpoint
    assert_eq!(
        config.api_endpoint_url(),
        "https://api.openai.com/v1/embeddings"
    );

    // Cleanup
    env::remove_var("EMBEDDING_API_ENDPOINT");
    env::remove_var("EMBEDDING_PROVIDER");
}
```

### 4. Required Test Coverage

**OpenAI Provider (3 tests)**:
1. `test_openai_uses_default_endpoint` - No env var set
2. `test_openai_ignores_ollama_endpoint` - Wrong provider endpoint
3. `test_openai_accepts_custom_openai_endpoint` - Valid custom endpoint

**Cohere Provider (2 tests)**:
1. `test_cohere_uses_default_endpoint` - No env var set
2. `test_cohere_ignores_wrong_endpoint` - Wrong provider endpoint

**Ollama Provider (2 tests)**:
1. `test_ollama_uses_custom_endpoint` - Custom endpoint from env
2. `test_ollama_uses_default_if_no_override` - No env var set

**Google Provider (1 test)**:
1. `test_google_ignores_embedding_api_endpoint` - Always uses region

### 5. Test Isolation

If tests interfere with each other, run serially:
```bash
cargo test config_tests -- --test-threads=1
```

## Implementation Notes

### Testing Approach
- Use Rust's built-in test framework (`cargo test`)
- Tests are inline in same file as implementation (config.rs)
- Each test is isolated (set/remove env vars)
- Tests verify both positive (works) and negative (rejects) cases

### Example Test Template

```rust
#[test]
fn test_provider_scenario() {
    // Setup environment
    env::set_var("EMBEDDING_PROVIDER", "provider_name");
    env::set_var("EMBEDDING_API_ENDPOINT", "custom_endpoint");

    // Create config
    let config = EmbeddingConfig::from_env().unwrap();

    // Verify behavior
    assert_eq!(config.api_endpoint_url(), "expected_endpoint");

    // Cleanup
    env::remove_var("EMBEDDING_PROVIDER");
    env::remove_var("EMBEDDING_API_ENDPOINT");
}
```

### Key Test Scenarios

1. **Default behavior**: Provider uses its default endpoint when no override
2. **Wrong provider endpoint**: Provider ignores endpoints from other providers
3. **Valid custom endpoint**: Provider accepts custom endpoints with matching domain
4. **Google special case**: Google ignores `EMBEDDING_API_ENDPOINT` entirely

### Success Criteria

Test output should show:
```
running 8 tests
test config_tests::test_openai_ignores_ollama_endpoint ... ok
test config_tests::test_openai_uses_default_endpoint ... ok
test config_tests::test_openai_accepts_custom_openai_endpoint ... ok
test config_tests::test_ollama_uses_custom_endpoint ... ok
test config_tests::test_ollama_uses_default_if_no_override ... ok
test config_tests::test_google_ignores_embedding_api_endpoint ... ok
test config_tests::test_cohere_uses_default_endpoint ... ok
test config_tests::test_cohere_ignores_wrong_endpoint ... ok

test result: ok. 8 passed; 0 failed
```

## Dependencies
- **Requires**: PROVFIX-1001 (endpoint resolution fix) must complete first
- **Blocks**: PROVFIX-3001 (can't remove CLI workaround until tests prove fix works)

## Risk Assessment
- **Risk**: Tests interfere with each other via shared environment
  - **Mitigation**: Use `env::set_var()` and `env::remove_var()` for isolation; run tests serially if needed with `cargo test -- --test-threads=1`

- **Risk**: Tests don't catch all edge cases
  - **Mitigation**: Focus on critical paths that prevent the actual bug; avoid over-testing edge cases that don't matter for MVP

- **Risk**: Environment variable state pollution between tests
  - **Mitigation**: Explicit cleanup in each test; document that tests may need `--test-threads=1` if parallel execution causes issues

## Files/Packages Affected
- `/workspace/crates/maproom/src/embedding/config.rs` (add test module at end of file)

## Planning References
- Quality Strategy: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/quality-strategy.md`
  - Section: "1. Rust Configuration Loading (Unit Tests)"
- Plan: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/plan.md`
  - Phase 1, Ticket 2
