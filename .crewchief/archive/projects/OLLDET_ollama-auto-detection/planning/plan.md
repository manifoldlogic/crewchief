# Execution Plan: Ollama Auto-Detection

## Overview

Single-phase implementation to add fallback chain for Ollama endpoint detection.

## Phase 1: Implementation

**Agent:** rust-indexer-engineer

### Ticket 1001: Implement Ollama Endpoint Detection Fallback

**Summary:** Replace `is_ollama_available()` with `detect_ollama_endpoint()` that tries multiple endpoints, with full verification.

**Acceptance Criteria:**
- [ ] Add `extract_base_url()` function (handles trailing slashes)
- [ ] Add `detect_ollama_endpoint()` function with fallback chain
- [ ] Update `create_provider_from_env()` to use detected endpoint
- [ ] Add unit tests for `extract_base_url()` (including trailing slash cases)
- [ ] Existing tests pass (adjust `test_ollama_detection_timeout` if needed - see note below)
- [ ] Logs show which endpoint was detected
- [ ] Manual verification: localhost detection works (native env)
- [ ] Manual verification: host.docker.internal detection works (devcontainer)
- [ ] Manual verification: explicit endpoint override takes priority

**Implementation Details:**

1. Add helper function:
```rust
fn extract_base_url(endpoint: &str) -> Option<String>
```

2. Replace `is_ollama_available()` with:
```rust
async fn detect_ollama_endpoint() -> Option<String>
```

3. Update `create_provider_from_env()` to:
   - Call `detect_ollama_endpoint()` during auto-detection
   - Use detected endpoint when creating Ollama provider

4. Add tests:
   - `test_extract_base_url_embed_suffix`
   - `test_extract_base_url_embeddings_suffix`
   - `test_extract_base_url_trailing_slash`
   - `test_extract_base_url_no_suffix`
   - `test_extract_base_url_empty`

**Files Changed:**
- `crates/maproom/src/embedding/factory.rs`

**Note on Timeout Test:**
The existing `test_ollama_detection_timeout` test expects detection to complete within 3 seconds. With the fallback chain trying up to 3 endpoints (2s timeout each), worst case is 6 seconds. The test may need adjustment to allow 7 seconds, or use mocked endpoints to avoid real network timeouts.

**Manual Verification Steps:**

1. Native (localhost):
```bash
unset MAPROOM_EMBEDDING_API_ENDPOINT
cargo run -p crewchief-maproom -- scan --path . --repo test
# Expect: "Ollama detected at: http://localhost:11434"
```

2. DevContainer:
```bash
unset MAPROOM_EMBEDDING_API_ENDPOINT
cargo run -p crewchief-maproom -- scan --path . --repo test
# Expect: "Ollama detected at: http://host.docker.internal:11434"
```

3. Explicit endpoint:
```bash
export MAPROOM_EMBEDDING_API_ENDPOINT=http://custom:11434/api/embed
cargo run -p crewchief-maproom -- scan --path . --repo test
# Expect: Checks custom endpoint first in logs
```

## Workflow

```
OLLDET-1001 (Implementation + Verification)
    │
    ├── rust-indexer-engineer implements changes
    ├── unit-test-runner verifies tests pass
    ├── Manual verification of detection scenarios
    ├── verify-ticket confirms acceptance criteria
    └── commit-ticket creates commit
```

## Success Metrics

1. **Functional:** Ollama auto-detected in devcontainer without explicit config
2. **Backward Compatible:** Existing localhost detection still works
3. **Observable:** Logs show which endpoint was detected
4. **Tested:** All existing tests pass, new unit tests added

## Timeline Notes

This is a focused, single-file change:
- Implementation: ~30 minutes
- Testing: ~15 minutes
- Verification: ~15 minutes

Total: ~1 hour of work.

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Break existing detection | Low | High | localhost remains in fallback chain |
| Slow startup | Low | Low | Timeout already 2s, max 6s total |
| Platform differences | Medium | Low | Document Linux Docker limitation |
| Timeout test fails | Medium | Low | Adjust test expectation or use mocks |

## Dependencies

- None (uses existing crates)

## Out of Scope

- Parallel endpoint detection
- Kubernetes service discovery
- OLLAMA_URL env var support
- Caching detected endpoint
