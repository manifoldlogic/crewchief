# Ticket: [WTCLEAN-2004b]: Add Error Handling for Branch Deletion

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

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
Enhance error handling for branch deletion failures, ensuring detailed logging and manual recovery guidance for all failure scenarios.

## Background
Branch deletion can fail for various reasons (not fully merged, checked out elsewhere, protected branch, etc.). Users need clear feedback about what failed and how to safely resolve it manually.

This ticket implements part of Phase 2, Deliverable 4 from the plan: Error handling for branch deletion.

## Acceptance Criteria
- [ ] All branch deletion errors caught and handled gracefully
- [ ] Not fully merged error includes safe delete guidance
- [ ] Checked out elsewhere error includes worktree context
- [ ] Branch doesn't exist handled silently (not an error)
- [ ] Unknown errors include error details
- [ ] All errors logged as warnings (yellow), not errors (red)
- [ ] Manual recovery commands provided for each scenario
- [ ] Error messages user-friendly (not raw git errors)
- [ ] Cleanup continues after branch errors (best-effort)
- [ ] Tests added for all error scenarios

## Technical Requirements
- Enhance try-catch block in branch deletion section
- Catch specific git error types when possible
- Provide context-specific manual recovery instructions
- Use `logger.warn()` for failures
- Use `logger.info()` for recovery instructions
- Include relevant error details
- Handle "branch doesn't exist" as special case (not a failure)
- Don't expose raw stack traces or cryptic git messages
- Test all error scenarios with mocks

## Implementation Notes
Enhance the error handling from WTCLEAN-2003:

```typescript
if (branch && !opts.keepBranch) {
  try {
    const mergeService = new GitMergeService()
    await mergeService.deleteBranch(branch)
    logger.success(`Deleted branch ${branch}`)
  } catch (err) {
    const errorMsg = err instanceof Error ? err.message : String(err)

    // Provide context-specific guidance based on error type
    if (errorMsg.includes('not fully merged')) {
      logger.warn(`Branch ${branch} not fully merged - skipped deletion`)
      logger.info('To delete anyway (CAUTION: may lose work):')
      logger.info(`  git branch -D ${branch}`)
      logger.info('Or merge the branch first:')
      logger.info(`  git checkout main && git merge ${branch}`)
    } else if (errorMsg.includes('checked out')) {
      logger.warn(`Branch ${branch} checked out in another worktree - skipped deletion`)
      logger.info('Switch the other worktree to a different branch, then:')
      logger.info(`  git branch -d ${branch}`)
    } else if (errorMsg.includes("branch '") && errorMsg.includes("' not found")) {
      // Branch doesn't exist - this is fine, no warning needed
      logger.info(`Branch ${branch} already removed`)
    } else if (errorMsg.includes('Cannot delete')) {
      logger.warn(`Cannot delete branch ${branch}: protected or in use`)
      logger.info(`Check branch status: git branch -vv`)
      logger.info(`Manual deletion: git branch -d ${branch}`)
    } else {
      logger.warn(`Could not delete branch ${branch}:`, errorMsg)
      logger.info(`Delete manually: git branch -d ${branch}`)
    }
  }
}
```

**Error categories and responses:**

1. **Not fully merged**
   - Detection: Error includes "not fully merged"
   - Message: "Branch not fully merged - skipped deletion"
   - Recovery: Force delete warning OR merge instruction

2. **Checked out elsewhere**
   - Detection: Error includes "checked out"
   - Message: "Branch checked out in another worktree"
   - Recovery: Switch other worktree, then delete

3. **Branch doesn't exist**
   - Detection: Error includes "not found"
   - Message: Info only (not a warning) - "Branch already removed"
   - Recovery: None needed (already done)

4. **Protected/Cannot delete**
   - Detection: Error includes "Cannot delete"
   - Message: "Cannot delete branch: protected or in use"
   - Recovery: Check status, manual deletion

5. **Unknown errors**
   - Detection: Any other error
   - Message: Include error details
   - Recovery: Generic manual command

**User-friendly error messages:**
- Translate git error codes to plain English
- Provide both safe and force options for unmerged branches
- Explain WHY deletion failed (context matters)
- Don't assume user knows git internals
- Provide actionable next steps

**Force delete guidance:**
- Always show safe option first (`git branch -d`)
- Show force option with CAUTION warning
- Explain potential data loss
- Provide alternative (merge first)

**Testing approach:**
- Mock `GitMergeService.deleteBranch()` to throw different error types
- Assert correct warning messages logged
- Assert correct recovery instructions provided
- Assert cleanup doesn't fail (continues after error)

## Dependencies
- **WTCLEAN-2003** (Branch deletion integration) - Enhances error handling in this ticket

## Risk Assessment
- **Risk**: Force delete guidance too prominent, users lose work
  - **Mitigation**: Show safe option first, add CAUTION labels
- **Risk**: Error messages don't cover all git scenarios
  - **Mitigation**: Catch-all case handles unknown errors
- **Risk**: Users confused by "already removed" info message
  - **Mitigation**: Use info level (not warning), clear wording

## Files/Packages Affected
- `packages/cli/src/cli/worktree.ts` (enhance error handling)
- Test files for error scenarios

## Verification Notes
Verify-ticket agent should check:
- [ ] Error handling enhanced with specific scenarios
- [ ] Not fully merged error handled specifically
- [ ] Checked out elsewhere error handled specifically
- [ ] Branch doesn't exist treated as info (not warning)
- [ ] Protected branch error handled specifically
- [ ] Unknown errors have catch-all handler
- [ ] All errors use `logger.warn()` (except "already removed" uses `logger.info()`)
- [ ] Manual recovery instructions provided for each scenario
- [ ] Force delete shown with CAUTION warning
- [ ] Error messages are user-friendly (translate git errors)
- [ ] Tests added for each error scenario
- [ ] Tests verify correct warning messages
- [ ] All tests pass
