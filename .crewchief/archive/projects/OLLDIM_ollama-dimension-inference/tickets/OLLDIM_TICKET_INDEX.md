# OLLDIM Ticket Index

Project: Ollama Dimension Inference
Slug: OLLDIM
Total Tickets: 4
Phase: 1 (Single-phase implementation)

## Ticket Overview

| Ticket ID | Title | Status | Estimated Time | Dependencies |
|-----------|-------|--------|----------------|--------------|
| OLLDIM-1001 | Helper Function - infer_ollama_dimension() | Not Started | 1 hour | None |
| OLLDIM-1002 | Inference Logic in from_env() | Not Started | 1.5 hours | OLLDIM-1001 |
| OLLDIM-1003 | Integration Test in Factory | Not Started | 30 minutes | OLLDIM-1001, OLLDIM-1002 |
| OLLDIM-1004 | Documentation Update | Not Started | 30 minutes | OLLDIM-1001, OLLDIM-1002, OLLDIM-1003 |

**Total Estimated Time:** 3.5 hours

## Phase 1: Dimension Inference Implementation

### Sequence

1. **OLLDIM-1001**: Create helper function that maps model names to dimensions
2. **OLLDIM-1002**: Integrate helper into config loading with inference logic
3. **OLLDIM-1003**: Add end-to-end test through factory
4. **OLLDIM-1004**: Document the feature for users and maintainers

### Critical Path

```
OLLDIM-1001 → OLLDIM-1002 → OLLDIM-1003 → OLLDIM-1004
```

All tickets are sequential. Each ticket must be completed before the next begins.

## Ticket Details

### OLLDIM-1001: Helper Function - infer_ollama_dimension()
**Agent:** rust-developer
**File:** `crates/maproom/src/embedding/config.rs`
**Scope:** Implement prefix-matching helper function that returns dimensions for known Ollama models
**Tests:** 3 unit tests (known models, model tags, unknown models)
**Time:** 1 hour

### OLLDIM-1002: Inference Logic in from_env()
**Agent:** rust-developer
**File:** `crates/maproom/src/embedding/config.rs`
**Scope:** Add model defaulting and dimension inference logic to config loading with logging
**Tests:** 6 unit tests (mxbai, nomic, explicit override, unknown, provider isolation, zero-config)
**Time:** 1.5 hours
**Dependencies:** OLLDIM-1001 (helper function must exist)

### OLLDIM-1003: Integration Test in Factory
**Agent:** rust-developer
**File:** `crates/maproom/src/embedding/factory.rs`
**Scope:** Add end-to-end async test verifying dimension flows through factory
**Tests:** 1 integration test (zero-config through factory)
**Time:** 30 minutes
**Dependencies:** OLLDIM-1001, OLLDIM-1002 (inference must be working)

### OLLDIM-1004: Documentation Update
**Agent:** rust-developer
**File:** `crates/maproom/CLAUDE.md`
**Scope:** Document automatic inference, supported models, override mechanism, and upgrade notes
**Tests:** N/A (documentation-only)
**Time:** 30 minutes
**Dependencies:** OLLDIM-1001, OLLDIM-1002, OLLDIM-1003 (feature must be complete)

## Success Metrics

### Code Quality
- [ ] All tests pass: `cargo test -p crewchief-maproom`
- [ ] No clippy warnings: `cargo clippy -p crewchief-maproom`
- [ ] Code formatted: `cargo fmt --check`

### Functionality
- [ ] Zero-config with mxbai-embed-large uses 1024 dimensions
- [ ] Zero-config with nomic-embed-text uses 768 dimensions
- [ ] Explicit MAPROOM_EMBEDDING_DIMENSION overrides inference
- [ ] Unknown models log warning and use default
- [ ] Non-Ollama providers unaffected

### Documentation
- [ ] Helper function has clear docstring
- [ ] Inference logic has explanatory comments
- [ ] User-facing documentation complete
- [ ] Warning messages are actionable

## Testing Summary

**Total Tests:** 10 (9 unit + 1 integration)

### Unit Tests (config.rs)
1. `test_infer_ollama_dimension_known_models` - Helper correctness
2. `test_infer_ollama_dimension_with_tags` - Prefix matching
3. `test_infer_ollama_dimension_unknown_model` - Unknown handling
4. `test_from_env_infers_dimension_mxbai` - mxbai inference
5. `test_from_env_infers_dimension_nomic` - nomic inference
6. `test_from_env_explicit_dimension_overrides_inference` - Explicit wins
7. `test_from_env_unknown_model_keeps_default` - Unknown default
8. `test_from_env_inference_only_for_ollama` - Provider isolation
9. `test_from_env_zero_config_ollama` - Zero-config with model defaulting

### Integration Tests (factory.rs)
1. `test_zero_config_infers_dimension_mxbai` - End-to-end zero-config

## Commit Strategy

Each ticket will result in one commit:

1. **OLLDIM-1001**: "feat(embedding): add helper function for Ollama dimension inference"
2. **OLLDIM-1002**: "fix(embedding): infer Ollama dimensions from model name"
3. **OLLDIM-1003**: "test(embedding): add integration test for dimension inference"
4. **OLLDIM-1004**: "docs(maproom): document automatic dimension inference"

## Files Modified

### Implementation Files
- `crates/maproom/src/embedding/config.rs` (OLLDIM-1001, OLLDIM-1002)
- `crates/maproom/src/embedding/factory.rs` (OLLDIM-1003)
- `crates/maproom/CLAUDE.md` (OLLDIM-1004)

### Test Files
All tests added to existing test modules (no new files)

## Risk Mitigation

### Low Risk
- Small code change (~30 lines of logic)
- High test coverage (10 tests)
- Backward compatible (only affects zero-config)
- No external dependencies

### Rollback Plan
If issues arise, revert commits in reverse order:
1. Revert OLLDIM-1004 (docs only, no risk)
2. Revert OLLDIM-1003 (test only, no risk)
3. Revert OLLDIM-1002 (inference logic - main change)
4. Revert OLLDIM-1001 (helper function)

## Verification Checklist

After completing all tickets:
- [ ] Run full test suite: `cargo test -p crewchief-maproom`
- [ ] Test zero-config workflow manually
- [ ] Verify logs show inference decisions
- [ ] Check validation still warns on mismatches
- [ ] Verify explicit config still works
- [ ] Test unknown model warning appears

## References

- **Plan:** `.crewchief/projects/OLLDIM_ollama-dimension-inference/planning/plan.md`
- **Architecture:** `.crewchief/projects/OLLDIM_ollama-dimension-inference/planning/architecture.md`
- **Quality Strategy:** `.crewchief/projects/OLLDIM_ollama-dimension-inference/planning/quality-strategy.md`
