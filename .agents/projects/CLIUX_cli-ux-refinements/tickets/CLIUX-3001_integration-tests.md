# Ticket: CLIUX-3001: Integration tests and final verification

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- verify-ticket
- commit-ticket

## Summary

Create integration tests that verify the end-to-end CLI UX changes work correctly. Execute manual testing checklist to validate all behaviors. This is the final validation ticket before the project is complete.

## Background

This ticket implements Phase 3 of the CLI UX Refinements plan. After the individual command changes (CLIUX-1001, CLIUX-1002, CLIUX-2001), we need to verify:

1. The full worktree workflow (create → use) works correctly
2. Stdout isolation works for `cd $(...)` usage
3. All help text is accurate
4. The agent spawn command is accessible at the new location

**Reference**: See `planning/quality-strategy.md` for the test patterns and manual testing checklist.

## Acceptance Criteria

- [ ] Integration test file created at `packages/cli/src/cli/__tests__/integration/cli-ux.test.ts`
- [ ] All integration tests pass
- [ ] Manual testing checklist completed (documented in this ticket)
- [ ] All `--help` outputs verified accurate
- [ ] `cd $(crewchief worktree use ...)` works in bash
- [ ] All unit tests from previous tickets still pass

## Technical Requirements

### Integration Test File

Create `packages/cli/src/cli/__tests__/integration/cli-ux.test.ts`:

```typescript
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { execSync, spawnSync } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

describe('CLI UX Integration', () => {
  let testDir: string

  beforeAll(() => {
    // Create temp git repo for testing
    testDir = mkdtempSync(join(tmpdir(), 'cliux-test-'))
    execSync('git init', { cwd: testDir })
    execSync('git config user.email "test@test.com"', { cwd: testDir })
    execSync('git config user.name "Test"', { cwd: testDir })
    execSync('git commit --allow-empty -m "init"', { cwd: testDir })
  })

  afterAll(() => {
    rmSync(testDir, { recursive: true, force: true })
  })

  describe('worktree create → use workflow', () => {
    it('creates worktree and returns path', () => {
      const result = spawnSync('crewchief', ['worktree', 'create', 'test-wt'], {
        cwd: testDir,
        encoding: 'utf-8'
      })
      expect(result.status).toBe(0)
      expect(result.stdout.trim()).toContain('test-wt')
    })

    it('uses existing worktree and returns path', () => {
      const result = spawnSync('crewchief', ['worktree', 'use', 'test-wt'], {
        cwd: testDir,
        encoding: 'utf-8'
      })
      expect(result.status).toBe(0)
      expect(result.stdout.trim()).toMatch(/test-wt$/)
    })
  })

  describe('worktree use error handling', () => {
    it('fails with exit code 1 for nonexistent worktree', () => {
      const result = spawnSync('crewchief', ['worktree', 'use', 'nonexistent'], {
        cwd: testDir,
        encoding: 'utf-8'
      })
      expect(result.status).toBe(1)
      expect(result.stderr).toContain('not found')
      expect(result.stderr).toContain('worktree create')
    })
  })

  describe('stdout isolation for cd $()', () => {
    it('outputs only path to stdout (no logger messages)', () => {
      const result = spawnSync('crewchief', ['worktree', 'use', 'test-wt'], {
        cwd: testDir,
        encoding: 'utf-8'
      })
      // stdout should be just the path, one line
      const lines = result.stdout.trim().split('\n')
      expect(lines).toHaveLength(1)
      expect(result.stdout).not.toContain('[')  // No logger prefixes
      expect(result.stdout).not.toContain('ok')
      expect(result.stdout).not.toContain('info')
    })
  })

  describe('agent spawn accessibility', () => {
    it('agent spawn command is accessible', () => {
      const result = spawnSync('crewchief', ['agent', 'spawn', '--help'], {
        encoding: 'utf-8'
      })
      expect(result.status).toBe(0)
      expect(result.stdout).toContain('spawn')
    })

    it('top-level spawn command is not accessible', () => {
      const result = spawnSync('crewchief', ['spawn', '--help'], {
        encoding: 'utf-8'
      })
      // Should fail or show error about unknown command
      expect(result.status).not.toBe(0)
    })
  })

  describe('help text accuracy', () => {
    it('worktree use --help shows --shell flag', () => {
      const result = spawnSync('crewchief', ['worktree', 'use', '--help'], {
        encoding: 'utf-8'
      })
      expect(result.stdout).toContain('--shell')
      expect(result.stdout).not.toContain('--branch')  // Removed
    })

    it('worktree create --help shows --shell flag', () => {
      const result = spawnSync('crewchief', ['worktree', 'create', '--help'], {
        encoding: 'utf-8'
      })
      expect(result.stdout).toContain('--shell')
      expect(result.stdout).not.toContain('--no-cd')  // Removed
    })

    it('agent --help shows spawn subcommand', () => {
      const result = spawnSync('crewchief', ['agent', '--help'], {
        encoding: 'utf-8'
      })
      expect(result.stdout).toContain('spawn')
    })
  })
})
```

### Manual Testing Checklist

Execute these tests manually and document results:

#### Worktree Use
- [ ] `crewchief worktree use <existing>` prints path to stdout only
- [ ] `crewchief worktree use <nonexistent>` shows error (exit 1) with suggestion
- [ ] `crewchief worktree use <existing> --shell` opens subshell
- [ ] `cd $(crewchief worktree use <existing>)` works in bash/zsh

#### Worktree Create
- [ ] `crewchief worktree create <name>` prints path to stdout only
- [ ] `crewchief worktree create <name> --shell` opens subshell
- [ ] `cd $(crewchief worktree create <name>)` works in bash/zsh

#### Agent Spawn
- [ ] `crewchief agent spawn claude` works (or appropriate error if no iTerm)
- [ ] `crewchief agent spawn claude "task"` works
- [ ] `crewchief spawn` no longer works (shows error)

#### Help Text
- [ ] `crewchief --help` lists agent, not spawn at top level
- [ ] `crewchief worktree --help` accurate
- [ ] `crewchief worktree use --help` shows --shell, examples
- [ ] `crewchief worktree create --help` shows --shell, examples
- [ ] `crewchief agent --help` shows spawn
- [ ] `crewchief agent spawn --help` complete

## Implementation Notes

1. **Create test directory**:
   ```bash
   mkdir -p packages/cli/src/cli/__tests__/integration
   ```
2. **Build CLI before testing**: `pnpm build` is **required** before running integration tests that execute the `crewchief` command
3. **Run all unit tests first**: `pnpm test` to ensure previous tickets' tests pass
4. **Run integration tests**: Tests require git to be available
5. **Document manual test results**: Update this ticket with pass/fail for each item

### Running Tests

```bash
# Build first (REQUIRED for integration tests)
pnpm build

# Run all tests
pnpm test

# Run specific integration test
pnpm test cli-ux.test.ts
```

## Dependencies

- CLIUX-1001 (Modify `worktree use`) - must be complete
- CLIUX-1002 (Modify `worktree create`) - must be complete
- CLIUX-2001 (Migrate spawn) - must be complete

## Risk Assessment

- **Risk**: Integration tests flaky due to git operations
  - **Mitigation**: Use temp directory; clean setup/teardown

- **Risk**: Manual tests require iTerm for agent spawn
  - **Mitigation**: Test in headless mode if iTerm unavailable; document limitations

## Files/Packages Affected

- `packages/cli/src/cli/__tests__/integration/cli-ux.test.ts` - New integration test file
