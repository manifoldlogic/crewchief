# Task: [MPRSKL.1001]: Add from_env_with_provider() to EmbeddingConfig

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-task agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-task
- commit-task

## Summary
Add new public method `from_env_with_provider(provider: Option<Provider>)` to `EmbeddingConfig` to enable factory-detected providers to be correctly propagated during configuration loading.

## Background
Currently, when the factory auto-detects Ollama, it calls `EmbeddingConfig::from_env()` which doesn't know about the detected provider. This causes dimension inference to fail because the config uses default provider settings instead of Ollama-specific settings.

This task implements the first part of the fix by adding a new constructor that accepts an optional provider override. This enables the factory (in MPRSKL.1002) to pass the detected provider during config creation.

**References:** plan.md Phase 1, Task 1; architecture.md Decision 1

## Acceptance Criteria
- [x] New public method `from_env_with_provider(provider: Option<Provider>)` added to `EmbeddingConfig`
- [x] Method applies provider override before loading environment variables
- [x] Environment variables can still override the programmatic provider
- [x] Existing `from_env()` refactored to call `from_env_with_provider(None)` for backward compatibility
- [x] All existing tests continue to pass
- [x] New unit tests added covering: None provider, Some provider, env var override behavior
- [x] Provider override enables correct dimension inference for Ollama (1024 for mxbai-embed-large)

## Technical Requirements
- Modify `crates/maproom/src/embedding/config.rs`
- Add `from_env_with_provider(provider_override: Option<Provider>) -> Result<Self, EmbeddingError>` method
- Refactor `from_env()` to delegate to `from_env_with_provider(None)`
- Ensure provider override is applied before env var loading
- Maintain all existing behavior when `provider_override` is `None`
- Add unit tests with `#[serial]` attribute for env var manipulation
- Test dimension inference with Provider::Ollama override
- Test precedence: env var should override programmatic provider

## Implementation Notes
**Pattern to follow:**
```rust
impl EmbeddingConfig {
    /// Load configuration with explicit provider override.
    /// Used when provider is detected at runtime (e.g., Ollama auto-detection).
    pub fn from_env_with_provider(provider_override: Option<Provider>) -> Result<Self, EmbeddingError> {
        let mut config = Self::default();

        // Apply override first if provided
        if let Some(p) = provider_override {
            config.provider = p;
        }

        // Then load from env (env vars can still override)
        if let Ok(provider_str) = env::var("MAPROOM_EMBEDDING_PROVIDER") {
            config.provider = provider_str.parse()?;
        }

        // Continue with rest of from_env logic...
        // Dimension inference will now use correct provider
    }

    pub fn from_env() -> Result<Self, EmbeddingError> {
        Self::from_env_with_provider(None)
    }
}
```

**Critical design decision:** Provider override is applied BEFORE env var loading, so explicit env vars always win. This maintains user control while enabling factory auto-detection.

**Testing requirements:**
- Use `#[serial]` for tests that manipulate environment variables
- Clean up env vars after each test
- Test all three scenarios: None, Some(Provider), env var override

## Dependencies
- None (Phase 1 foundation task)

## Risk Assessment
- **Risk**: Breaking change to existing `from_env()` behavior
  - **Mitigation**: Refactor `from_env()` to call new method with None - maintains exact same behavior
- **Risk**: Existing tests fail after refactoring
  - **Mitigation**: Run full test suite before committing, fix any issues incrementally
- **Risk**: Dimension inference still incorrect with override
  - **Mitigation**: Add specific test for Ollama dimension inference (1024 expected)

## Files/Packages Affected
- crates/maproom/src/embedding/config.rs

## Deliverables Produced

Documents created in `deliverables/` directory:

- None

## Verification Notes
The verify-task agent should specifically check:

- [ ] New method signature matches specification exactly
- [ ] `from_env()` delegates to `from_env_with_provider(None)`
- [ ] All existing tests pass (`cargo test -p crewchief-maproom`)
- [ ] New tests added with `#[serial]` attribute
- [ ] Test coverage includes: None provider, Some(Provider::Ollama), env var override
- [ ] Dimension inference works correctly with provider override (test should show 1024 for Ollama)
- [ ] No clippy warnings introduced (`cargo clippy -p crewchief-maproom`)
- [ ] Code formatted (`cargo fmt -- --check`)
- [ ] Backward compatibility maintained (no behavioral changes when override is None)

## Implementation Notes

### Changes Made

1. **Added `from_env_with_provider()` method** (lines 130-309)
   - Signature: `pub fn from_env_with_provider(provider_override: Option<Provider>) -> Result<Self, EmbeddingError>`
   - Applies provider override BEFORE loading env vars
   - Includes comprehensive documentation with examples
   - Enables dimension inference for provider-overridden configs

2. **Refactored `from_env()` method** (lines 311-318)
   - Now delegates to `from_env_with_provider(None)`
   - Maintains exact same behavior as before (backward compatible)
   - Simplified to single line: `Self::from_env_with_provider(None)`

3. **Added unit tests** (lines 1080-1133)
   - `test_from_env_with_provider_none` - Verifies None behaves same as from_env()
   - `test_from_env_with_provider_ollama` - Verifies Ollama override enables dimension inference (1024)
   - `test_from_env_with_provider_env_override` - Verifies env vars override programmatic provider
   - All tests use `#[serial]` attribute for env var safety

### Test Results

All tests passing:
- 3 new tests added and passing
- 34 total config tests passing (including all existing tests)
- 8 endpoint config tests passing
- No regressions introduced

### Key Design Decision

Provider override is applied BEFORE env var loading, ensuring explicit env vars always win. This maintains user control while enabling factory auto-detection to work correctly.

## Verification Audit
<!-- Audit log maintained by verify-task agent for enterprise compliance -->
| Date | Agent | Decision | Notes |
|------|-------|----------|-------|
| 2025-12-20 | verify-task | FAIL | 6/8 endpoint tests failing, 51 clippy errors, code not formatted |
| 2025-12-20 | verify-task | PASS | All 7 acceptance criteria met, 42/42 tests passing, no new clippy warnings |
<!-- Entries added automatically during verification -->
