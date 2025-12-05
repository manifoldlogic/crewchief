# Project: Enhanced Worktree Clean

**Slug:** WTCLEAN_enhanced-worktree-clean
**Priority:** High
**Effort:** M (2-3 days)

## Summary

Extend `crewchief worktree clean` to perform complete cleanup: remove directory, delete git branch, and delete maproom database records. Current implementation only removes directory and git worktree metadata.

## Deliverables

1. **Maproom cleanup integration** - Call maproom to delete worktree database records
2. **Git branch deletion** - Delete the worktree branch after cleanup
3. **Error handling** - Graceful handling when maproom unavailable or records don't exist
4. **Repository detection** - Extract repo name from worktree path for maproom cleanup
5. **Updated tests** - Verify all cleanup steps and failure scenarios
6. **Documentation** - Update README with comprehensive cleanup behavior

## Dependencies

**Requires:** MRBIN (binary resolution logic)
**Optional:** WTPATH (path utilities)

## Value Proposition

Single command removes all traces of a worktree, preventing database bloat and stale search results. Developers no longer need to manually clean maproom separately or remember to delete branches.

## Technical Approach

1. Update `worktree clean` command flow:
   ```typescript
   // Get branch name before removing worktree
   const branch = matches[0].branch

   // Remove git worktree and directory (existing)
   await wt.removeWorktree(targetPath)
   removeDirSync(targetPath)

   // NEW: Delete maproom records (best-effort)
   try {
     const maproomBin = findMaproomBinary(config) // from MRBIN
     const { repoName, worktreeName } = parseWorktreePath(targetPath)
     // Call: crewchief-maproom cleanup --repo <repo> --worktree <name>
     // Or use daemon client if available
   } catch (err) {
     logger.warn('Could not clean maproom records:', err.message)
   }

   // NEW: Delete git branch
   if (branch) {
     await GitMergeService.deleteBranch(branch)
   }
   ```

2. Add `--keep-branch` flag to preserve branch if needed
3. Add `--keep-maproom` flag to skip database cleanup (for testing)
4. Test failure scenarios: maproom binary missing, db locked, branch in use

## Acceptance Criteria

- [ ] `clean` command deletes directory (existing)
- [ ] `clean` command deletes git worktree metadata (existing)
- [ ] `clean` command deletes git branch (new)
- [ ] `clean` command deletes maproom records (new)
- [ ] Cleanup succeeds even if maproom unavailable (best-effort)
- [ ] Clear logging for each cleanup step
- [ ] `--keep-branch` flag preserves branch
- [ ] Tests cover failure scenarios

## Breaking Changes

**Non-breaking:** This is an enhancement to existing command. Adds new cleanup steps but doesn't change command interface.

**New behavior:** Branches are now deleted by default. Add `--keep-branch` flag for users who want to preserve branches.

## Edge Cases to Handle

- Maproom binary not found → log warning, continue
- Worktree not in database → log info, continue
- Branch doesn't exist → log info, continue
- Database locked → log warning, continue
- Branch currently checked out elsewhere → error, skip deletion
