# MXBAI Ticket Index

Project: Make mxbai-embed-large the Default Model

## Overview

This index tracks all tickets for the MXBAI project, organized by phase. The project updates default model references from nomic-embed-text (768-dim) to mxbai-embed-large (1024-dim) across Rust code, TypeScript integration layers, and documentation.

**Total Tickets**: 10 (6 in Phase 1, 4 in Phase 2)
**Estimated Time**: 5-7 hours total

## Phase 1: Code and Test Updates (2-3 hours)

Phase 1 updates Rust and TypeScript constants and test assertions to use mxbai-embed-large as default.

### MXBAI-1001: Update Rust Constants
**Status**: Not started
**Agent**: rust-indexer-engineer
**Duration**: 30 min
**Summary**: Change DEFAULT_MODEL constant, default_config() dimension, and factory fallback in ollama.rs and factory.rs
**Files**:
- `crates/maproom/src/embedding/ollama.rs` (lines 116, 270)
- `crates/maproom/src/embedding/factory.rs` (line 210)

### MXBAI-1002: Update TypeScript Constants
**Status**: Not started
**Agent**: vscode-extension-specialist
**Duration**: 30 min
**Summary**: Change DEFAULT_EMBEDDING_MODEL in VSCode extension and model validation in MCP server
**Files**:
- `packages/vscode-maproom/src/ollama/model-manager.ts` (line 16)
- `packages/maproom-mcp/src/utils/provider-detection.ts` (line 126)

### MXBAI-1003: Update Configuration Examples
**Status**: Not started
**Agent**: documentation-writer
**Duration**: 15 min
**Summary**: Update .env.example with new default model and dimension values
**Files**:
- `crates/maproom/.env.example` (lines 38, 44)

### MXBAI-1004: Update Rust Test Assertions
**Status**: Not started
**Agent**: rust-indexer-engineer
**Duration**: 60-90 min
**Summary**: Update 90+ Rust test assertions (15+ DEFAULT_MODEL, 37+ dimension, 50+ fixtures) and add backward compatibility test
**Files**:
- `crates/maproom/src/embedding/ollama.rs` (tests)
- `crates/maproom/src/embedding/factory.rs` (tests)
- Integration test files

**Dependencies**: MXBAI-1001

### MXBAI-1005: Update TypeScript Test Assertions
**Status**: Not started
**Agent**: vscode-extension-specialist
**Duration**: 30 min
**Summary**: Update TypeScript test assertions in VSCode extension and MCP server (18+ test updates)
**Files**:
- `packages/vscode-maproom/src/ollama/model-manager.test.ts`
- `packages/maproom-mcp/tests/provider-detection.test.ts`

**Dependencies**: MXBAI-1002

### MXBAI-1006: Phase 1 Verification Scan
**Status**: Not started
**Agent**: rust-indexer-engineer, unit-test-runner
**Duration**: 20 min
**Summary**: Run all test suites, verification grep scan, and manual CLI tests to confirm Phase 1 completion
**Files**: All files from MXBAI-1001 through MXBAI-1005 (verification only)

**Dependencies**: MXBAI-1001, MXBAI-1002, MXBAI-1003, MXBAI-1004, MXBAI-1005

**Quality Gate**: All tests must pass before proceeding to Phase 2

## Phase 2: Documentation Updates (3-4 hours)

Phase 2 updates all documentation to reflect new defaults and provides migration guidance.

### MXBAI-2001: Documentation Audit
**Status**: Not started
**Agent**: documentation-writer
**Duration**: 15 min
**Summary**: Categorize 132+ .md files with "nomic-embed-text" into must-update (7 files) vs preserve (125+ files)
**Files**: All .md files (audit only, no changes)

**Dependencies**: MXBAI-1006 (Phase 1 complete)

### MXBAI-2002: Update Active Documentation
**Status**: Not started
**Agent**: documentation-writer
**Duration**: 60 min
**Summary**: Update 7 active documentation files to show mxbai-embed-large as default
**Files**:
- `docs/providers/ollama-setup.md`
- `crates/maproom/CLAUDE.md`
- `README.md`
- `packages/vscode-maproom/README.md`
- `packages/maproom-mcp/README.md`
- `crates/maproom/.env.example` (verification)

**Dependencies**: MXBAI-2001

### MXBAI-2003: Create Migration Guide
**Status**: Not started
**Agent**: documentation-writer
**Duration**: 90-120 min
**Summary**: Create comprehensive migration guide with 7 required sections (executive summary, zero-config path, explicit config, re-embedding, storage impact, FAQ, model comparison)
**Files**:
- `docs/guides/migrating-to-mxbai.md` (new file)

**Dependencies**: MXBAI-2001

### MXBAI-2004: Documentation Consistency Check
**Status**: Not started
**Agent**: documentation-writer
**Duration**: 15 min
**Summary**: Final grep scan and consistency verification across all documentation
**Files**: All documentation (verification only)

**Dependencies**: MXBAI-2001, MXBAI-2002, MXBAI-2003

**Quality Gate**: Documentation must be consistent before project completion

## Success Metrics

### Phase 1 Success Criteria
- [ ] All Rust tests pass (`cargo test -p crewchief-maproom` exit 0)
- [ ] All TypeScript tests pass (`pnpm test` in vscode-maproom and maproom-mcp, exit 0)
- [ ] Zero-config CLI uses mxbai-embed-large (verified via manual test)
- [ ] Explicit nomic-embed-text config still works (verified via manual test)
- [ ] Verification scan shows only expected references

### Phase 2 Success Criteria
- [ ] All 7 active documentation files updated
- [ ] Migration guide complete with all required sections
- [ ] No conflicting default references in documentation
- [ ] Archived documentation untouched
- [ ] Example commands tested and working

### Overall Success Criteria
- [ ] Zero test failures
- [ ] Zero breaking changes
- [ ] Zero documentation inconsistencies
- [ ] Backward compatibility verified in all layers (Rust, VSCode, MCP)

## Workflow Notes

**Phase 1 Execution Order**:
1. MXBAI-1001, MXBAI-1002, MXBAI-1003 (can be parallel)
2. MXBAI-1004, MXBAI-1005 (after code changes, can be parallel)
3. MXBAI-1006 (must be last in Phase 1)

**Phase 2 Execution Order**:
1. MXBAI-2001 (must be first)
2. MXBAI-2002, MXBAI-2003 (after audit, can be parallel)
3. MXBAI-2004 (must be last)

**Agent Assignments**:
- rust-indexer-engineer: MXBAI-1001, MXBAI-1004, MXBAI-1006
- vscode-extension-specialist: MXBAI-1002, MXBAI-1005
- documentation-writer: MXBAI-1003, MXBAI-2001, MXBAI-2002, MXBAI-2003, MXBAI-2004
- unit-test-runner: Test execution in MXBAI-1004, MXBAI-1005, MXBAI-1006
- verify-ticket: All tickets
- commit-ticket: All tickets

## Dependencies Summary

**External Dependencies**:
- DIM1024 project (completed - vec_code_1024 table exists)
- None other

**Inter-ticket Dependencies**:
- MXBAI-1004 depends on MXBAI-1001
- MXBAI-1005 depends on MXBAI-1002
- MXBAI-1006 depends on MXBAI-1001 through MXBAI-1005
- MXBAI-2001 depends on MXBAI-1006
- MXBAI-2002 depends on MXBAI-2001
- MXBAI-2003 depends on MXBAI-2001
- MXBAI-2004 depends on MXBAI-2001, MXBAI-2002, MXBAI-2003

## Risk Mitigation

**High-Risk Tickets**:
- MXBAI-1004: 90+ test updates, high chance of missing locations
  - Mitigation: Grep audit upfront, verification scan in MXBAI-1006
- MXBAI-1006: Critical quality gate
  - Mitigation: Comprehensive test execution and manual verification
- MXBAI-2003: Large, complex document
  - Mitigation: Follow specification exactly, test all examples

**Medium-Risk Tickets**:
- MXBAI-2002: Multiple files to update consistently
  - Mitigation: MXBAI-2004 consistency check
- MXBAI-2004: Must catch all inconsistencies
  - Mitigation: Multiple grep patterns, manual verification

## Notes

- Ticket numbering follows phase-based convention (1xxx for Phase 1, 2xxx for Phase 2)
- All tickets include verify-ticket and commit-ticket agents in workflow
- Tests must be executed and passing (not just present) for "Tests pass" checkbox
- Documentation-only tickets use "Tests pass - N/A"
- Each ticket includes specific file paths and line numbers from planning docs
