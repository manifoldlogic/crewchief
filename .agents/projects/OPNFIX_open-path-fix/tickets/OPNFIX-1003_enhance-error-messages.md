# Ticket: OPNFIX-1003: Enhance Error Messages for Path Resolution Failures

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update error messages in open.ts to provide actionable debugging information when path resolution fails, including candidate count, database pollution detection, and cleanup suggestions.

## Background
Current error messages when path resolution fails are unhelpful: "ENOENT: no such file or directory". Users need to understand WHY paths are wrong and what to do about it.

With the multi-candidate fallback implementation from OPNFIX-1001, we now have additional context (number of candidates tried, detection of database pollution) that should be surfaced in error messages to guide users to remediation.

This ticket completes Phase 1 by ensuring the fix is observable and actionable for users.

## Acceptance Criteria
- [ ] Error for "no worktrees" lists available worktrees
- [ ] Error for "all failed" includes candidate count
- [ ] Error messages mention database pollution when detected
- [ ] Error includes suggestion to run cleanup command
- [ ] Debug logs show each candidate path attempted
- [ ] Debug logs show successful selection
- [ ] No sensitive path information leaked in production errors
- [ ] Follows existing pino logging patterns

## Technical Requirements
- Use existing ValidationError class from utils/validation.ts
- Use pino logger (imported as `log`) for debug messages
- Error messages format: `File '${relpath}' not accessible in worktree '${worktreeName}'`
- Debug log level for candidate attempts
- Info log level for successful resolution
- Error log level for complete failures
- Ensure error messages are user-friendly and actionable
- Internal paths may appear in debug logs but not in error messages shown to users

## Implementation Notes

### File Location
`packages/maproom-mcp/src/tools/open.ts` (lines 51-85, error handling in getWorktreePath)

### Error Message Scenarios

#### Scenario 1: No Worktrees Found in Database
**Trigger**: SQL query returns 0 rows

**Error Message**:
```typescript
throw new ValidationError(
  `File '${relpath}' not found in worktree '${worktreeName}'. ` +
  `No matching worktree found in database. ` +
  `Ensure the repository is indexed and the worktree name is correct.`,
  'FILE_NOT_FOUND'
)
```

#### Scenario 2: All Candidates Failed Filesystem Validation
**Trigger**: Loop through all rows, none pass fileExists() check

**Error Message**:
```typescript
throw new ValidationError(
  `File '${relpath}' not accessible in worktree '${worktreeName}'. ` +
  `Tried ${rows.length} candidate path${rows.length > 1 ? 's' : ''} but none exist on disk. ` +
  `This indicates database pollution. Run 'maproom db cleanup-stale' to fix.`,
  'FILE_NOT_FOUND'
)
```

**Debug Log** (before throwing):
```typescript
log.error({
  relpath,
  worktreeName,
  candidatesAttempted: rows.length,
  issue: 'database_pollution'
}, 'All candidate worktree paths failed filesystem validation')
```

### Debug Logging for Candidate Attempts

**During Loop** (each candidate):
```typescript
for (const row of rows) {
  const fullPath = path.join(row.abs_path, relpath)
  const exists = await fileExists(fullPath)

  log.debug({
    candidate: row.abs_path,
    relpath,
    fullPath,
    exists
  }, 'Checking candidate worktree path')

  if (exists) {
    log.info({
      selected: row.abs_path,
      relpath,
      worktreeName
    }, 'Selected valid worktree path')
    return row.abs_path
  }
}
```

### Security Considerations
- **Production errors**: Only include user-provided parameters (relpath, worktreeName)
- **Debug logs**: Can include internal paths (abs_path) since debug logs are for developers
- **No password/credentials**: None involved in path resolution
- **Path traversal**: Already handled by validateWithinRepo() (not in scope for this ticket)

## Dependencies
- **Requires**: OPNFIX-1001 (getWorktreePath updates that provide candidate context)
- **Blocks**: None (enhances existing functionality)

## Risk Assessment
- **Risk**: Error messages might expose too much internal information
  - **Mitigation**: Review all error messages to ensure only user-provided data in production errors
- **Risk**: Excessive debug logging could impact performance
  - **Mitigation**: Debug logs only at appropriate points; pino is performant
- **Risk**: Error message changes might break client-side error parsing
  - **Mitigation**: ValidationError code ('FILE_NOT_FOUND') remains constant; message is supplemental

## Files/Packages Affected
- `packages/maproom-mcp/src/tools/open.ts` (error handling and logging in getWorktreePath)
