# Ticket: [WTCLEAN-2003]: Integrate Branch Deletion in Clean Command

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
Integrate git branch deletion into the `worktree clean` command flow, using `GitMergeService.deleteBranch()` to remove the branch unless `--keep-branch` flag is set.

## Background
After removing a worktree, the associated git branch often becomes obsolete. This integration ensures branch deletion happens automatically as part of cleanup, matching user expectations of complete cleanup.

This ticket implements Phase 2, Deliverable 3 from the plan: Branch deletion integration.

**CRITICAL SEQUENCING REQUIREMENT:** Branch name MUST be extracted from worktree metadata BEFORE calling `removeWorktree()`. Once the worktree is removed, git metadata is gone and the branch name cannot be determined.

## Acceptance Criteria
- [ ] Branch name extracted BEFORE worktree removal (CRITICAL)
- [ ] Branch deletion called after directory removal
- [ ] Deletion only runs if `--keep-branch` flag NOT set
- [ ] Uses `GitMergeService.deleteBranch()` (already exists)
- [ ] Uses safe delete (`git branch -d`, not force delete `-D`)
- [ ] Success logged when branch deleted
- [ ] Errors are caught and do NOT block cleanup (best-effort)
- [ ] Clear warning logged if deletion fails
- [ ] Manual recovery instructions provided when deletion fails
- [ ] Works in both single worktree and `--all` modes
- [ ] Handles case where branch name is unknown/missing

## Technical Requirements
- Modify file: `packages/cli/src/cli/worktree.ts`
- Extract branch name from worktree match BEFORE calling `removeWorktree()`
- Add deletion call after maproom cleanup step
- Import `GitMergeService` from `packages/cli/src/git/merge.ts`
- Instantiate service: `const mergeService = new GitMergeService()`
- Call: `await mergeService.deleteBranch(branch)`
- Wrap in try-catch for graceful error handling
- Check `opts.keepBranch` flag before calling deletion
- Check that branch name exists before attempting deletion
- Use `logger.success()` for success, `logger.warn()` for failures
- Provide manual cleanup command in warning message

## Implementation Notes
**CRITICAL: Extract branch name BEFORE worktree removal:**

```typescript
// At the start of clean action, BEFORE removeWorktree:
const branch = matches[0].branch  // Capture branch name early

// EXISTING: Remove git worktree metadata
await wt.removeWorktree(targetPath)

// EXISTING: Remove directory
if (!opts.keepDir) {
  removeDirSync(targetPath)
}
logger.success(`Removed worktree ${targetPath}`)

// EXISTING: Clean maproom (from WTCLEAN-2002)
if (!opts.keepMaproom) {
  try {
    await cleanMaproomRecords()
    logger.info('Cleaned maproom database records')
  } catch (err) {
    logger.warn('Could not clean maproom records:', err.message)
    logger.info('Run manually: crewchief-maproom db cleanup-stale --confirm')
  }
}

// NEW: Delete git branch (best-effort)
if (branch && !opts.keepBranch) {
  try {
    const mergeService = new GitMergeService()
    await mergeService.deleteBranch(branch)
    logger.success(`Deleted branch ${branch}`)
  } catch (err) {
    logger.warn(`Could not delete branch ${branch}:`, err.message)
    logger.info(`Delete manually: git branch -d ${branch}`)
  }
}
```

**Why safe delete (`-d`) not force (`-D`):**
- Prevents accidental loss of unmerged work
- Matches existing `worktree merge` behavior
- Error message guides user to manual cleanup if needed
- Can add `--force` flag in future if needed

**Branch name handling:**
- Extract from `matches[0].branch` BEFORE removal
- Check `if (branch && !opts.keepBranch)` before deletion
- If branch is undefined/null, skip deletion (no warning needed)
- In `--all` mode, extract branch for each worktree in loop

**Error cases that should NOT fail cleanup:**
- Branch not fully merged → Warning, continue
- Branch doesn't exist → Warning, continue
- Branch checked out elsewhere → Warning, continue
- Any other git error → Warning, continue

**Manual recovery instructions:**
When deletion fails, user sees:
```
Warning: Could not delete branch feature-123: not fully merged
Delete manually: git branch -d feature-123
  (or git branch -D feature-123 to force delete)
```

**Sequencing diagram:**
```
1. Extract branch name from worktree metadata ← MUST BE FIRST
2. Remove git worktree metadata
3. Remove directory
4. Clean maproom records (if not --keep-maproom)
5. Delete branch (if not --keep-branch) ← Uses branch from step 1
```

## Dependencies
- **WTCLEAN-2001** (CLI flags) - For `--keep-branch` flag
- **WTCLEAN-2002** (Maproom cleanup) - Should run before branch deletion

## Risk Assessment
- **Risk**: Branch name extracted AFTER worktree removal (won't work)
  - **Mitigation**: Explicit extraction at start of action, integration test validates
- **Risk**: Force delete loses unmerged work
  - **Mitigation**: Use safe delete (-d), require manual force if needed
- **Risk**: Branch protected or checked out elsewhere
  - **Mitigation**: Catch error, log warning, continue cleanup
- **Risk**: Branch deletion attempted when branch undefined
  - **Mitigation**: Check `if (branch && ...)` before attempting deletion

## Files/Packages Affected
- `packages/cli/src/cli/worktree.ts` (modify clean command action)
- Import `GitMergeService` from `packages/cli/src/git/merge.ts`

## Verification Notes
Verify-ticket agent should check:
- [ ] **CRITICAL**: Branch name extracted BEFORE `removeWorktree()` call
- [ ] `GitMergeService` imported correctly
- [ ] Branch deletion called after maproom cleanup
- [ ] Deletion wrapped in try-catch
- [ ] `opts.keepBranch` check present
- [ ] `branch` existence check present (handles undefined)
- [ ] Success message logged on completion
- [ ] Warning message logged on failure
- [ ] Manual recovery command provided in warning
- [ ] Uses safe delete (`deleteBranch()`, not force delete)
- [ ] No errors thrown that block cleanup
- [ ] Integration works in both single and `--all` modes
- [ ] Existing tests still pass
- [ ] Integration test validates branch extraction timing
