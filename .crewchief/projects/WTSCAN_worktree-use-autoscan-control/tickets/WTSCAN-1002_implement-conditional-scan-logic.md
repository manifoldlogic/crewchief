# Ticket: [WTSCAN-1002]: Implement Conditional Scan Logic in WorktreeService

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
- typescript-dev
- test-runner
- verify-ticket
- commit-ticket

## Summary
Modify `WorktreeService.createWorktree()` to load config once and conditionally call `runMaproomScan()` based on the `autoScanOnWorktreeUse` config field, with graceful error handling.

## Background
Currently, `WorktreeService.createWorktree()` unconditionally calls `runMaproomScan()` at line 143 in `packages/cli/src/git/worktrees.ts`. This ticket implements the conditional logic to check the config field (added in WTSCAN-1001) and only run the scan when explicitly enabled.

This implements the core logic portion of Phase 1, enabling user control over auto-scanning behavior while maintaining error resilience.

## Acceptance Criteria
- [x] Config is loaded once and reused for both `copyIgnoredFiles` and `autoScanOnWorktreeUse` checks
- [x] `runMaproomScan()` is called only when `config.worktree?.autoScanOnWorktreeUse === true`
- [x] `runMaproomScan()` is skipped when config is `false`, `undefined`, or missing
- [x] Config loading errors are caught and logged as warnings
- [x] Config loading errors do not prevent worktree creation
- [x] Existing `runMaproomScan()` method remains unchanged
- [x] Code follows existing error handling patterns
- [x] All existing tests still pass (no regression)

## Technical Requirements
- Modify `createWorktree()` method in `packages/cli/src/git/worktrees.ts`
- Load config once before both operations (efficiency)
- Wrap config loading in try-catch for error resilience
- Use optional chaining (`?.`) to safely access nested config properties
- Log warnings for config errors but continue worktree creation
- Keep `runMaproomScan()` method unchanged (no modifications needed)

## Implementation Notes
**Current Code** (lines 128-145 in `packages/cli/src/git/worktrees.ts`):
```typescript
// Copy ignored files if configured and not skipped
if (!skipCopyIgnored) {
  try {
    const config = await loadConfig()
    if (config.worktree?.copyIgnoredFiles?.length) {
      console.log('\n📁 Copying ignored files to worktree...')
      await copyIgnoredFiles({
        sourceRoot: this.cwd,
        worktreeRoot: wtPath,
        config,
      })
    }
  } catch (error) {
    console.warn('⚠️  Failed to copy ignored files:', error instanceof Error ? error.message : error)
  }
}

// Run maproom scan to index the new worktree
await this.runMaproomScan(wtPath)
```

**New Code** (replace lines 128-145):
```typescript
// Load config once for both operations (efficiency + consistency)
let config: CrewChiefConfig | null = null
try {
  config = await loadConfig()
} catch (error) {
  console.warn('⚠️  Failed to load config:', error instanceof Error ? error.message : error)
}

// Copy ignored files if configured and not skipped
if (!skipCopyIgnored && config?.worktree?.copyIgnoredFiles?.length) {
  try {
    console.log('\n📁 Copying ignored files to worktree...')
    await copyIgnoredFiles({
      sourceRoot: this.cwd,
      worktreeRoot: wtPath,
      config,
    })
  } catch (error) {
    console.warn('⚠️  Failed to copy ignored files:', error instanceof Error ? error.message : error)
  }
}

// Run maproom scan if configured (opt-in)
if (config?.worktree?.autoScanOnWorktreeUse) {
  await this.runMaproomScan(wtPath)
}
```

**Key Design Points**:
- Single config load at the top - cleaner and more efficient
- Config errors don't crash worktree creation - resilience is critical
- Optional chaining (`?.`) handles undefined config gracefully
- No changes to `runMaproomScan()` - keeps this PR small and focused
- Consistent error handling with existing patterns

## Dependencies
- **Prerequisite**: WTSCAN-1001 (config schema field must exist)
- **External**: loadConfig() function from `packages/cli/src/config/loader.ts`
- **Pattern**: Follows existing copyIgnoredFiles check pattern

## Risk Assessment
- **Risk**: Config loading errors break worktree creation
  - **Mitigation**: Wrap in try-catch, log warning, continue with null config. Tested in WTSCAN-1003.
- **Risk**: Optional chaining doesn't work as expected
  - **Mitigation**: TypeScript compiler enforces correct usage. Verify with tests.
- **Risk**: Breaking existing functionality
  - **Mitigation**: All existing tests must pass (regression prevention). Manual testing verifies worktree creation still works.

## Files/Packages Affected
- `packages/cli/src/git/worktrees.ts` - Modify createWorktree() method (lines 128-145)

## Verification Notes
**verify-ticket agent should check**:
1. Config is loaded exactly once (not twice)
2. Optional chaining (`?.`) is used correctly for config access
3. Error handling is present (try-catch around config load)
4. `runMaproomScan()` is only called when `autoScanOnWorktreeUse === true`
5. No modifications to `runMaproomScan()` method itself
6. Console warnings are user-friendly
7. TypeScript compiles without errors
8. Code follows existing patterns (compare to copyIgnoredFiles check)

**Manual verification**:
- Create worktree without config → Should complete quickly, no scan
- Create worktree with `autoScanOnWorktreeUse: false` → Should skip scan
- Create worktree with `autoScanOnWorktreeUse: true` → Should run scan
- Break config file → Should still create worktree with warning
