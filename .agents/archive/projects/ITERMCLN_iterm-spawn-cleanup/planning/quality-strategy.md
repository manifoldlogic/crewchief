# Quality Strategy: iTerm Spawn Command Cleanup

## Testing Philosophy

This project is primarily a **cleanup and consolidation** effort. The testing strategy focuses on:

1. **Regression prevention** - Ensure existing working features continue to work
2. **Critical path coverage** - Test the spawn → message → list → close workflow
3. **Provider parity** - Both iTerm and headless providers behave consistently

We are NOT aiming for exhaustive coverage. This is CLI tooling where manual testing is practical and valuable.

## Test Categories

### 1. Unit Tests (TypeScript)

**Target: Provider implementations**

```typescript
// packages/cli/src/terminal/providers/__tests__/iterm.test.ts

describe('ITermProvider', () => {
  describe('isAvailable', () => {
    it('returns true when TERM_PROGRAM is iTerm.app and scripts exist', () => {})
    it('returns false when TERM_PROGRAM is not iTerm.app', () => {})
    it('returns false when scripts directory is missing', () => {})
  })

  describe('spawn', () => {
    it('calls spawn_agent.py with correct arguments', () => {})
    it('returns AgentInfo on success', () => {})
    it('throws AgentError on script failure', () => {})
  })

  describe('sendMessage', () => {
    it('calls send_to_pane.py with correct arguments', () => {})
    it('returns true on success', () => {})
    it('returns false when agent not found', () => {})
  })

  describe('list', () => {
    it('parses list_panes.py output correctly', () => {})
    it('filters for agent panes (name__type format)', () => {})
    it('returns empty array when no agents', () => {})
  })
})
```

```typescript
// packages/cli/src/terminal/providers/__tests__/headless.test.ts

describe('HeadlessProvider', () => {
  describe('spawn', () => {
    it('spawns child process with correct command', () => {})
    it('creates message file for IPC', () => {})
    it('tracks agent in internal map', () => {})
    it('returns AgentInfo with process ID', () => {})
  })

  describe('sendMessage', () => {
    it('writes to process stdin when available', () => {})
    it('returns false when agent not found', () => {})
    it('returns false when stdin closed', () => {})
  })

  describe('close', () => {
    it('sends SIGTERM to process', () => {})
    it('removes agent from tracking', () => {})
    it('cleans up message file', () => {})
  })

  describe('closeAll', () => {
    it('closes all tracked agents', () => {})
    it('handles partial failures gracefully', () => {})
  })
})
```

### 2. Integration Tests (CLI)

**Target: End-to-end command behavior**

```typescript
// packages/cli/src/cli/__tests__/spawn-integration.test.ts

describe('spawn command integration', () => {
  // These tests run actual commands but mock the terminal provider

  it('spawn with single agent creates agent and exits', async () => {
    const result = await runCLI(['spawn', 'claude', 'test-task'])
    expect(result.exitCode).toBe(0)
    expect(result.stdout).toContain('Agent spawned successfully')
  })

  it('spawn with --headless keeps process alive', async () => {
    const proc = runCLIAsync(['spawn', 'claude', '--headless'])
    await waitFor(() => proc.stdout.includes('Running in headless mode'))
    proc.kill()
  })
})
```

```typescript
// packages/cli/src/cli/__tests__/agent-integration.test.ts

describe('agent commands integration', () => {
  it('agent list shows no agents when none spawned', async () => {
    const result = await runCLI(['agent', 'list'])
    expect(result.stdout).toContain('No agent panes found')
  })

  it('agent message fails gracefully when agent not found', async () => {
    const result = await runCLI(['agent', 'message', 'nonexistent', 'hello'])
    expect(result.exitCode).not.toBe(0)
  })
})
```

### 3. Manual Testing Protocol

**Critical for iTerm2 integration** - Automated tests cannot fully verify visual terminal behavior.

#### Pre-release Checklist

```markdown
## Manual Test: iTerm Provider

Environment: macOS with iTerm2 installed

### Spawn Tests
- [ ] `crewchief spawn claude` opens new iTerm window/pane
- [ ] Pane is labeled with agent name (badge visible)
- [ ] Agent CLI starts and is responsive
- [ ] Working directory is correct

### Message Tests
- [ ] `crewchief agent list` shows spawned agent
- [ ] `crewchief agent message <name> "hello"` sends text to pane
- [ ] Text appears in agent's input area
- [ ] Special characters handled correctly

### Close Tests
- [ ] `crewchief agent close <name>` terminates agent
- [ ] Pane can be closed manually without errors

## Manual Test: Headless Provider

Environment: Any terminal (not iTerm2) or CI

### Spawn Tests
- [ ] `crewchief spawn claude --headless` starts background process
- [ ] Process visible in `ps` output
- [ ] Logs appear in terminal

### Message Tests
- [ ] `crewchief agent message <name> "hello"` sends to stdin
- [ ] Agent receives and processes message

### Close Tests
- [ ] Ctrl+C terminates all agents
- [ ] `crewchief agent close <name>` terminates specific agent
- [ ] No orphan processes remain
```

## Test Infrastructure

### Mocking Strategy

```typescript
// packages/cli/src/terminal/__tests__/mocks.ts

export function mockSpawnSync(returnValue: Partial<SpawnSyncReturns<string>>) {
  return jest.spyOn(child_process, 'spawnSync').mockReturnValue({
    status: 0,
    stdout: '',
    stderr: '',
    pid: 12345,
    signal: null,
    output: ['', '', ''],
    ...returnValue,
  } as SpawnSyncReturns<string>)
}

export function mockListPanesOutput(panes: Array<{label: string, sessionId: string}>) {
  const output = panes.map((p, i) =>
    `  ${i + 1}. [${p.label}]     Window:1 Tab:1 ID:${p.sessionId}`
  ).join('\n')
  return mockSpawnSync({ stdout: output })
}
```

### Test Fixtures

```
packages/cli/src/terminal/__tests__/fixtures/
├── list_panes_empty.txt       # No panes output
├── list_panes_agents.txt      # Multiple agent panes
├── list_panes_mixed.txt       # Agent and non-agent panes
└── spawn_agent_success.json   # Spawn result
```

## Coverage Goals

| Component | Target | Rationale |
|-----------|--------|-----------|
| ITermProvider | 80% | Core functionality, mocked Python |
| HeadlessProvider | 90% | Pure TypeScript, easy to test |
| AgentOrchestrator | 70% | Thin wrapper, integration tested |
| CLI commands | 60% | Manual testing supplements |
| Python scripts | 0% | Tested via TypeScript integration |

## Quality Gates

### PR Merge Requirements

1. **All unit tests pass** - `pnpm test`
2. **No type errors** - `pnpm typecheck`
3. **Lint clean** - `pnpm lint`
4. **Build succeeds** - `pnpm build`

### Release Requirements

1. All PR requirements
2. **Manual testing complete** - Checklist signed off
3. **README updated** - Document any behavior changes

## Regression Testing

### After Dead Code Removal (Phase 1)

Verify these commands still work identically:

```bash
# Must still work
crewchief spawn claude
crewchief spawn claude --headless
crewchief agent list
crewchief agent message <name> "text"
```

### After Consolidation (Phase 2-3)

Same commands, plus:

```bash
# New capability
crewchief agent message <headless-agent> "text"  # Should work now
```

## Known Test Limitations

1. **Cannot automate iTerm visual verification** - Manual testing required
2. **Headless tests may leave orphan processes** - Test cleanup is best-effort
3. **Python script behavior assumed correct** - Not unit testing Python
4. **CI cannot run iTerm tests** - Requires macOS with iTerm2

## Test Maintenance

- Tests live alongside source: `src/**/__tests__/*.test.ts`
- Fixtures in `__tests__/fixtures/`
- Shared mocks in `__tests__/mocks.ts`
- Integration tests marked with `describe.skip` if environment not available
