# Task: [MPRSKL.3001]: Enhance dimension mismatch error

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
Improve the dimension mismatch error message to include configuration context, root cause explanation, and actionable solutions, enabling users and agents to diagnose and fix the issue without external help.

## Background
Current dimension mismatch error is opaque: "expected 1536 dimensions but got 1024". This doesn't explain why the mismatch occurred or how to fix it.

With Phase 1's bug fix, this error should be less common (auto-detected Ollama will work). However, when it does occur (custom models, explicit misconfigurations), the error message should guide the user to a solution.

**References:** plan.md Phase 3, Task 6; architecture.md Decision 3; quality-strategy.md Error Handling

## Acceptance Criteria
- [x] Enhanced error message includes current configuration context (provider, model, dimension)
- [x] Error explains likely cause (provider/model mismatch)
- [x] Error lists 3 actionable solutions with example commands
- [x] Error message is formatted for readability (newlines, sections)
- [x] Test added that verifies error message contains expected elements
- [x] Error message doesn't break existing error handling or logging
- [x] All existing tests pass

## Technical Requirements
- Modify error handling in `crates/maproom/src/embedding/error.rs` or provider implementations
- Add configuration context to DimensionMismatch error variant if needed
- Format error message with multiple lines and clear sections
- Include current config values in error output
- Provide specific solutions based on common scenarios
- Test error message format and content
- Ensure error implements Display trait correctly

## Implementation Notes
**Enhanced error format (from architecture.md):**

```rust
// In error.rs or appropriate location
impl fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmbeddingError::DimensionMismatch { expected, actual, config } => {
                write!(
                    f,
                    "Dimension mismatch: expected {} dimensions but got {}.\n\n\
                     Current configuration:\n\
                     - Provider: {}\n\
                     - Model: {}\n\
                     - Dimension: {}\n\n\
                     This usually means the embedding provider configuration doesn't match the actual provider.\n\n\
                     Solutions:\n\
                     1. Set provider explicitly:\n\
                        export MAPROOM_EMBEDDING_PROVIDER={}\n\
                     2. Set dimension to match your model:\n\
                        export MAPROOM_EMBEDDING_DIMENSION={}\n\
                     3. Skip embeddings if not needed:\n\
                        crewchief-maproom scan --generate-embeddings=false\n\n\
                     See troubleshooting guide: .crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/troubleshooting.md",
                    expected,
                    actual,
                    config.provider,
                    config.model,
                    config.dimension,
                    // Suggest correct provider based on actual dimension
                    infer_provider_from_dimension(*actual),
                    actual
                )
            }
            // ... other error variants
        }
    }
}

// Helper function
fn infer_provider_from_dimension(dim: usize) -> &'static str {
    match dim {
        1024 => "ollama",
        1536 => "openai",
        768 => "ollama",  // nomic-embed-text
        _ => "unknown"
    }
}
```

**Alternative: Enhance existing error without changing enum structure:**
- Locate where DimensionMismatch error is created
- Add context before returning error
- Format message at creation site with config details

**Testing approach:**
```rust
#[test]
fn test_dimension_mismatch_error_message() {
    let config = EmbeddingConfig {
        provider: Provider::OpenAI,
        model: "text-embedding-ada-002".to_string(),
        dimension: 1536,
        // ... other fields
    };

    let error = EmbeddingError::DimensionMismatch {
        expected: 1536,
        actual: 1024,
        config: config,
    };

    let error_msg = format!("{}", error);

    // Verify message contains expected elements
    assert!(error_msg.contains("expected 1536"));
    assert!(error_msg.contains("got 1024"));
    assert!(error_msg.contains("Provider: OpenAI"));
    assert!(error_msg.contains("MAPROOM_EMBEDDING_PROVIDER"));
    assert!(error_msg.contains("Solutions:"));
    assert!(error_msg.contains("--generate-embeddings=false"));
}
```

**Critical design considerations:**
- Error message length: Balance helpfulness vs overwhelming output
- Configuration access: Error may need reference to current config
- Backward compatibility: Don't break existing error consumers

## Dependencies
- **MPRSKL.1001, MPRSKL.1002** (Phase 1 bug fix) - Error should be less common after fix, but still needs improvement
- **MPRSKL.2003** (troubleshooting.md) - Error message can reference troubleshooting guide

## Risk Assessment
- **Risk**: Error message too verbose, clutters output
  - **Mitigation**: Use newlines for readability; keep total length under 20 lines
- **Risk**: Breaking existing error handling code
  - **Mitigation**: Run all tests; verify error implements Display correctly
- **Risk**: Configuration context not available at error creation site
  - **Mitigation**: Pass config to error variant or capture earlier in call stack

## Files/Packages Affected
- crates/maproom/src/embedding/error.rs
- Possibly provider implementations (crates/maproom/src/embedding/providers/*.rs)

## Deliverables Produced

Documents created in `deliverables/` directory:

- None

## Verification Notes
The verify-task agent should specifically check:

- [ ] Error message includes all required elements: expected, actual, config, cause, solutions
- [ ] Configuration context shown: provider, model, dimension
- [ ] At least 3 solutions listed with specific commands
- [ ] Error message is readable (proper newlines and formatting)
- [ ] Test added verifying error message content
- [ ] All existing tests pass (`cargo test -p crewchief-maproom`)
- [ ] No clippy warnings (`cargo clippy -p crewchief-maproom`)
- [ ] Code formatted (`cargo fmt -- --check`)
- [ ] Error can be triggered to manually verify output

**Manual verification:**
```bash
# Trigger dimension mismatch to see enhanced error
MAPROOM_EMBEDDING_DIMENSION=512 crewchief-maproom scan --path .

# Verify error message shows:
# - Expected vs actual dimensions
# - Current configuration
# - 3 solutions with commands
# - Readable formatting
```

**Test verification:**
```bash
# Run new error message test
cargo test -p crewchief-maproom test_dimension_mismatch_error_message

# Verify test checks for key message elements
```

## Verification Audit
<!-- Audit log maintained by verify-task agent for enterprise compliance -->
| Date | Agent | Decision | Notes |
|------|-------|----------|-------|
| 2025-12-20 | verify-task | PASS | All 7 acceptance criteria met, 4 dimension mismatch tests passing |
<!-- Entries added automatically during verification -->
