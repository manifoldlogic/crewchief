# Analysis: iTerm Spawn Command Cleanup

## Problem Definition

The CrewChief CLI's agent spawning and communication system has accumulated technical debt through an abandoned architectural pivot. The codebase contains two parallel implementations:

1. **JSON-RPC Bridge Approach** (abandoned, ~700+ lines of dead code)
   - Complex Python bridge server with HTTP JSON-RPC
   - TypeScript client with async RPC calls
   - Never fully completed or tested

2. **Direct Script Approach** (working, actively used)
   - Simple synchronous calls to standalone Python scripts
   - Used by `agent message` and `agent list` commands
   - Works but limited to iTerm2 only

The result is confusing code, broken features, and no headless support for agent communication.

## Current State Analysis

### File Inventory

**TypeScript - Terminal Layer:**
| File | Lines | Status | Purpose |
|------|-------|--------|---------|
| `src/terminal/interface.ts` | ~90 | Active | Provider interface definitions |
| `src/terminal/factory.ts` | ~35 | Active | Provider auto-detection |
| `src/terminal/providers/iterm.ts` | ~55 | Active | Uses ITermService (complex) |
| `src/terminal/providers/headless.ts` | ~80 | Active | Process spawning |
| `src/terminal/iterm.adapter.ts` | ~155 | **Dead** | Unused wrapper for ITermService |

**TypeScript - iTerm Services:**
| File | Lines | Status | Purpose |
|------|-------|--------|---------|
| `src/iterm/iterm.service.ts` | ~414 | **Dead** | JSON-RPC client |
| `src/iterm/iterm-simple.service.ts` | ~158 | Active | Direct script calls |
| `src/iterm/iterm.types.ts` | ~94 | **Partial** | Types for dead service |

**TypeScript - CLI Commands:**
| File | Lines | Status | Purpose |
|------|-------|--------|---------|
| `src/cli/spawn.ts` | ~86 | Active | Spawn command |
| `src/cli/agent.ts` | ~193 | Active | Agent message/list commands |

**TypeScript - Orchestration:**
| File | Lines | Status | Purpose |
|------|-------|--------|---------|
| `src/orchestrator/scheduler.ts` | ~85 | Active | Assigns agents to terminals |
| `src/orchestrator/runManager.ts` | ~150 | Active | Persists run state |

**Python Scripts (Complete Inventory):**

*Active - Used by CLI:*
| File | Lines | Status | Purpose |
|------|-------|--------|---------|
| `send_to_pane.py` | ~200 | **Active** | Send text to iTerm pane (used by agent.ts) |
| `list_panes.py` | ~76 | **Active** | List iTerm panes (used by agent.ts) |
| `agent_config.py` | ~33 | **Active** | Agent Enter key config (imported by send_to_pane.py) |

*Manual Tools - Standalone scripts:*
| File | Lines | Status | Purpose |
|------|-------|--------|---------|
| `spawn_agent.py` | ~210 | Manual | Standalone spawn tool |
| `spawn_agent_smart.py` | ~287 | Manual | Enhanced spawn tool |
| `spawn_multi_agents.py` | ~312 | Manual | **Multi-agent spawn (potential Phase 4 reuse)** |
| `list_agents.py` | ~121 | Manual | List agents with formatting |
| `label_pane.py` | ~83 | Manual | Label panes |
| `kill_agent.py` | ~119 | Manual | Kill agent processes |
| `pane_manager.py` | ~236 | Manual | Pane tree visualization |
| `split_horizontal.py` | ~22 | Manual | Split pane helper |
| `split_vertical.py` | ~22 | Manual | Split pane helper |
| `init_primary.py` | ~35 | Manual | Initialize primary pane |

*Dead Code - To be deleted:*
| File | Lines | Status | Purpose |
|------|-------|--------|---------|
| `iterm_bridge.py` | ~309 | **Dead** | JSON-RPC server (never worked) |
| `iterm_controller.py` | ~255 | **Dead** | Bridge controller |
| `iterm_agent_manager.py` | ~351 | **Dead** | Bridge agent manager |
| `test_bridge.py` | ~250 | **Dead** | Bridge tests |
| `test_badge.py` | ~60 | **Dead** | Badge tests |
| `test_enter.py` | ~145 | **Dead** | Enter key tests |
| `test_agent_detection.py` | ~50 | **Dead** | Agent detection tests |
| `demo_smart_spawning.py` | ~140 | **Dead** | Demo script |
| `debug_send.py` | ~90 | **Dead** | Debug utility |

*Utility/Docs:*
| File | Lines | Status | Purpose |
|------|-------|--------|---------|
| `__init__.py` | ~3 | Keep | Package marker |
| `requirements.txt` | ~3 | Keep | Python dependencies |
| `README.md` | ~150 | Update | Documentation |
| `start_bridge.sh` | ~27 | **Dead** | Bridge startup script |
| `AGENT_MANAGEMENT.md` | ~200 | Update | Agent management docs |
| `PANE_COMMUNICATION.md` | ~130 | Update | Pane communication docs |

### Architecture Diagram (Current)

```
┌─────────────────────────────────────────────────────────────────────┐
│                        CLI Entry Points                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  crewchief spawn <agent>          crewchief agent message/list      │
│         │                                    │                       │
│         ▼                                    ▼                       │
│  ┌─────────────┐                    ┌──────────────────┐            │
│  │  spawn.ts   │                    │    agent.ts      │            │
│  └──────┬──────┘                    └────────┬─────────┘            │
│         │                                    │                       │
│         ▼                                    ▼                       │
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
│ ┌───────┐ ┌──────────┐                                              │
│ │iTerm  │ │ Headless │                                              │
│ │Provider│ │ Provider │                                              │
│ └───┬───┘ └────┬─────┘                                              │
│     │          │                                                     │
│     ▼          ▼                                                     │
│ ┌───────────┐  ┌─────────────┐                                      │
│ │ITermService│  │ChildProcess │◄─ WORKING                           │
│ │(JSON-RPC) │  │  spawning   │                                      │
│ └─────┬─────┘  └─────────────┘                                      │
│       │                                                              │
│       ▼                                                              │
│ ┌─────────────┐                                                     │
│ │iterm_bridge │◄─ DEAD CODE (never starts properly)                 │
│ │   .py       │                                                     │
│ └─────────────┘                                                     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### What Works Today

1. **`crewchief spawn claude`** - **BROKEN** for iTerm users
   - Flow: `spawn.ts` → `TerminalFactory.autoDetect()` → `ITermProvider`
   - `ITermProvider.initialize()` calls `ITermService.startBridge()`
   - `startBridge()` tries to start `iterm_bridge.py` as HTTP server on port 8765
   - `waitForBridge()` polls `/health` for 30 seconds then fails with "Bridge failed to start"
   - **There is NO fallback to ITermSimpleService patterns**
   - **CRITICAL: This command fails after 30-second timeout**

2. **`crewchief spawn claude --headless`** - Works
   - Works via `HeadlessProvider` → `child_process.spawn()`
   - Process management, stdout capture works
   - Note: Uses `shell: true` (see security review)

3. **`crewchief agent list`** - Works (iTerm only)
   - Works via `ITermSimpleService` → `list_panes.py`
   - iTerm2 only, no headless support

4. **`crewchief agent message <name> <text>`** - Works (iTerm only)
   - Works via `ITermSimpleService` → `send_to_pane.py`
   - iTerm2 only, no headless support

### Current Spawn Failure Analysis

The spawn command for iTerm users follows this broken path:

```
spawn.ts:52   → TerminalFactory.autoDetect()
factory.ts   → Returns ITermProvider (if TERM_PROGRAM === 'iTerm.app')
spawn.ts:53  → terminal.initialize()
iterm.ts:17  → this.service.startBridge()
iterm.service.ts:63 → spawn('python3', ['iterm_bridge.py', '--port', '8765'])
iterm.service.ts:89 → fetch('http://localhost:8765/health') for 30 seconds
                    → Fails with "Bridge failed to start"
```

The `iterm_bridge.py` script is dead code that was never fully implemented:
- It tries to start an HTTP server for JSON-RPC
- The server never responds to /health requests properly
- After 30 seconds, the spawn command fails

**This is why Phase 1-2 is actually a BUG FIX, not just cleanup.**

### What's Broken

1. **iTerm spawn** - Fails after 30-second timeout (see analysis above)
2. **Multi-agent spawn** - Explicitly disabled ("not yet supported")
3. **Agent messaging in headless** - Crashes with "iTerm2 required"
4. **Tab creation** - Falls back to window creation
5. **No integration tests** - Critical paths untested

## User Journey Analysis

### Intended Workflow

```
1. User has a task to delegate to AI agents
2. User spawns agent(s) in terminal panes:
   $ crewchief spawn claude "implement feature X"

3. Agent starts in dedicated pane with worktree isolation
4. User can monitor multiple agents visually in iTerm2 grid

5. User sends follow-up instructions:
   $ crewchief agent message implement-feature-x__claude "focus on tests"

6. User checks agent status:
   $ crewchief agent list

7. When done, user closes panes manually
```

### Current Pain Points

1. **No headless messaging** - Can't send messages to headless agents
2. **Manual pane management** - No way to programmatically close agents
3. **Naming convention required** - Agents must be named `task__type` format
4. **No multi-agent** - Can't spawn multiple agents at once
5. **Dead code confusion** - Developers don't know what's active

## Root Cause Analysis

The JSON-RPC bridge approach was designed for:
- Bidirectional communication (agent → CLI callbacks)
- Persistent connection for monitoring
- Complex agent orchestration

But the simpler approach suffices because:
- Agents are mostly autonomous (don't need callbacks)
- Status checks can be polling-based
- iTerm2's Python API already handles complexity

The pivot to simple scripts happened but cleanup never followed.

## Industry Comparison

### tmux-based Agent Managers
- Use tmux sessions for process isolation
- Simple `tmux send-keys` for communication
- Works headlessly out of the box

### VS Code Extension Approach
- Terminal API for pane management
- No cross-terminal messaging needed
- Extension handles UI

### Our Hybrid Approach
- iTerm2 for visual orchestration (macOS)
- Headless for CI/automation
- Need unified messaging abstraction

## Recommendations Summary

1. **Remove dead JSON-RPC code** (~700 lines)
2. **Consolidate on ITermSimpleService** approach
3. **Add headless messaging** via named pipes or file-based IPC
4. **Clean up Python scripts** - remove unused, document active
5. **Add minimal integration tests** for spawn → message → list flow
6. **Enable multi-agent spawn** (low-hanging fruit)

## Constraints

- Must maintain backward compatibility for existing workflows
- macOS with iTerm2 is primary use case
- Headless support for CI/automation is secondary
- Python scripts must remain (iTerm2 API is Python-only)
