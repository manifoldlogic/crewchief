# Project: Make mxbai-embed-large the Default Model

**Slug:** MXBAI
**Status:** Planning
**Created:** 2025-12-03

## Summary

Update all default model references from nomic-embed-text (768-dim) to mxbai-embed-large (1024-dim) across Rust code and documentation. This enables the better model by default, especially for the VSCode extension where "no configuration should be required."

**Key Changes**:
- Update 3 Rust constants (ollama.rs, factory.rs)
- Update test assertions to match new defaults
- Update documentation to show mxbai-embed-large as default
- Create migration guide for existing users

**Duration**: 3-5 hours

## Problem Statement

After successfully adding mxbai-embed-large support (DIM1024 project), it remains opt-in via configuration. Users must explicitly set environment variables to use the superior model. The VSCode extension especially should work zero-config with the best available model.

**Why mxbai-embed-large is better**:
- No tokenization crashes on special characters (|, [], (), Unicode)
- No content sanitization needed (better embedding quality)
- Better overall embedding quality (MTEB benchmarks)
- Handles all content types without workarounds

**Current problem**:
- Default is still nomic-embed-text (768-dim)
- Requires explicit configuration to use mxbai-embed-large
- VSCode users need to set env vars (not zero-config)
- Documentation shows old model as default

## Proposed Solution

**Configuration Changes** (Rust):
1. `ollama.rs` line 116: `DEFAULT_MODEL = "mxbai-embed-large"`
2. `ollama.rs` line 270: `default_config()` dimension = 1024
3. `factory.rs` line 210: fallback = "mxbai-embed-large"

**Test Updates**:
- Update assertions to expect new defaults
- Verify backward compatibility (explicit nomic config still works)

**Documentation Updates**:
- `docs/providers/ollama-setup.md`: Update examples
- `crates/maproom/CLAUDE.md`: Update default references
- `packages/vscode-maproom/README.md`: Update setup instructions
- `README.md`: Update quickstart (if applicable)
- Create `docs/guides/migrating-to-mxbai.md`: Migration guide

**No Code Changes Needed**:
- VSCode extension (relies on Rust defaults)
- MCP server (relies on Rust defaults)
- Database schema (vec_code_1024 already exists)

## Relevant Agents

**Planning & Execution:**
- project-planner (planning phase) - Complete
- ticket-creator (ticket generation) - Next step

**Implementation:**
- rust-developer (Rust constant changes, test updates)
- unit-test-runner (test execution)
- documentation-writer (docs updates, migration guide)

**Verification & Commit:**
- verify-ticket (acceptance criteria)
- commit-ticket (atomic commits)

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis, locations requiring changes, success criteria
- [architecture.md](planning/architecture.md) - Design decisions, component changes, data flow
- [plan.md](planning/plan.md) - 2-phase execution plan, validation, timeline (3-5 hours)
- [quality-strategy.md](planning/quality-strategy.md) - Testing approach, critical paths, quality gates
- [security-review.md](planning/security-review.md) - Security assessment (no security concerns)

## Execution Phases

### Phase 1: Code and Test Updates (1-2 hours)
**Objective**: Update Rust constants and test assertions

**Deliverables**:
- Update ollama.rs DEFAULT_MODEL and default_config() dimension
- Update factory.rs fallback model
- Update test assertions to expect new defaults
- Run test suite, verify all pass

**Agent**: rust-developer, unit-test-runner

**Success Criteria**:
- All tests pass (`cargo test` exit code 0)
- Zero-config uses mxbai-embed-large
- Explicit nomic-embed-text config still works

### Phase 2: Documentation Updates (2-3 hours)
**Objective**: Update all docs to reflect new defaults

**Deliverables**:
- Update ollama-setup.md, CLAUDE.md, README.md examples
- Create comprehensive migration guide
- Update .env.example

**Agent**: documentation-writer

**Success Criteria**:
- All docs show mxbai-embed-large as default
- Migration guide complete
- No conflicting default references

## Key Design Decisions

1. **Update constants only**: No architectural changes, just configuration
2. **Preserve backward compatibility**: Explicit nomic-embed-text config still works
3. **Zero-config VSCode**: Extension automatically uses new defaults (no code changes needed)
4. **Comprehensive documentation**: Migration guide addresses existing user concerns
5. **Test all paths**: Both zero-config and explicit config scenarios

## Success Criteria

**Functional**:
- [ ] Zero-config uses mxbai-embed-large (1024-dim)
- [ ] VSCode extension works without configuration
- [ ] Explicit nomic-embed-text config still works
- [ ] Conditional sanitization preserved

**Quality**:
- [ ] All tests pass
- [ ] Documentation consistent
- [ ] Migration guide complete
- [ ] No breaking changes

**User Experience**:
- [ ] Fresh installs work zero-config
- [ ] Existing users unaffected (explicit config preserved)
- [ ] Clear migration path documented

## Locations Requiring Changes

**Rust Code** (3 lines):
- `/workspace/crates/maproom/src/embedding/ollama.rs` line 116, 270
- `/workspace/crates/maproom/src/embedding/factory.rs` line 210

**Tests** (multiple assertions):
- `/workspace/crates/maproom/src/embedding/ollama.rs` (test section)
- `/workspace/crates/maproom/src/embedding/factory.rs` (test section)

**Documentation** (5-6 files):
- `/workspace/docs/providers/ollama-setup.md`
- `/workspace/crates/maproom/CLAUDE.md`
- `/workspace/README.md`
- `/workspace/packages/vscode-maproom/README.md`
- `/workspace/crates/maproom/.env.example`
- `/workspace/docs/guides/migrating-to-mxbai.md` (new file)

**No Changes Needed**:
- packages/vscode-maproom/ (relies on Rust defaults)
- packages/maproom-mcp/ (relies on Rust defaults)
- Database schema (vec_code_1024 already exists from DIM1024)

## Dependencies

**Completed**:
- DIM1024 project (vec_code_1024 table, configurable dimensions, conditional sanitization)
- Full test coverage for 1024-dim support
- Mixed dimension search validated

**External**:
- None (all infrastructure exists)

## Timeline

| Phase | Duration | Deliverables |
|-------|----------|--------------|
| Phase 1: Code & Tests | 1-2 hours | Rust changes, test updates, test pass |
| Phase 2: Documentation | 2-3 hours | Doc updates, migration guide |
| **Total** | **3-5 hours** | All changes complete |

## Next Steps

1. **Review planning** (recommended): `/workstream:project-review MXBAI`
2. **Generate tickets**: `/workstream:project-tickets MXBAI`
3. **Execute**: `/workstream:project-work MXBAI` or `/workstream:ticket MXBAI-1001`

## Context

**Background**: The DIM1024 project successfully added mxbai-embed-large (1024-dim) support but intentionally left defaults unchanged for backward compatibility during implementation. Now that the feature is complete and tested, we need to make it the default.

**Why now**:
- Infrastructure complete and validated
- Better user experience (zero-config)
- Superior model quality (no crashes, no sanitization)
- VSCode extension specifically requires zero-config

**Related Projects**:
- DIM1024 (completed): Added 1024-dim support
- EMBPERF (archived): Validated mxbai-embed-large performance
- LOCAL (archived): Documented mxbai-embed-large as alternative

## Risk Assessment

**Low Risk Project**:
- Configuration changes only (no algorithmic changes)
- Full backward compatibility maintained
- Comprehensive testing strategy
- Clear rollback plan

**Primary Risk**: Breaking existing users with explicit config
**Mitigation**: Preserve all explicit configuration, test backward compat

## Resources

**Code References**:
- `crates/maproom/src/embedding/ollama.rs` - OllamaProvider implementation
- `crates/maproom/src/embedding/factory.rs` - Provider auto-detection
- Migration #10 (vec_code_1024) - Database support for 1024-dim

**Documentation**:
- `docs/providers/ollama-setup.md` - Ollama setup guide
- `crates/maproom/CLAUDE.md` - Developer reference
- DIM1024 project docs - Context on 1024-dim implementation

**External**:
- [mxbai-embed-large on Ollama](https://ollama.com/library/mxbai-embed-large)
- [MTEB Leaderboard](https://huggingface.co/spaces/mteb/leaderboard) - Model benchmarks
