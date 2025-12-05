# Ticket: [WTCLEAN-2002]: Integrate Maproom Cleanup in Clean Command

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Integrate maproom database cleanup into the `worktree clean` command flow, calling `cleanMaproomRecords()` after directory removal unless `--keep-maproom` flag is set.

## Background
After removing a worktree directory, stale records remain in the maproom database. This integration ensures database cleanup happens automatically as part of the clean command.

This ticket implements Phase 2, Deliverable 2 from the plan: Maproom cleanup integration.

## Acceptance Criteria
- [ ] Maproom cleanup called after directory removal
- [ ] Cleanup only runs if `--keep-maproom` flag NOT set
- [ ] Import and use `cleanMaproomRecords()` from WTCLEAN-1002
- [ ] Success logged when cleanup completes
- [ ] Errors are caught and do NOT block cleanup (best-effort)
- [ ] Clear warning logged if cleanup fails
- [ ] Manual recovery instructions provided when cleanup fails
- [ ] Works in both single worktree and `--all` modes

## Technical Requirements
- Modify file: `packages/cli/src/cli/worktree.ts`
- Add cleanup call after `removeDirSync(targetPath)` but before final success message
- Import `cleanMaproomRecords()` from appropriate location
- Wrap in try-catch for graceful error handling
- Check `opts.keepMaproom` flag before calling cleanup
- Use `logger.info()` for success, `logger.warn()` for failures
- Provide manual cleanup command in warning message
- Ensure cleanup happens after ALL worktrees removed in `--all` mode

## Implementation Notes
Add the cleanup step to the clean command action handler:

```typescript
// EXISTING: Remove directory
if (!opts.keepDir) {
  removeDirSync(targetPath)
}
logger.success(`Removed worktree ${targetPath}`)

// NEW: Clean maproom database records (best-effort)
if (!opts.keepMaproom) {
  try {
    await cleanMaproomRecords()
    logger.info('Cleaned maproom database records')
  } catch (err) {
    logger.warn('Could not clean maproom records:', err.message)
    logger.info('Run manually: crewchief-maproom db cleanup-stale --confirm')
  }
}
```

**Error handling strategy:**
- Best-effort cleanup - failures are warnings, not errors
- Don't block cleanup completion if maproom step fails
- Provide clear manual recovery instructions
- Log enough detail for debugging but keep messages user-friendly

**For `--all` mode:**
- Run maproom cleanup ONCE after all worktrees removed
- Batch cleanup detects all stale worktrees automatically
- Don't run cleanup for each individual worktree (inefficient)

**Manual recovery instructions:**
When cleanup fails, user sees:
```
Warning: Could not clean maproom records: [error message]
Run manually: crewchief-maproom db cleanup-stale --confirm
```

**Timing considerations:**
- Cleanup runs AFTER directory removal (directory must be gone for stale detection)
- Cleanup runs BEFORE final success message (logical flow)
- In `--all` mode, cleanup runs after ALL removals complete

## Dependencies
- **WTCLEAN-1001** (Binary discovery utility)
- **WTCLEAN-1002** (Cleanup helper function)
- **WTCLEAN-2001** (CLI flags) - For `--keep-maproom` flag

## Risk Assessment
- **Risk**: Maproom binary not found blocks cleanup
  - **Mitigation**: Catch errors, log warning, continue cleanup
- **Risk**: Database locked during cleanup
  - **Mitigation**: Error caught, manual recovery instructions provided
- **Risk**: Cleanup called too early (directory not removed yet)
  - **Mitigation**: Call cleanup AFTER `removeDirSync()` completes
- **Risk**: Cleanup called multiple times in `--all` mode
  - **Mitigation**: Run cleanup once after all removals, not per worktree

## Files/Packages Affected
- `packages/cli/src/cli/worktree.ts` (modify clean command action)
- Import from `packages/cli/src/git/worktrees.ts` or wherever `cleanMaproomRecords()` lives

## Verification Notes
Verify-ticket agent should check:
- [ ] `cleanMaproomRecords()` imported correctly
- [ ] Cleanup called after directory removal
- [ ] Cleanup wrapped in try-catch
- [ ] `opts.keepMaproom` check present
- [ ] Success message logged on completion
- [ ] Warning message logged on failure
- [ ] Manual recovery command provided in warning
- [ ] No errors thrown that block cleanup
- [ ] Integration works in both single and `--all` modes
- [ ] Existing tests still pass
