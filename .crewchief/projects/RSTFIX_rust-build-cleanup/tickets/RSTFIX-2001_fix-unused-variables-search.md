# Ticket: RSTFIX-2001: Fix Unused Variables in Search Executors

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
Fix unused variable warnings in the search module files (fts.rs, vector.rs, graph.rs, signals.rs) - approximately 12 warnings related to variables like `chunk_id`, `store`, `max_depth`, and `options`.

## Background
The search module contains numerous unused variables that were left behind from the PostgreSQL → SQLite migration (SQLIMPL project). Many variables are declared but never used in the new SQLite implementation. This is Phase 2 of the RSTFIX cleanup project.

Reference: Phase 2 in planning/plan.md - "Unused Variables (Medium Risk)"

## Acceptance Criteria
- [ ] No unused variable warnings in `src/search/fts.rs`
- [ ] No unused variable warnings in `src/search/vector.rs`
- [ ] No unused variable warnings in `src/search/graph.rs`
- [ ] No unused variable warnings in `src/search/signals.rs`
- [ ] All 906 tests pass

## Technical Requirements
- For truly unused variables: prefix with `_` (e.g., `_store`, `_chunk_id`)
- For variables that SHOULD be used (incomplete wiring): investigate and either wire up or document as TODO
- Do NOT remove variables that are part of function signatures (use `_` prefix instead)
- Preserve behavior - this is cleanup only

## Implementation Notes
**Decision tree for each unused variable:**
1. Is it a function parameter? → Prefix with `_` (e.g., `_options`)
2. Is it obviously unused (e.g., debug remnant)? → Remove the declaration entirely
3. Does it suggest incomplete implementation? → Prefix with `_` and add TODO comment

**Common patterns from SQLIMPL migration:**
- Variables capturing query results that are now handled differently
- Store references that are no longer needed
- Configuration options that aren't yet implemented

**Variables likely affected:**
- `chunk_id` in multiple search functions
- `store` references passed but unused
- `max_depth` in graph traversal
- `options` in various executors

## Dependencies
- RSTFIX-1001: Auto-fix imports (import removal may surface additional variable issues)

## Risk Assessment
- **Risk**: Medium - some unused variables may indicate incomplete implementation
  - **Mitigation**: Use `_` prefix rather than deletion; document any suspicious findings
- **Risk**: Variable may actually be used in macro expansion
  - **Mitigation**: Run full test suite after each file change

## Files/Packages Affected
- `crates/maproom/src/search/fts.rs`
- `crates/maproom/src/search/vector.rs`
- `crates/maproom/src/search/graph.rs`
- `crates/maproom/src/search/signals.rs`
