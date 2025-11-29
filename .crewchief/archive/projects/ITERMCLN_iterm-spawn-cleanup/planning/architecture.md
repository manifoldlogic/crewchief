# Architecture: iTerm Spawn Command Cleanup

## Design Goals

1. **Fix** - Spawn command is currently BROKEN for iTerm users (30-second timeout)
2. **Simplify** - Remove dead JSON-RPC code, consolidate on working patterns
3. **Unify** - Extend existing `TerminalProvider` interface for messaging
4. **Enable** - Headless messaging support via stdin pipe
5. **Maintain** - Backward compatibility with existing workflows

## Target Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────────────┐
│                        CLI Commands                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│     spawn.ts                              agent.ts                   │
│        │                                     │                       │
│        ▼                                     ▼                       │
│  ┌─────────────────┐                ┌──────────────────┐            │
│  │   Scheduler     │                │ ITermSimpleService│◄─ WORKING │
│  └────────┬────────┘                └────────┬─────────┘            │
│           │                                  │                       │
│           ▼                                  ▼                       │
│  ┌─────────────────┐                ┌──────────────────┐            │
│  │ TerminalFactory │                │  Python Scripts  │            │
│  └────────┬────────┘                │  (send_to_pane,  │            │
│           │                         │   list_panes)    │            │
│     ┌─────┴─────┐                   └──────────────────┘            │
│     │           │                                                    │
│     ▼           ▼                                                    │
│ ┌───────────┐ ┌──────────────┐                                      │
│ │ITermProvider│ │HeadlessProvider│                                   │
│ │(FIXED)    │ │(+ messaging)  │                                      │
│ └─────┬─────┘ └──────┬───────┘                                      │
│       │              │                                               │
│       ▼              ▼                                               │
│ ┌───────────────┐ ┌─────────────┐                                   │
│ │Python Scripts │ │  stdin pipe │                                   │
│ │(spawn_agent,  │ │ (direct I/O)│                                   │
│ │ send_to_pane) │ └─────────────┘                                   │
│ └───────────────┘                                                   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Architecture Decision: Extend TerminalProvider (NOT new interface)

**Decision**: Extend existing `TerminalProvider` interface rather than creating new `AgentOrchestrator`

**Rationale**:
1. `TerminalProvider` already exists and is actively used by both providers
2. `IAgentTerminalService` (in non-existent `terminal.interface.ts`) was never committed to main
3. `iterm.adapter.ts` is DEAD CODE - it imports from a missing file
4. Creating new interface adds unnecessary abstraction
5. Extending existing interface maintains consistency

### Component Details

#### 1. Extended TerminalProvider Interface

Add messaging methods to existing interface:

```typescript
// packages/cli/src/terminal/interface.ts (EXISTING - extend)

export interface TerminalProvider {
  // Existing methods (unchanged)
  readonly id: string
  initialize(): Promise<void>
  dispose(): Promise<void>
  createWindow(options?: WindowOptions): Promise<string>
  createTab(windowId: string): Promise<string>
  splitPane(targetId: string, direction: SplitDirection): Promise<string>
  runCommand(paneId: string, command: string): Promise<void>
  focus(paneId: string): Promise<void>

  // NEW: Messaging methods (Phase 3)
  sendMessage?(paneId: string, message: string): Promise<boolean>
  listAgents?(): Promise<AgentInfo[]>
}

// NEW: Agent info type
export interface AgentInfo {
  id: string           // paneId or process ID
  name: string         // Human-readable name (task__type format)
  type: string         // claude, gemini, codex, etc.
  status: 'running' | 'stopped'
}
```

#### 2. ITermProvider (FIXED - Implements TerminalProvider)

Remove JSON-RPC dependency, use direct script calls (like ITermSimpleService):

```typescript
// packages/cli/src/terminal/providers/iterm.ts

export class ITermProvider implements TerminalProvider {
  readonly id = 'iterm'
  private scriptsDir: string

  constructor() {
    // Find scripts directory (same pattern as ITermSimpleService)
    this.scriptsDir = this.findScriptsDir()
  }

  async initialize(): Promise<void> {
    // CHANGED: No longer starts broken JSON-RPC bridge
    if (process.env.TERM_PROGRAM !== 'iTerm.app') {
      throw new Error('ITermProvider requires running in iTerm.app')
    }
    if (!this.scriptsDir) {
      throw new Error('iTerm scripts not found')
    }
  }

  async createWindow(options?: WindowOptions): Promise<string> {
    // Call spawn_agent.py directly (like ITermSimpleService would)
    const args = ['python3', join(this.scriptsDir, 'spawn_agent.py')]
    if (options?.title) args.push('--name', options.title)
    if (options?.workingDirectory) args.push('--project-dir', options.workingDirectory)

    const result = spawnSync(args[0], args.slice(1), { encoding: 'utf-8' })
    // Parse session ID from output
    return this.parseSessionId(result.stdout)
  }

  // NEW: Messaging method (Phase 3)
  async sendMessage(paneId: string, message: string, agentType?: string): Promise<boolean> {
    const args = [
      join(this.scriptsDir, 'send_to_pane.py'),
      '--to', paneId,
      '--text', message,
    ]
    if (agentType) args.push('--agent', agentType)

    const result = spawnSync('python3', args, { encoding: 'utf-8' })
    return result.status === 0
  }

  // NEW: List agents (Phase 3)
  async listAgents(): Promise<AgentInfo[]> {
    const result = spawnSync('python3', [
      join(this.scriptsDir, 'list_panes.py'),
    ], { encoding: 'utf-8' })

    // Parse output, filter for agent panes (name__type format)
    return this.parsePaneList(result.stdout)
  }

  // ... other TerminalProvider methods
}
```

#### 3. HeadlessProvider (Enhanced with stdin Messaging)

Add stdin pipe messaging for headless agents (NO file-based IPC - simpler and more reliable):

```typescript
// packages/cli/src/terminal/providers/headless.ts

interface HeadlessAgent {
  child: ChildProcess
  name: string
  type: string
}

export class HeadlessProvider implements TerminalProvider {
  readonly id = 'headless'
  private agents: Map<string, HeadlessAgent> = new Map()

  async initialize(): Promise<void> {
    // Setup signal handlers (existing)
    process.on('SIGINT', () => this.dispose())
    process.on('SIGTERM', () => this.dispose())
  }

  async runCommand(paneId: string, command: string): Promise<void> {
    // Existing implementation - spawns process
    const child = spawn(command, {
      shell: true,  // Note: kept for compatibility, see security review
      stdio: 'pipe',
    })

    if (child.pid) {
      this.agents.set(paneId, {
        child,
        name: paneId,
        type: this.parseAgentType(paneId),
      })
    }
    // ... existing stdout/stderr handling
  }

  // NEW: Messaging method (Phase 3) - uses stdin pipe directly
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

  // NEW: List agents (Phase 3)
  async listAgents(): Promise<AgentInfo[]> {
    return Array.from(this.agents.entries()).map(([id, agent]) => ({
      id,
      name: agent.name,
      type: agent.type,
      status: agent.child.exitCode === null ? 'running' : 'stopped',
    }))
  }

  async dispose(): Promise<void> {
    // Kill all tracked processes (existing)
    for (const [, agent] of this.agents) {
      agent.child.kill('SIGTERM')
    }
    this.agents.clear()
  }
}
```

**Why stdin pipe over file-based IPC:**
1. **Simpler** - No file system operations
2. **Cleaner** - No cleanup needed
3. **Reliable** - No race conditions
4. **Standard** - Common pattern for CLI tools
5. **Secure** - No file permission concerns

### Files to Delete (Dead Code)

**TypeScript (packages/cli/src/):**
```
src/iterm/iterm.service.ts        # 414 lines - JSON-RPC client (DEAD)
src/iterm/iterm.types.ts          # 94 lines - Types for dead service (DEAD)
src/terminal/iterm.adapter.ts     # 155 lines - Imports missing file (DEAD)
```

Note: `iterm.adapter.ts` imports from `./terminal.interface.js` which doesn't exist in main.
This file was never completed - it's abandoned work, not dead code.

**Python (scripts/iterm_scripts/):**
```
iterm_bridge.py          # 309 lines - JSON-RPC server (DEAD)
iterm_controller.py      # 255 lines - Bridge controller (DEAD)
iterm_agent_manager.py   # 351 lines - Bridge manager (DEAD)
test_bridge.py           # ~250 lines - Bridge tests (DEAD)
test_badge.py            # ~60 lines - Badge tests (DEAD)
test_enter.py            # ~145 lines - Enter key tests (DEAD)
test_agent_detection.py  # ~50 lines - Agent detection tests (DEAD)
demo_smart_spawning.py   # ~140 lines - Demo script (DEAD)
debug_send.py            # ~90 lines - Debug utility (DEAD)
start_bridge.sh          # ~27 lines - Bridge startup (DEAD)
```

**Total removal: ~1,750 lines of dead/unused code**

### Files to Keep and Clean

**TypeScript (packages/cli/src/):**
```
src/iterm/iterm-simple.service.ts  # Keep patterns, may merge into ITermProvider
src/terminal/providers/iterm.ts    # REWRITE to use direct scripts (not ITermService)
src/terminal/providers/headless.ts # Enhance with messaging methods
src/terminal/factory.ts            # Keep, no changes needed
src/terminal/interface.ts          # Extend with optional messaging methods
```

**Python (scripts/iterm_scripts/):**
```
# Active - Used by CLI
send_to_pane.py        # Keep - used by agent.ts
list_panes.py          # Keep - used by agent.ts
agent_config.py        # Keep - imported by send_to_pane.py

# Manual tools - Keep for standalone use
spawn_agent.py         # Keep - will use from ITermProvider
spawn_multi_agents.py  # Keep - potential Phase 4 reuse
list_agents.py         # Keep - useful utility
kill_agent.py          # Keep - will use for agent close
label_pane.py          # Keep - useful utility
pane_manager.py        # Keep - useful utility
spawn_agent_smart.py   # Keep - advanced spawn options
split_horizontal.py    # Keep - utility
split_vertical.py      # Keep - utility
init_primary.py        # Keep - utility

# Docs/Config - Update
README.md              # Update - remove bridge references
AGENT_MANAGEMENT.md    # Update - remove bridge references
PANE_COMMUNICATION.md  # Update - current patterns only
requirements.txt       # Keep
__init__.py            # Keep
```

**Note on Python consolidation**: The plan originally proposed creating `common.py` to share patterns. This is a nice-to-have but NOT required for MVP. The scripts work fine as-is.

### Configuration

No new configuration needed. Existing patterns work:

```javascript
// crewchief.config.js
module.exports = {
  terminal: {
    backend: 'iterm',  // or 'headless', 'auto'
  },
}
```

### Error Handling Strategy

```typescript
export class AgentError extends Error {
  constructor(
    message: string,
    public readonly code: 'SPAWN_FAILED' | 'MESSAGE_FAILED' | 'NOT_FOUND' | 'PROVIDER_UNAVAILABLE',
    public readonly agentId?: string,
  ) {
    super(message)
  }
}

// Usage in providers
async spawn(options: SpawnOptions): Promise<AgentInfo> {
  if (!this.isAvailable()) {
    throw new AgentError(
      'iTerm2 not running or Python scripts not found',
      'PROVIDER_UNAVAILABLE'
    )
  }
  // ...
}
```

### Migration Path

1. **Phase 1**: Delete dead code (no functional change)
2. **Phase 2**: Consolidate ITermSimpleService into ITermProvider
3. **Phase 3**: Add headless messaging
4. **Phase 4**: Enable multi-agent spawn
5. **Phase 5**: Add integration tests

Each phase is independently deployable and testable.

## Technology Decisions

### Keep Python for iTerm2 Integration
- iTerm2's official API is Python-only
- Overhead of `spawnSync` is acceptable (~100ms)
- Alternative would require complex AppleScript

### Stdin Pipe for Headless Messaging (NOT file-based)
- **Simpler** - No file system operations needed
- **Cleaner** - No cleanup required
- **Reliable** - No race conditions or file watching
- **Standard** - Common pattern for CLI tools
- **Secure** - No file permission concerns
- Works with any agent CLI that reads stdin (claude, gemini, etc.)

**Why NOT file-based IPC:**
1. Adds file system complexity
2. Requires cleanup on agent close
3. Race conditions between writes and reads
4. File permission concerns in shared environments
5. Extra dependencies for file watching

### Extend Existing Interface (NOT new interface)
- `TerminalProvider` interface already exists and works
- Adding optional `sendMessage()` and `listAgents()` methods
- No need for new `AgentOrchestrator` abstraction
- Maintains consistency with existing codebase

### No Process Manager Daemon
- Keep it simple - processes are child processes
- RunManager already persists state to disk
- Daemon would add complexity without clear benefit

## Performance Considerations

| Operation | Current | Target | Notes |
|-----------|---------|--------|-------|
| Spawn agent | ~500ms | ~500ms | Python startup dominates |
| Send message | ~200ms | ~200ms | Script invocation |
| List agents | ~300ms | ~300ms | Enumerate iTerm sessions |
| Close agent | ~100ms | ~100ms | Kill signal |

No significant performance changes expected. The bottleneck is Python interpreter startup, which is acceptable for CLI operations.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| iTerm spawn is currently BROKEN | High | Phase 1-2 FIXES this by switching to direct scripts |
| Breaking agent.ts workflows | Medium | Keep ITermSimpleService patterns in new ITermProvider |
| Headless stdin not writable | Low | Check `stdin.writable` before write, return false |
| Python script path issues | Low | Already fixed (scripts/iterm_scripts), add tests |
| iTerm2 API changes | Low | Pin iterm2 package version |

**Note**: The primary risk is that spawn is ALREADY broken. This project fixes it.
