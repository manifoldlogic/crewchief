# Ticket: [WTCLEAN-2004c]: Add Logging for All Cleanup Steps

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
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
Add comprehensive logging for all cleanup steps to provide clear feedback about what was done, what was skipped, and what failed during the cleanup process.

## Background
Users need visibility into what the clean command is doing, especially with the new multi-step cleanup flow. Clear logging helps users understand what succeeded, what was skipped (via flags), and what failed.

This ticket implements part of Phase 2, Deliverable 4 from the plan: Logging for all cleanup steps.

## Acceptance Criteria
- [x] Each cleanup step has clear start/completion logging
- [x] Success messages use `logger.success()` (green checkmarks)
- [x] Skipped steps logged with `logger.info()` explaining why
- [x] Progress visible for `--all` mode (show count/total)
- [x] Final summary shows what was cleaned
- [x] Logging consistent across single and `--all` modes
- [x] No redundant or verbose logging (keep it clean)
- [x] Log messages help debug issues without overwhelming users

## Technical Requirements
- Add logging for each cleanup step:
  - Directory removal (already exists)
  - Maproom cleanup (enhance)
  - Branch deletion (enhance)
- Add logging for skipped steps:
  - `--keep-dir` skips directory removal
  - `--keep-maproom` skips maproom cleanup
  - `--keep-branch` skips branch deletion
- Add progress logging for `--all` mode:
  - "Cleaning worktree 1 of 5..."
  - Summary at end: "Cleaned 5 worktrees, deleted 4 branches"
- Use consistent log levels:
  - `logger.success()` for successful operations
  - `logger.info()` for skipped steps and informational messages
  - `logger.warn()` for best-effort failures
  - `logger.error()` for fatal errors only
- Add summary at end of cleanup showing totals

## Implementation Notes
Enhance logging throughout the clean command:

```typescript
// Progress for --all mode
if (matches.length > 1) {
  logger.info(`Found ${matches.length} worktrees to clean`)
}

for (const [index, match] of matches.entries()) {
  if (matches.length > 1) {
    logger.info(`\nCleaning worktree ${index + 1} of ${matches.length}: ${match.path}`)
  }

  // Extract branch name first
  const branch = match.branch

  // Remove worktree metadata
  await wt.removeWorktree(targetPath)
  logger.success('Removed worktree metadata')

  // Remove directory
  if (opts.keepDir) {
    logger.info('Keeping directory (--keep-dir flag)')
  } else {
    removeDirSync(targetPath)
    logger.success(`Removed directory ${targetPath}`)
  }

  // Maproom cleanup
  if (opts.keepMaproom) {
    logger.info('Skipping maproom cleanup (--keep-maproom flag)')
  } else {
    try {
      await cleanMaproomRecords()
      logger.success('Cleaned maproom database records')
    } catch (err) {
      // Error handling from WTCLEAN-2004a
    }
  }

  // Branch deletion
  if (!branch) {
    logger.info('No branch associated with worktree')
  } else if (opts.keepBranch) {
    logger.info(`Keeping branch ${branch} (--keep-branch flag)`)
  } else {
    try {
      const mergeService = new GitMergeService()
      await mergeService.deleteBranch(branch)
      logger.success(`Deleted branch ${branch}`)
    } catch (err) {
      // Error handling from WTCLEAN-2004b
    }
  }
}

// Final summary for --all mode
if (matches.length > 1) {
  logger.success(`\nCleaned ${matches.length} worktrees`)
}
```

**Logging best practices:**
- Be informative but concise
- Use color-coded levels (success=green, warn=yellow, info=white)
- Group related messages (blank lines between worktrees in `--all` mode)
- Show what happened, not what's happening (past tense)
- Don't log internal implementation details

**Progress tracking for `--all` mode:**
- Show total count at start
- Show current/total for each worktree
- Show summary at end
- Use blank lines to separate worktree sections

**Skipped step messages:**
- Explain WHY skipped (flag name)
- Use info level (not warning or success)
- Be brief: "Keeping directory (--keep-dir flag)"

**Summary format:**
```
Cleaned 5 worktrees
- Removed 5 directories
- Deleted 4 branches (1 skipped: not fully merged)
- Cleaned maproom database records
```

## Dependencies
- **WTCLEAN-2002** (Maproom cleanup) - Enhance logging
- **WTCLEAN-2003** (Branch deletion) - Enhance logging
- **WTCLEAN-2004a** (Maproom error handling) - Works with logging
- **WTCLEAN-2004b** (Branch error handling) - Works with logging

## Risk Assessment
- **Risk**: Logging too verbose, clutters output
  - **Mitigation**: Be concise, only log meaningful events
- **Risk**: Logging inconsistent between modes
  - **Mitigation**: Test both single and `--all` modes, ensure consistency
- **Risk**: Summary counts incorrect
  - **Mitigation**: Track counts in variables, verify with tests

## Files/Packages Affected
- `packages/cli/src/cli/worktree.ts` (enhance logging throughout)

## Verification Notes
Verify-ticket agent should check:
- [ ] Each cleanup step has logging
- [ ] Success messages use `logger.success()`
- [ ] Skipped steps logged with explanation
- [ ] `--all` mode shows progress (N of M)
- [ ] `--all` mode shows final summary
- [ ] Logging consistent in single and `--all` modes
- [ ] No redundant or verbose logging
- [ ] Log messages are user-friendly
- [ ] Manual test shows clear, readable output
- [ ] All tests pass
