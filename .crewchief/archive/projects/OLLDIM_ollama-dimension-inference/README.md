# Project: Ollama Dimension Inference

**Slug:** OLLDIM
**Status:** ✅ Complete
**Created:** 2025-12-03
**Completed:** 2025-12-05

## Summary

Fix critical bug in `EmbeddingConfig::from_env()` where embedding dimensions are not inferred from Ollama model names, causing dimension mismatch errors in zero-config workflows.

## Problem Statement

When users rely on Maproom's zero-config auto-detection for Ollama embeddings, the system fails to infer the correct embedding dimension from the model name. This results in dimension mismatch errors:

**Current Behavior:**
- User runs Maproom with no configuration (zero-config)
- Ollama detected, default model `mxbai-embed-large` selected
- Dimension defaults to 1536 (OpenAI default, incorrect for Ollama)
- Embedding generation fails: "Dimension mismatch: expected 1536 but got 1024"

**Expected Behavior:**
- System infers dimension 1024 from model name `mxbai-embed-large`
- Embedding generation succeeds

## Proposed Solution

Add dimension inference logic to `EmbeddingConfig::from_env()` that:
1. Detects Ollama model names (nomic-embed-text, mxbai-embed-large)
2. Infers appropriate dimensions (768, 1024)
3. Only activates when `MAPROOM_EMBEDDING_DIMENSION` not explicitly set
4. Logs warnings for unknown models

**Key Design Principles:**
- **Backward Compatible:** Explicit configuration always wins
- **MVP Focused:** Static mapping of 2 known models only
- **Non-Breaking:** Unknown models handled gracefully
- **Simple:** ~30 lines of code, no new dependencies

## Relevant Agents

### Planning Phase
- **project-planner** - Created all planning documents (this phase)

### Implementation Phase
- **rust-developer** - Implement helper function and inference logic
- **unit-test-runner** - Create and run 7 unit tests + 1 integration test
- **verify-ticket** - Run full test suite, verify backward compatibility
- **commit-ticket** - Create commit with conventional commit message

## Planning Documents

- [analysis.md](planning/analysis.md) - Deep problem analysis and research findings
- [architecture.md](planning/architecture.md) - Solution design and data flow
- [plan.md](planning/plan.md) - Single-phase execution plan with detailed deliverables
- [quality-strategy.md](planning/quality-strategy.md) - Pragmatic testing approach (8 tests total)
- [security-review.md](planning/security-review.md) - Security assessment (MINIMAL risk, PASSED)

## Key Findings

### From Analysis
- **Root Cause:** Separation of model and dimension loading with no cross-validation
- **Existing Infrastructure:** Validation code already knows correct dimensions
- **User Impact:** Zero-config users encounter errors, explicit config users unaffected
- **Simple Fix:** Connect existing pieces (model name → dimension inference)

### From Architecture
- **Implementation:** ~20-line helper function + inference logic in `from_env()`
- **Integration:** No changes needed in factory or provider layers
- **Performance:** Negligible (O(1) string match, once per startup)
- **Risk:** Minimal (backward compatible, comprehensive tests)

### From Quality Strategy
- **Test Coverage:** 7 unit tests + 1 integration test
- **Critical Paths:** Zero-config, explicit override, known models, unknown models, provider isolation
- **Regression:** All existing tests must pass unchanged
- **Confidence:** High (simple logic, clear test cases)

### From Security Review
- **Risk Level:** MINIMAL (no security implications)
- **Attack Surface:** None (internal configuration logic only)
- **Dependencies:** Zero new dependencies
- **Assessment:** PASSED (approved for implementation)

## Estimated Effort

**Total:** 2-3 hours

- **Implementation:** 1 hour (helper function + inference logic + logging)
- **Testing:** 1-1.5 hours (7 unit tests + 1 integration test + manual verification)
- **Documentation:** 30 minutes (code comments + CLAUDE.md update)

## Success Criteria

### Functional
- [ ] Zero-config with mxbai-embed-large infers 1024 dimensions
- [ ] Zero-config with nomic-embed-text infers 768 dimensions
- [ ] Explicit `MAPROOM_EMBEDDING_DIMENSION` overrides inference
- [ ] Unknown models log warning and use default
- [ ] Non-Ollama providers unaffected

### Technical
- [ ] All 8 tests pass (7 unit + 1 integration)
- [ ] Existing regression tests pass unchanged
- [ ] No clippy warnings
- [ ] Code formatted correctly

### User Experience
- [ ] No dimension mismatch errors in zero-config
- [ ] Clear warning messages for unknown models
- [ ] Debug logs confirm inference decisions

## Files Modified

1. **`crates/maproom/src/embedding/config.rs`**
   - Add `infer_ollama_dimension()` helper function
   - Add inference logic in `from_env()` (lines ~118-130)
   - Add 7 unit tests

2. **`crates/maproom/src/embedding/factory.rs`**
   - Add 1 integration test (no implementation changes)

3. **`crates/maproom/CLAUDE.md`** (post-implementation)
   - Document dimension configuration options

## Next Steps

**Recommended:** Run `/review-project OLLDIM` to validate planning before creating tickets.

**After Review:** Run `/workstream:project-tickets OLLDIM` to generate implementation tickets.

## Dependencies

- None (self-contained bug fix)

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Incorrect inference | Low | Medium | Validation warns on mismatches |
| Model name variations | Medium | Low | Use exact string matching initially |
| Breaking explicit config | Very Low | High | Tests verify explicit config wins |
| Unknown model confusion | Low | Low | Warning message guides explicit config |

## Context

### Recent Changes
- Default model changed from `nomic-embed-text` (768-dim) to `mxbai-embed-large` (1024-dim)
- Default dimension remained at 1536 (OpenAI), causing mismatch

### Codebase State
- Validation logic already knows correct dimensions (config.rs:265-286)
- OllamaProvider accepts any dimension parameter
- Factory pattern established and working

### User Expectation
- Zero-config workflow should "just work"
- Current behavior breaks this expectation for Ollama users
- This bug fix restores intended zero-config experience

## Notes

- This is a **bug fix**, not a feature
- Backward compatible (explicit config unchanged)
- MVP focused (2 models only, static mapping)
- High confidence (simple logic, comprehensive tests)
- No security implications (internal config logic only)
