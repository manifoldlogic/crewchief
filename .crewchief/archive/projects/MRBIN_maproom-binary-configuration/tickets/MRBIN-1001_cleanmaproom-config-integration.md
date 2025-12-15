# Ticket: [MRBIN-1001]: Clean Maproom Records Config Integration

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - all tests passing
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update the `cleanMaproomRecords()` function in `packages/cli/src/git/worktrees.ts` to support config-based binary resolution, enabling it to use `maproomBinaryPath` from the configuration file like other maproom commands.

## Background
This project completes an existing binary configuration feature. Most implementation already exists:
- Config schema field (`maproomBinaryPath`) - complete
- Binary resolution function (`findMaproomBinary()`) - complete
- Comprehensive test coverage (26 tests) - complete
- User documentation - complete

However, one function (`cleanMaproomRecords`) doesn't use config-based resolution. This ticket fixes that gap, making binary resolution consistent across all CLI commands.

## Acceptance Criteria
- [x] `cleanMaproomRecords()` function signature accepts optional `config` parameter
- [x] Config is loaded internally if not provided to function
- [x] Config path (`config.repository.maproomBinaryPath`) is passed to `findMaproomBinary()`
- [x] `configFileLocation` parameter intentionally omitted (relative paths resolve from CWD, not config file - acceptable MVP limitation)
- [x] Error handling prevents crashes if config is invalid or missing (graceful fallback)
- [x] Existing three call sites (lines 216, 328, 390) continue to work unchanged
- [x] All existing tests pass
- [x] No TypeScript compilation errors

## Technical Requirements
- Import `loadConfig` from `../config/index.js` if not already imported
- Import `CrewChiefConfig` type from `../config/schema.js` if not already imported
- Modify function signature: `export async function cleanMaproomRecords(config?: CrewChiefConfig): Promise<void>`
- Add config loading logic with try-catch error handling
- Pass config path to `findMaproomBinary({ configPath: resolvedConfig?.repository.maproomBinaryPath })`
- Do NOT pass `configFileLocation` parameter (intentional design decision for MVP)
- Maintain backwards compatibility - function works with or without config parameter

## Implementation Notes

**Current implementation (line 240):**
```typescript
export async function cleanMaproomRecords(): Promise<void> {
  const result = findMaproomBinary()
  // ...
}
```

**Target implementation:**
```typescript
export async function cleanMaproomRecords(config?: CrewChiefConfig): Promise<void> {
  let resolvedConfig = config
  if (!resolvedConfig) {
    try {
      resolvedConfig = await loadConfig()
    } catch {
      // Config not found or invalid - continue without it
      // Binary resolution will fall back to env var/global/packaged
    }
  }

  const result = findMaproomBinary({
    configPath: resolvedConfig?.repository.maproomBinaryPath
    // Note: configFileLocation intentionally omitted
    // Relative paths will be relative to CWD, not config file
    // This is an acceptable MVP limitation
  })
  // ... rest of function unchanged
}
```

**Why configFileLocation is omitted:**
- MVP scope limitation to keep changes minimal
- Most users use absolute paths or paths relative to project root
- Relative path resolution from CWD is acceptable for this use case
- Can be enhanced in future if needed
- Doesn't break existing functionality

## Dependencies
None - This is the first ticket of the project. All required infrastructure already exists:
- Config schema defined
- Resolution function implemented
- Test infrastructure in place

## Risk Assessment
- **Risk**: Breaking existing usage of `cleanMaproomRecords()`
  - **Mitigation**: Config parameter is optional; all three existing call sites work without changes
- **Risk**: Config load errors crash the function
  - **Mitigation**: Try-catch around `loadConfig()` with graceful fallback to undefined config
- **Risk**: TypeScript type errors with optional parameter
  - **Mitigation**: Use optional chaining (`resolvedConfig?.repository.maproomBinaryPath`)
- **Risk**: Regression in binary resolution
  - **Mitigation**: Run full test suite; 26+ existing tests catch issues

## Files/Packages Affected
- `packages/cli/src/git/worktrees.ts` (line 240-265, function signature and body)
- No changes needed to call sites (lines 216, 328, 390)

## Verification Notes
Verify that:
1. Function signature includes optional config parameter
2. Config is loaded when not provided
3. Config path is correctly passed to `findMaproomBinary()`
4. Error handling works (test with invalid config scenario)
5. All three call sites compile without errors
6. All existing tests pass
7. No TypeScript compilation errors
8. Binary resolution works with and without config parameter

## Planning References
- Plan: `.crewchief/projects/MRBIN_maproom-binary-configuration/planning/plan.md` (Phase 1)
- Architecture: `.crewchief/projects/MRBIN_maproom-binary-configuration/planning/architecture.md` (Component Design section)
