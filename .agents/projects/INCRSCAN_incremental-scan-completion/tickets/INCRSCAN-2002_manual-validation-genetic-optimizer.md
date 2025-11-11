# Ticket: INCRSCAN-2002: Manual Validation with Genetic Optimizer

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (manual validation, not automated tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- This is a manual validation ticket, not an automated test creation ticket
- "Tests pass" is marked N/A because this ticket validates the system manually
- Success is measured by acceptance criteria, not test suite execution

## Agents
- verify-ticket
- commit-ticket

## Summary
Validate the incremental scanning feature using the genetic optimizer script that creates 12 identical worktrees. Verify the first worktree performs a full scan and the remaining 11 skip scanning, completing setup in < 2 minutes instead of 24+ hours.

## Background
From analysis.md: "Impact - Genetic optimizer unusable (24-36 hours for 12 worktrees). Expected Performance (with tree SHA check): Identical worktree scan: 5-10ms (just tree SHA comparison + DB query)."

This manual validation provides real-world confidence that the fix works using the exact scenario that prompted this entire project. The genetic optimizer script creates 12 identical worktrees pointing to the same commit. Before the incremental scan feature, each worktree required a full scan (2+ hours each = 24+ hours total). After implementing tree SHA checking (INCRSCAN-1001) and state persistence (INCRSCAN-1002), only the first worktree should scan, with the remaining 11 detecting no changes and skipping.

This is the acid test that proves the feature delivers the promised value.

## Acceptance Criteria

- [ ] **First worktree performs full scan** (~30-60 seconds, all files processed, state record created)

- [ ] **Remaining 11 worktrees skip scanning** (~1 second each, "No changes detected (tree SHA match), skipping scan" logged)

- [ ] **Total setup time < 2 minutes** (vs 24+ hours before fix, demonstrating 720x speedup)

- [ ] **All worktrees have state saved** (12 records in worktree_index_state with same tree SHA)

- [ ] **Genetic optimizer completes successfully** (all worktrees created, no errors, ready for optimization runs)

## Technical Requirements

### 1. Pre-Validation Setup
- Clear existing state: Execute `DELETE FROM worktree_index_state WHERE repo = 'crewchief';` to ensure clean slate
- Verify database is accessible and clean
- Locate genetic optimizer script: `packages/cli/scripts/run-genetic-optimizer-ultra.ts`
- Ensure adequate disk space for 12 worktrees (~500MB per worktree = 6GB total)

### 2. Execution
- Run command: `cd /workspace/packages/cli && pnpm tsx scripts/run-genetic-optimizer-ultra.ts`
- Monitor console output during worktree creation and scanning
- Observe timing and log messages for each worktree setup
- Allow script to complete fully

### 3. Observation Points
- **First worktree (ultra-run-XXXXXXXXX-gen-000):**
  - Should perform full scan (~30-60 seconds)
  - Should log file processing progress
  - Should create state record with tree SHA

- **Remaining 11 worktrees (ultra-run-XXXXXXXXX-gen-001 through gen-011):**
  - Should skip scanning (~1 second each)
  - Should log "No changes detected (tree SHA match), skipping scan"
  - Should have same tree SHA as first worktree

- **Total setup time:**
  - Entire script should complete in < 2 minutes
  - Compare to previous behavior (24+ hours)

### 4. Database Verification
After script completes:
```sql
-- Verify 12 worktrees indexed
SELECT worktree_name, last_tree_sha, last_indexed_at
FROM worktree_index_state
WHERE repo = 'crewchief'
ORDER BY last_indexed_at;

-- Verify all have same tree SHA
SELECT COUNT(DISTINCT last_tree_sha)
FROM worktree_index_state
WHERE repo = 'crewchief';
-- Expected: 1 (all identical)
```

### 5. Log Verification
- Capture console output during execution
- Verify skip messages appear for worktrees 2-12
- Verify no error messages or warnings
- Confirm timing matches expectations

## Implementation Notes

### Expected Console Output Pattern

```
Creating worktree: ultra-run-1234567890-gen-000
Scanning worktree...
[████████████████████████████████████████] 100% | Files: 1543/1543
Scan complete. Files processed: 1543, Duration: 45s

Creating worktree: ultra-run-1234567890-gen-001
Scanning worktree...
No changes detected (tree SHA match), skipping scan
Scan complete. Files processed: 0, Duration: 0.8s

Creating worktree: ultra-run-1234567890-gen-002
Scanning worktree...
No changes detected (tree SHA match), skipping scan
Scan complete. Files processed: 0, Duration: 0.7s

[... repeat for gen-003 through gen-011 ...]
```

### Performance Metrics to Capture

| Metric | Before Fix | After Fix | Improvement |
|--------|-----------|-----------|-------------|
| First worktree scan | ~2 hours | ~30-60 sec | Same (no change) |
| Remaining 11 scans | ~22 hours | ~11 sec | 7200x faster |
| Total setup time | 24+ hours | < 2 minutes | 720x faster |
| Developer productivity | Unusable | Fully usable | ∞ |

### Troubleshooting

If validation fails:

1. **First worktree doesn't scan:**
   - Check database connectivity
   - Verify migrations applied
   - Check logs for errors

2. **Remaining worktrees don't skip:**
   - Verify tree SHA logic implemented (INCRSCAN-1001)
   - Check state was persisted after first scan (INCRSCAN-1002)
   - Query database to verify state exists

3. **Script errors:**
   - Check disk space
   - Verify git configuration
   - Check database permissions

### Success Indicators

- Script completes without errors
- Console shows clear skip pattern (11 skips after 1 full scan)
- Database has 12 records with identical tree SHA
- Total time dramatically reduced (< 2 minutes vs 24+ hours)
- Developer can now use genetic optimizer productively

## Dependencies

- **INCRSCAN-1001** - tree-sha-check-skip-logic (MUST be implemented and working)
- **INCRSCAN-1002** - state-persistence (MUST be implemented and working)
- **INCRSCAN-2001** - integration-tests-scan-modes (SHOULD pass first for confidence)

## Risk Assessment

- **Risk:** Validation fails due to incomplete implementation
  - **Mitigation:** Run INCRSCAN-2001 integration tests first to verify core logic

- **Risk:** Script creates 12 worktrees that consume disk space
  - **Mitigation:** Ensure adequate disk space before running (~6GB needed)

- **Risk:** Database not accessible or migrations not applied
  - **Mitigation:** Verify database connection and schema before validation

- **Risk:** False positive (appears to work but doesn't)
  - **Mitigation:** Verify database state manually, check logs carefully, run multiple times

## Files/Packages Affected

- **Executed Script:** `packages/cli/scripts/run-genetic-optimizer-ultra.ts`
- **Database Table:** `worktree_index_state` (12 new records)
- **Git Worktrees:** Creates 12 worktrees in `packages/cli/.crewchief/genetic-iterations/ultra-run-XXXXXXXXX/`
- **No Code Changes:** This is validation only, no files modified

## Priority & Complexity

- **Priority:** P0 (final validation before considering feature complete)
- **Complexity:** Low (execution and observation only, no coding)
- **Estimated Time:** 30 minutes (including database verification and documentation)
- **Phase:** 2 (Testing & Verification)
