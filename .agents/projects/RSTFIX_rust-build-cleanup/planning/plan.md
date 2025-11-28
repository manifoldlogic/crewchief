# Plan: Rust Build Cleanup

## Objective

Eliminate all Rust compiler warnings and fix the failing test in `crewchief-maproom`.

## Phases

### Phase 1: Unused Imports (Low Risk)
Remove unused imports across all affected files. These are safe to remove - the compiler guarantees they're not used.

**Approach:**
1. First run `cargo fix --lib -p crewchief-maproom --allow-dirty` to auto-fix ~15 imports
2. Manually clean up any remaining imports
3. Fix `disabled_postgresql_test` cfg warnings (use `#[cfg(never)]` or remove tests)

**Tickets:**
- RSTFIX-1001: Auto-fix imports and cleanup remaining warnings

**Agent**: rust-indexer-engineer

### Phase 2: Unused Variables (Medium Risk)
Fix unused variable warnings. Most will be prefixed with `_` to indicate intentional non-use; some may reveal incomplete implementations.

**Tickets:**
- RSTFIX-2001: Fix unused variables in search executors
- RSTFIX-2002: Fix unused variables in context module
- RSTFIX-2003: Fix unused variables in incremental module

**Agent**: rust-indexer-engineer

### Phase 3: Dead Code (Higher Risk)
Remove dead functions, methods, and structs that are never used. Each requires verification that removal doesn't break anything.

**Tickets:**
- RSTFIX-3001: Remove dead functions and methods
- RSTFIX-3002: Remove unused struct fields

**Agent**: rust-indexer-engineer

### Phase 4: Test Fix
Fix the failing `test_invalid_config_rejected` test.

**Tickets:**
- RSTFIX-4001: Fix config validation test failure

**Agent**: rust-indexer-engineer

### Phase 5: Final Verification
Run complete verification suite and ensure no regressions.

**Tickets:**
- RSTFIX-5001: Final build and test verification

**Agent**: unit-test-runner

## Success Metrics

| Metric | Before | After |
|--------|--------|-------|
| Rust warnings | ~58 | 0 |
| Test failures | 1 | 0 |
| Clippy issues | Unknown | 0 |

## Dependencies

- No external dependencies
- All phases can be executed sequentially
- Phase 1 must complete before Phase 2 (import removal may surface more variable issues)

## Risks

1. **Dead code may be intended for future use**
   - Mitigation: Check git history, look for TODO comments
   - Use `#[allow(dead_code)]` with explanatory comment if intentional
   - All removed code is recoverable from git history

2. **Unused variables may indicate incomplete implementation**
   - Mitigation: Document any found issues in ticket notes
   - Prefix with `_` if intentionally unused (e.g., `_store`)
   - Check if variable should actually be used (incomplete wiring)

3. **Test failure may have environment-dependent behavior**
   - Observation: Test passed in isolation but failed after clean rebuild
   - Mitigation: Root cause investigation in Phase 4
   - Worst case: Test expectation needs adjustment, not a blocker

## Timeline

5 phases, 8 tickets. Estimated completion: single session.

**Automation note:** `cargo fix` handles ~15 warnings automatically, significantly reducing manual effort in Phase 1.
