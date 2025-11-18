# Ticket: IDXCLEAN-5003: Production Verification

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (verification/validation ticket, no code changes)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- verify-ticket
- commit-ticket

## Summary
Execute cleanup on production database and verify it works correctly with measurable search quality improvement.

## Background
This is the final validation gate for the index stale worktree cleanup feature. After completing development (Phases 1-2), testing (Phase 3), automation (Phase 4), and deployment preparation (Phase 5 tickets 5001-5002), we must verify that cleanup works in the actual production environment, improves search quality as expected, and causes no issues.

This ticket implements Phase 5 - Production Deployment, specifically ticket IDXCLEAN-5003 from the project plan (lines 757-782 of plan.md).

## Acceptance Criteria
- [ ] Dry-run executed on production database
- [ ] Results reviewed for accuracy (stale worktrees correctly identified)
- [ ] Cleanup with --confirm executed successfully
- [ ] Search quality improved measurably (duplicate results reduced)
- [ ] No errors in logs during or after cleanup
- [ ] Performance within acceptable limits (< 2 seconds target)
- [ ] Monitoring shows healthy metrics for 48 hours post-cleanup

## Technical Requirements
- Must run on actual production database (not staging)
- Team approval required before executing --confirm
- Before/after search quality comparison with concrete metrics
- Performance measurement for cleanup execution time
- Monitor error rates for 48 hours minimum
- Document actual results vs. expected results
- Verify database size reduction
- Verify worktree count stability

## Implementation Notes
```bash
# Production verification workflow

# 1. Dry-run on production database
maproom db cleanup-stale --verbose > production-dry-run.txt

# 2. Team review of dry-run results
# - Count stale worktrees found
# - Spot-check paths are actually stale (compare with active worktrees)
# - Verify chunk counts are reasonable
# - Get team approval before proceeding

# 3. Execute cleanup with timing
time maproom db cleanup-stale --confirm

# 4. Verify search quality improvement
# Before cleanup: Search for known duplicated symbols
# After cleanup: Same searches should show fewer duplicates
# Target improvement: 15x duplication → 1-2x duplication
# Example: "function authenticate" should return ~20 results, not ~300

# 5. Monitor production health
# - Check logs for errors (48 hours minimum)
# - Verify database size reduced appropriately
# - Verify worktree count remains stable
# - Verify no increase in error rates
# - Verify search performance acceptable
```

**Documentation Requirements:**
- Capture dry-run output
- Document team approval decision
- Record cleanup execution results (chunks deleted, timing)
- Document before/after search quality metrics
- Record monitoring observations over 48 hours
- Update this ticket with production verification report

## Dependencies
- IDXCLEAN-5002 (deployment procedure must be complete)
- All Phase 1-4 tickets must be complete and deployed

## Risk Assessment
- **Risk**: Cleanup deletes incorrect data in production
  - **Mitigation**: Mandatory dry-run review, team approval gate, database backups before execution

- **Risk**: Cleanup performance impacts production search
  - **Mitigation**: Execute during low-traffic period, monitor performance metrics, rollback plan ready

- **Risk**: Search quality doesn't improve as expected
  - **Mitigation**: Document actual results, investigate root cause, may require additional development

- **Risk**: Unforeseen production issues not caught in staging
  - **Mitigation**: 48-hour monitoring period, incident response playbook ready, rollback procedure documented

## Files/Packages Affected
- This ticket file (update with production verification report)
- Production database (cleanup execution)
- Potentially deployment/monitoring documentation if issues discovered
