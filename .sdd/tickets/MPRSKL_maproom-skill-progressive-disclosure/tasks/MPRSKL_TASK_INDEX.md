# MPRSKL Task Index

## Ticket Overview
**Ticket ID:** MPRSKL
**Title:** Maproom Skill Progressive Disclosure
**Total Tasks:** 7 (2 Phase 1, 3 Phase 2, 2 Phase 3)

## Task Summary

### Phase 1: Factory/Config Bug Fix (CRITICAL)
Critical bug fixes to enable zero-config Ollama workflow.

| Task ID | Title | Agent | Est. Hours | Status |
|---------|-------|-------|-----------|--------|
| MPRSKL.1001 | Add from_env_with_provider() to EmbeddingConfig | rust-indexer-engineer | 2-3 | Not Started |
| MPRSKL.1002 | Update factory.rs to propagate provider | rust-indexer-engineer | 1-2 | Not Started |

**Phase 1 Objective:** Fix dimension mismatch bug when Ollama is auto-detected without explicit configuration.

**Phase 1 Dependencies:** None - foundation for all other work

**Phase 1 Deliverables:**
- `EmbeddingConfig::from_env_with_provider()` method
- Updated factory.rs Ollama branch
- Unit and integration tests

---

### Phase 2: Progressive Disclosure Skill Restructure
Reorganize maproom skill documentation for optimal AI agent consumption.

| Task ID | Title | Agent | Est. Hours | Status |
|---------|-------|-------|-----------|--------|
| MPRSKL.2001 | Create brief SKILL.md | general | 1-2 | Not Started |
| MPRSKL.2002 | Create cli-reference.md | general | 2-3 | Not Started |
| MPRSKL.2003 | Create troubleshooting.md | general | 1-2 | Not Started |

**Phase 2 Objective:** Restructure skill docs with brief SKILL.md and detailed references for progressive disclosure.

**Phase 2 Dependencies:**
- MPRSKL.1001, MPRSKL.1002 (Phase 1 bug fix should be mentioned in troubleshooting)

**Phase 2 Deliverables:**
- Brief SKILL.md (under 50 lines)
- Complete CLI reference documentation
- Troubleshooting guide

---

### Phase 3: CLI Error Enhancement
Improve error messages and CLI documentation for better user experience.

| Task ID | Title | Agent | Est. Hours | Status |
|---------|-------|-------|-----------|--------|
| MPRSKL.3001 | Enhance dimension mismatch error | rust-indexer-engineer | 1-2 | Not Started |
| MPRSKL.3002 | Improve --generate-embeddings documentation | rust-indexer-engineer | 0.5-1 | Not Started |

**Phase 3 Objective:** Improve error messages to include configuration context and actionable guidance.

**Phase 3 Dependencies:**
- MPRSKL.1001, MPRSKL.1002 (Phase 1 bug fix reduces error occurrence)
- MPRSKL.2003 (error can reference troubleshooting guide)

**Phase 3 Deliverables:**
- Enhanced dimension mismatch error with context and solutions
- Improved CLI help text for --generate-embeddings flag

---

## Task Details

### MPRSKL.1001 - Add from_env_with_provider() to EmbeddingConfig
**File:** `MPRSKL.1001_add-from-env-with-provider.md`
**Summary:** Add new public method to enable factory-detected providers to be correctly propagated during configuration loading.
**Key Changes:**
- New `from_env_with_provider(provider: Option<Provider>)` method
- Refactor `from_env()` to delegate to new method
- Unit tests for provider override behavior

---

### MPRSKL.1002 - Update factory.rs to propagate provider
**File:** `MPRSKL.1002_update-factory-provider-propagation.md`
**Summary:** Update Ollama branch to call `from_env_with_provider(Some(Provider::Ollama))` to fix dimension mismatch.
**Key Changes:**
- Modify Ollama branch in `create_provider_from_env()`
- Add integration test for auto-detection flow
- Verify zero-config workflow works

---

### MPRSKL.2001 - Create brief SKILL.md
**File:** `MPRSKL.2001_create-brief-skill-md.md`
**Summary:** Rewrite SKILL.md to under 50 lines with progressive disclosure pattern.
**Key Changes:**
- Brief capability summary
- When-to-use table (maproom vs grep vs glob)
- Quick reference commands
- Links to detailed references

---

### MPRSKL.2002 - Create cli-reference.md
**File:** `MPRSKL.2002_create-cli-reference.md`
**Summary:** Create comprehensive CLI reference covering all commands, flags, and options.
**Key Changes:**
- Document all crewchief-maproom commands
- Organize by category (search, index, manage, daemon)
- Include examples for each command
- Document environment variables

---

### MPRSKL.2003 - Create troubleshooting.md
**File:** `MPRSKL.2003_create-troubleshooting.md`
**Summary:** Create troubleshooting guide with common errors, causes, and solutions.
**Key Changes:**
- Document dimension mismatch error (with Phase 1 fix reference)
- Document repository not found error
- Document embeddings unavailable error
- Configuration verification steps

---

### MPRSKL.3001 - Enhance dimension mismatch error
**File:** `MPRSKL.3001_enhance-dimension-mismatch-error.md`
**Summary:** Improve error message to include configuration context and actionable solutions.
**Key Changes:**
- Add config context to error message
- List 3 specific solutions with commands
- Format for readability
- Test error message content

---

### MPRSKL.3002 - Improve --generate-embeddings documentation
**File:** `MPRSKL.3002_improve-generate-embeddings-docs.md`
**Summary:** Enhance help text for --generate-embeddings flag to explain purpose and usage.
**Key Changes:**
- Clarify what embeddings enable (vector search)
- Explain when to skip (config issues, FTS-only)
- Show both flag syntaxes (--generate-embeddings=false, --no-generate-embeddings)

---

## Execution Order

**Recommended sequence:**
1. **MPRSKL.1001** (Foundation) - Add new config method
2. **MPRSKL.1002** (Complete fix) - Update factory to use new method
3. **MPRSKL.2001, MPRSKL.2002, MPRSKL.2003** (Parallel) - Documentation restructure
4. **MPRSKL.3001, MPRSKL.3002** (Polish) - CLI improvements

**Critical path:** MPRSKL.1001 → MPRSKL.1002 (bug fix must be sequential)

**Parallel work:** Phase 2 tasks can be done simultaneously after Phase 1 completes

---

## Success Metrics

### Phase 1 Success
- [ ] `crewchief-maproom scan` works without env vars when Ollama is running
- [ ] Dimension correctly detected as 1024 for mxbai-embed-large
- [ ] All existing tests pass
- [ ] New integration test for auto-detection passes

### Phase 2 Success
- [ ] SKILL.md is under 50 lines
- [ ] All references/ files exist and are linked
- [ ] Content is accurate for current CLI
- [ ] Links resolve correctly

### Phase 3 Success
- [ ] Error messages include actionable guidance
- [ ] --generate-embeddings help text is clear
- [ ] Users can self-diagnose configuration issues

---

## Files Affected

### Rust Code
- `crates/maproom/src/embedding/config.rs` (MPRSKL.1001)
- `crates/maproom/src/embedding/factory.rs` (MPRSKL.1002)
- `crates/maproom/src/embedding/error.rs` (MPRSKL.3001)
- `crates/maproom/src/main.rs` (MPRSKL.3002)

### Documentation
- `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/SKILL.md` (MPRSKL.2001)
- `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/cli-reference.md` (MPRSKL.2002 - new)
- `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/troubleshooting.md` (MPRSKL.2003 - new)

---

## Risk Summary

### High Priority Risks
- **Breaking config API**: Mitigated by making `from_env()` a wrapper
- **Test failures**: Mitigated by incremental testing and CI validation
- **Skill structure breaks plugin**: Mitigated by preserving YAML frontmatter

### Medium Priority Risks
- **Documentation becomes stale**: Mitigated by validation against current CLI
- **Integration test flakiness**: Mitigated by `#[ignore]` attribute

### Low Priority Risks
- **Error message too verbose**: Mitigated by review and iteration
- **Line count exceeds 50**: Mitigated by ruthless brevity

---

## Total Estimated Effort
**8-14 hours** across all 7 tasks

**Breakdown:**
- Phase 1 (Critical): 3-5 hours
- Phase 2 (High): 4-7 hours
- Phase 3 (Medium): 1.5-3 hours

---

## Notes
- Phase 1 must complete before Phase 2 (troubleshooting doc references the fix)
- Phase 2 tasks can be parallelized
- Phase 3 builds on both previous phases
- All tasks include verification requirements and test coverage specifications
