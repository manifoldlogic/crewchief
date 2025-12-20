# Plan: Maproom Skill Progressive Disclosure (MPRSKL)

## Overview

This plan implements the factory/config bug fix, skill restructure, and CLI improvements in three sequential phases. Each phase is independently testable and deployable.

## Phases

### Phase 1: Factory/Config Bug Fix (CRITICAL)

**Objective:** Fix the dimension mismatch bug when Ollama is auto-detected without explicit configuration.

**Deliverables:**
- `EmbeddingConfig::from_env_with_provider()` method in `config.rs`
- Updated factory.rs to use new constructor for Ollama branch
- Refactored `from_env()` to maintain backward compatibility
- Unit tests for new constructor
- Integration test for auto-detection flow

**Agent Assignments:**
- rust-engineer: Implement config and factory changes
- verify-task: Verify tests pass and bug is fixed

**Tasks:**

1. **MPRSKL-1001: Add from_env_with_provider() to EmbeddingConfig**
   - Add new public method `from_env_with_provider(provider: Option<Provider>)`
   - Extract common logic from `from_env()` into internal helper
   - Make `from_env()` call `from_env_with_provider(None)`
   - Add unit tests for new method with various provider overrides
   - Files: `crates/maproom/src/embedding/config.rs`

2. **MPRSKL-1002: Update factory.rs to propagate provider**
   - Change Ollama branch to call `from_env_with_provider(Some(Provider::Ollama))`
   - Verify dimension is correctly inferred (1024 for mxbai-embed-large)
   - Add integration test: auto-detect Ollama -> correct dimension
   - Files: `crates/maproom/src/embedding/factory.rs`

**Dependencies:** None (Phase 1 is foundation)

**Verification:**
```bash
# Zero-config with Ollama running
unset MAPROOM_EMBEDDING_PROVIDER
unset MAPROOM_EMBEDDING_DIMENSION
cargo test -p crewchief-maproom test_zero_config
```

---

### Phase 2: Progressive Disclosure Skill Restructure

**Objective:** Reorganize maproom skill documentation for optimal AI agent consumption with progressive disclosure.

**Deliverables:**
- Brief SKILL.md (under 50 lines)
- New references/cli-reference.md (complete command docs)
- New references/troubleshooting.md (error recovery guide)
- Updated search-best-practices.md (minor refinements if needed)

**Agent Assignments:**
- docs-writer: Create restructured documentation
- verify-task: Validate structure and content quality

**Tasks:**

3. **MPRSKL-2001: Create brief SKILL.md**
   - Rewrite SKILL.md to under 50 lines
   - Include: capability summary, when-to-use table, quick reference commands
   - Add references to detailed docs
   - Files: `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/SKILL.md`

4. **MPRSKL-2002: Create cli-reference.md**
   - Document all crewchief-maproom commands
   - Include all flags with examples
   - Organize by command category (search, index, manage)
   - Files: `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/cli-reference.md`

5. **MPRSKL-2003: Create troubleshooting.md**
   - Document common errors with causes and solutions
   - Include dimension mismatch, repository not found, embeddings unavailable
   - Add configuration verification steps
   - Files: `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/troubleshooting.md`

**Dependencies:** Phase 1 (bug fix should be mentioned in troubleshooting)

**Verification:**
- SKILL.md line count < 50
- All referenced files exist
- Content is accurate for current CLI

---

### Phase 3: CLI Error Enhancement

**Objective:** Improve error messages to include configuration context and actionable guidance.

**Deliverables:**
- Enhanced dimension mismatch error message
- Optional: `--show-config` flag or config in status output
- Updated help text for `--generate-embeddings` flag

**Agent Assignments:**
- rust-engineer: Implement error message improvements
- verify-task: Verify error messages are helpful

**Tasks:**

6. **MPRSKL-3001: Enhance dimension mismatch error**
   - Add configuration context to error message
   - Include suggested solutions (set env var, use --generate-embeddings=false)
   - Update error handling in embedding pipeline
   - Files: `crates/maproom/src/embedding/error.rs`, provider implementations

7. **MPRSKL-3002: Improve --generate-embeddings documentation**
   - Update CLI help text to be clearer about skipping embeddings
   - Add example usage in scan --help output
   - Files: `crates/maproom/src/main.rs`

**Dependencies:** Phase 1 (bug fix reduces error occurrence)

**Verification:**
```bash
# Trigger dimension mismatch (for testing error message)
MAPROOM_EMBEDDING_DIMENSION=512 crewchief-maproom scan --path .
# Error should include helpful guidance

# Check help text
crewchief-maproom scan --help
# Should clearly explain --generate-embeddings
```

---

## Dependencies

```
Phase 1 (Bug Fix)
    |
    +-- Phase 2 (Skill Restructure) - references bug fix in troubleshooting
    |
    +-- Phase 3 (CLI Enhancement) - builds on fixed foundation
```

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| API breaking change in config.rs | Low | High | Maintain `from_env()` as wrapper, add not replace |
| Existing tests fail | Medium | Medium | Run tests early, fix issues incrementally |
| Skill structure breaks plugin loading | Low | Medium | Test with actual Claude agent before merge |
| Error message too verbose | Low | Low | Review with agent perspective, iterate |

## Success Metrics

- [ ] `crewchief-maproom scan` works without env vars when Ollama is running locally
- [ ] Dimension is correctly detected as 1024 for mxbai-embed-large
- [ ] SKILL.md is under 50 lines
- [ ] All skill references/ files exist and are linked
- [ ] Error messages include actionable guidance
- [ ] All existing tests pass
- [ ] New integration test for auto-detection flow passes

## Task Summary

| ID | Phase | Title | Priority |
|----|-------|-------|----------|
| MPRSKL-1001 | 1 | Add from_env_with_provider() to EmbeddingConfig | Critical |
| MPRSKL-1002 | 1 | Update factory.rs to propagate provider | Critical |
| MPRSKL-2001 | 2 | Create brief SKILL.md | High |
| MPRSKL-2002 | 2 | Create cli-reference.md | High |
| MPRSKL-2003 | 2 | Create troubleshooting.md | High |
| MPRSKL-3001 | 3 | Enhance dimension mismatch error | Medium |
| MPRSKL-3002 | 3 | Improve --generate-embeddings documentation | Medium |

## Recommended Execution Order

1. **Start with MPRSKL-1001** - Foundation for all other work
2. **Immediately follow with MPRSKL-1002** - Completes bug fix
3. **Parallel: MPRSKL-2001, 2002, 2003** - Documentation can be done together
4. **Then: MPRSKL-3001, 3002** - CLI improvements

Total estimated effort: 4-6 hours of implementation time
