# Plan: iTerm Spawn Command Cleanup

## Overview

This project **FIXES** the broken iTerm spawn command, removes dead JSON-RPC code, and adds headless messaging support. The spawn command currently fails with 30-second timeout for iTerm users because it tries to start a non-functional JSON-RPC bridge.

**Critical Finding**: `crewchief spawn claude` is BROKEN for iTerm users. Phase 1-2 is a BUG FIX, not just cleanup.

## Phase 1: Dead Code Removal + Spawn Fix Preparation (Medium Risk)

**Goal**: Remove ~1,750 lines of dead JSON-RPC code and prepare for spawn fix

**Why Medium Risk (not Low)**:
- Deleting `iterm.service.ts` removes the import in `iterm.ts` provider
- The provider will fail to compile until Phase 2 rewrites it
- These two phases should be done in quick succession

**Deliverables**:
- Delete TypeScript files: `iterm.service.ts`, `iterm.types.ts`, `iterm.adapter.ts`
- Delete Python files: `iterm_bridge.py`, `iterm_controller.py`, `iterm_agent_manager.py`, test/demo scripts
- **Start Phase 2 immediately** to fix the broken import

**Files to Delete**:

*TypeScript (packages/cli/src/):*
```
src/iterm/iterm.service.ts        (414 lines) - JSON-RPC client
src/iterm/iterm.types.ts          (94 lines) - Types for dead service
src/terminal/iterm.adapter.ts     (155 lines) - Imports missing file, dead code
```

*Python (scripts/iterm_scripts/):*
```
iterm_bridge.py          (309 lines) - JSON-RPC server
iterm_controller.py      (255 lines) - Bridge controller
iterm_agent_manager.py   (351 lines) - Bridge manager
test_bridge.py           (~250 lines) - Bridge tests
test_badge.py            (~60 lines) - Badge tests
test_enter.py            (~145 lines) - Enter key tests
test_agent_detection.py  (~50 lines) - Agent detection tests
demo_smart_spawning.py   (~140 lines) - Demo script
debug_send.py            (~90 lines) - Debug utility
start_bridge.sh          (~27 lines) - Bridge startup
```

**Verification** (after Phase 2 completes):
- `pnpm build` succeeds
- `pnpm test` passes
- `crewchief spawn claude` **WORKS** (was broken, now fixed)
- `crewchief agent list` still works
- `crewchief agent message` still works

**Agent**: General TypeScript cleanup

**Risk**: Medium - removes broken code but creates temporary compile failure

---

## Phase 2: ITermProvider Fix (Medium-High Risk)

**Goal**: REWRITE ITermProvider to use direct script calls, FIXING the broken spawn

**Why This Phase is Critical**:
- Spawn command is currently BROKEN (30-second timeout)
- After Phase 1 deletes `iterm.service.ts`, ITermProvider won't compile
- This phase restores functionality using the working ITermSimpleService patterns

**Deliverables**:
- REWRITE `ITermProvider` to call Python scripts directly via `spawnSync`
- Use the same patterns that work in `ITermSimpleService`
- Keep `ITermSimpleService` for now (don't delete - agent.ts still uses it)
- **Spawn command works again** after this phase

**Changes**:

```typescript
// packages/cli/src/terminal/providers/iterm.ts (COMPLETE REWRITE)

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
    // Call spawn_agent.py directly (WORKING pattern)
    const args = [join(this.scriptsDir!, 'spawn_agent.py')]
    if (options?.title) args.push('--name', options.title)
    if (options?.workingDirectory) args.push('--project-dir', options.workingDirectory)

    const result = spawnSync('python3', args, { encoding: 'utf-8' })
    if (result.status !== 0) {
      throw new Error(`spawn_agent.py failed: ${result.stderr}`)
    }
    return this.parseSessionId(result.stdout)
  }

  // ... other TerminalProvider methods
}
```

**Files to Modify**:
- `packages/cli/src/terminal/providers/iterm.ts` - Complete rewrite (remove ITermService dependency)

**Files to Keep** (for now):
- `packages/cli/src/iterm/iterm-simple.service.ts` - Still used by agent.ts

**Verification**:
- `pnpm build` succeeds
- `pnpm test` passes
- **`crewchief spawn claude` WORKS** (was broken, now fixed!)
- `crewchief agent list` still works
- `crewchief agent message` still works

**Verification Checkpoint** (before proceeding to Phase 3):
1. Run `crewchief spawn claude` in iTerm2
2. Confirm: pane opens, agent starts, badge appears
3. If this doesn't work, stop and debug before continuing

**Agent**: General TypeScript development

**Risk**: Medium-High - rewriting core provider, but follows proven patterns

---

## Phase 3: Headless Messaging Support (Medium Risk)

**Goal**: Enable `crewchief agent message` to work with headless agents via stdin pipe

**Approach**: Simple stdin pipe messaging (NO file-based IPC - simpler and more reliable)

**Deliverables**:
- Add `sendMessage()` method to HeadlessProvider using stdin pipe
- Add `listAgents()` method to HeadlessProvider
- Extend `TerminalProvider` interface with optional messaging methods
- Update `agent.ts` to detect provider and use appropriate method

**Design**:

```typescript
// packages/cli/src/terminal/interface.ts (EXTEND existing interface)

export interface TerminalProvider {
  // ... existing methods unchanged ...

  // NEW: Optional messaging methods
  sendMessage?(paneId: string, message: string): Promise<boolean>
  listAgents?(): Promise<AgentInfo[]>
}

export interface AgentInfo {
  id: string
  name: string
  type: string
  status: 'running' | 'stopped'
}
```

```typescript
// packages/cli/src/terminal/providers/headless.ts (ADD methods)

export class HeadlessProvider implements TerminalProvider {
  private agents: Map<string, { child: ChildProcess; name: string; type: string }> = new Map()

  // EXISTING: runCommand already spawns with stdio: 'pipe'
  // Just need to track the agents

  async sendMessage(paneId: string, message: string): Promise<boolean> {
    const agent = this.agents.get(paneId)
    if (!agent) return false

    // Simple stdin write - no file system involvement
    if (agent.child.stdin?.writable) {
      agent.child.stdin.write(message + '\n')
      return true
    }
    return false
  }

  async listAgents(): Promise<AgentInfo[]> {
    return Array.from(this.agents.entries()).map(([id, agent]) => ({
      id,
      name: agent.name,
      type: agent.type,
      status: agent.child.exitCode === null ? 'running' : 'stopped',
    }))
  }
}
```

**Files to Modify**:
- `packages/cli/src/terminal/interface.ts` - Add optional messaging methods
- `packages/cli/src/terminal/providers/headless.ts` - Add sendMessage + listAgents
- `packages/cli/src/cli/agent.ts` - Support both iTerm and headless providers

**Verification**:
- `crewchief spawn claude --headless` works
- `crewchief agent list` shows headless agents
- `crewchief agent message <headless-agent> "text"` sends to stdin
- No regression for iTerm mode

**Agent**: General TypeScript development

**Risk**: Medium - new functionality but simple stdin pattern

---

## Phase 4: Multi-Agent Spawn (Low Risk)

**Goal**: Re-enable spawning multiple agents with comma syntax

**Note**: Check `scripts/iterm_scripts/spawn_multi_agents.py` (~312 lines) - may already implement this for iTerm. Evaluate reuse vs TypeScript implementation.

**Deliverables**:
- Update `spawn.ts` to handle comma-separated agent types
- Spawn agents in parallel
- Report results for each agent

**Changes**:

```typescript
// packages/cli/src/cli/spawn.ts

.action(async (agents: string, task: string | undefined, options: SpawnOptions) => {
  const agentTypes = agents.split(',').map(a => a.trim())

  if (agentTypes.length === 1) {
    // Single agent - existing logic
    await scheduler.assignSingleAgent(task, agentTypes[0])
  } else {
    // Multi-agent spawn
    console.log(chalk.cyan(`🚀 Spawning ${agentTypes.length} agents...`))

    const results = await Promise.allSettled(
      agentTypes.map(type => scheduler.assignSingleAgent(task, type))
    )

    results.forEach((result, i) => {
      if (result.status === 'fulfilled') {
        console.log(chalk.green(`✅ ${agentTypes[i]}: ${result.value}`))
      } else {
        console.log(chalk.red(`❌ ${agentTypes[i]}: ${result.reason}`))
      }
    })
  }
})
```

**Files to Modify**:
- `packages/cli/src/cli/spawn.ts` - Multi-agent logic (remove explicit disable)
- `packages/cli/src/orchestrator/scheduler.ts` - May need parallel spawn support

**Files to Evaluate for Reuse**:
- `scripts/iterm_scripts/spawn_multi_agents.py` - existing Python implementation

**Verification**:
- `crewchief spawn claude,gemini "task"` spawns both agents
- Each agent gets its own pane/process
- Failures don't block other agents

**Agent**: General TypeScript development

**Risk**: Low - extending existing functionality, may reuse existing Python script

---

## Phase 5: Testing and Documentation

**Goal**: Add minimal test coverage and update documentation

**Deliverables**:
- Unit tests for ITermProvider and HeadlessProvider
- Integration tests for spawn/message/list flow
- Update CLI README with current behavior
- Update Python scripts README
- Clean up unused Python scripts documentation

**Test Files to Create**:
```
packages/cli/src/terminal/providers/__tests__/iterm.test.ts
packages/cli/src/terminal/providers/__tests__/headless.test.ts
packages/cli/src/cli/__tests__/spawn.test.ts
packages/cli/src/cli/__tests__/agent.test.ts
```

**Documentation Updates**:
- `packages/cli/README.md` - Agent section
- `scripts/iterm_scripts/README.md` - Remove references to deleted files

**Verification**:
- `pnpm test` includes new tests
- Test coverage for critical paths
- Documentation matches actual behavior

**Agent**: General TypeScript testing

**Risk**: Low - adding safety net

---

## Phase Summary

| Phase | Tickets | Risk | Dependencies | Key Outcome |
|-------|---------|------|--------------|-------------|
| 1. Dead Code Removal | 2-3 | Medium | None | Remove broken bridge code |
| 2. ITermProvider Fix | 2-3 | Medium-High | Phase 1 | **FIX broken spawn** |
| 3. Headless Messaging | 2-3 | Medium | Phase 2 | stdin-based messaging |
| 4. Multi-Agent Spawn | 1-2 | Low | Phase 2 | comma-separated agents |
| 5. Testing & Docs | 2-3 | Low | Phase 3, 4 | regression safety |

**Total Estimated Tickets**: 9-14

**Critical Path**: Phase 1 → Phase 2 must be done together (spawn is broken until Phase 2)

## Success Criteria

1. **Bug Fix**: `crewchief spawn claude` WORKS (currently broken)
2. **Code Reduction**: ~1,750 lines removed
3. **Feature Parity**: All existing commands work (agent list, agent message)
4. **New Capability**: Headless messaging works via stdin
5. **Test Coverage**: Critical paths tested
6. **Documentation**: Accurate and complete

## Agent Assignments

This project doesn't require specialized agents. All work is general TypeScript/Python cleanup and development:

- **Phases 1-4**: General development agent
- **Phase 5**: General development agent with testing focus

## Rollback Strategy

Each phase is independently deployable. If issues arise:

1. **Phase 1+2**: Git revert - these must be done together
2. **Phase 3**: Disable headless messaging, return error (iTerm still works)
3. **Phase 4**: Disable multi-agent, return error message
4. **Phase 5**: Tests don't affect runtime behavior

**Note**: Phase 1 and 2 should be committed together because Phase 1 breaks compilation until Phase 2 is complete.
