# Ticket: IDXCLEAN-3004: Manual Validation on Staging Database

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (manual validation task)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Execute database cleanup on staging environment and perform manual verification of results to ensure production readiness. This validates that the cleanup correctly identifies stale worktrees, removes them safely, and improves search quality in a production-like environment.

## Background
Automated tests can't catch everything. This ticket requires human validation on a real staging database to ensure the cleanup works correctly with production-like data and that search quality actually improves. This is the final production readiness gate before deployment.

References plan.md: Phase 3 - Integration Testing and Safety Validation, ticket IDXCLEAN-3004 (lines 505-529)

## Acceptance Criteria
- [ ] Dry-run executed on staging database
- [ ] Output reviewed for accuracy (all reported stale worktrees are actually stale)
- [ ] Cleanup with --confirm executed successfully
- [ ] Search quality improved (result duplication reduced)
- [ ] No valid worktrees accidentally deleted
- [ ] Performance acceptable (< 2 seconds)
- [ ] Validation report documented in this ticket

## Technical Requirements
- Access to staging database required
- Manual verification of each reported stale worktree
- Before/after search quality comparison
- Performance measurement (time the operation)
- Document findings in ticket for audit trail

## Implementation Notes

### Validation Workflow
```bash
# Step 1: Connect to staging
export MAPROOM_DATABASE_URL=<staging-url>

# Step 2: Dry-run and review
maproom db cleanup-stale --verbose > dry-run-results.txt

# Step 3: Manual verification
# - Review each path in dry-run-results.txt
# - Verify paths actually don't exist
# - Check for any false positives

# Step 4: Execute cleanup
maproom db cleanup-stale --confirm

# Step 5: Verify search improvement
# - Search for known duplicated symbols
# - Count result duplication before/after
# - Document improvement
```

### What to Verify
1. **Stale Detection Accuracy**: Every worktree flagged as stale should have a non-existent root_path
2. **No False Positives**: No valid worktrees should be flagged for deletion
3. **Search Quality**: After cleanup, searches should return fewer duplicate results
4. **Performance**: Operation should complete in under 2 seconds
5. **Safety**: Database integrity maintained, no unintended data loss

### Documentation Template
Document findings as a comment/update in this ticket:
```
## Validation Report - [Date]

### Environment
- Database: [staging URL/host]
- Date/Time: [timestamp]
- Operator: [name]

### Dry-Run Results
- Total worktrees in database: X
- Stale worktrees detected: Y
- Sample paths verified: [list 3-5 examples]
- False positives found: [none/list issues]

### Cleanup Execution
- Execution time: X.XX seconds
- Worktrees deleted: Y
- Errors: [none/describe]

### Search Quality Assessment
- Test query: "[example query]"
- Duplicate results before: X
- Duplicate results after: Y
- Improvement: Z% reduction

### Recommendation
[Pass/Fail] - [Brief explanation]
```

## Dependencies
- IDXCLEAN-2003 (output formatting) - must be complete for readable dry-run output
- IDXCLEAN-3001 (unit tests) - must pass
- IDXCLEAN-3002 (integration tests) - must pass
- IDXCLEAN-3003 (edge case tests) - must pass

All automated tests must pass before staging validation begins.

## Risk Assessment
- **Risk**: Staging database doesn't accurately represent production data
  - **Mitigation**: Document any differences found; consider additional production validation with extra safety measures

- **Risk**: Manual verification misses edge cases
  - **Mitigation**: Verify a representative sample; focus on boundary cases (longest paths, special characters, recent timestamps)

- **Risk**: Timing/performance differs between staging and production
  - **Mitigation**: Document performance characteristics; plan for monitoring during production rollout

## Files/Packages Affected
- This ticket (add validation report in comments/updates section below)

---

## Validation Updates

<!-- Document validation attempts and results here -->
