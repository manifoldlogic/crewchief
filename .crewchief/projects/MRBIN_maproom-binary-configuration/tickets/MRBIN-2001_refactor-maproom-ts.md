# Ticket: [MRBIN-2001]: Refactor maproom.ts to Use Shared Utility

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
- test-runner
- verify-ticket
- commit-ticket

## Summary
Refactor packages/cli/src/cli/maproom.ts to use the shared findMaproomBinary() utility with config-based resolution, removing the old resolvePackagedMaproomBin() function and implementing improved error messages with resolution path details.

## Background
This ticket completes the CLI integration by replacing the duplicated binary resolution logic in maproom.ts with the shared utility. This enables config-based binary path configuration and removes ~86 lines of duplicated code.

This is where the behavior change occurs: global installs now take priority over packaged binaries. The change is intentional and improves the user experience by avoiding stale packaged binaries when a global install exists.

## Acceptance Criteria
- [ ] Import findMaproomBinary and loadConfig at top of file
- [ ] runMaproomForward() loads config and passes maproomBinaryPath to utility
- [ ] Config loading handles missing config gracefully (no error, falls through)
- [ ] Old resolvePackagedMaproomBin() function removed
- [ ] Error message shows all resolution attempts with paths
- [ ] Error message includes configuration guidance
- [ ] All existing maproom integration tests pass
- [ ] Commands work without config file (backwards compatible)
- [ ] Resolution order is: env > config > global > packaged

## Technical Requirements
- Import: `import { findMaproomBinary } from '../utils/maproom-binary.js'`
- Import: `import { loadConfig } from '../config/loader.js'`
- Load config with try-catch (handle missing config)
- Pass config.repository.maproomBinaryPath to findMaproomBinary()
- Update error message to show resolution attempts
- Remove old resolvePackagedMaproomBin() function
- Verify action handlers are async (from MRBIN-1004)

## Implementation Notes
Update runMaproomForward() as specified in architecture.md:

```typescript
async function runMaproomForward(args: string[]) {
  // ... existing validation logic ...

  // Load config to get binary path (handle missing config gracefully)
  let configPath: string | undefined
  try {
    const config = await loadConfig()
    configPath = config.repository.maproomBinaryPath
  } catch (error) {
    // Config file missing or invalid - continue with defaults
    logger.debug('No config file found, using default binary resolution')
  }

  const result = findMaproomBinary({ configPath })

  if (!result.path) {
    console.error(
      'crewchief-maproom not found. Options:\n' +
      '1. Install globally: npm install -g @crewchief/cli\n' +
      '2. Set CREWCHIEF_MAPROOM_BIN environment variable\n' +
      '3. Add maproomBinaryPath to crewchief.config.js\n\n' +
      'Resolution attempts:\n' +
      '- Environment: ' + (process.env.CREWCHIEF_MAPROOM_BIN || 'not set') + '\n' +
      '- Config: ' + (configPath || 'not configured') + '\n' +
      '- Global: not found\n' +
      '- Packaged: not found'
    )
    process.exitCode = 1
    return
  }

  const res = spawnSync(result.path, args, { stdio: 'inherit' })
  if (res.status !== 0) process.exitCode = res.status ?? 1
}
```

Remove the old resolvePackagedMaproomBin() function entirely (~60 lines).

## Dependencies
- MRBIN-1001 (Config schema must exist)
- MRBIN-1002 (Utility must exist)
- MRBIN-1003 (Tests must pass)
- MRBIN-1004 (Async conversion must be complete)

## Risk Assessment
- **Risk**: Breaking existing maproom commands
  - **Mitigation**: Run all integration tests, test each command manually
- **Risk**: Config loading errors breaking commands
  - **Mitigation**: Wrap in try-catch, fall through gracefully
- **Risk**: Priority order change affects users with both installs
  - **Mitigation**: Intentional improvement, document in release notes

## Files/Packages Affected
- packages/cli/src/cli/maproom.ts

## Verification Notes
Verify that:
1. All maproom commands work: scan, search, show, chat
2. Commands work without config file present
3. Environment variable override still works
4. Error message is helpful and shows resolution attempts
5. All integration tests pass
6. No resolvePackagedMaproomBin() references remain
7. Config loading is wrapped in try-catch
8. Binary resolution uses correct precedence order
9. TypeScript compilation succeeds
10. Code diff shows ~60 lines removed
