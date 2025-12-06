# Ticket: [WTCLEAN-1002]: Create Maproom Cleanup Helper Function

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
Create a helper function that invokes `crewchief-maproom db cleanup-stale --confirm` to remove stale worktree records from the maproom database.

## Background
After removing a worktree directory, the maproom database still contains records pointing to the deleted path. This helper function calls the maproom binary to clean up these stale records using batch cleanup (detects all stale worktrees, not just one).

This ticket implements Phase 1, Deliverable 2 from the plan: Maproom cleanup helper function.

## Acceptance Criteria
- [ ] `cleanMaproomRecords()` function created in appropriate location
- [ ] Function uses `findMaproomBinary()` to locate binary
- [ ] Function calls `spawnSync(maproomBin, ['db', 'cleanup-stale', '--confirm'])`
- [ ] Function handles exit code 0 (success) gracefully
- [ ] Function handles exit code 2 (no stale worktrees) as success
- [ ] Function throws error on exit code 1 (actual error)
- [ ] Function throws descriptive error when binary not found
- [ ] Error messages include first line of stderr/stdout for debugging

## Technical Requirements
- Add function to `packages/cli/src/git/worktrees.ts` or inline in command
- Function signature: `async function cleanMaproomRecords(): Promise<void>`
- Use Node.js `child_process.spawnSync` for synchronous execution
- Set encoding to `utf8` for readable output
- Configure stdio as `['pipe', 'pipe', 'pipe']` to capture all streams
- Parse exit codes:
  - `0` = success, records deleted
  - `2` = no stale worktrees found (not an error)
  - `1` or other = actual error, throw
- Extract first line of error output for user-friendly messages
- Import and use `findMaproomBinary()` from ticket WTCLEAN-1001

## Implementation Notes
Follow the pattern from architecture document:

```typescript
async function cleanMaproomRecords(): Promise<void> {
  // Find maproom binary
  const maproomBin = findMaproomBinary()

  if (!maproomBin) {
    throw new Error('Maproom binary not found')
  }

  // Run cleanup command
  const result = spawnSync(maproomBin, ['db', 'cleanup-stale', '--confirm'], {
    encoding: 'utf8',
    stdio: ['pipe', 'pipe', 'pipe'],
  })

  if (result.status !== 0 && result.status !== 2) {
    // Exit code 2 means "no stale worktrees", which is fine
    const errorMsg = result.stderr || result.stdout || 'Unknown error'
    throw new Error(errorMsg.split('\n')[0])
  }

  // Parse output for user feedback
  const output = result.stdout
  if (output.includes('Deleted')) {
    logger.info('Cleaned maproom database records')
  }
}
```

**Why batch cleanup:**
- Simpler implementation (no repo name parsing needed)
- More robust (cleans ANY stale worktrees, not just current one)
- Already exists in maproom binary (IDXCLEAN project built it)
- Minimal performance impact (<1 second difference)

**Exit code handling:**
- Exit code 2 is not an error - maproom returns this when no stale worktrees found
- Only exit code 1 indicates actual errors (database locked, permission denied, etc.)
- Extract first line of stderr for clear error messages

## Dependencies
- **WTCLEAN-1001** (Binary discovery utility) - MUST be completed first

## Risk Assessment
- **Risk**: Exit code 2 incorrectly treated as error
  - **Mitigation**: Explicitly check for both 0 and 2 as success codes
- **Risk**: Cryptic error messages from maproom binary
  - **Mitigation**: Extract first line of stderr/stdout for clarity
- **Risk**: Binary returns unexpected exit codes
  - **Mitigation**: Test with actual maproom binary, handle unknown codes as errors

## Files/Packages Affected
- `packages/cli/src/git/worktrees.ts` (add function) OR
- `packages/cli/src/cli/worktree.ts` (inline in command)
- Import `findMaproomBinary` from `packages/cli/src/utils/maproom-binary.ts`

## Verification Notes
Verify-ticket agent should check:
- [ ] `cleanMaproomRecords()` function exists and is accessible
- [ ] Function calls `findMaproomBinary()` from WTCLEAN-1001
- [ ] Function uses `spawnSync` with correct arguments
- [ ] Exit codes 0 and 2 both treated as success
- [ ] Exit code 1 throws error with descriptive message
- [ ] Binary not found throws error
- [ ] Function signature matches: `Promise<void>`
- [ ] No TypeScript compilation errors
- [ ] Error handling includes stderr/stdout parsing
