# Ticket: OLLDET-1001: Implement Ollama Endpoint Detection Fallback

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (20/20 factory tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Replace the hardcoded `is_ollama_available()` function with `detect_ollama_endpoint()` that tries multiple endpoints in order: explicit config, localhost, then Docker host. This enables Ollama auto-detection in DevContainer environments.

## Background
The current `is_ollama_available()` function is hardcoded to check `localhost:11434`. This fails in containerized environments (DevContainer, Docker) where Ollama runs on the host machine and is accessible via `host.docker.internal`. Users must manually set `MAPROOM_EMBEDDING_PROVIDER=ollama` and `MAPROOM_EMBEDDING_API_ENDPOINT` to work around this.

**Plan Reference:** See `planning/plan.md` Phase 1: Implementation

## Acceptance Criteria
- [x] Add `extract_base_url()` function that handles trailing slashes
- [x] Add `detect_ollama_endpoint()` function with fallback chain:
      1. `MAPROOM_EMBEDDING_API_ENDPOINT` (extract base)
      2. `localhost:11434`
      3. `host.docker.internal:11434`
- [x] Update `create_provider_from_env()` to use detected endpoint
- [x] Add unit tests for `extract_base_url()` covering:
      - Standard `/api/embed` suffix
      - Alternative `/api/embeddings` suffix
      - Trailing slash handling
      - No recognized suffix (returns None)
      - Empty string
- [x] Existing tests pass (adjusted `test_ollama_detection_timeout` to 7s timeout)
- [x] Logs show which endpoint was detected (info level)
- [x] Debug logs show fallback chain and each endpoint check

## Technical Requirements

### `extract_base_url()` Function
```rust
fn extract_base_url(endpoint: &str) -> Option<String> {
    // Handle trailing slashes: "http://host:port/api/embed/" → "http://host:port"
    let endpoint = endpoint.trim_end_matches('/');
    endpoint
        .strip_suffix("/api/embed")
        .or_else(|| endpoint.strip_suffix("/api/embeddings"))
        .map(|s| s.to_string())
}
```

### `detect_ollama_endpoint()` Function
- 2-second timeout per endpoint (existing behavior)
- Sequential checks (not parallel)
- Total max: 6 seconds worst case
- Use `/api/tags` for health check (returns 200 when healthy)
- Log fallback chain at debug level before starting
- Log each endpoint check at debug level
- Log detected endpoint at info level

### `create_provider_from_env()` Changes
- When no explicit `MAPROOM_EMBEDDING_PROVIDER` is set:
  - Call `detect_ollama_endpoint()`
  - If `Some(endpoint)`, use it for Ollama provider creation
  - If `None`, return error (no provider available)
- When creating Ollama provider, use detected endpoint if available

## Implementation Notes

1. Keep `is_ollama_available()` for backward compatibility or fully replace it
2. The function signature change from `async fn is_ollama_available() -> bool` to `async fn detect_ollama_endpoint() -> Option<String>` allows returning the actual endpoint
3. Consider adding a unit test for the fallback order (mocked or timeout-based)
4. The timeout test `test_ollama_detection_timeout` may need adjustment:
   - Current: expects completion within 3 seconds
   - After: worst case is 6 seconds (3 endpoints × 2s each)
   - Options: increase to 7s, or use wiremock to mock endpoints

### Manual Verification Steps (after implementation)

**1. Native (localhost):**
```bash
unset MAPROOM_EMBEDDING_API_ENDPOINT
cargo run -p crewchief-maproom -- scan --path . --repo test
# Expect log: "Ollama detected at: http://localhost:11434"
```

**2. DevContainer:**
```bash
unset MAPROOM_EMBEDDING_API_ENDPOINT
cargo run -p crewchief-maproom -- scan --path . --repo test
# Expect log: "Ollama detected at: http://host.docker.internal:11434"
```

**3. Explicit endpoint:**
```bash
export MAPROOM_EMBEDDING_API_ENDPOINT=http://custom:11434/api/embed
cargo run -p crewchief-maproom -- scan --path . --repo test
# Expect log: Checks custom endpoint first in logs
```

## Dependencies
- None (uses existing crates: reqwest, tracing, std::env)

## Risk Assessment
- **Risk**: Break existing localhost detection
  - **Mitigation**: localhost remains second in fallback chain, unchanged behavior for native dev
- **Risk**: Slow startup when no Ollama available
  - **Mitigation**: Max 6 seconds total is acceptable for cold start; most users have Ollama somewhere
- **Risk**: Docker host not resolvable on Linux
  - **Mitigation**: `host.docker.internal` is last in chain, fails gracefully; Linux Docker requires `--add-host` flag (documented limitation)
- **Risk**: Timeout test may fail
  - **Mitigation**: Adjust test expectation to 7 seconds or use mocked endpoints

## Files/Packages Affected
- `crates/maproom/src/embedding/factory.rs`
