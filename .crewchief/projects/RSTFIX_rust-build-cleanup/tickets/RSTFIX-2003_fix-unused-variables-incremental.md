# Ticket: RSTFIX-2003: Fix Unused Variables in Incremental Module

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Fix unused variable warnings in the incremental indexing module including detector.rs, edge_updater.rs, processor.rs, and tree_sha_update.rs. Approximately 8-10 warnings related to incremental processing state and edge detection variables.

## Background
The incremental module handles efficient re-indexing by detecting file changes and updating only affected code chunks. Several variables were declared for planned features or left behind from refactoring. This is Phase 2 of the RSTFIX cleanup project.

Reference: Phase 2 in planning/plan.md - "Unused Variables (Medium Risk)"

## Acceptance Criteria
- [ ] No unused variable warnings in `src/incremental/detector.rs`
- [ ] No unused variable warnings in `src/incremental/edge_updater.rs`
- [ ] No unused variable warnings in `src/incremental/processor.rs`
- [ ] No unused variable warnings in `src/incremental/tree_sha_update.rs`
- [ ] All 906 tests pass

## Technical Requirements
- For truly unused variables: prefix with `_`
- Pay special attention to variables that track incremental state - may indicate incomplete implementation
- Do NOT modify the core incremental detection logic
- Preserve behavior - this is cleanup only

## Implementation Notes
**Files and likely issues:**
- `incremental/detector.rs` - File change detection variables
- `incremental/edge_updater.rs` - Code relationship edge update variables
- `incremental/processor.rs` - Chunk processing state variables
- `incremental/tree_sha_update.rs` - Git tree SHA tracking variables

**Incremental module is performance-critical:**
- Variables tracking state may be for debugging/metrics
- Be conservative - prefer `_` prefix over removal
- If a variable captures timing or count data, check if it should be logged

**Common patterns:**
- `_old_sha`, `_new_sha` - May be for debugging
- `_affected_chunks` - May be for metrics
- `_edge_count` - May be for logging

## Dependencies
- RSTFIX-1001: Auto-fix imports must complete first
- Can run in parallel with RSTFIX-2001 and RSTFIX-2002 after imports are fixed

## Risk Assessment
- **Risk**: Medium - incremental module affects indexing performance and correctness
  - **Mitigation**: Run incremental-related tests specifically after changes
- **Risk**: Variables may be used for debugging in production issues
  - **Mitigation**: Check if variables are logged or traced before removing; prefer `_` prefix

## Files/Packages Affected
- `crates/maproom/src/incremental/detector.rs`
- `crates/maproom/src/incremental/edge_updater.rs`
- `crates/maproom/src/incremental/processor.rs`
- `crates/maproom/src/incremental/tree_sha_update.rs`
