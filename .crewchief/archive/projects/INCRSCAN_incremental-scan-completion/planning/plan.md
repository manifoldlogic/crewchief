# Execution Plan: Incremental Scan Completion

## Project Overview

**Objective:** Complete the incremental scanning feature by adding tree SHA checking and state persistence to the scan command.

**Timeline:** 1-2 days
**Effort:** 8-16 hours
**Complexity:** Low (surgical fix to existing infrastructure)

## Phases

### Phase 1: Core Implementation (P0)

**Objective:** Add tree SHA check and state update to scan command

**Deliverables:**
1. Tree SHA retrieval before scan
2. State query and comparison logic
3. Skip decision with early return
4. State persistence after scan
5. Error handling for all paths

**Agent:** `rust-indexer-engineer`

**Acceptance Criteria:**
- ✅ Unchanged worktrees skip scanning (< 1 second)
- ✅ Changed worktrees perform full scan
- ✅ `--force` flag overrides skip logic
- ✅ First-time scans work as before
- ✅ Errors fallback to full scan (safe default)
- ✅ State table populated after every scan

**Tickets:**
1. `INCRSCAN-1001`: Add tree SHA check and skip logic to scan command
2. `INCRSCAN-1002`: Add state persistence after scan completion

**Estimated Time:** 4-6 hours

---

### Phase 2: Testing & Verification (P0)

**Objective:** Comprehensive testing of skip logic and state persistence

**Deliverables:**
1. Integration tests for all scan modes
2. Error handling tests
3. Manual validation with genetic optimizer

**Agent:** `integration-tester`

**Acceptance Criteria:**
- ✅ All P0 integration tests pass
- ✅ All P1 error tests pass
- ✅ No regression in existing tests
- ✅ Manual validation successful (genetic optimizer < 2 minutes)

**Tickets:**
1. `INCRSCAN-1003`: Create integration tests for scan modes
2. `INCRSCAN-1004`: Create error handling tests
3. `INCRSCAN-1005`: Manual validation with genetic optimizer

**Estimated Time:** 3-5 hours

---

### Phase 3: Documentation & Cleanup (P1)

**Objective:** Document changes and update codebase

**Deliverables:**
1. Code comments explaining logic
2. CHANGELOG entry
3. Update INCREMENTAL_INTEGRATION_NOTE.md
4. README update (if needed)

**Agent:** `rust-indexer-engineer` or general-purpose

**Acceptance Criteria:**
- ✅ All functions have clear comments
- ✅ CHANGELOG updated with feature
- ✅ Integration note updated (Phase 1 complete)

**Tickets:**
1. `INCRSCAN-1006`: Documentation and changelog

**Estimated Time:** 1-2 hours

---

## Ticket Breakdown

### INCRSCAN-1001: Add Tree SHA Check and Skip Logic

**Priority:** P0
**Complexity:** Medium
**Estimated Time:** 2-3 hours

**Tasks:**
1. Add `get_git_tree_sha()` call before scan
2. Query `worktree_index_state` for last SHA
3. Compare current vs last SHA
4. Implement skip decision logic
5. Add early return if unchanged and not --force
6. Add logging for each decision path

**Files Modified:**
- `/crates/maproom/src/main.rs` (scan command handler)
- `/crates/maproom/src/git/mod.rs` (re-export existing function if needed)

**Acceptance Criteria:**
- Unchanged tree → scan skipped (logged)
- Changed tree → full scan (logged)
- Force flag → always full scan (logged)
- Git errors → fallback to full scan (logged)
- DB errors → fallback to full scan (logged)

**Testing:**
- Unit test skip decision logic
- Integration test with unchanged repo

---

### INCRSCAN-1002: Add State Persistence After Scan

**Priority:** P0
**Complexity:** Low
**Estimated Time:** 1-2 hours

**Tasks:**
1. Collect scan statistics (files/chunks processed)
2. Get worktree ID from repo/worktree names
3. Call `update_index_state()` after scan completes
4. Handle state update errors (non-fatal)
5. Add logging for state updates

**Files Modified:**
- `/crates/maproom/src/main.rs` (after scan completion)

**Acceptance Criteria:**
- State saved after successful scan
- State includes correct tree SHA
- Stats (files/chunks) tracked
- Update errors are non-fatal (logged)
- Works for both parallel and sequential scans

**Testing:**
- Integration test verifies state persistence
- Check database after scan

---

### INCRSCAN-1003: Create Integration Tests for Scan Modes

**Priority:** P0
**Complexity:** Medium
**Estimated Time:** 2-3 hours

**Tasks:**
1. Create `tests/incremental_scan_integration.rs`
2. Write `test_unchanged_tree_skip`
3. Write `test_changed_tree_scan`
4. Write `test_force_flag_override`
5. Write `test_first_scan_state_creation`
6. Write `test_concurrent_scans`

**Files Created:**
- `/crates/maproom/tests/incremental_scan_integration.rs`

**Acceptance Criteria:**
- All 5 tests pass
- Tests cover critical paths
- Tests use real database (not mocks)
- Tests create temp git repos

**Testing:**
- Run: `cargo test incremental_scan_integration`

---

### INCRSCAN-1004: Create Error Handling Tests

**Priority:** P1
**Complexity:** Low
**Estimated Time:** 1-2 hours

**Tasks:**
1. Create `tests/scan_error_handling.rs`
2. Write `test_git_failure_fallback`
3. Write `test_db_query_failure`
4. Write `test_state_update_failure`

**Files Created:**
- `/crates/maproom/tests/scan_error_handling.rs`

**Acceptance Criteria:**
- All 3 tests pass
- Tests verify safe fallbacks
- Tests confirm non-fatal state update errors

**Testing:**
- Run: `cargo test scan_error_handling`

---

### INCRSCAN-1005: Manual Validation with Genetic Optimizer

**Priority:** P0
**Complexity:** Low
**Estimated Time:** 30 minutes

**Tasks:**
1. Clear existing state: `DELETE FROM worktree_index_state;`
2. Run genetic optimizer script
3. Observe worktree creation and scanning
4. Verify skip behavior for identical worktrees
5. Confirm total time < 2 minutes

**Files Modified:**
- None (validation only)

**Acceptance Criteria:**
- First worktree: full scan (~30 seconds)
- Remaining 11 worktrees: skipped (~1 second each)
- Total setup time: < 2 minutes
- All worktrees state saved

**Testing:**
- Manual observation
- Check database for state records

---

### INCRSCAN-1006: Documentation and Changelog

**Priority:** P1
**Complexity:** Low
**Estimated Time:** 1 hour

**Tasks:**
1. Add code comments to new functions
2. Update CHANGELOG.md
3. Update INCREMENTAL_INTEGRATION_NOTE.md
4. Update README if needed

**Files Modified:**
- `/crates/maproom/CHANGELOG.md`
- `/crates/maproom/INCREMENTAL_INTEGRATION_NOTE.md`
- `/crates/maproom/src/main.rs` (comments)

**Acceptance Criteria:**
- All new code has clear comments
- CHANGELOG has entry for this feature
- Integration note reflects Phase 1 complete

**Testing:**
- Documentation review

---

## Dependencies

**Phase 1 → Phase 2:**
- Must complete implementation before testing

**Phase 2 → Phase 3:**
- Must verify tests pass before documenting

**Tickets:**
- INCRSCAN-1002 depends on INCRSCAN-1001 (state persistence needs skip logic)
- INCRSCAN-1003 depends on INCRSCAN-1001, INCRSCAN-1002 (test implementation)
- INCRSCAN-1005 depends on INCRSCAN-1001, INCRSCAN-1002 (manual validation)

## Risk Management

### Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Git commands fail in edge cases | Medium | Low | Fallback to full scan (safe default) |
| Database queries slow | Low | Very Low | Queries are indexed, fast |
| State update fails | Low | Low | Non-fatal error (logged only) |
| Tests don't catch regressions | High | Medium | Comprehensive test coverage + manual validation |
| Performance regression | Medium | Very Low | Before/after timing verification |

### Rollback Plan

**If Issues Discovered:**
1. Revert commits (simple git revert)
2. No database migration needed (table exists, unused won't break anything)
3. System returns to current behavior (all full scans)

**Database Cleanup (if needed):**
```sql
-- Clear state table (optional)
DELETE FROM maproom.worktree_index_state;
```

## Success Metrics

### Quantitative

1. **Scan Time (unchanged worktree):** < 1 second (currently 2-3 hours)
2. **Genetic optimizer setup:** < 2 minutes (currently 24+ hours)
3. **Test coverage:** 100% of critical paths
4. **Zero false skips:** Correctness maintained

### Qualitative

1. **User Experience:** Clear logging, predictable behavior
2. **Code Quality:** Well-commented, maintainable
3. **Error Handling:** Graceful degradation
4. **Documentation:** Clear explanation of feature

## Timeline

**Day 1:**
- Morning: INCRSCAN-1001 (tree SHA check)
- Afternoon: INCRSCAN-1002 (state persistence)
- Evening: INCRSCAN-1003 (integration tests)

**Day 2:**
- Morning: INCRSCAN-1004 (error tests)
- Afternoon: INCRSCAN-1005 (manual validation)
- Evening: INCRSCAN-1006 (documentation)

**Total:** 1-2 days depending on testing complexity

## Go-Live Checklist

### Pre-Deployment

- [ ] All P0 tests pass
- [ ] Manual validation successful
- [ ] Code reviewed (self-review sufficient for small project)
- [ ] Documentation complete
- [ ] CHANGELOG updated

### Deployment

- [ ] Merge to main branch
- [ ] Rebuild binaries (`pnpm build:rust`)
- [ ] Update packages (if published)

### Post-Deployment

- [ ] Run genetic optimizer as smoke test
- [ ] Monitor for unexpected behavior
- [ ] Check state table population
- [ ] Verify skip rate in logs

### Rollback Triggers

- ❌ Scans failing unexpectedly
- ❌ Performance regression
- ❌ False skips detected (correctness issue)
- ❌ Database corruption

## Future Enhancements (Out of Scope)

**Phase 2 (Future Project):**
- Integrate `git diff-tree` for true incremental (only changed files)
- Refactor `scan_worktree()` for pluggable file discovery
- Parallel tree SHA checks

**Phase 3 (Future Project):**
- Remote state caching
- Predictive indexing
- Smart embedding cache

## Notes

**Simplicity Over Complexity:**
This project intentionally avoids the complex refactoring proposed in INCREMENTAL_INTEGRATION_NOTE.md. Instead, we take the minimal path: add tree SHA checking at the command level.

**Why This Works:**
- 99% of cases: identical tree SHA → skip entire scan
- 1% of cases: different tree SHA → full scan (same as today)
- Future: refactor to process only changed files (Phase 2)

**The Perfect is the Enemy of the Good:**
We could refactor the entire scan pipeline for pluggable file discovery. But that's weeks of work for marginal benefit. This fix delivers 10,000x speedup in 8 hours.

**Ship It:**
Get this working, validate with genetic optimizer, then move on. Perfect incremental scanning can wait.
