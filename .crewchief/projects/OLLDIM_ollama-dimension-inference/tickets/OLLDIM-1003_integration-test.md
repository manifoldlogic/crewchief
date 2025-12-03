# Ticket: [OLLDIM-1003]: Integration Test in Factory

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

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
Add end-to-end integration test in `factory.rs` that verifies dimension inference flows correctly through the entire provider creation pipeline for zero-config Ollama setups.

## Background
While unit tests in OLLDIM-1002 verify the inference logic works in `EmbeddingConfig::from_env()`, we need an integration test to ensure the inferred dimension correctly flows through the factory pattern to the actual provider creation.

This test validates the complete zero-config workflow: no environment variables set → Ollama detected → mxbai-embed-large defaulted → 1024 dimensions inferred → provider created with correct dimension.

Reference: Phase 1 Deliverable 4 from plan.md

## Acceptance Criteria
- [ ] Test `test_zero_config_infers_dimension_mxbai` exists in factory.rs
- [ ] Test is marked with `#[tokio::test]` and `#[serial]`
- [ ] Test cleans up all environment variables (provider, model, dimension, endpoint)
- [ ] Test sets minimal Ollama configuration (provider=ollama, model=mxbai-embed-large)
- [ ] Test calls `create_provider_from_env().await` successfully
- [ ] Test verifies provider name is "ollama"
- [ ] Test verifies dimension is 1024 (correctly inferred)
- [ ] Test passes when run with `cargo test -p crewchief-maproom`

## Technical Requirements
- Location: `crates/maproom/src/embedding/factory.rs` in existing `#[cfg(test)] mod tests` block
- Test must be async (`#[tokio::test]`)
- Test must use `#[serial]` attribute (modifies environment)
- Must clean environment before setting test values
- Must clean environment in cleanup section
- Must verify both provider name and dimension

## Implementation Notes

**Exact implementation provided in plan.md lines 272-297:**

```rust
#[tokio::test]
#[serial]
async fn test_zero_config_infers_dimension_mxbai() {
    // Clean environment
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_MODEL");
    env::remove_var("MAPROOM_EMBEDDING_DIMENSION");
    env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");

    // Set up minimal config for Ollama
    env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");
    env::set_var("MAPROOM_EMBEDDING_MODEL", "mxbai-embed-large");

    let result = create_provider_from_env().await;

    // Cleanup
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_MODEL");

    assert!(result.is_ok(), "Provider creation should succeed");
    let provider = result.unwrap();
    assert_eq!(provider.provider_name(), "ollama");
    assert_eq!(provider.dimension(), 1024); // Correctly inferred
}
```

**Key points:**
- Clean environment first to ensure test isolation
- Set only provider and model (dimension should be inferred)
- Clean up even on test failure (cleanup before assertions)
- Verify both provider type and dimension value

## Dependencies
- **Prerequisites**:
  - OLLDIM-1001 (helper function exists)
  - OLLDIM-1002 (inference logic integrated)
- Uses `tokio::test` (already in dev dependencies)
- Uses `serial_test` crate (already in dev dependencies)

## Risk Assessment
- **Risk**: Test flakiness due to environment pollution from other tests
  - **Mitigation**: Use `#[serial]` to prevent concurrent test execution. Clean environment both before setup and after test.

- **Risk**: Test fails if Ollama not available
  - **Mitigation**: Test doesn't require actual Ollama connection - only tests config/factory logic. Provider creation succeeds even without Ollama running.

- **Risk**: Test passes but real workflow fails
  - **Mitigation**: This is an integration test through the factory - it exercises the actual code path users hit. Manual testing recommended as final verification.

## Files/Packages Affected
- `crates/maproom/src/embedding/factory.rs` (add 1 integration test in existing `#[cfg(test)] mod tests` block)

## Verification Notes
The verify-ticket agent should confirm:
1. Test exists in factory.rs test module
2. Test has both `#[tokio::test]` and `#[serial]` attributes
3. Test cleans all 4 environment variables before setup
4. Test sets only provider and model (not dimension)
5. Test cleans environment before assertions (fail-safe cleanup)
6. Test verifies provider name equals "ollama"
7. Test verifies dimension equals 1024
8. Test passes when run individually: `cargo test -p crewchief-maproom test_zero_config_infers_dimension_mxbai`
9. Test passes in full suite: `cargo test -p crewchief-maproom`
10. No clippy warnings
