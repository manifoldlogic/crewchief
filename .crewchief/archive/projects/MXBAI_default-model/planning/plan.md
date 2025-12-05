# Execution Plan: Make mxbai-embed-large the Default Model

## Overview

This is a focused configuration change project to update all default model references from nomic-embed-text (768-dim) to mxbai-embed-large (1024-dim). The work is organized into two phases: code/test updates and documentation.

**Total Estimated Time**: 5-7 hours

## Phase 1: Code and Test Updates

**Objective**: Update Rust and TypeScript constants and test assertions to use mxbai-embed-large as default

**Duration**: 2-3 hours

### Deliverables

1. **Update Rust Constants** (30 min)
   - Change `DEFAULT_MODEL` in ollama.rs line 116
   - Change dimension in ollama.rs line 270 (`default_config()`)
   - Change fallback in factory.rs line 210

2. **Update TypeScript Constants** (30 min)
   - Change `DEFAULT_EMBEDDING_MODEL` in vscode-maproom/src/ollama/model-manager.ts line 16
   - Change model validation in maproom-mcp/src/utils/provider-detection.ts line 126
   - Update warning message to suggest `ollama pull mxbai-embed-large`

3. **Update Configuration Examples** (15 min)
   - Change model in crates/maproom/.env.example line 38
   - Change dimension in crates/maproom/.env.example line 44

4. **Update Test Assertions** (90-120 min)
   - **Rust tests (60 min):**
     - Fix `test_ollama_provider_default_config()` in ollama.rs
     - Update 15+ DEFAULT_MODEL assertions
     - Update 37+ dimension assertions
     - Fix 50+ test fixtures with model references
   - **TypeScript tests (30 min):**
     - Update model-manager.test.ts (8+ assertions)
     - Update provider-detection.test.ts (10+ test cases)
     - Fix test mocks in both packages
   - Based on grep audit: 90+ total test updates required

5. **Run Test Suites** (20 min)
   - `cargo test -p crewchief-maproom` (Rust)
   - `pnpm test` in vscode-maproom and maproom-mcp (TypeScript)
   - Verify all tests pass
   - Check for any missed test failures

6. **Verification Scan** (10 min)
   - After code changes, re-grep for "nomic-embed-text" and "768"
   - Verify only expected references remain (backward compat sections)
   - Fail if unexpected references found

### Agent Assignments

- **rust-developer**: Make Rust code changes (ollama.rs, factory.rs, .env.example)
- **typescript-developer**: Make TypeScript changes (model-manager.ts, provider-detection.ts)
- **rust-developer**: Update Rust test assertions
- **typescript-developer**: Update TypeScript test assertions
- **unit-test-runner**: Execute test suites (Rust + TypeScript), report results

### Success Criteria

- [ ] All Rust constant changes applied
- [ ] All TypeScript constant changes applied
- [ ] Configuration examples updated
- [ ] All tests pass (Rust: `cargo test` exit 0, TypeScript: `pnpm test` exit 0)
- [ ] Backward compatibility verified (explicit nomic-embed-text still works)
- [ ] Verification scan shows no unexpected references

### Risks

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Tests fail due to missed assertion | High | Grep audit identifies 90+ test updates upfront |
| Breaking existing configs | Critical | Test explicit nomic-embed-text config in all layers |
| Missed code locations | High | Verification scan after changes, fail if references found |
| TypeScript/Rust inconsistency | High | Update both layers together, validate end-to-end flow |

## Phase 2: Documentation Updates

**Objective**: Update all documentation to reflect new defaults and provide migration guidance

**Duration**: 3-4 hours

### Deliverables

1. **Documentation Audit** (15 min)
   - Categorize 132 .md files with "nomic-embed-text" references:
     - **Must update** (7 files): Active documentation
     - **Preserve** (125+ files): Archived projects, historical context
   - Explicit rule: Do NOT update `.crewchief/archive/` or `.crewchief/projects/DIM1024_*`

2. **Update Example Code** (60 min)
   - Update `docs/providers/ollama-setup.md`: Replace nomic examples with mxbai (20 min)
   - Update `crates/maproom/CLAUDE.md`: Change default model references (15 min)
   - Update `packages/vscode-maproom/README.md`: Update setup instructions (10 min)
   - Update `packages/maproom-mcp/README.md`: Update MCP docs if model mentioned (10 min)
   - Update `README.md`: Check quickstart, examples, default mentions (15 min)

3. **Create Migration Guide** (90-120 min)
   - Create `docs/guides/migrating-to-mxbai.md`
   - Required sections (per architecture.md specification):
     1. Executive summary (why/what changed)
     2. Zero-config users section (no action needed)
     3. Explicit config users section (how to keep nomic-embed-text with env vars)
     4. Re-embedding guide with specific commands
     5. Storage impact calculator (33% increase explanation)
     6. Troubleshooting FAQ (8+ common issues)
     7. Model comparison table
   - Target audience: CLI users, VSCode users, MCP server users

4. **Documentation Consistency Check** (15 min)
   - Grep for remaining "nomic-embed-text" references
   - Verify only expected occurrences (migration guide, backward compat sections)
   - Ensure no conflicting default references

### Agent Assignments

- **documentation-writer**: Perform documentation audit
- **documentation-writer**: Update all 7 active doc files
- **documentation-writer**: Create comprehensive migration guide
- **documentation-writer**: Run consistency check

### Success Criteria

- [ ] Documentation audit complete (7 must-update, 125+ preserve identified)
- [ ] All 7 active docs updated to show mxbai-embed-large as default
- [ ] Migration guide complete with all 7 required sections
- [ ] No remaining nomic-embed-text default references (except in migration guide/backward compat sections)
- [ ] Backward compatibility documented with specific env var examples
- [ ] Consistency check passes (no conflicting references)

### Risks

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Inconsistent documentation | Medium | Documentation consistency check with grep audit |
| Accidentally updating archived docs | Medium | Explicit "do not update" rule for .crewchief/archive/ |
| Unclear migration guidance | High | Include specific examples, commands, and troubleshooting |
| Missing migration guide sections | Medium | Follow architecture.md specification (7 required sections) |

## Dependencies

### Completed Dependencies
- DIM1024 project (vec_code_1024 table, configurable dimensions, conditional sanitization)
- Full test coverage for 1024-dim support
- Mixed dimension search validated

### External Dependencies
- None (all infrastructure exists)

## Validation Plan

### Phase 1 Validation (After Code Changes)

**Unit Tests**:
```bash
cd crates/maproom
cargo test -p crewchief-maproom
```

**Manual CLI Test**:
```bash
# Verify default model without env vars
unset MAPROOM_EMBEDDING_MODEL
unset MAPROOM_EMBEDDING_DIMENSION
cargo run --bin crewchief-maproom -- status --repo test
# Should log: "Using provider: ollama (model: mxbai-embed-large, dimension: 1024)"
```

**Backward Compat Test**:
```bash
# Verify explicit nomic-embed-text still works
export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
export MAPROOM_EMBEDDING_DIMENSION=768
cargo run --bin crewchief-maproom -- status --repo test
# Should log: "Using provider: ollama (model: nomic-embed-text, dimension: 768)"
```

### Phase 2 Validation (After Documentation)

**Documentation Consistency**:
- Grep for remaining "nomic-embed-text" references:
  ```bash
  grep -r "nomic-embed-text" docs/ crates/maproom/CLAUDE.md README.md packages/vscode-maproom/README.md
  ```
- Verify only expected occurrences (migration guide, backward compat sections)

**Migration Guide Review**:
- Check all topics covered
- Verify code examples are correct
- Test commands in guide actually work

### End-to-End Validation

**VSCode Extension Test** (manual):
1. Fresh install of vscode-maproom extension
2. Open workspace with no existing config
3. Trigger indexing
4. Verify embeddings go to vec_code_1024 table
5. Verify search works across both old (768) and new (1024) embeddings

## Rollback Plan

### If Tests Fail in Phase 1

1. **Identify failure**: Check test output for specific assertion
2. **Fix assertion**: Update test to match new defaults
3. **Re-run tests**: Verify fix

### If Breaking Change Discovered

1. **Immediate revert**:
   ```bash
   git revert <commit-hash>
   ```
2. **Analyze issue**: Understand what broke
3. **Fix and re-apply**: Address issue, reapply changes

### If Documentation Issues Found

1. **Quick fix**: Documentation can be patched without rollback
2. **Update migration guide**: Add missing information
3. **No code rollback needed**: Docs separate from code

## Timeline

| Phase | Duration | Start | End |
|-------|----------|-------|-----|
| Phase 1: Code & Tests | 2-3 hours | T+0 | T+3h |
| Phase 2: Documentation | 3-4 hours | T+3h | T+7h |
| **Total** | **5-7 hours** | | |

**Assumptions**:
- Single developer (or rust-developer + typescript-developer working in parallel)
- No major issues discovered during implementation
- Test suite runs in < 10 minutes total (Rust + TypeScript)
- Realistic estimates based on grep audit findings (90+ test updates)

## Communication Plan

### Stakeholders

**Internal**:
- Development team: Review planning docs before execution
- QA validation: Pre-release testing (VSCode zero-config, backward compat)

**External**:
- VSCode extension users: Extension update notification (popup on first launch post-update)
- MCP server users: GitHub release notes, documentation banner
- CLI users: Release notes, migration guide link
- Direct users with explicit configs: Unaffected, no communication needed

### Timeline

**Pre-release (2 days before)**:
- Update all documentation
- Add banner to docs site (if applicable): "Default model changed to mxbai-embed-large"
- Prepare extension update notification message

**Release day**:
- VSCode extension update notification: "Default model upgraded to mxbai-embed-large for better quality. No action needed for most users. See migration guide if you prefer nomic-embed-text."
- GitHub release notes with migration guide link
- Update Slack/Discord (if applicable)

**Post-release (1 week)**:
- Monitor GitHub issues for breaking change reports
- Update FAQ based on user questions
- Track adoption metrics (if available)

### Release Notes Template

```markdown
## Default Model Change: mxbai-embed-large

**What Changed**: The default embedding model for Ollama provider changed from nomic-embed-text (768-dim) to mxbai-embed-large (1024-dim).

**Why**: mxbai-embed-large provides:
- Better embedding quality
- No crashes on special characters (|, [], (), Unicode)
- No content sanitization needed

**Action Required**: None for most users. Fresh installs automatically use the new model.

**If you want to keep using nomic-embed-text**:
```bash
export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
export MAPROOM_EMBEDDING_DIMENSION=768
```

**Migration Guide**: See docs/guides/migrating-to-mxbai.md

**Storage Impact**: ~33% more storage per embedding (1024 vs 768 floats)

**Model Size**: 670MB vs 274MB (one-time download)
```

## Success Metrics

### Technical Metrics
- [ ] All tests pass (exit code 0)
- [ ] Zero-config generates 1024-dim embeddings
- [ ] Explicit nomic-embed-text config still works
- [ ] Mixed dimension search verified

### Documentation Metrics
- [ ] Migration guide complete
- [ ] All docs consistent (no conflicting defaults)
- [ ] Backward compatibility documented

### User Experience Metrics
- [ ] Fresh VSCode install works without configuration
- [ ] No user reports of breaking changes
- [ ] Clear error messages if model unavailable

## Post-Deployment

### Monitoring

**Week 1**:
- Monitor GitHub issues for reports of breaking changes
- Check user feedback on extension updates
- Review any confusion around migration

**Week 2-4**:
- Track adoption (how many users switch vs stay on nomic)
- Monitor storage/performance feedback
- Update FAQ based on common questions

### Documentation Maintenance

- Keep migration guide updated based on user questions
- Add troubleshooting entries as issues arise
- Update comparison docs if new models added
