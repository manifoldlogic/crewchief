# CrewChief: Multi-Agent Orchestration Tool Specification

> **IMPLEMENTATION STATUS**: This document describes the full vision for CrewChief. Features marked with ✅ are implemented, ⚠️ are partially implemented, and ❌ are planned but not yet built.

## Executive Summary

CrewChief is a TypeScript-based orchestration tool that enables multiple AI agents to collaborate on a single repository using isolated git worktrees, with visual coordination through iTerm2 on macOS. It streamlines complex multi-agent workflows into a single, ergonomic entrypoint: running `crewchief`.

On first run, `crewchief` auto-detects project state and performs setup if needed. It then creates (or connects to) an iTerm2 session with dedicated tabs and panes for each agent. When using agents, the tool transparently creates and reuses per‑agent worktrees while still exposing `worktree`commands for worktree management, regardless of agent usage. These worktree commands are general-purpose and operate on all git worktrees in the repository, regardless of whether they were created by CrewChief or manually.

## Implementation Summary

### Fully Implemented ✅

- Git worktree management (create, list, clean, cd)
- Maproom code indexing and search
- Basic agent spawning in iTerm2
- Run tracking and logging
- Configuration management
- Agent communication protocol (message bus)

### Partially Implemented ⚠️

- Agent management (basic spawn/message/close works, advanced features missing)
- Competition mode (commands exist, evaluation metrics basic)
- Result integration (basic merge, no quality checks)
- Observability (basic logging, no correlation IDs)

## Core Objectives

### Primary Goals

1. **Simplify git worktree management** - Abstract complex worktree commands into intuitive operations
2. **Enable parallel AI agent collaboration** - Multiple agents working simultaneously in isolated environments
3. **Provide visual orchestration** - Real-time visibility of all agents through iTerm2 panes
4. **Support competitive agent evaluation** - Compare different agents/configurations on identical tasks
5. **Minimize command complexity** - Single commands for multi-step workflows

## System Architecture

### Component Overview

```text
┌─────────────────────────────────────────┐
│           ORCHESTRATOR (Main Pane)      │
│  - Project planning                     │
│  - Task distribution                    │
│  - Quality evaluation                   │
│  - Worktree merging                     │
└─────────────┬───────────────────────────┘
              │
    ┌─────────┴──────┬────────────┬────────────┐
    │                │            │            │
┌───▼─────┐      ┌───▼─────┐  ┌───▼─────┐  ┌───▼─────┐
│ Agent 1 │      │ Agent 2 │  │ Agent 3 │  │ Agent N │
│ Worktree│      │ Worktree│  │ Worktree│  │ Worktree│
│ (Claude)│      │ (Gemini)│  │ (Claude)│  │ (Custom)│
└─────────┘      └─────────┘  └─────────┘  └─────────┘
```

## Detailed Requirements

### Git Worktree Management ✅

Users should not need to think about worktrees for common workflows. CrewChief automatically creates, names, and reuses per‑agent worktrees behind the scenes when agents are spawned or when competition mode is used. In addition, the `worktree` subcommands provide general repository worktree management: they list and clean any worktrees detected in the repo, annotating agent-associated ones when applicable.

#### CLI (user‑facing) - IMPLEMENTED ✅

- `crewchief worktree create <name> [--branch <base>] [--base-path <dir>]` - Create a worktree from a base branch into a storage directory
- `crewchief worktree list` - Show all active worktrees and the agent (if any) associated with each
- `crewchief worktree clean [--stale | --all]` - Remove completed/abandoned worktrees. Safety: refuses to remove the current worktree or any worktree that contains the current working directory; switch directories before cleaning.
- `crewchief worktree use <name> [--print]` - Use a worktree (creates if needed): switches to it if exists, creates it first if not. Starts a subshell by default; use `--print` to output the absolute path instead

#### Internal (non‑user‑facing)

- Automatic worktree creation on agent spawn/competition
- Best‑effort deterministic naming derived from agent identity, task, and timestamp
- Base branch detection from config with safe fallbacks

### iTerm2 Integration ⚠️ (Partially Implemented)

#### Visual Layout and Agent Commands ✅ (IMPLEMENTED)

- `crewchief agent spawn <typeOrId> [--count N] [--task "..."] [--branch <base>] [--env KEY=VAL...]`
  - Creates one or more panes, provisions per‑agent worktrees, and starts the requested agent(s)
- `crewchief agent message <agentId> <message>` - Send instructions to a specific agent (via iTerm2 API + message bus)

---

### Configuration Management ✅ (Implemented)

#### Main Configuration File (`crewchief.config.ts`)

```typescript
export default {
  repository: {
    mainBranch: 'main',
    worktreeBasePath: '.crewchief/worktrees'
  },
  orchestrator: {
    model: 'claude-opus-4-1',
    maxConcurrentAgents: 5,
    defaultTimeout: 30 * 60 * 1000, // 30 minutes
  },
  launch: {
    autoRunDefaultAgents: true,            // if multiple are defined, prompt to choose at launch
    askToUpdateLlmGuides: true             // on setup, offer to update CLAUDE.md / etc. with CrewChief usage tips
  },
  agents: {
    claude: {
      command: 'claude',
      defaultArgs: ['--model', 'claude-3-opus'],
      agentsDir: '.claude/agents/',      // Native Claude agent definitions
      commandsDir: '.claude/commands/'   // Reusable command files
    },
    gemini: {
      command: 'gemini',
      defaultArgs: ['--model', 'gemini-pro'],
      agentsDir: '.gemini/agents/'       // Gemini agent definitions with @file references
    }
  },
  defaults: {
    rootAgents: [                         // agents to run when `crewchief` starts
      // e.g. { id: 'project-manager', platform: 'claude' }
    ]
  },
  iterm: {
    sessionName: 'crewchief',
  },
  evaluation: {
    autoMergeThreshold: 0.95,
    requireTestsPass: true,
    requireReview: false
  }
};
```

### CLI Interactive Setup ⚠️ (Partially Implemented)

**STATUS**: The `crewchief setup` command exists but automatic invocation on first run of `crewchief` is NOT implemented.

On first invocation of `crewchief` in an unconfigured directory, the setup wizard runs automatically. It is idempotent and can be re‑run any time via `crewchief setup`.

#### Configuration Wizard

```bash
$ crewchief setup

Welcome to CrewChief Setup!

? Repository type: (monorepo/standard)
? Main branch name: (main)
? Maximum concurrent agents: (5)
? Default agent platform: (claude/gemini/both)
? Configure default root agent(s) to launch on start? (multi‑select)
? Enable competition mode by default? (y/n)
? Configure iTerm2 layout? (y/n)
  ? Orchestrator pane size: (40%)
  ? Agent arrangement: (tiled/vertical/horizontal)
? Set up agent definitions now? (y/n)
? Update LLM guide files (e.g., CLAUDE.md) with instructions on using `crewchief` to spawn agents? (y/n)

Configuration saved to crewchief.config.ts
```

### Competition Mode ⚠️ (Partially Implemented)

#### Multi-Agent Comparison

```typescript
interface Competition {
  id: string;
  task: Task;
  participants: {
    agentId: string;
    type: AgentType;
    configuration: any;
    worktreeId: string;
  }[];
  evaluationMetrics: EvaluationMetric[];
  winner?: string;
  results: CompetitionResult[];
}

interface EvaluationMetric {
  name: string;
  weight: number;
  evaluate: (worktreePath: string) => Promise<number>;
}
```

---

## Success Metrics

- **Setup Time**: < 5 minutes from install to first agent running
- **Command Simplicity**: Single command for complex workflows
- **Agent Efficiency**: 3x faster than manual agent coordination
- **Success Rate**: > 90% automatic merge success
- **Developer Satisfaction**: Intuitive enough for non-experts

## Technical Considerations

### Error Handling

- Graceful worktree cleanup on failure
- Automatic pane recovery
- Transaction-like operations with rollback
- Comprehensive logging for debugging

### Security Considerations

- Sandboxed worktree operations
- API key management in secure storage
- Agent output sanitization
- Rate limiting for API calls
