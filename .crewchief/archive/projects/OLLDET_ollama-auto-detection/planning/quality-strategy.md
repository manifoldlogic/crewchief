# Quality Strategy: Ollama Auto-Detection

## Testing Approach

This is a focused change with clear behavior. Testing emphasizes:
1. Unit tests for URL extraction logic
2. Integration tests for fallback chain behavior
3. Manual verification in both native and container environments

## Unit Tests

### URL Extraction

```rust
#[test]
fn test_extract_base_url() {
    // Standard /api/embed suffix
    assert_eq!(
        extract_base_url("http://localhost:11434/api/embed"),
        Some("http://localhost:11434".to_string())
    );

    // Alternative /api/embeddings suffix
    assert_eq!(
        extract_base_url("http://host:8080/api/embeddings"),
        Some("http://host:8080".to_string())
    );

    // Custom host with path
    assert_eq!(
        extract_base_url("http://ollama.local:11434/api/embed"),
        Some("http://ollama.local:11434".to_string())
    );

    // Trailing slash handling
    assert_eq!(
        extract_base_url("http://localhost:11434/api/embed/"),
        Some("http://localhost:11434".to_string())
    );

    // No recognized suffix - returns None
    assert_eq!(
        extract_base_url("http://localhost:11434/custom"),
        None
    );

    // Empty string
    assert_eq!(extract_base_url(""), None);
}
```

### Fallback Order

```rust
#[tokio::test]
async fn test_detect_ollama_endpoint_order() {
    // This test verifies the order of fallbacks
    // When all fail, ensure all three are tried in order

    // Clear any env vars
    std::env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");

    // With no Ollama running, should return None
    // (Can't easily test successful detection without mock server)
    let result = detect_ollama_endpoint().await;
    // Just verify it doesn't panic and returns None when nothing is available
}
```

## Integration Tests

### Mock Server Testing

For reliable integration testing, use a mock HTTP server:

```rust
#[tokio::test]
async fn test_detect_with_custom_endpoint() {
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    // Start mock server
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"models": []})))
        .mount(&mock_server)
        .await;

    // Set env to point to mock
    std::env::set_var(
        "MAPROOM_EMBEDDING_API_ENDPOINT",
        format!("{}/api/embed", mock_server.uri())
    );

    let result = detect_ollama_endpoint().await;
    assert_eq!(result, Some(mock_server.uri()));

    std::env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");
}
```

### Existing Tests

The existing tests in `factory.rs` should continue to pass:
- `test_ollama_detection_timeout` - Verifies timeout behavior (may need adjustment - see below)
- `test_create_provider_with_explicit_ollama` - Explicit config works
- `test_create_provider_no_config_no_ollama` - Error when nothing available

**Note on `test_ollama_detection_timeout`:**
The current test expects detection to complete within 3 seconds. With the fallback chain trying up to 3 endpoints (2s timeout each), worst case is 6 seconds. Options:
1. Update test to allow 7 seconds (3 × 2s + 1s margin)
2. Use wiremock to mock endpoints and avoid real timeouts
3. Accept that test may be flaky if all endpoints timeout

Recommendation: Update timeout expectation to 7 seconds for simplicity.

## Manual Testing

### Native Environment (Ollama on localhost)

```bash
# 1. Ensure Ollama is running
ollama list

# 2. Clear any overrides
unset MAPROOM_EMBEDDING_PROVIDER
unset MAPROOM_EMBEDDING_API_ENDPOINT

# 3. Run scan - should auto-detect localhost
cargo run -p crewchief-maproom -- scan --path . --repo test

# 4. Verify log shows "Ollama detected at: http://localhost:11434"
```

### DevContainer Environment

```bash
# 1. Inside devcontainer, verify host access
curl http://host.docker.internal:11434/api/tags

# 2. Clear any overrides
unset MAPROOM_EMBEDDING_PROVIDER
unset MAPROOM_EMBEDDING_API_ENDPOINT

# 3. Run scan - should auto-detect host.docker.internal
cargo run -p crewchief-maproom -- scan --path . --repo test

# 4. Verify log shows "Ollama detected at: http://host.docker.internal:11434"
```

### Explicit Endpoint Override

```bash
# 1. Set custom endpoint
export MAPROOM_EMBEDDING_API_ENDPOINT=http://ollama.custom:11434/api/embed

# 2. Run scan (will fail if endpoint doesn't exist, but test fallback)
cargo run -p crewchief-maproom -- scan --path . --repo test

# 3. Verify log shows check for custom endpoint first
```

## Test Coverage Focus

| Component | Priority | Reason |
|-----------|----------|--------|
| `extract_base_url()` | High | Pure function, easy to test exhaustively |
| Fallback order | Medium | Verify endpoints tried in correct order |
| Timeout behavior | Low | Already tested, behavior unchanged |
| Error messages | Low | Already tested, no change |

## Risk Mitigation

### Risk: Breaking existing localhost detection

**Mitigation:** localhost remains second in the fallback chain, so behavior is unchanged for native development without explicit config.

### Risk: Slow startup when no Ollama

**Mitigation:**
- Timeout remains 2 seconds per endpoint
- Max 6 seconds total (acceptable for cold start)
- Most users have Ollama somewhere, so 1-2 endpoints succeed quickly

### Risk: Docker host not resolvable on some systems

**Mitigation:**
- `host.docker.internal` is last in chain, only tried if others fail
- Returns None gracefully if not resolvable
- Linux Docker requires `--add-host` flag (documented limitation)

## Acceptance Criteria

- [ ] All existing tests pass
- [ ] New unit tests for `extract_base_url()` pass
- [ ] Manual test: localhost detection works (native env)
- [ ] Manual test: host.docker.internal detection works (devcontainer)
- [ ] Manual test: explicit endpoint override works
- [ ] Logs show which endpoint was detected
