# Ticket: [WTCLEAN-2004a]: Add Error Handling for Maproom Cleanup

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
Enhance error handling for maproom cleanup failures, ensuring detailed logging and manual recovery guidance for all failure scenarios.

## Background
Maproom cleanup can fail for various reasons (binary not found, database locked, etc.). Users need clear feedback about what failed and how to recover manually.

This ticket implements part of Phase 2, Deliverable 4 from the plan: Error handling for maproom cleanup.

## Acceptance Criteria
- [x] All maproom cleanup errors caught and handled gracefully
- [x] Binary not found error includes helpful message
- [x] Database locked error includes retry guidance
- [x] Unknown errors include error details
- [x] All errors logged as warnings (yellow), not errors (red)
- [x] Manual recovery command provided for each scenario
- [x] Error messages user-friendly (not raw technical errors)
- [x] Cleanup continues after maproom errors (best-effort)
- [x] Tests added for all error scenarios

## Technical Requirements
- Enhance try-catch block in maproom cleanup section
- Catch specific error types when possible
- Provide context-specific manual recovery instructions
- Use `logger.warn()` for all maproom failures
- Use `logger.info()` for recovery instructions
- Include relevant error details (first line of error message)
- Don't expose raw stack traces to users
- Test all error scenarios with mocks

## Implementation Notes
Enhance the error handling from WTCLEAN-2002:

```typescript
if (!opts.keepMaproom) {
  try {
    await cleanMaproomRecords()
    logger.info('Cleaned maproom database records')
  } catch (err) {
    const errorMsg = err instanceof Error ? err.message : String(err)

    // Provide context-specific guidance based on error type
    if (errorMsg.includes('Binary not found') || errorMsg.includes('ENOENT')) {
      logger.warn('Maproom binary not found - database cleanup skipped')
      logger.info('To clean manually:')
      logger.info('  1. Install crewchief-maproom (pnpm install -g)')
      logger.info('  2. Run: crewchief-maproom db cleanup-stale --confirm')
    } else if (errorMsg.includes('database is locked') || errorMsg.includes('SQLITE_BUSY')) {
      logger.warn('Maproom database is locked - cleanup skipped')
      logger.info('Wait for other maproom processes to complete, then run:')
      logger.info('  crewchief-maproom db cleanup-stale --confirm')
    } else {
      logger.warn('Could not clean maproom records:', errorMsg)
      logger.info('Run manually: crewchief-maproom db cleanup-stale --confirm')
    }
  }
}
```

**Error categories and responses:**

1. **Binary not found**
   - Detection: Error message includes "Binary not found" or ENOENT
   - Message: "Maproom binary not found - database cleanup skipped"
   - Recovery: Install instructions + manual command

2. **Database locked**
   - Detection: Error includes "database is locked" or SQLITE_BUSY
   - Message: "Maproom database is locked - cleanup skipped"
   - Recovery: Wait instruction + manual command

3. **Permission denied**
   - Detection: Error includes "permission denied" or EACCES
   - Message: "Permission denied accessing maproom database"
   - Recovery: Check permissions + manual command

4. **Unknown errors**
   - Detection: Any other error
   - Message: Include first line of error for debugging
   - Recovery: Generic manual command

**User-friendly error messages:**
- Avoid raw stack traces
- Use first line of error message only
- Add context about what failed
- Provide actionable recovery steps
- Don't make cleanup feel broken (it's best-effort)

**Testing approach:**
- Mock `cleanMaproomRecords()` to throw different error types
- Assert correct warning messages logged
- Assert correct recovery instructions provided
- Assert cleanup doesn't fail (continues after error)

## Dependencies
- **WTCLEAN-2002** (Maproom cleanup integration) - Enhances error handling in this ticket

## Risk Assessment
- **Risk**: Error detection too specific, misses edge cases
  - **Mitigation**: Catch-all case handles unknown errors
- **Risk**: Error messages too verbose
  - **Mitigation**: Keep messages concise, actionable
- **Risk**: Users don't read recovery instructions
  - **Mitigation**: Clear, step-by-step instructions, prominent logging

## Files/Packages Affected
- `packages/cli/src/cli/worktree.ts` (enhance error handling)
- Test files for error scenarios

## Verification Notes
Verify-ticket agent should check:
- [ ] Error handling enhanced with specific scenarios
- [ ] Binary not found error handled specifically
- [ ] Database locked error handled specifically
- [ ] Unknown errors have catch-all handler
- [ ] All errors use `logger.warn()` (not `logger.error()`)
- [ ] Manual recovery instructions provided for each scenario
- [ ] Error messages are user-friendly (no raw stack traces)
- [ ] Tests added for each error scenario
- [ ] Tests verify correct warning messages
- [ ] All tests pass
