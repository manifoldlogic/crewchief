# Quality Strategy: CLI UX Refinements

## Testing Philosophy

This project modifies CLI command behavior with relatively straightforward changes. Testing should focus on:

1. **Behavior verification** - Commands do what they claim
2. **Error handling** - Clear messages for error cases
3. **Help text accuracy** - Documentation matches behavior

## Test Directory Convention

Follow the existing colocated test pattern used in the codebase:

```
packages/cli/src/cli/__tests__/           # New directory for CLI tests
├── worktree-use.test.ts                  # worktree use command tests
├── worktree-create.test.ts               # worktree create command tests
├── agent-spawn.test.ts                   # agent spawn command tests
└── integration/
    └── cli-ux.test.ts                    # Integration tests
```

**Reference pattern**: `packages/cli/src/terminal/__tests__/smoke.test.ts` - uses Vitest with `describe`/`it`/`expect` and `beforeEach`/`afterEach` lifecycle hooks.

## Mocking Strategy

Use Vitest's built-in mocking capabilities. Follow patterns from existing tests:

```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { WorktreeService } from '../../git/worktrees'
import { spawn } from 'node:child_process'

// Mock modules
vi.mock('../../git/worktrees')
vi.mock('node:child_process', () => ({
  spawn: vi.fn()
}))

describe('worktree use', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('prints path when worktree exists', async () => {
    // Setup mock
    vi.mocked(WorktreeService.prototype.listWorktrees).mockResolvedValue([
      { path: '/path/to/worktree', branch: 'feature-x' }
    ])

    // Capture stdout
    const stdoutSpy = vi.spyOn(process.stdout, 'write').mockImplementation(() => true)

    // Execute command (import and call action handler directly)
    // ...

    // Assert
    expect(stdoutSpy).toHaveBeenCalledWith('/path/to/worktree\n')
  })
})
```

## Test Categories

### 1. Unit Tests (Vitest)

**Location**: `packages/cli/src/cli/__tests__/`

#### Worktree Use Tests

**File**: `worktree-use.test.ts`

```typescript
describe('worktree use', () => {
  it('prints path to stdout when worktree exists', async () => {
    // Mock listWorktrees to return a match
    // Assert stdout.write called with path + '\n'
    // Assert exit code is 0
  })

  it('exits with code 1 when worktree does not exist', async () => {
    // Mock listWorktrees to return empty
    // Assert process.exitCode = 1
    // Assert error message includes suggestion
  })

  it('spawns shell with --shell flag', async () => {
    // Mock child_process.spawn
    // Call with --shell option
    // Assert spawn called with correct shell and cwd
  })

  it('lists candidates when selector is ambiguous', async () => {
    // Mock multiple matches
    // Assert error lists all candidates
  })

  it('--print flag is accepted (no-op alias)', async () => {
    // Verify --print doesn't cause error
    // Behavior identical to default
  })
})
```

#### Worktree Create Tests

**File**: `worktree-create.test.ts`

```typescript
describe('worktree create', () => {
  it('prints path to stdout after creation', async () => {
    // Mock createWorktree
    // Assert stdout.write called with created path
  })

  it('spawns shell with --shell flag', async () => {
    // Call with --shell
    // Assert spawn called
  })

  it('passes options to WorktreeService', async () => {
    // Test --branch, --base-path, --no-copy-ignored
    // Assert createWorktree called with correct params
  })
})
```

#### Agent Spawn Tests

**File**: `agent-spawn.test.ts`

```typescript
describe('agent spawn', () => {
  it('is accessible as subcommand of agent', async () => {
    // Verify command registration
  })

  it('spawns agent via scheduler', async () => {
    // Mock Scheduler and TerminalFactory
    // Assert scheduler.assignSingleAgent called
  })

  it('passes all options correctly', async () => {
    // Test -n, -v, -a, --no-label, --backend, --headless
  })
})
```

### 2. Integration Tests

**File**: `packages/cli/src/cli/__tests__/integration/cli-ux.test.ts`

```typescript
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { execSync } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

describe('CLI UX Integration', () => {
  let testDir: string

  beforeAll(() => {
    // Create temp git repo
    testDir = mkdtempSync(join(tmpdir(), 'cliux-test-'))
    execSync('git init', { cwd: testDir })
    execSync('git commit --allow-empty -m "init"', { cwd: testDir })
  })

  afterAll(() => {
    rmSync(testDir, { recursive: true, force: true })
  })

  it('worktree create → use workflow', () => {
    // Create worktree
    const createOutput = execSync('crewchief worktree create test-wt', {
      cwd: testDir,
      encoding: 'utf-8'
    })
    expect(createOutput).toContain('test-wt')

    // Use worktree
    const useOutput = execSync('crewchief worktree use test-wt', {
      cwd: testDir,
      encoding: 'utf-8'
    })
    expect(useOutput.trim()).toMatch(/test-wt$/)
  })

  it('worktree use nonexistent fails with suggestion', () => {
    expect(() => {
      execSync('crewchief worktree use nonexistent', { cwd: testDir })
    }).toThrow()
  })

  it('stdout isolation for cd $()', () => {
    // This test verifies that only the path goes to stdout
    const output = execSync('crewchief worktree use test-wt 2>/dev/null', {
      cwd: testDir,
      encoding: 'utf-8'
    })
    // Should be just the path, no extra text
    expect(output.trim().split('\n')).toHaveLength(1)
    expect(output).not.toContain('[')  // No logger prefixes
  })
})
```

### 3. Help Text Verification

**File**: `packages/cli/src/cli/__tests__/help-output.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import { execSync } from 'node:child_process'

describe('CLI help output', () => {
  it('worktree use --help documents --shell flag', () => {
    const output = execSync('crewchief worktree use --help').toString()
    expect(output).toContain('--shell')
    expect(output).not.toContain('--branch')  // Removed option
  })

  it('worktree create --help documents --shell flag', () => {
    const output = execSync('crewchief worktree create --help').toString()
    expect(output).toContain('--shell')
    expect(output).not.toContain('--no-cd')  // Removed option
  })

  it('agent --help shows spawn subcommand', () => {
    const output = execSync('crewchief agent --help').toString()
    expect(output).toContain('spawn')
  })

  it('top-level help does not show spawn', () => {
    const output = execSync('crewchief --help').toString()
    expect(output).toContain('agent')
    // spawn should only appear within agent description, not as top-level
  })
})
```

## Test Coverage Targets

| Component | Target | Rationale |
|-----------|--------|-----------|
| worktree.ts changes | 90%+ | Core user-facing behavior |
| agent.ts spawn | 80%+ | Moved code, well-understood |
| Help text | Smoke test | Manual verification sufficient |

## Manual Testing Checklist

Before considering complete, manually verify:

### Worktree Use
- [ ] `crewchief worktree use <existing>` prints path to stdout only
- [ ] `crewchief worktree use <nonexistent>` shows error (exit 1) with suggestion
- [ ] `crewchief worktree use <existing> --shell` opens subshell
- [ ] `cd $(crewchief worktree use <existing>)` works in bash/zsh

### Worktree Create
- [ ] `crewchief worktree create <name>` prints path to stdout only
- [ ] `crewchief worktree create <name> --shell` opens subshell
- [ ] `cd $(crewchief worktree create <name>)` works

### Agent Spawn
- [ ] `crewchief agent spawn claude` works
- [ ] `crewchief agent spawn claude "task"` works
- [ ] `crewchief spawn` no longer works

### Help Text
- [ ] `crewchief --help` lists agent, not spawn at top level
- [ ] `crewchief worktree --help` accurate
- [ ] `crewchief worktree use --help` shows --shell, examples
- [ ] `crewchief worktree create --help` shows --shell, examples
- [ ] `crewchief agent --help` shows spawn
- [ ] `crewchief agent spawn --help` complete

## Risk Mitigation

### High-Risk Areas

1. **Stdout/stderr separation**: Critical for `cd $(...)` usage
2. **Subshell spawning logic**: Core functionality, well-tested
3. **Path output format**: Simple string, low risk

### Mitigation Strategies

1. **Stdout isolation test**: Dedicated integration test for `cd $(...)` compatibility
2. **Preserve existing test coverage**: Don't remove tests for moved code
3. **Add regression tests**: For each behavior change
4. **Manual smoke testing**: In both bash and zsh before merge

## CI Integration

Existing CI pipeline runs:
- `pnpm test` - Vitest unit tests
- `pnpm lint` - ESLint
- `pnpm build` - TypeScript compilation

No new CI steps required. Existing infrastructure sufficient.

## Test Data Requirements

- Git repository for integration tests (temp directory, created in beforeAll)
- Mock implementations for unit tests:
  - `WorktreeService` - mock `listWorktrees()` and `createWorktree()`
  - `child_process.spawn` - mock for shell spawning tests
  - `Scheduler` - mock for agent spawn tests
  - `TerminalFactory` - mock to return headless provider
