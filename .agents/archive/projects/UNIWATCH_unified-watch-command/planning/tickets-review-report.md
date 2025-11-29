# UNIWATCH Tickets Review Report

**Review Date:** 2025-01-28
**Reviewer:** Ticket Review Agent
**Total Tickets:** 7
**Overall Assessment:** READY WITH MINOR FIXES

## Executive Summary

| Metric | Count |
|--------|-------|
| Tickets Reviewed | 7 |
| Critical Issues | 1 |
| Warnings | 3 |
| Recommendations | 4 |

**Overall Verdict:** The ticket set is well-structured and aligns with the project architecture. One critical issue requires fix before execution (test count discrepancy), and three warnings should be addressed. The dependency chain is valid and the execution order is correct.

## Critical Issues

### Issue 1: UNIWATCH-4001 Test Count Mismatch

**Ticket:** UNIWATCH-4001
**Problem:** Ticket claims 4 disabled tests, but only 3 exist in the codebase.

**Evidence from codebase search:**
```
#[cfg(disabled_postgresql_test)] occurrences:
- Line 778: test_worktree_tracking_initialization
- Line 933: test_handle_branch_switch_updates_state
- Line 1101: test_handle_branch_switch_skips_if_same_branch
```

The ticket incorrectly lists `test_event_loop_handles_both_file_and_head_events` as disabled, but `test_event_loop_handles_both_sources` at line 1496 is **NOT disabled** (no `#[cfg(disabled_postgresql_test)]` annotation).

**Impact:** Implementer will look for a test that doesn't exist, causing confusion.

**Required Action:** Update UNIWATCH-4001 to:
1. Remove `test_event_loop_handles_both_file_and_head_events` from "Tests to migrate" table
2. Change "4 UNIWATCH-prefixed unit tests" to "3 UNIWATCH-prefixed unit tests"
3. Update acceptance criteria count from 4 to 3

---

## Warnings

### Warning 1: Test Function Name Mismatch

**Ticket:** UNIWATCH-4001
**Concern:** Ticket references test names that don't exactly match codebase:
- Ticket: `test_worktree_tracking_state_initialization`
- Actual: `test_worktree_tracking_initialization` (missing "state_")

**Impact:** Minor confusion during implementation.

**Suggested Remediation:** Update test names in ticket to match actual codebase names.

### Warning 2: UNIWATCH-3001 Missing Error Handling for Async Context

**Ticket:** UNIWATCH-3001
**Concern:** The provided code shows `get_current_branch(watch_path)?` but this is a sync function being called in an async context. If it panics or blocks, it could affect the async runtime.

**Impact:** Potential runtime issues under edge cases.

**Suggested Remediation:** Document that `get_current_branch()` is a fast sync operation (file read only) and blocking is acceptable, OR use `tokio::task::spawn_blocking` for safety.

### Warning 3: E2E Test Script Existence

**Ticket:** UNIWATCH-4003
**Concern:** The E2E test script exists and already uses PostgreSQL (`psql` commands at lines 34-35, 180-209). The ticket correctly identifies this needs migration, but the script is more extensive than described (~245 lines, not ~50).

**Impact:** Scope may be larger than estimated.

**Suggested Remediation:** Update Files/Packages Affected to reflect "~100 lines modified" instead of "~50 lines".

---

## Recommendations

### Recommendation 1: Add Verification Step to UNIWATCH-2001

**Affected Tickets:** UNIWATCH-2001
**Suggestion:** Add acceptance criteria to verify the HEAD watcher actually fires events. Currently the ticket only checks that the watcher is created, not that it works.

**Expected Benefit:** Catch HEAD watcher configuration issues early.

### Recommendation 2: Consider Merging UNIWATCH-4001 and UNIWATCH-4003

**Affected Tickets:** UNIWATCH-4001, UNIWATCH-4003
**Suggestion:** Both tickets are assigned to rust-indexer-engineer and involve test migration from PostgreSQL to SQLite. They could be combined into a single ticket.

**Expected Benefit:** Reduces context switching, ensures consistent migration approach.

**Counter-argument:** Keeping them separate allows for incremental progress and easier verification. Current separation is acceptable.

### Recommendation 3: Add DebouncedHandler State Reset in Tests

**Affected Tickets:** UNIWATCH-4002
**Suggestion:** Integration tests for debouncing (Test 2) need to ensure the DebouncedHandler is reset between test runs or tests use fresh instances.

**Expected Benefit:** Prevents flaky tests due to shared debounce state.

### Recommendation 4: Document get_short_commit_sha Helper

**Affected Tickets:** UNIWATCH-3001
**Suggestion:** The `get_short_commit_sha()` helper function spawns a subprocess. Consider documenting error handling for when git is not installed or the command fails.

**Expected Benefit:** Better error messages in edge cases.

---

## Ticket Actions Required

### Tickets to Rework

| Ticket | Required Changes |
|--------|------------------|
| UNIWATCH-4001 | Fix test count (4→3), correct test name (`test_worktree_tracking_initialization`), remove non-existent test reference |

### Tickets to Defer

None.

### Tickets to Skip

None.

### Tickets to Split

None.

### Tickets to Merge

None recommended (see Recommendation 2 for discussion).

---

## Integration Assessment

### Overall Integration Health: GOOD

The tickets properly integrate with existing code:

| Component | Status | Notes |
|-----------|--------|-------|
| `setup_head_watcher()` | Exists at line 668 | Needs `pub` export |
| `DebouncedHandler` | Exists at line 33 | Needs `pub` export |
| `BranchSwitchEvent` | Exists at line 112 | Needs `pub` export |
| `get_current_branch()` | Public, line 71 | Ready to use |
| `get_or_create_worktree()` | Public, line 155 | Ready to use |
| `incremental_update()` | Public, line 226 | Ready to use |
| Watch handler | Lines 1105-1216 | Target for modification |

### Key Integration Points

1. **Phase 0 → Phase 1-3**: Module exports are prerequisite - correctly sequenced
2. **Phase 1 → Phase 2**: Dynamic state must exist before HEAD watcher - correctly sequenced
3. **Phase 2 → Phase 3**: HEAD watcher must exist before handler - correctly sequenced
4. **Phase 3 → Phase 4**: Implementation must complete before tests - correctly sequenced

### Risks to Existing Functionality

| Risk | Mitigation |
|------|------------|
| File watching regression | Each ticket includes "existing watch functionality unchanged" criterion |
| Lock contention | Using `std::sync::RwLock` with brief holds as per architecture |
| Event ordering | `tokio::select!` sequential processing documented |

---

## Dependency Analysis

### Dependency Chain Validation: VALID

```
UNIWATCH-0001 (no deps)
    ↓
UNIWATCH-1001 (depends: 0001)
    ↓
UNIWATCH-2001 (depends: 0001, 1001)
    ↓
UNIWATCH-3001 (depends: 0001, 1001, 2001)
    ↓
UNIWATCH-4001 (depends: 3001)
    ↓
UNIWATCH-4002 (depends: 3001, 4001)
    ↓
UNIWATCH-4003 (depends: 3001, 4002)
```

- No circular dependencies
- Chain is achievable
- Sequential execution required (no parallelization possible)

### Problematic Dependencies

None identified.

### Sequencing Recommendations

Current sequence is optimal. No changes recommended.

---

## Recommendations for Execution

### Suggested Execution Order

Execute tickets sequentially as numbered:
1. UNIWATCH-0001 (Phase 0)
2. UNIWATCH-1001 (Phase 1)
3. UNIWATCH-2001 (Phase 2)
4. UNIWATCH-3001 (Phase 3)
5. UNIWATCH-4001 (Phase 4 - unit tests)
6. UNIWATCH-4002 (Phase 4 - integration tests)
7. UNIWATCH-4003 (Phase 4 - E2E tests)

### Risk Mitigation Strategies

1. **After UNIWATCH-0001**: Run `cargo check` to verify exports work
2. **After UNIWATCH-1001**: Manually test existing file watching still works
3. **After UNIWATCH-3001**: Manual test branch switching before proceeding to tests
4. **After UNIWATCH-4001**: Run unit tests before integration tests

### Key Checkpoints

| Checkpoint | After Ticket | Verification |
|------------|--------------|--------------|
| Exports valid | UNIWATCH-0001 | `cargo check -p crewchief-maproom` |
| No regression | UNIWATCH-1001 | Manual: edit file, verify indexed |
| HEAD detection | UNIWATCH-2001 | Manual: `git checkout`, verify log output |
| Full feature | UNIWATCH-3001 | Manual: switch branch, verify NDJSON output |
| Unit coverage | UNIWATCH-4001 | `cargo test -p crewchief-maproom` |

### Success Criteria for Project Completion

From plan.md:
- [ ] `maproom watch` detects branch switches within 2 seconds
- [ ] File changes after switch index to the new worktree
- [ ] Rapid branch switches (< 2s apart) are debounced
- [ ] `BranchSwitchEvent` NDJSON emitted on branch change
- [ ] No regressions to existing file watching
- [ ] All tests pass

---

## Summary

The UNIWATCH ticket set is well-prepared for execution. One critical fix is required (test count in UNIWATCH-4001), and three warnings should be addressed for smoother implementation. The dependency chain is valid, integration points are verified against the codebase, and the execution order is optimal.

**Next step:** Fix UNIWATCH-4001 test count issue, then proceed to `/work-on-project UNIWATCH` to execute tickets.
