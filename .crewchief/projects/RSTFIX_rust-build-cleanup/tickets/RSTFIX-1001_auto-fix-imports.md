# Ticket: RSTFIX-1001: Auto-fix Imports and Cleanup Warnings

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
Use Rust's cargo fix to automatically remove ~15 unused imports, then manually clean up any remaining import warnings and cfg attribute warnings.

## Background
The crewchief-maproom Rust crate has accumulated ~17 unused import warnings during development, particularly after the PostgreSQL → SQLite migration (SQLIMPL project). This is the first phase of the RSTFIX cleanup project and must complete before Phase 2 (unused variables) since import removal may surface additional variable issues.

Reference: Phase 1 in planning/plan.md - "Unused Imports (Low Risk)"

## Acceptance Criteria
- [ ] `cargo build --bin crewchief-maproom` produces no unused import warnings
- [ ] All `disabled_postgresql_test` cfg warnings are resolved (use `#[cfg(never)]` or remove tests)
- [ ] Build command (excluding vendor C warnings) shows 0 import-related warnings
- [ ] Full test suite passes (906 tests)

## Technical Requirements
- Run `cargo fix --lib -p crewchief-maproom --allow-dirty` to auto-fix imports
- Manually address any remaining import warnings cargo fix cannot resolve
- Fix cfg attribute warnings for `disabled_postgresql_test` configuration
- Preserve all existing functionality - this is import cleanup only

## Implementation Notes
**Automated approach:**
```bash
cargo fix --lib -p crewchief-maproom --allow-dirty
```
This will auto-remove ~15 imports. For remaining warnings, manually edit files.

**cfg warnings:** The `disabled_postgresql_test` cfg in `src/indexer/mod.rs` causes unexpected cfg warnings. Either:
1. Use standard `#[cfg(never)]` to disable tests
2. Remove the tests entirely if PostgreSQL support is deprecated

**Files likely affected:**
- `src/ab_testing/logger.rs` - 1 unused import
- `src/context/cache.rs` - 2 unused imports
- `src/context/detectors/*.rs` - unused Context imports
- `src/context/graph.rs` - 1 unused import
- `src/search/*.rs` - various unused imports from SQLIMPL migration
- `src/indexer/mod.rs` - cfg attribute warnings

## Dependencies
- None - this is the first ticket in the project

## Risk Assessment
- **Risk**: Very low - removing unused imports is compiler-verified safe
  - **Mitigation**: Run full test suite after changes to confirm no regressions

## Files/Packages Affected
- `crates/maproom/src/ab_testing/logger.rs`
- `crates/maproom/src/context/cache.rs`
- `crates/maproom/src/context/detectors/hooks.rs`
- `crates/maproom/src/context/detectors/jsx.rs`
- `crates/maproom/src/context/graph.rs`
- `crates/maproom/src/search/fts.rs`
- `crates/maproom/src/search/vector.rs`
- `crates/maproom/src/search/graph.rs`
- `crates/maproom/src/search/signals.rs`
- `crates/maproom/src/indexer/mod.rs`
