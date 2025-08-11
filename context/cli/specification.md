# CrewChief: Multi-Agent Orchestration Tool Specification

## Executive Summary

CrewChief is a TypeScript-based orchestration tool that enables multiple AI agents to collaborate on a single repository using isolated git worktrees, with visual coordination through tmux. It streamlines complex multi-agent workflows into a single, ergonomic entrypoint: running `crewchief`.

On first run, `crewchief` auto-detects project state and performs setup if needed. It then starts (or attaches to) a tmux session. When using agents, the tool transparently creates and reuses per‑agent worktrees while still exposing `worktree`commands for worktree management, regardless of agent usage. These worktree commands are general-purpose and operate on all git worktrees in the repository, regardless of whether they were created by CrewChief or manually.

## Core Objectives

### Primary Goals

1. **Simplify git worktree management** - Abstract complex worktree commands into intuitive operations
2. **Enable parallel AI agent collaboration** - Multiple agents working simultaneously in isolated environments
3. **Provide visual orchestration** - Real-time visibility of all agents through tmux panes
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

### 1. Git Worktree Management

Users should not need to think about worktrees for common workflows. CrewChief automatically creates, names, and reuses per‑agent worktrees behind the scenes when agents are spawned or when competition mode is used. In addition, the `worktree` subcommands provide general repository worktree management: they list and clean any worktrees detected in the repo, annotating agent-associated ones when applicable.

#### CLI (user‑facing)

- `crewchief worktree create <name> [--branch <base>] [--base-path <dir>]` - Create a worktree from a base branch into a storage directory
- `crewchief worktree list` - Show all active worktrees and the agent (if any) associated with each
- `crewchief worktree clean [--stale | --all]` - Remove completed/abandoned worktrees
- `crewchief worktree cd <selector> [--print]` - Resolve a worktree by branch/name/path and start a subshell there by default; use `--print` to output the absolute path instead

#### Internal (non‑user‑facing)

- Automatic worktree creation on agent spawn/competition
- Best‑effort deterministic naming derived from agent identity, task, and timestamp
- Base branch detection from config with safe fallbacks

#### Worktree Configuration Schema

```typescript
interface WorktreeConfig {
  name: string;
  baseBranch: string;
  path: string;
  agent?: AgentAssignment;
  status: 'active' | 'completed' | 'failed' | 'pending';
  createdAt: Date;
  metadata: Record<string, any>;
}
```

### 2. Agent Management

#### Agent Type Definitions

```typescript
interface AgentType {
  id: string;
  name: string;
  platform: 'claude' | 'gemini' | 'custom';
  capabilities: string[];
  agentDefinitionPath: string;  // Path to .claude/agents/*.md or .gemini/agents/*
  executionCommand: string;
  environmentVars?: Record<string, string>;
}

// Example predefined types using native agent mechanisms
const agentTypes = {
  'project-manager': {
    capabilities: ['planning', 'delegation', 'review'],
    agentDefinitionPath: '.claude/agents/project-manager.md'  // Uses Claude's native agent format
  },
  'backend-developer': {
    capabilities: ['api', 'database', 'testing'],
    agentDefinitionPath: '.claude/agents/backend-developer.md'
  },
  'frontend-developer': {
    capabilities: ['ui', 'components', 'styling'],
    agentDefinitionPath: '.gemini/agents/frontend-developer.txt'  // Uses @file references for Gemini
  }
};
```

### 3. Tmux Integration

#### Primary Entry

- `crewchief` - Start or attach to the CrewChief tmux session.
  - First‑run behavior: if the current directory is not configured, automatically run interactive `setup` before launching the session.
  - Launch the configured default root agent(s) automatically, or prompt the user to pick an agent if multiple are configured.
  - Present a minimal orchestrator/home pane as needed, but default to showing the active agent(s).

#### Visual Layout and Agent Commands

- `crewchief agent spawn <typeOrId> [--count N] [--task "..."] [--branch <base>] [--env KEY=VAL...]`
  - Creates one or more panes, provisions per‑agent worktrees, and starts the requested agent(s)
- `crewchief agent message <agentId> <message>` - Send instructions to a specific agent (via tmux send + message bus)
- `crewchief agent close <agentId> [--merge auto|manual|skip]` - Close pane and optionally merge work

#### Tmux Automation Functions

```typescript
interface TmuxCommands {
  createPane(layout: 'horizontal' | 'vertical'): string;
  sendKeys(paneId: string, command: string): void;
  captureOutput(paneId: string): string;
  closePane(paneId: string): void;
  resizePane(paneId: string, size: number): void;
}
```

### 4. Cross‑Agent Input Injection

Policy‑controlled keystroke routing from one agent pane to another, mediated by the orchestrator and the message bus for traceability.

#### Requirements

- Explicit command to inject input from source to destination pane
- Policy checks: allow/deny by agent type, whitelist of commands, rate limits
- Audit trail: every injection emits a bus event with correlation id

#### Realm CLI

- `crewchief agent inject <fromAgentId> <toAgentId> "<keys>" [--enter] [--dry-run]`

#### Realm Types

```typescript
interface InputInjectionPolicy {
  allowList?: string[];        // commands or regexes
  denyList?: string[];
  maxPerMinute?: number;
}

interface InputInjectionEvent {
  id: string;
  fromAgentId: string;
  toAgentId: string;
  keys: string;
  pressedEnter: boolean;
  timestamp: Date;
  correlationId: string;
}
```

---

### 5. Task Distribution & Evaluation

#### Task Assignment Flow

```typescript
interface Task {
  id: string;
  description: string;
  requirements: string[];
  acceptanceCriteria: AcceptanceCriteria[];
  competitionMode?: {
    enabled: boolean;
    agentCount: number;
    evaluationStrategy: 'automatic' | 'manual' | 'hybrid';
  };
}

interface TaskAssignment {
  taskId: string;
  agentId: string;
  worktreeId: string;
  startTime: Date;
  deadline?: Date;
  status: 'assigned' | 'in-progress' | 'complete' | 'failed';
}
```

### 6. Result Integration

#### Merge Strategy

```typescript
interface MergeStrategy {
  type: 'automatic' | 'manual' | 'cherry-pick';
  conflictResolution: 'orchestrator' | 'manual' | 'ai-assisted';
  qualityChecks: QualityCheck[];
  rollbackOnFailure: boolean;
}

interface QualityCheck {
  type: 'tests' | 'linting' | 'build' | 'custom';
  command: string;
  successCriteria: string;
}
```

### 7. Configuration Management

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
  tmux: {
    sessionName: 'crewchief',
    orchestratorPaneSize: 40, // percentage
    agentPaneArrangement: 'tiled'
  },
  evaluation: {
    autoMergeThreshold: 0.95,
    requireTestsPass: true,
    requireReview: false
  }
};
```

### 8. CLI Interactive Setup

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
? Configure tmux layout? (y/n)
  ? Orchestrator pane size: (40%)
  ? Agent arrangement: (tiled/vertical/horizontal)
? Set up agent definitions now? (y/n)
? Update LLM guide files (e.g., CLAUDE.md) with instructions on using `crewchief` to spawn agents? (y/n)

Configuration saved to crewchief.config.ts
```

### 9. Agent Communication Protocol

#### Bidirectional Messaging

```typescript
interface AgentMessage {
  type: 'instruction' | 'result' | 'status' | 'error';
  from: 'orchestrator' | string; // agent-id
  to: 'orchestrator' | string;    // agent-id
  payload: any;
  timestamp: Date;
  worktreeContext?: {
    branch: string;
    modifiedFiles: string[];
    lastCommit: string;
  };
}

// Communication channels
class MessageBus {
  send(message: AgentMessage): void;
  onMessage(handler: (msg: AgentMessage) => void): void;
  waitForResponse(messageId: string, timeout?: number): Promise<AgentMessage>;
}
```

### 10. Competition Mode

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

## Implementation Priorities

### Phase 1: Core Foundation (Week 1)

1. Git worktree wrapper functions (automatic provisioning; user‑facing list/clean only)
2. Basic tmux session management with `crewchief` as the primary entrypoint
3. Configuration file structure including `launch` and `defaults.rootAgents`
4. Simple agent spawning and auto‑worktree creation

### Phase 2: Agent Integration (Week 2)

1. Claude CLI integration patterns
2. Gemini CLI integration patterns
3. Message passing system
4. Result capture mechanisms

### Phase 3: Orchestration (Week 3)

1. Task distribution logic
2. Quality evaluation framework
3. Automatic merging strategies
4. Competition mode basics

### Phase 4: Polish & Optimization (Week 4)

---

### 11. Deprecations & Aliases

- Remove `crewchief init`. Its responsibilities are covered by `setup`, which also runs automatically on first run.
- Replace `crewchief session start` with simply `crewchief`.
- Keep `worktree list` and `worktree clean` as advanced maintenance commands. Worktree creation is automatic and not exposed by default.

---

### 12. Realm & Semantic Retrieval

Central module managing a meaning ontology, embeddings, and vector indexes for semantic search across repository artifacts and bus transcripts.

#### Features

- Ontology: typed graph of entities (Tasks, Runs, Agents, Files, Artifacts)
- Embedding provider abstraction: `openai`, `vertex`, `local` (pluggable)
- Indexing: per‑agent and global vector indexes; incremental updates
- Sources: repo files, commit messages, bus transcripts, evaluation summaries
- Query API: lexical + semantic hybrid with reranking hooks

#### Benchmarking CLI

- `crewchief realm build [--full | --incremental] [--agent <id>]`
- `crewchief realm query "<question>" [--agent <id>] [--topK 8]`

#### Benchmarking Types

```typescript
interface OntologyNode {
  id: string;
  type: 'Task' | 'Run' | 'Agent' | 'File' | 'Artifact' | 'Message';
  properties: Record<string, unknown>;
  relations: { type: string; to: string }[];
}

interface EmbeddingProvider {
  embed(texts: string[], options?: Record<string, unknown>): Promise<number[][]>;
  dim: number;
  model: string;
}

interface VectorIndex {
  upsert(items: { id: string; vector: number[]; metadata: Record<string, unknown> }[]): Promise<void>;
  query(vector: number[], topK: number): Promise<{ id: string; score: number }[]>;
}
```

---

### 13. Observability Extensions

Richer JSONL envelopes and run introspection.

#### Additions

- Correlation ids on all events; parent/child relationships
- Work phases: `setup`, `assignment`, `execution`, `evaluation`, `merge`
- Context snapshots: worktree metadata, modified files, last commit
- Tailored `runs logs --tail` and `runs events` formatting

```typescript
interface BusEnvelope<TPayload = unknown> {
  id: string;
  correlationId?: string;
  parentId?: string;
  phase?: 'setup' | 'assignment' | 'execution' | 'evaluation' | 'merge';
  payload: TPayload;
  createdAt: Date;
  worktree?: { branch: string; lastCommit: string; changedFiles: string[] };
}
```

---

### 14. Benchmarking & Tournaments

Scenario‑based benchmarking to compare agents and configurations.

#### Concepts

- Scenario: named task with fixtures, constraints, and expected artifacts
- Batch: execution of a scenario across an agent set
- Leaderboard: persisted scores with metric weights and time windows

#### CLI

- `crewchief eval benchmark <scenarioId> [--agents a,b,c] [--repeat 3]`

#### Types

```typescript
interface BenchmarkScenario {
  id: string;
  name: string;
  description: string;
  task: Task;
  fixturesDir?: string;
  metrics: EvaluationMetric[];
}

interface BenchmarkResult {
  scenarioId: string;
  agentId: string;
  runId: string;
  scores: Record<string, number>; // metric -> score
  weightedTotal: number;
  createdAt: Date;
}
```

1. Interactive CLI setup wizard (auto‑invoked on first run)
2. Advanced competition features
3. Performance optimizations
4. Error recovery mechanisms

## Success Metrics

- **Setup Time**: < 5 minutes from install to first agent running
- **Command Simplicity**: Single command for complex workflows
- **Agent Efficiency**: 3x faster than manual agent coordination
- **Success Rate**: > 90% automatic merge success
- **Developer Satisfaction**: Intuitive enough for non-experts

## Technical Considerations

### Dependencies

```json
{
  "dependencies": {
    "@types/node": "^20.0.0",
    "commander": "^11.0.0",
    "simple-git": "^3.0.0",
    "node-pty": "^1.0.0",
    "chalk": "^5.0.0",
    "inquirer": "^9.0.0",
    "zod": "^3.0.0"
  }
}
```

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
