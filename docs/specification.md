# CrewChief: Multi-Agent Orchestration Tool Specification

## Executive Summary

CrewChief is a TypeScript-based orchestration tool that enables multiple AI agents to collaborate on a single repository using isolated git worktrees, with visual coordination through tmux. It streamlines complex multi-agent workflows into simple, configuration-driven commands.

## Core Objectives

### Primary Goals

1. **Simplify git worktree management** - Abstract complex worktree commands into intuitive operations
2. **Enable parallel AI agent collaboration** - Multiple agents working simultaneously in isolated environments
3. **Provide visual orchestration** - Real-time visibility of all agents through tmux panes
4. **Support competitive agent evaluation** - Compare different agents/configurations on identical tasks
5. **Minimize command complexity** - Single commands for multi-step workflows

## System Architecture

### Component Overview

```
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

#### Commands to Implement

- `crewchief init` - Initialize repository for multi-agent work
- `crewchief worktree create <name> [--branch]` - Create named worktree
- `crewchief worktree list` - Show all active worktrees and their agents
- `crewchief worktree clean` - Remove completed/abandoned worktrees

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

#### Visual Layout Commands

- `crewchief session start` - Initialize tmux session with orchestrator
- `crewchief agent spawn <type> <task>` - Create pane, worktree, and start agent
- `crewchief agent message <agent-id> <message>` - Send instructions to specific agent
- `crewchief agent close <agent-id>` - Close pane and optionally merge work

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

### 4. Task Distribution & Evaluation

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

### 5. Result Integration

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

### 6. Configuration Management

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
  agents: {
    claude: {
      command: 'claude-cli',
      defaultArgs: ['--model', 'claude-3-opus'],
      agentsDir: '.claude/agents/',      // Native Claude agent definitions
      commandsDir: '.claude/commands/'   // Reusable command files
    },
    gemini: {
      command: 'gemini-cli',
      defaultArgs: ['--model', 'gemini-pro'],
      agentsDir: '.gemini/agents/'       // Gemini agent definitions with @file references
    }
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

### 7. CLI Interactive Setup

#### Configuration Wizard

```bash
$ crewchief setup

Welcome to CrewChief Setup!

? Repository type: (monorepo/standard)
? Main branch name: (main)
? Maximum concurrent agents: (5)
? Default agent platform: (claude/gemini/both)
? Enable competition mode by default? (y/n)
? Configure tmux layout? (y/n)
  ? Orchestrator pane size: (40%)
  ? Agent arrangement: (tiled/vertical/horizontal)
? Set up agent definitions now? (y/n)

Configuration saved to crewchief.config.ts
```

### 8. Agent Communication Protocol

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

### 9. Competition Mode

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

1. Git worktree wrapper functions
2. Basic tmux session management
3. Configuration file structure
4. Simple agent spawning

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

1. Interactive CLI setup wizard
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
