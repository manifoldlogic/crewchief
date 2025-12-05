# Ticket: [MRBIN-1004]: Convert Maproom Action Handlers to Async

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
- test-runner
- verify-ticket
- commit-ticket

## Summary
Convert all Commander action handlers in packages/cli/src/cli/maproom.ts to async functions to support async config loading required in Phase 2. This is a prerequisite for integrating the shared binary utility.

## Background
Phase 2 will integrate config loading into maproom.ts to retrieve maproomBinaryPath. Config loading is async (via loadConfig()), so action handlers must be async to use await. This ticket converts the handlers to async without changing behavior.

This conversion is safe because:
1. Commander.js fully supports async action handlers
2. The pattern already exists in worktrees.ts (runMaproomScan is async)
3. No behavior changes - just wrapping in async/await

This ticket can run in parallel with MRBIN-1002 (utility implementation) since they don't conflict.

## Acceptance Criteria
- [x] All maproom command action handlers are async functions
- [x] runMaproomForward() function signature is async
- [x] Action handlers use await when calling runMaproomForward()
- [x] Existing maproom commands still work correctly
- [x] All existing maproom integration tests pass
- [x] No TypeScript errors related to async/await
- [x] Error handling preserved (exit codes, error messages)

## Technical Requirements
- Change function signature: `function runMaproomForward(args)` → `async function runMaproomForward(args)`
- Update action handlers: `.action((args) => runMaproomForward([...]))` → `.action(async (args) => await runMaproomForward([...]))`
- Preserve all existing error handling logic
- Maintain process.exitCode setting behavior
- No changes to spawnSync calls (those remain synchronous)

## Implementation Notes
Pattern to follow:

```typescript
// Before
function runMaproomForward(args: string[]) {
  // ... validation ...
  const result = findMaproomBinary()
  const res = spawnSync(result.path, args, { stdio: 'inherit' })
  if (res.status !== 0) process.exitCode = res.status ?? 1
}

program
  .command('scan')
  .action((args) => runMaproomForward(['scan', ...(args || [])]))

// After
async function runMaproomForward(args: string[]) {
  // ... validation ...
  const result = findMaproomBinary()
  const res = spawnSync(result.path, args, { stdio: 'inherit' })
  if (res.status !== 0) process.exitCode = res.status ?? 1
}

program
  .command('scan')
  .action(async (args) => await runMaproomForward(['scan', ...(args || [])]))
```

Commander.js documentation confirms async action support: https://github.com/tj/commander.js#action-handler-subcommand-as-argument

Apply this pattern to all maproom subcommands: scan, search, show, chat, etc.

## Dependencies
None - This can run in parallel with other Phase 1 tickets.

## Risk Assessment
- **Risk**: Async conversion breaks existing command behavior
  - **Mitigation**: Test all maproom commands after conversion, run integration tests
- **Risk**: Error handling changes inadvertently
  - **Mitigation**: Preserve all existing error handling logic exactly, verify exit codes
- **Risk**: Commander.js doesn't support async actions (unlikely)
  - **Mitigation**: Verify with existing async patterns in codebase, check Commander docs

## Files/Packages Affected
- packages/cli/src/cli/maproom.ts

## Verification Notes
Verify that:
1. All action handlers are async functions
2. All maproom commands still execute correctly
3. Error handling still works (exit codes, error messages)
4. Integration tests pass without modification
5. No TypeScript compilation errors
6. Manual testing: `crewchief maproom scan` still works
7. Process exit behavior unchanged
8. Pattern is consistent across all commands
