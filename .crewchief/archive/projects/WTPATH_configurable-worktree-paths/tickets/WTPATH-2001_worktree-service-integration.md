# Ticket: [WTPATH-2001]: WorktreeService Integration

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
Integrate path expansion utilities into WorktreeService.createWorktree() method, enabling worktrees to be created with expanded absolute paths while maintaining backward compatibility with relative paths.

## Background
Phase 1 created tested path expansion utilities. Now we integrate them into the core worktree creation logic without changing the default configuration. This allows the system to handle absolute paths, tilde expansion, and repository placeholders when users configure them, while existing relative paths continue to work unchanged.

This ticket implements Phase 2 of the Configurable Worktree Paths project.

**Planning References:**
- `/workspace/.crewchief/projects/WTPATH_configurable-worktree-paths/planning/plan.md` (Phase 2)
- `/workspace/.crewchief/projects/WTPATH_configurable-worktree-paths/planning/architecture.md` (WorktreeService Integration section)

## Acceptance Criteria
- [x] WorktreeService calls `expandWorktreePath()` before constructing worktree path
- [x] Relative paths still work (e.g., `.crewchief/worktrees`)
- [x] Absolute paths work without joining to cwd (e.g., `/tmp/worktrees`)
- [x] Tilde paths expand correctly (e.g., `~/worktrees`)
- [x] `<repo-name>` placeholder expands correctly
- [x] Relative paths (`.crewchief/worktrees`) resolve correctly without expansion side effects
- [x] Existing worktree unit tests pass with mocked expansion
- [x] Integration test creates real worktree with expanded path
- [x] Integration test verifies worktree exists at expanded location
- [x] Integration test cleans up worktree after test
- [x] Expansion errors are caught and re-thrown with context (original path + error reason)
- [x] Error messages include expanded path when creation fails

## Technical Requirements

### Modify WorktreeService
File: `/workspace/packages/cli/src/git/worktrees.ts`

**Current code** (approximate line numbers, verify during implementation):
```typescript
async createWorktree(name: string, basePath: string, branch?: string): Promise<string> {
  const wtPath = path.join(this.cwd, basePath, name)
  // ... git worktree add logic
}
```

**New code**:
```typescript
import { expandWorktreePath } from '../utils/paths'

async createWorktree(name: string, basePath: string, branch?: string): Promise<string> {
  // Expand path before construction
  const expandedBasePath = await expandWorktreePath(basePath, this.cwd)

  // Don't join with cwd - expansion already resolved absolute paths
  const wtPath = path.join(expandedBasePath, name)

  // ... rest of git worktree add logic
}
```

**Key Change**: Remove implicit `path.join(this.cwd, basePath, ...)` behavior. Let `expandWorktreePath()` handle absolute vs relative resolution.

### Update Unit Tests
File: `/workspace/packages/cli/src/cli/__tests__/worktree-create.test.ts`

Mock `expandWorktreePath()` to return predictable paths:
```typescript
import * as paths from '../../utils/paths'

vi.mock('../../utils/paths', () => ({
  expandWorktreePath: vi.fn((p) => Promise.resolve(p)) // passthrough for tests
}))
```

Verify existing tests pass with mocked expansion.

### Add Integration Tests
File: `/workspace/packages/cli/src/git/__tests__/worktrees.integration.test.ts`

Create integration tests that actually create worktrees:
1. **Tilde path expansion**: Config `~/test-worktrees`, verify created in home directory
2. **Repo placeholder expansion**: Config `/tmp/<repo-name>-wt`, verify placeholder replaced
3. **Backward compatibility**: Config `.crewchief/worktrees`, verify created relative to repo root
4. **Absolute path**: Config `/tmp/test-worktrees`, verify created at absolute path

Each test must:
- Set up temp directory or use home directory
- Create real worktree
- Verify worktree exists at expected location
- Clean up worktree and directory after test

## Implementation Notes

### Integration Test Setup
```typescript
import os from 'os'
import fs from 'fs'
import path from 'path'

describe('WorktreeService integration', () => {
  let tempDir: string

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'wtpath-test-'))
  })

  afterEach(() => {
    // Clean up worktrees and temp directory
    // Log cleanup errors but don't fail test - cleanup failures shouldn't break test suite
    try {
      fs.rmSync(tempDir, { recursive: true, force: true })
    } catch (error) {
      console.warn('Cleanup failed:', error)
      // Don't throw - cleanup failures shouldn't fail tests
    }
  })

  it('creates worktree with tilde expansion', async () => {
    // Test implementation
  })
})
```

**Cleanup Strategy:** Integration tests must clean up temporary worktrees and directories after each test. Use `force: true` to handle locked files. Catch and log cleanup errors without failing the test - cleanup failures indicate environment issues, not test failures. This prevents test suite failures due to permission issues or file locks.

### Error Handling
If path expansion throws (e.g., system directory), error should include:
- Original configured path
- Reason for rejection
- Example of valid path

```typescript
try {
  const expandedBasePath = await expandWorktreePath(basePath, this.cwd)
} catch (error) {
  throw new Error(`Invalid worktree path "${basePath}": ${error.message}`)
}
```

### Backward Compatibility Verification
Existing tests should pass without modification (except adding mock). This proves:
- Relative paths still work
- Existing behavior unchanged
- No breaking changes for current users

### Performance Consideration
Path expansion adds ~10-50ms (git command overhead). This is acceptable since worktree creation is infrequent and not in hot path.

## Dependencies
- **WTPATH-1001** (Path Expansion Utilities) - Must be completed first

## Risk Assessment
- **Risk**: Mocking breaks existing tests unexpectedly
  - **Mitigation**: Mock returns passthrough by default; tests should pass unchanged

- **Risk**: Integration tests fail on CI due to permissions
  - **Mitigation**: Use temp directories that CI has access to; skip tests if home directory not writable

- **Risk**: Path expansion errors not caught properly
  - **Mitigation**: Wrap expansion in try-catch with clear error messages

- **Risk**: Git worktree cleanup fails in integration tests
  - **Mitigation**: Use `force: true` in cleanup; catch and log cleanup errors

## Files/Packages Affected
- `/workspace/packages/cli/src/git/worktrees.ts` (modify WorktreeService)
- `/workspace/packages/cli/src/cli/__tests__/worktree-create.test.ts` (add mocks)
- `/workspace/packages/cli/src/git/__tests__/worktrees.integration.test.ts` (new file)

## Verification Notes

Verify-ticket agent should check:
- [ ] All acceptance criteria checkboxes are met
- [ ] WorktreeService correctly imports and calls expandWorktreePath
- [ ] Unit tests pass with mocked expansion
- [ ] Integration tests exist and test all path types (tilde, absolute, relative, placeholder)
- [ ] Integration test output shows worktrees created at correct locations
- [ ] Cleanup logic in integration tests prevents test directory buildup
- [ ] Error messages are helpful when path expansion fails
- [ ] No breaking changes to existing worktree creation behavior
- [ ] TypeScript compilation succeeds with no errors
