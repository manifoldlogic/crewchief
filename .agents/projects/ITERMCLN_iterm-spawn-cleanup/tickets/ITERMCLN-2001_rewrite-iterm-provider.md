# Ticket: ITERMCLN-2001: Rewrite ITermProvider to Use Direct Script Calls

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
- general-development
- unit-test-runner
- verify-ticket
- commit-ticket (together with ITERMCLN-1002)

## Summary
COMPLETELY REWRITE `ITermProvider` to use direct Python script calls via `spawnSync`, removing the broken JSON-RPC bridge dependency. This is a BUG FIX - the spawn command is currently broken for iTerm users (30-second timeout).

## Background
The current `ITermProvider.initialize()` calls `ITermService.startBridge()` which attempts to start a non-functional JSON-RPC bridge. This causes `crewchief spawn claude` to fail with a 30-second timeout for all iTerm users.

The working pattern already exists in `ITermSimpleService` - it calls Python scripts directly via `spawnSync`. This ticket rewrites `ITermProvider` to use the same pattern.

**Reference**: ITERMCLN plan.md Phase 2 - ITermProvider Fix

## Acceptance Criteria
- [ ] `ITermProvider` rewrites to use direct `spawnSync` calls to Python scripts
- [ ] No dependency on `ITermService` (will be deleted in ITERMCLN-1002)
- [ ] `crewchief spawn claude` works in iTerm2 (creates pane, starts agent)
- [ ] `pnpm build` succeeds with no TypeScript errors
- [ ] All existing tests pass

## Technical Requirements
Rewrite `packages/cli/src/terminal/providers/iterm.ts` to follow the working pattern from `ITermSimpleService`:

```typescript
import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join } from 'node:path'
import { TerminalProvider, WindowOptions, SplitDirection } from '../interface'

export class ITermProvider implements TerminalProvider {
  readonly id = 'iterm'
  private scriptsDir: string | null = null

  constructor() {
    // Find scripts directory (same pattern as ITermSimpleService)
    const possiblePaths = [
      join(__dirname, '..', '..', '..', '..', 'scripts', 'iterm_scripts'),
      join(process.cwd(), 'scripts', 'iterm_scripts'),
    ]
    for (const path of possiblePaths) {
      if (existsSync(join(path, 'spawn_agent.py'))) {
        this.scriptsDir = path
        break
      }
    }
  }

  async initialize(): Promise<void> {
    // NO MORE JSON-RPC BRIDGE - just check environment
    if (process.env.TERM_PROGRAM !== 'iTerm.app') {
      throw new Error('ITermProvider requires running in iTerm.app')
    }
    if (!this.scriptsDir) {
      throw new Error('iTerm scripts not found')
    }
  }

  async createWindow(options?: WindowOptions): Promise<string> {
    const args = [join(this.scriptsDir!, 'spawn_agent.py')]
    if (options?.title) args.push('--name', options.title)
    if (options?.workingDirectory) args.push('--project-dir', options.workingDirectory)

    const result = spawnSync('python3', args, { encoding: 'utf-8' })
    if (result.status !== 0) {
      throw new Error(`spawn_agent.py failed: ${result.stderr}`)
    }
    return this.parseSessionId(result.stdout)
  }

  // Implement other TerminalProvider methods similarly...
}
```

**Key Points**:
- Use `spawnSync` with array args (not shell string) for security
- Reference `ITermSimpleService` at `src/iterm/iterm-simple.service.ts` for working patterns
- Parse session ID from Python script output
- Throw meaningful errors when scripts fail

## Implementation Notes

**Working Pattern Reference**: `ITermSimpleService` already demonstrates the correct approach:
1. Find scripts directory in constructor using multiple search paths
2. Use `spawnSync('python3', [scriptPath, ...args])` for direct execution
3. Check exit status and parse stdout for results
4. Throw descriptive errors on failure

**Script Path Resolution**: Use the same logic as `ITermSimpleService` to locate scripts:
- Check relative to compiled location: `__dirname/../../../../scripts/iterm_scripts`
- Check relative to working directory: `process.cwd()/scripts/iterm_scripts`
- Verify by checking for `spawn_agent.py` existence

**No JSON-RPC Bridge**: The entire JSON-RPC bridge infrastructure is broken and being removed:
- Do NOT call `ITermService.startBridge()`
- Do NOT import or reference `ITermService`
- Simple environment checks in `initialize()` are sufficient

**Keep ITermSimpleService**: Do not modify or consolidate `ITermSimpleService` in this ticket. It is used by `agent.ts` and may be consolidated in a future phase.

## Dependencies
- **Must be committed with**: ITERMCLN-1002 (TypeScript dead code deletion)
  - These two tickets must be in the same commit to avoid broken intermediate state
  - ITERMCLN-1002 removes `ITermService` which this ticket stops using
- **Should be done before**: ITERMCLN-1001 (Python dead code deletion)
  - Not blocking, but logical sequence

## Risk Assessment
- **Risk**: Python scripts not found at runtime in production builds
  - **Mitigation**: Use same path resolution as ITermSimpleService (known working in production)

- **Risk**: Output parsing differs from expected session ID format
  - **Mitigation**: Test with actual spawn_agent.py output; examine ITermSimpleService's parsing logic

- **Risk**: Breaking change for users if script arguments change
  - **Mitigation**: Use exact same script arguments as current working code in ITermSimpleService

## Files/Packages Affected
- `packages/cli/src/terminal/providers/iterm.ts` - COMPLETE REWRITE (remove all JSON-RPC code)
