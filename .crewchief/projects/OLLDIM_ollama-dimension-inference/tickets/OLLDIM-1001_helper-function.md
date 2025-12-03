# Ticket: [OLLDIM-1001]: Helper Function - infer_ollama_dimension()

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-developer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement helper function `infer_ollama_dimension()` that returns embedding dimensions for known Ollama model names using prefix matching to handle model tags.

## Background
The zero-config embedding workflow fails because `EmbeddingConfig::from_env()` doesn't infer dimensions from Ollama model names. This causes dimension mismatches when users don't explicitly set `MAPROOM_EMBEDDING_DIMENSION`.

This ticket implements the first component: a helper function that maps known Ollama model names to their expected dimensions.

Reference: Phase 1 Deliverable 1 from plan.md

## Acceptance Criteria
- [x] Function `infer_ollama_dimension()` compiles and passes clippy
- [x] Returns `Some(768)` for models starting with "nomic-embed-text"
- [x] Returns `Some(1024)` for models starting with "mxbai-embed-large"
- [x] Returns `None` for unknown models
- [x] Uses prefix matching (`.starts_with()`) to handle model tags like ":latest"
- [x] Comprehensive docstring explains purpose, supported models, and return values
- [x] Unit tests pass: helper function tests (3 tests)

## Technical Requirements
- Location: `crates/maproom/src/embedding/config.rs` after RetryConfig impl, before tests
- Signature: `fn infer_ollama_dimension(model: &str) -> Option<usize>`
- Visibility: module-private (no `pub`)
- Implementation: Simple if-else chain with `starts_with()` matching
- No external dependencies
- Zero allocations (returns static values)

## Implementation Notes

**Exact implementation provided in plan.md lines 22-48:**

```rust
/// Infer embedding dimension from known Ollama model names.
///
/// Uses prefix matching to handle model tags (e.g., "mxbai-embed-large:latest").
/// Returns the expected dimension for well-known models, or None for unknown models.
/// This enables zero-config workflows where dimension is automatically determined
/// from the model name without requiring explicit MAPROOM_EMBEDDING_DIMENSION.
///
/// # Supported Models
///
/// - `nomic-embed-text*`: 768 dimensions (matches tags like "nomic-embed-text:latest")
/// - `mxbai-embed-large*`: 1024 dimensions (matches tags like "mxbai-embed-large:v1")
///
/// # Returns
///
/// - `Some(dimension)` for known models
/// - `None` for unknown models (caller should warn and use default)
fn infer_ollama_dimension(model: &str) -> Option<usize> {
    if model.starts_with("nomic-embed-text") {
        Some(768)
    } else if model.starts_with("mxbai-embed-large") {
        Some(1024)
    } else {
        None
    }
}
```

**Unit tests required (plan.md lines 125-146):**

```rust
#[test]
fn test_infer_ollama_dimension_known_models() {
    assert_eq!(infer_ollama_dimension("nomic-embed-text"), Some(768));
    assert_eq!(infer_ollama_dimension("mxbai-embed-large"), Some(1024));
}

#[test]
fn test_infer_ollama_dimension_with_tags() {
    // Test prefix matching for model tags
    assert_eq!(infer_ollama_dimension("nomic-embed-text:latest"), Some(768));
    assert_eq!(infer_ollama_dimension("mxbai-embed-large:latest"), Some(1024));
    assert_eq!(infer_ollama_dimension("mxbai-embed-large:v1"), Some(1024));
}

#[test]
fn test_infer_ollama_dimension_unknown_model() {
    assert_eq!(infer_ollama_dimension("custom-model"), None);
    assert_eq!(infer_ollama_dimension("unknown"), None);
}
```

## Dependencies
- None (self-contained helper function)

## Risk Assessment
- **Risk**: Model dimensions could change upstream in Ollama
  - **Mitigation**: Very low likelihood (dimensions are fixed at training time). Validation layer will catch mismatches. Users can always override with explicit config.

- **Risk**: Model name variations not handled by prefix matching
  - **Mitigation**: Prefix matching handles standard Ollama tag format. Unknown variants return None and trigger warning.

## Files/Packages Affected
- `crates/maproom/src/embedding/config.rs` (add helper function after RetryConfig impl)
- `crates/maproom/src/embedding/config.rs` (add 3 unit tests in existing `#[cfg(test)] mod tests` block)

## Verification Notes
The verify-ticket agent should confirm:
1. Helper function is located in correct file position
2. Function signature matches specification exactly
3. All three test cases exist and pass
4. Docstring is comprehensive and accurate
5. No clippy warnings generated
6. Code formatted with `cargo fmt`
