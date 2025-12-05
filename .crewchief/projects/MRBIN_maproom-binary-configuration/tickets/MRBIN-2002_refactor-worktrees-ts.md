# Ticket: [MRBIN-2002]: Refactor worktrees.ts to Use Shared Utility

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
Refactor packages/cli/src/git/worktrees.ts to use the shared findMaproomBinary() utility in the runMaproomScan() method, removing ~40 lines of duplicated binary resolution logic and enabling config-based resolution for worktree auto-indexing.

## Background
The WorktreeService class in worktrees.ts has its own inline binary resolution logic for the auto-indexing feature (triggered during worktree creation). This ticket replaces that logic with the shared utility, completing the consolidation of binary resolution across the CLI.

After this change, both maproom commands and worktree auto-indexing will use identical resolution logic with consistent precedence order and config support.

## Acceptance Criteria
- [ ] Import findMaproomBinary at top of file
- [ ] runMaproomScan() uses shared utility instead of inline resolution
- [ ] Config loading uses existing loadConfig() pattern
- [ ] Removed ~40 lines of duplicated resolution logic
- [ ] Auto-indexing works with config-based binary path
- [ ] Worktree creation tests pass
- [ ] Warning message shown when binary not found
- [ ] Resolution order is: env > config > global > packaged

## Technical Requirements
- Import: `import { findMaproomBinary } from '../utils/maproom-binary.js'`
- Load config with await (already in async context)
- Pass config.repository.maproomBinaryPath to findMaproomBinary()
- Remove inline platform detection code
- Remove inline packaged path logic
- Preserve warning behavior when binary not found
- Handle missing config gracefully

## Implementation Notes
Update runMaproomScan() as specified in architecture.md:

```typescript
private async runMaproomScan(worktreePath: string): Promise<void> {
  try {
    const config = await loadConfig()
    const result = findMaproomBinary({
      configPath: config.repository.maproomBinaryPath
    })

    if (!result.path) {
      console.log('⚠️  Maproom binary not found, skipping indexing for new worktree')
      return
    }

    console.log('🔍 Running maproom scan for new worktree...')

    const scanResult = spawnSync(result.path, ['scan'], {
      cwd: worktreePath,
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe'],
    })

    // ... rest of output handling (unchanged) ...
  } catch (error) {
    console.warn('⚠️  Failed to run maproom scan:', error instanceof Error ? error.message : error)
  }
}
```

Remove the inline binary resolution code that checks:
- Platform detection (process.platform, execName)
- CREWCHIEF_MAPROOM_BIN check
- Packaged path construction
- fs.existsSync checks

All of this logic now lives in the shared utility.

## Dependencies
- MRBIN-2001 (maproom.ts must be refactored first to validate utility works)

## Risk Assessment
- **Risk**: Breaking worktree auto-indexing
  - **Mitigation**: Test worktree creation with and without config, verify scan runs
- **Risk**: Config loading errors breaking worktree creation
  - **Mitigation**: Already wrapped in try-catch, preserves existing error handling
- **Risk**: Different behavior from maproom.ts
  - **Mitigation**: Both use same utility, guaranteed identical behavior

## Files/Packages Affected
- packages/cli/src/git/worktrees.ts

## Verification Notes
Verify that:
1. Worktree creation still triggers auto-indexing
2. Auto-indexing works with config-based binary path
3. Auto-indexing works without config file (backwards compatible)
4. Warning shown when binary not found (doesn't break worktree creation)
5. All worktree tests pass
6. Code diff shows ~40 lines removed
7. No inline binary resolution logic remains
8. Error handling preserved (try-catch around scan)
9. TypeScript compilation succeeds
10. Manual test: `crewchief worktree create test-branch` runs scan
