# INCRSCAN Ticket Index

**Project:** Incremental Scan Completion
**Status:** Ready for Implementation
**Total Tickets:** 6

## Overview

This project completes the incremental scanning feature by adding tree SHA checking and state persistence to the scan command. The fix is surgical (20 lines of glue code) with massive impact (10,000x speedup for unchanged worktrees).

**Timeline:** 1-2 days (8-12 hours)
**Estimated Cost:** Low (minimal implementation, comprehensive testing)

---

## Phase 1: Core Implementation (P0)

**Objective:** Add tree SHA check and state update to scan command

**Duration:** 4-6 hours

### INCRSCAN-1001: Add Tree SHA Check and Skip Logic to Scan Command
- **Status:** 🔴 Not Started
- **Priority:** P0 (blocks all other tickets)
- **Complexity:** Medium
- **Estimated Time:** 2-3 hours
- **Agent:** rust-indexer-engineer
- **Dependencies:** None
- **File:** `INCRSCAN-1001_tree-sha-check-skip-logic.md`

**Summary:** Implement tree SHA checking before scan operations to determine if worktree code changed. If unchanged and not --force, skip entire scan (early return). Enables 10,000x speedup.

**Key Deliverables:**
- Get git tree SHA before scan
- Query worktree_index_state for last SHA
- Compare current vs last SHA
- Skip if match (and not --force)
- Fail-safe error handling (default to full scan)

**Acceptance Criteria:**
- ✅ Unchanged tree → scan skipped (< 1 second)
- ✅ Changed tree → full scan (existing behavior)
- ✅ Force flag → always full scan
- ✅ First-time scans work
- ✅ Errors fallback to full scan

---

### INCRSCAN-1002: Add State Persistence After Scan Completion
- **Status:** 🔴 Not Started
- **Priority:** P0
- **Complexity:** Low
- **Estimated Time:** 1-2 hours
- **Agent:** rust-indexer-engineer
- **Dependencies:** INCRSCAN-1001
- **File:** `INCRSCAN-1002_add-state-persistence-after-scan.md`

**Summary:** Call update_index_state() after scan operations complete to save git tree SHA and statistics. Enables skip logic to work on subsequent scans.

**Key Deliverables:**
- Collect scan statistics (files/chunks processed)
- Get worktree ID
- Call update_index_state() with tree SHA and stats
- Handle errors (non-fatal - scan succeeded)

**Acceptance Criteria:**
- ✅ State saved after successful scan
- ✅ Correct tree SHA stored
- ✅ Stats accurately tracked
- ✅ Update errors non-fatal
- ✅ Works for all scan modes (sequential, parallel, force)

---

## Phase 2: Testing & Verification (P0)

**Objective:** Comprehensive testing of skip logic and state persistence

**Duration:** 3-5 hours

### INCRSCAN-2001: Create Integration Tests for Scan Modes
- **Status:** 🔴 Not Started
- **Priority:** P0
- **Complexity:** Medium
- **Estimated Time:** 2-3 hours
- **Agent:** integration-tester
- **Dependencies:** INCRSCAN-1001, INCRSCAN-1002
- **File:** `INCRSCAN-2001_integration-tests-scan-modes.md`

**Summary:** Create comprehensive integration tests covering all scan modes using real database and temp git repos.

**Test Coverage:**
1. test_unchanged_tree_skip - Verify early return on unchanged tree
2. test_changed_tree_scan - Verify full scan when tree changes
3. test_force_flag_override - Verify --force bypasses skip logic
4. test_first_scan_state_creation - Verify state creation
5. test_concurrent_scans - Verify race condition handling

**Acceptance Criteria:**
- ✅ All 5 tests pass
- ✅ Tests cover critical paths
- ✅ Tests use real database (not mocks)
- ✅ Tests create temp git repos

---

### INCRSCAN-1004: Create Error Handling Tests
- **Status:** 🔴 Not Started
- **Priority:** P1
- **Complexity:** Low
- **Estimated Time:** 1-2 hours
- **Agent:** integration-tester
- **Dependencies:** INCRSCAN-1001, INCRSCAN-1002, INCRSCAN-2001
- **File:** `INCRSCAN-1004_error-handling-tests.md`

**Summary:** Create integration tests for error scenarios to verify fail-safe fallback behavior.

**Test Coverage:**
1. test_git_failure_fallback - Git errors → full scan
2. test_db_query_failure - DB errors → full scan
3. test_state_update_failure - Update errors → non-fatal

**Acceptance Criteria:**
- ✅ All 3 tests pass
- ✅ Tests verify safe fallbacks
- ✅ Tests confirm non-fatal state update errors

**Note:** Uses ticket ID INCRSCAN-1004 per explicit request in plan, though Phase 2 numbering convention would suggest 2xxx.

---

### INCRSCAN-2002: Manual Validation with Genetic Optimizer
- **Status:** 🔴 Not Started
- **Priority:** P0 (critical validation)
- **Complexity:** Low
- **Estimated Time:** 30 minutes
- **Agent:** verify-ticket
- **Dependencies:** INCRSCAN-1001, INCRSCAN-1002, INCRSCAN-2001
- **File:** `INCRSCAN-2002_manual-validation-genetic-optimizer.md`

**Summary:** Validate feature using genetic optimizer script (12 identical worktrees). Real-world acid test that proves the feature works.

**Validation Steps:**
1. Clear state: `DELETE FROM worktree_index_state;`
2. Run: `pnpm tsx scripts/run-genetic-optimizer-ultra.ts`
3. Observe: First worktree full scan, remaining 11 skip
4. Verify: Total time < 2 minutes (vs 24+ hours)

**Acceptance Criteria:**
- ✅ First worktree: full scan (~30-60 seconds)
- ✅ Remaining 11: skip (~1 second each)
- ✅ Total setup: < 2 minutes
- ✅ All worktrees: state saved
- ✅ Genetic optimizer: completes successfully

---

## Phase 3: Documentation & Cleanup (P1)

**Objective:** Document changes and update codebase

**Duration:** 1-2 hours

### INCRSCAN-3001: Documentation and Changelog
- **Status:** 🔴 Not Started
- **Priority:** P1
- **Complexity:** Low
- **Estimated Time:** 1 hour
- **Agent:** rust-indexer-engineer
- **Dependencies:** INCRSCAN-1001, INCRSCAN-1002, INCRSCAN-2001, INCRSCAN-2002
- **File:** `INCRSCAN-3001_documentation-and-changelog.md`

**Summary:** Add code comments, update CHANGELOG.md, update INCREMENTAL_INTEGRATION_NOTE.md to reflect Phase 1 completion.

**Key Deliverables:**
- Code comments explaining tree SHA check logic
- CHANGELOG entry for feature
- INCREMENTAL_INTEGRATION_NOTE.md status update
- README update (if needed)

**Acceptance Criteria:**
- ✅ All new code has clear comments
- ✅ CHANGELOG has entry for feature
- ✅ Integration note reflects Phase 1 complete

---

## Dependency Graph

```
Phase 1 (Implementation):
    INCRSCAN-1001 (tree SHA check)
            ↓
    INCRSCAN-1002 (state persistence)
            ↓
Phase 2 (Testing):
    INCRSCAN-2001 (integration tests) ←┐
            ↓                           │
    INCRSCAN-1004 (error tests) ───────┘
            ↓
    INCRSCAN-2002 (manual validation)
            ↓
Phase 3 (Documentation):
    INCRSCAN-3001 (documentation)
```

**Critical Path:** 1001 → 1002 → 2001 → 2002 → 3001

**Parallel Opportunities:**
- INCRSCAN-1004 can run after 2001 (not blocking)
- Documentation (3001) can start while validation (2002) runs

---

## Success Metrics

### Quantitative
1. **Scan Time (unchanged):** < 1 second (currently 2-3 hours) ✅ Target: 10,000x speedup
2. **Genetic optimizer:** < 2 minutes (currently 24+ hours) ✅ Target: 720x speedup
3. **Test Coverage:** 100% of critical paths ✅ Target: 8 integration tests
4. **Zero False Skips:** Correctness maintained ✅ Target: No missed changes

### Qualitative
1. **User Experience:** Clear logging, predictable behavior
2. **Code Quality:** Well-commented, maintainable
3. **Error Handling:** Graceful degradation, fail-safe
4. **Documentation:** Clear explanation of feature

---

## Execution Sequence

**Day 1:**
1. Morning: INCRSCAN-1001 (2-3h)
2. Afternoon: INCRSCAN-1002 (1-2h)
3. Evening: INCRSCAN-2001 (2-3h)

**Day 2:**
1. Morning: INCRSCAN-1004 (1-2h)
2. Afternoon: INCRSCAN-2002 (30m) + INCRSCAN-3001 (1h)
3. Evening: Final review, commit

**Total:** 8-12 hours over 1-2 days

---

## Definition of Done

### Code Complete
- ✅ Tree SHA check implemented (INCRSCAN-1001)
- ✅ State persistence added (INCRSCAN-1002)
- ✅ Error handling verified
- ✅ Logging added

### Tests Pass
- ✅ All integration tests pass (INCRSCAN-2001)
- ✅ All error tests pass (INCRSCAN-1004)
- ✅ Manual validation successful (INCRSCAN-2002)
- ✅ No regression in existing tests

### Documentation
- ✅ Code comments explain logic (INCRSCAN-3001)
- ✅ CHANGELOG updated
- ✅ INCREMENTAL_INTEGRATION_NOTE.md updated

### Performance
- ✅ Unchanged scans < 1 second
- ✅ No regression in full scan speed
- ✅ Genetic optimizer completes in < 2 minutes

---

## Planning References

All tickets derived from comprehensive planning documents:

- **analysis.md** - Problem definition, root cause, performance impact
- **architecture.md** - System design, integration points, data flow
- **quality-strategy.md** - Test strategy, risk-based coverage, acceptance criteria
- **security-review.md** - Threat model, security analysis, sign-off
- **plan.md** - Phased execution, ticket breakdown, dependencies

See: `/workspace/.crewchief/projects/INCRSCAN_incremental-scan-completion/planning/`

---

## Notes

**Why This is Simple:**
- Infrastructure exists (table, functions tested)
- Just need to wire together at scan command level
- 20 lines of glue code → 10,000x speedup

**Why Not Full Refactor:**
- Current implementation: Skip all or process all
- Future Phase 2: Process only changed files (git diff-tree)
- Ship value now (90% benefit), defer complexity

**Rollback Plan:**
- Simple git revert (no schema changes)
- Database table harmless (advisory only)
- System returns to current behavior

---

**Status Legend:**
- 🔴 Not Started
- 🟡 In Progress
- 🟢 Complete
- ⚪ Blocked

**Last Updated:** 2025-01-11
