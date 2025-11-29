# BRANCHX Ticket Index

**Project**: Branch-Aware Indexing
**Total Tickets**: 15 (14 implementation + 1 critical path test)
**Timeline**: 5-6 days

## Overview

This project enables worktree-specific indexing and incremental updates using git tree SHA. The implementation is organized into 5 phases:

1. **Phase 1: Worktree Tracking Schema** (Days 1-2) - Database foundation
2. **Phase 2: Git Integration** (Days 2-3) - Tree SHA and diff-tree
3. **Phase 3: Incremental Update Logic** (Days 3-4) - Core algorithm
4. **Phase 4: CLI Updates** (Day 5) - User-facing features
5. **Phase 5: Documentation** (Day 6) - Architecture and usage docs

## Phase 1: Worktree Tracking Schema (BRANCHX-1001 to 1003)

### BRANCHX-1001: Add worktree_ids JSONB column to chunks table
- **Agent**: database-engineer
- **Status**: Pending
- **Files**: `packages/maproom-mcp/migrations/004_add_worktree_tracking.sql`
- **Summary**: Add JSONB column to track which worktrees contain each chunk, with GIN index for efficient queries
- **Dependencies**: BLOBSHA complete
- **Blocks**: BRANCHX-1002, 1008

### BRANCHX-1002: Create worktree_index_state table for tree SHA tracking
- **Agent**: database-engineer
- **Status**: Pending
- **Files**: `packages/maproom-mcp/migrations/004_add_worktree_tracking.sql` (append)
- **Summary**: Create table to track the last indexed git tree SHA for each worktree
- **Dependencies**: BRANCHX-1001 (can be in same migration)
- **Blocks**: BRANCHX-1005, 1007

### BRANCHX-1003: Test worktree tracking schema and JSONB queries
- **Agent**: unit-test-runner
- **Status**: Pending
- **Files**: `packages/maproom-mcp/tests/jsonb-queries.test.ts`
- **Summary**: Verify migration 004 succeeded and test JSONB query patterns for worktree filtering
- **Dependencies**: BRANCHX-1001, 1002
- **Blocks**: BRANCHX-1901

## Phase 2: Git Integration (BRANCHX-1004 to 1006)

### BRANCHX-1004: Implement git tree SHA and diff-tree functions
- **Agent**: rust-indexer-engineer
- **Status**: Pending
- **Files**: `crates/maproom/src/git.rs` (new)
- **Summary**: Create Rust functions to get git tree SHA and find changed files between two tree states
- **Dependencies**: Phase 1 complete
- **Blocks**: BRANCHX-1005, 1006, 1007

### BRANCHX-1005: Implement worktree index state database functions
- **Agent**: rust-indexer-engineer
- **Status**: Pending
- **Files**: `crates/maproom/src/index_state.rs` (new)
- **Summary**: Create Rust functions to query and update the worktree_index_state table
- **Dependencies**: BRANCHX-1002, 1004
- **Blocks**: BRANCHX-1007

### BRANCHX-1006: Test git integration and index state functions
- **Agent**: unit-test-runner
- **Status**: Pending
- **Files**: `crates/maproom/tests/git_integration.rs` (new)
- **Summary**: Verify git tree SHA detection, diff-tree parsing, and database state management
- **Dependencies**: BRANCHX-1004, 1005
- **Blocks**: BRANCHX-1901

## Phase 3: Incremental Update Logic (BRANCHX-1007 to 1010)

### BRANCHX-1007: Implement incremental update algorithm
- **Agent**: rust-indexer-engineer
- **Status**: Pending
- **Files**: `crates/maproom/src/incremental.rs` (new)
- **Summary**: Create the core incremental update function that compares tree SHA, finds changed files, and processes only changes
- **Dependencies**: BRANCHX-1004, 1005, 1006
- **Blocks**: BRANCHX-1008, 1009, 1011

### BRANCHX-1008: Update chunk upsert to track worktree_ids
- **Agent**: rust-indexer-engineer
- **Status**: Pending
- **Files**: `crates/maproom/src/upsert.rs` (new or modify)
- **Summary**: Modify the chunk upsert function to add worktree IDs to the worktree_ids JSONB array
- **Dependencies**: BRANCHX-1001, BLOBSHA, 1007
- **Blocks**: BRANCHX-1010

### BRANCHX-1009: Handle file deletions in incremental updates
- **Agent**: rust-indexer-engineer
- **Status**: Pending
- **Files**: `crates/maproom/src/incremental.rs` (add function)
- **Summary**: Remove worktree from chunks when files are deleted, and optionally clean up orphan chunks
- **Dependencies**: BRANCHX-1007, 1008
- **Blocks**: BRANCHX-1010

### BRANCHX-1010: Test incremental update logic and correctness
- **Agent**: unit-test-runner
- **Status**: Pending
- **Files**: `crates/maproom/tests/incremental_update.rs` (new)
- **Summary**: **CRITICAL** - Verify that incremental updates produce identical results to full scans
- **Dependencies**: BRANCHX-1007, 1008, 1009
- **Blocks**: BRANCHX-1011, 1901

## Phase 4: CLI Updates (BRANCHX-1011 to 1013)

### BRANCHX-1011: Update scan command to use incremental updates
- **Agent**: rust-indexer-engineer
- **Status**: Pending
- **Files**: `crates/maproom/src/cli.rs` (modify)
- **Summary**: Modify the maproom scan CLI command to use incremental_update by default, with --force flag for full scans
- **Dependencies**: BRANCHX-1007, 1010
- **Blocks**: BRANCHX-1013

### BRANCHX-1012: Add worktree filtering to MCP search
- **Agent**: general-purpose
- **Status**: Pending
- **Files**: `packages/maproom-mcp/src/search.ts` (modify)
- **Summary**: Update the MCP search tool to accept a worktree parameter and filter results by worktree_ids
- **Dependencies**: BRANCHX-1001, 1011
- **Blocks**: BRANCHX-1013

### BRANCHX-1013: E2E test for branch switch workflow
- **Agent**: unit-test-runner
- **Status**: Pending
- **Files**: `packages/maproom-mcp/tests/e2e/branch-workflow.test.ts` (new)
- **Summary**: Create end-to-end test verifying the complete workflow: index branch, switch branch, incremental update, search with filtering
- **Dependencies**: BRANCHX-1011, 1012, all Phase 1-3 tickets
- **Blocks**: BRANCHX-1014, 1901

## Phase 5: Documentation (BRANCHX-1014)

### BRANCHX-1014: Document branch-aware indexing architecture and usage
- **Agent**: general-purpose
- **Status**: Pending
- **Files**: `docs/architecture/branch-aware-indexing.md` (new), `CHANGELOG.md` (update)
- **Summary**: Create comprehensive documentation covering architecture, usage, migration guide, and CHANGELOG entry
- **Dependencies**: All Phase 1-4 tickets, BRANCHX-1013
- **Blocks**: None (final ticket)

## Critical Path Tests (BRANCHX-1901)

### BRANCHX-1901: Critical path test suite validation
- **Agent**: unit-test-runner
- **Status**: Pending
- **Files**: `crates/maproom/tests/critical_path_suite.rs` (new)
- **Summary**: **MUST PASS** - Run and validate the 4 most critical tests before merging:
  1. test_incremental_equals_full_scan (correctness)
  2. test_tree_sha_skip_unchanged (performance <100ms)
  3. test_worktree_filtering (query correctness)
  4. test_git_diff_tree_detection (change detection)
- **Dependencies**: BRANCHX-1003, 1006, 1010, 1013
- **Blocks**: Merge to main

## Execution Order

### Sequential Dependencies (Must Complete in Order)

1. **Foundation** (Days 1-2):
   - BRANCHX-1001 → BRANCHX-1002 → BRANCHX-1003

2. **Git Layer** (Days 2-3):
   - BRANCHX-1004 → BRANCHX-1005 → BRANCHX-1006

3. **Core Logic** (Days 3-4):
   - BRANCHX-1007 → BRANCHX-1008 → BRANCHX-1009 → BRANCHX-1010

4. **User Features** (Day 5):
   - BRANCHX-1011, BRANCHX-1012 (parallel) → BRANCHX-1013

5. **Finalization** (Day 6):
   - BRANCHX-1014 (documentation)
   - BRANCHX-1901 (critical path validation)

### Parallel Opportunities

- BRANCHX-1011 and BRANCHX-1012 can be developed in parallel (both depend on 1007, 1010)
- BRANCHX-1003, 1006 tests can run in parallel with next phase development

## Agent Assignments

- **database-engineer**: BRANCHX-1001, 1002
- **rust-indexer-engineer**: BRANCHX-1004, 1005, 1007, 1008, 1009, 1011
- **general-purpose**: BRANCHX-1012, 1014
- **unit-test-runner**: BRANCHX-1003, 1006, 1010, 1013, 1901

## Success Metrics

### Functional
- ✅ Worktree tracking works (chunks in multiple worktrees)
- ✅ Incremental updates only scan changed files
- ✅ Tree SHA optimization skips unchanged branches
- ✅ Search filtering by worktree works

### Performance
- ✅ Tree SHA check: <100ms
- ✅ Incremental update (20% changed): 5-10x faster than full scan
- ✅ Tree SHA skip (no changes): <1 second

### Quality
- ✅ All tests passing (BRANCHX-1901 critical path suite)
- ✅ Incremental = full scan (correctness verified)
- ✅ Branch isolation verified
- ✅ E2E workflow test passes

## Planning References

- **Project README**: `.crewchief/projects/BRANCHX_branch-aware-indexing/README.md`
- **Implementation Plan**: `.crewchief/projects/BRANCHX_branch-aware-indexing/planning/plan.md`
- **Architecture**: `.crewchief/projects/BRANCHX_branch-aware-indexing/planning/architecture.md`
- **Quality Strategy**: `.crewchief/projects/BRANCHX_branch-aware-indexing/planning/quality-strategy.md`
- **Analysis**: `.crewchief/projects/BRANCHX_branch-aware-indexing/planning/analysis.md`

## Notes

- All Phase 1-3 tickets use sequential numbering (1001-1010) even though logically grouped into phases
- Test tickets use 19xx numbering to indicate their cross-cutting nature
- BRANCHX-1901 (critical path suite) must pass before merging to main
- The project depends on BLOBSHA being complete (content-addressed storage)
- Migration 004 is critical - must be tested thoroughly before production deployment
