# Quality Strategy: Ollama Dimension Inference

## Testing Philosophy

**Confidence Over Coverage:** Test the inference logic thoroughly but pragmatically. Focus on the bug fix working correctly and not breaking existing functionality.

**Key Principle:** This is a small, focused bug fix. Testing should be proportional to the change - comprehensive for the new inference logic, smoke tests for existing paths.

## Test Types

### Unit Tests

**Scope:**
- Helper function `infer_ollama_dimension()` correctness
- Inference logic in `EmbeddingConfig::from_env()`
- Explicit configuration override behavior
- Unknown model handling
- Provider specificity (Ollama vs others)

**Tools:**
- Rust built-in test framework (`#[test]`)
- `serial_test` crate for env var tests (`#[serial]`)

**Coverage Target:**
- **100% for new code** (helper function and inference logic)
- **Existing regression tests** must pass unchanged

**Test Count:** 9 unit tests (3 for helper, 6 for integration)

### Integration Tests

**Scope:**
- End-to-end zero-config workflow through factory
- Dimension flowing correctly to OllamaProvider

**Approach:**
- Test factory pattern with environment configuration
- Verify dimension reaches provider correctly
- Minimal mocking (test against real Config and Provider structs)

**Test Count:** 1 integration test in factory.rs

### End-to-End Tests

**Scope:** Not required for this bug fix.

**Rationale:**
- Change is self-contained in configuration loading
- No new external dependencies or API changes
- Integration test covers the factory pattern sufficiently

## Critical Paths

The following paths MUST be tested:

### 1. Zero-Config Inference (Most Critical)
**Path:** No env vars → auto-detect Ollama → mxbai-embed-large → infer 1024 dimensions
**Test:** `test_zero_config_infers_dimension_mxbai`
**Why Critical:** This is the bug we're fixing - must work correctly

### 2. Explicit Configuration Override
**Path:** Set MAPROOM_EMBEDDING_DIMENSION → inference skipped → explicit value used
**Test:** `test_from_env_explicit_dimension_overrides_inference`
**Why Critical:** Backward compatibility - existing explicit configs must keep working

### 3. Known Model Inference
**Path:** Set MAPROOM_EMBEDDING_MODEL=nomic-embed-text → infer 768 dimensions
**Test:** `test_from_env_infers_dimension_nomic`
**Why Critical:** Both supported models must infer correctly

### 4. Unknown Model Handling
**Path:** Set MAPROOM_EMBEDDING_MODEL=custom-model → warn → use default
**Test:** `test_from_env_unknown_model_keeps_default`
**Why Critical:** Must not break for custom Ollama models

### 5. Provider Isolation
**Path:** Non-Ollama provider → no inference → use default
**Test:** `test_from_env_inference_only_for_ollama`
**Why Critical:** Must not affect OpenAI/Cohere/other providers

### 6. Model Tag Handling
**Path:** Set MAPROOM_EMBEDDING_MODEL=mxbai-embed-large:latest → infer 1024 dimensions
**Test:** `test_infer_ollama_dimension_with_tags`
**Why Critical:** Users may specify model tags; prefix matching must work

### 7. Zero-Config with Model Defaulting
**Path:** No env vars → Ollama provider → default to mxbai-embed-large → infer 1024
**Test:** `test_from_env_zero_config_ollama`
**Why Critical:** This is the primary bug fix - true zero-config must work

## Test Data Strategy

### Environment Variable Management
**Strategy:** Clean setup and teardown in every test

**Pattern:**
```rust
#[test]
#[serial]
fn test_example() {
    // Clean environment
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_MODEL");
    env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

    // Set up test state
    env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");

    // Run test
    let config = EmbeddingConfig::from_env().unwrap();
    assert_eq!(config.provider, Provider::Ollama);

    // Cleanup
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
}
```

### Model Names
**Strategy:** Use actual model names from production

**Known Models:**
- `"nomic-embed-text"` - Legacy 768-dim model
- `"mxbai-embed-large"` - Current 1024-dim model

**Unknown Models:**
- `"custom-model"` - Generic test case
- `"unknown"` - Ensure any unknown string works

### Dimensions
**Test Values:**
- `768` - nomic-embed-text dimension
- `1024` - mxbai-embed-large dimension
- `1536` - Default (OpenAI) dimension

## Quality Gates

### Before Commit
- [ ] All unit tests pass: `cargo test -p crewchief-maproom`
- [ ] No clippy warnings: `cargo clippy -p crewchief-maproom -- -D warnings`
- [ ] Code formatted: `cargo fmt --check`
- [ ] No new compiler warnings

### Before Verification
- [ ] Helper function tests pass (3 tests including tag handling)
- [ ] Inference logic tests pass (6 tests including zero-config)
- [ ] Integration test passes (1 test)
- [ ] Existing regression tests pass unchanged
- [ ] Validation warnings still fire correctly

### Before Merge
- [ ] Code review complete (inline comments clear)
- [ ] Backward compatibility verified (explicit config works)
- [ ] Zero-config workflow manually tested
- [ ] Warning messages are helpful and actionable

## Test Execution

### Local Development
```bash
# Run all maproom tests
cargo test -p crewchief-maproom

# Run only config tests
cargo test -p crewchief-maproom config::tests

# Run with output
cargo test -p crewchief-maproom -- --nocapture

# Check for warnings
cargo clippy -p crewchief-maproom

# Format check
cargo fmt --check
```

### CI Pipeline
**Requirements:**
- Must pass on all supported platforms (Linux, macOS, Windows)
- Must pass with all Rust toolchain versions supported
- No increase in build time (negligible code addition)

## Regression Testing Strategy

### Existing Tests Must Pass
**Critical Existing Tests:**
1. `test_ollama_validation_no_api_key` - Ollama doesn't require API key
2. `test_ollama_nomic_embed_text_correct_dimension` - Validation for 768
3. `test_ollama_mxbai_embed_large_dimension_1024` - Validation for 1024
4. `test_default_config` - Default values unchanged
5. `test_config_validation` - Validation logic preserved

**What We're Protecting:**
- Existing explicit configuration still works
- Validation warnings still fire on mismatches
- Default behavior for non-Ollama providers unchanged
- Factory pattern integration unaffected

### Smoke Test Scenarios
**Manual verification after implementation:**

1. **Zero-Config Workflow**
   ```bash
   # No env vars set
   cargo run --bin crewchief-maproom -- generate-embeddings --repo test
   # Should: Use mxbai-embed-large at 1024 dimensions
   ```

2. **Explicit Override**
   ```bash
   export MAPROOM_EMBEDDING_DIMENSION=768
   export MAPROOM_EMBEDDING_MODEL=mxbai-embed-large
   cargo run --bin crewchief-maproom -- generate-embeddings --repo test
   # Should: Use mxbai-embed-large at 768 dimensions (explicit wins)
   # Should: Show validation warning about dimension mismatch
   ```

3. **Custom Model**
   ```bash
   export MAPROOM_EMBEDDING_MODEL=custom-model
   cargo run --bin crewchief-maproom -- generate-embeddings --repo test
   # Should: Warn about unknown model
   # Should: Use default 1536 dimensions
   # Should: Guide user to set explicit dimension
   ```

## Performance Testing

**Scope:** Not required for this change.

**Rationale:**
- String comparison is O(1) for small fixed set
- Inference happens once at startup (not in hot path)
- No measurable performance impact expected

## Error Handling Verification

### Expected Errors
1. **Invalid Dimension Value**
   ```bash
   export MAPROOM_EMBEDDING_DIMENSION=invalid
   # Should: Return ConfigError with helpful message
   ```

2. **Unknown Model (Non-Error)**
   ```bash
   export MAPROOM_EMBEDDING_MODEL=unknown-model
   # Should: Log warning, continue with default
   ```

### Error Message Quality
**Checklist:**
- [ ] Warning for unknown models is actionable
- [ ] Debug log confirms successful inference
- [ ] Error messages don't leak internal details
- [ ] Suggestions are clear ("set MAPROOM_EMBEDDING_DIMENSION")

## Test Maintenance

### When to Update Tests
**Add new test when:**
- Adding support for new Ollama model (update helper function + add test)
- Changing default dimension (update assertion expectations)
- Adding new provider type (verify inference isolation)

**Update existing test when:**
- Changing model name strings (update test data)
- Modifying warning messages (update log assertions if checking)

### Test Longevity
**Expected Stability:** High
- Model names are stable (nomic-embed-text, mxbai-embed-large)
- Dimensions are fixed (768, 1024)
- Configuration pattern is established

**Risk of Breakage:** Low
- No external API dependencies
- No network calls
- No file I/O
- Pure logic tests

## Documentation Testing

### Code Comments
**Verify:**
- [ ] Helper function has clear docstring
- [ ] Inline comments explain inference logic
- [ ] Warning message is self-documenting

### Integration Documentation
**Future Work:** After merge, consider adding example to `crates/maproom/CLAUDE.md` showing dimension configuration options.

## Summary

**Testing Approach:** Proportional and pragmatic
- **9 unit tests** for comprehensive logic coverage (including tag handling and zero-config)
- **1 integration test** for end-to-end validation
- **0 new E2E tests** (not needed for this change)
- **Existing regression tests** must pass unchanged

**Confidence Level:** High
- All critical paths tested
- Backward compatibility verified
- Simple logic (minimal edge cases)
- Existing test infrastructure reused
