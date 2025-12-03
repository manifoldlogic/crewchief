# Ticket: [DIM1024-2001]: Make Ollama Provider Dimension Configurable

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
Remove hardcoded dimension=768 from OllamaProvider and make dimension configurable via constructor parameter, enabling mxbai-embed-large (1024-dim) and other Ollama models with different dimensions.

## Background
This ticket implements Phase 2 of the DIM1024 project. Currently, OllamaProvider has dimension=768 hardcoded, which blocks support for mxbai-embed-large (1024-dim). This ticket refactors the provider to accept dimension as a configuration parameter.

The change enables flexibility for different Ollama models with different dimensions (nomic-embed-text=768, mxbai-embed-large=1024) without requiring code changes. Configuration validation is updated from hard errors to helpful warnings, trusting user configuration.

Dependencies: This ticket requires DIM1024-1001 (Database Foundation) to be completed first, as the database must support 1024 dimensions before the provider can be configured to use them.

References: `/workspace/.crewchief/projects/DIM1024_embedding-dimension-1024/planning/plan.md` (Phase 2), `/workspace/.crewchief/projects/DIM1024_embedding-dimension-1024/planning/architecture.md` (Component 5, 6).

## Acceptance Criteria
- [ ] OllamaProvider has dimension field in struct
- [ ] OllamaProvider::new() accepts dimension parameter
- [ ] dimension() method returns configured value (not hardcoded 768)
- [ ] Config validation passes for Ollama provider with dimension=1024
- [ ] Warning logs for dimension mismatches (not hard errors)
- [ ] Unit test: OllamaProvider accepts dimension=1024
- [ ] Unit test: dimension() returns configured value
- [ ] Backward compatibility: Existing dimension=768 configurations still work
- [ ] All existing unit tests still pass

## Technical Requirements
- Add dimension field to OllamaProvider struct
- Update OllamaProvider::new() signature to accept dimension: usize parameter
- Update dimension() method implementation to return self.dimension (not hardcoded 768)
- Store dimension from constructor in provider instance
- Remove or modify hardcoded dimension validation in config.rs (lines 266-276)
- Replace hard errors with warning logs for model/dimension mismatches
- Add helpful warnings: "nomic-embed-text typically uses 768 dimensions, got {}"
- Add helpful warnings: "mxbai-embed-large typically uses 1024 dimensions, got {}"

## Implementation Notes
**Current Problem**: The OllamaProvider currently returns hardcoded 768 in the dimension() method, regardless of configuration. This blocks 1024-dim embeddings even after database support is added.

**Change Pattern**:
```rust
// OLD: Hardcoded
fn dimension(&self) -> usize {
    768
}

// NEW: Configurable
fn dimension(&self) -> usize {
    self.dimension  // Read from struct field
}
```

**Constructor Update**:
```rust
// Update signature
pub fn new(endpoint: String, model: String, dimension: usize) -> Result<Self, EmbeddingError> {
    // Store dimension in struct
}
```

**Validation Philosophy**: Replace hard errors with warnings. Trust user configuration but provide helpful guidance. Users may know better than code which model/dimension combinations work.

**Backward Compatibility**: All existing configurations with dimension=768 must continue working. This is purely additive functionality.

## Dependencies
- **DIM1024-1001**: Database Foundation (MUST be completed first)
  - Reason: Cannot configure dimension=1024 until database supports it
- **No external dependencies**: Configuration changes only affect internal code

## Risk Assessment
- **Risk**: Breaking existing Ollama configurations with hardcoded validation removal
  - **Mitigation**: Keep validation structure, replace errors with warnings, test with existing configs
- **Risk**: Users configure invalid dimension and get cryptic errors later
  - **Mitigation**: Clear warning messages listing typical dimensions for each model
- **Risk**: Dimension parameter not properly threaded through initialization
  - **Mitigation**: Unit tests verify dimension() returns configured value, not hardcoded 768

## Files/Packages Affected
- `/workspace/crates/maproom/src/embedding/ollama.rs`
- `/workspace/crates/maproom/src/embedding/config.rs`

## Verification Notes
The verify-ticket agent should specifically check:

1. **Field existence**: OllamaProvider struct has dimension: usize field
2. **Constructor signature**: new() accepts dimension parameter
3. **Method implementation**: dimension() returns self.dimension (not literal 768)
4. **Configuration validation**: 1024 allowed for Ollama provider without errors
5. **Warning logs**: Code includes warning messages for dimension mismatches
6. **Test execution**: Unit tests were EXECUTED and show passing output
7. **Backward compatibility**: Test with dimension=768 still passes
8. **No hardcoded 768**: Search codebase for hardcoded 768 values in ollama.rs
9. **Integration**: Dimension flows from config through provider to embedding generation
