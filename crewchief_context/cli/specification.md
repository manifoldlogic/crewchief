# CrewChief: Multi-Tool CLI Specification

> **IMPLEMENTATION STATUS**: This document describes the current state and vision for CrewChief. Features marked with ✅ are implemented, ⚠️ are partially implemented, and ❌ are planned but not yet built.
>
> **Last Updated**: January 2025 - Aligned with v0.1.22

## Executive Summary

CrewChief is a multi-tool CLI that combines git worktree management, semantic code search (Maproom), and AI agent orchestration. Written in TypeScript with Rust components for performance-critical operations, it enables multiple AI agents to collaborate on a single repository using isolated git worktrees, with visual coordination through iTerm2 on macOS.

**Note**: Agent orchestration features require macOS with iTerm2.

The tool provides three main functionalities:

1. **Worktree Management** - Simplify creating, listing, and managing git worktrees
2. **Semantic Code Search** - Index and search code using PostgreSQL and tree-sitter (Maproom)
3. **Agent Orchestration** - Spawn and coordinate AI agents in isolated environments

## Implementation Summary

### Fully Implemented ✅

- **Git worktree management** (create, list, clean, use, merge, copy-ignored)
- **Maproom code indexing and search** (scan, search, watch, MCP server)
- **Agent spawning and message sending** via `spawn` and `agent message` commands (iTerm2 integration)
- **Configuration management** (crewchief.config.js)
- **Setup wizard** (`crewchief setup` command)
- **Doctor/prerequisite checking** (`crewchief doctor`)

### Partially Implemented ⚠️

- **Competition mode** (commands exist but evaluation is basic)

## Installation

### Quick Start

```bash
# Option 1: Run without installing
npx crewchief --help

# Option 2: Install in project
npm install crewchief
# or
pnpm add crewchief

# Option 3: Install globally
npm install -g crewchief

# Setup configuration
crewchief setup

# For Maproom (semantic search), set up PostgreSQL
export PG_DATABASE_URL="postgres://user:password@localhost:5432/maproom"
crewchief maproom:db
crewchief maproom:scan
```

## Core Objectives

### Primary Goals

1. **Simplify git worktree management** - Abstract complex worktree commands into intuitive operations
2. **Provide semantic code search** - Fast, intelligent code discovery via Maproom
3. **Enable parallel AI agent collaboration** - Multiple agents working simultaneously in isolated environments
4. **Provide visual orchestration** - Real-time visibility of all agents through iTerm2 panes
5. **Support competitive agent evaluation** - Compare different agents/configurations on identical tasks
6. **Minimize command complexity** - Single commands for multi-step workflows

## Detailed Requirements

### Git Worktree Management ✅

Users should not need to think about worktrees for common workflows. CrewChief automatically creates, names, and reuses per‑agent worktrees behind the scenes when agents are spawned. In addition, the `worktree` subcommands provide general repository worktree management.

#### CLI Commands - IMPLEMENTED ✅

- `crewchief worktree create <name> [--branch <base>]` - Create a worktree from a base branch
- `crewchief worktree list` - Show all active worktrees
- `crewchief worktree clean [selector] [--all]` - Remove specific worktree or all with --all
- `crewchief worktree use <name> [--print]` - Use a worktree (creates if needed): switches to it if exists, creates it first if not. Starts a subshell by default; use `--print` to output the absolute path
- `crewchief worktree copy-ignored <selector>` - Copy ignored files (like .env) to worktree based on config
- `crewchief worktree merge <name> [--squash]` - Merge changes from worktree back to source branch

#### Internal (non‑user‑facing)

- Automatic worktree creation on agent spawn/competition
- Best‑effort deterministic naming derived from agent identity, task, and timestamp
- Base branch detection from config with safe fallbacks

### Terminal Integration ⚠️ (Partially Implemented)

#### Agent Commands - IMPLEMENTED ✅

- `crewchief spawn <agents> [task]` - Spawn AI agent(s) in dedicated terminal pane(s)
  - Supports single agent: `crewchief spawn claude "fix bug"`
  - Supports multiple agents: `crewchief spawn claude,gemini "review code"`
  - Options: `--name`, `--vertical`, `--args`, `--no-label`
- `crewchief agent message <pattern> [message]` - Send message to agent(s)
- `crewchief agent list` - List running agents in iTerm2
- `crewchief agent close <agentId>` - Close an agent (currently mock implementation)

### Maproom - Semantic Code Search ✅ (Implemented)

#### CLI Commands - IMPLEMENTED ✅

- `crewchief maproom:db` - Initialize/migrate PostgreSQL database
- `crewchief maproom:scan` - Scan and index repository (auto-detects git context)
- `crewchief maproom:upsert <paths>` - Update specific files in the index
- `crewchief maproom:watch` - Watch for changes and auto-index
- `crewchief maproom:search <query>` - Semantic search across indexed code
- `crewchief maproom [args...]` - Forward other commands to Rust binary

#### Features

- **Multi-language support**: TypeScript, JavaScript, Rust, Markdown, JSON, YAML, TOML
- **MCP Server integration**: Works with AI assistants (Claude, Cursor)
- **Auto-detection**: Commands automatically detect repo, worktree, path, and commit
- **Platform binaries**: Pre-built for multiple platforms in `packages/cli/bin/<platform>/`

---

### Configuration Management ✅ (Implemented)

#### Main Configuration File (`crewchief.config.js`)

```javascript
export default {
  repository: {
    mainBranch: 'main',
    worktreeBasePath: '.crewchief/worktrees'
  },
  
  // Terminal configuration (iTerm2 preferred)
  terminal: {
    backend: 'iterm', // or 'auto' for auto-detection
    iterm: {
      sessionName: 'crewchief'
    }
  },
  
  evaluation: {
    autoMergeThreshold: 0.95,
    requireTestsPass: true,
    requireReview: false,
    qualityChecks: [
      {
        type: 'tests',
        command: 'pnpm test'
      },
      {
        type: 'linting',
        command: 'pnpm lint'
      },
      {
        type: 'build',
        command: 'pnpm build'
      }
    ]
  },
  
  launch: {
    autoRunDefaultAgents: false,
    askToUpdateLlmGuides: true
  },
  
  defaults: {
    rootAgents: [
      { id: 'planner', platform: 'claude' },
      { id: 'coder', platform: 'claude' },
      { id: 'reviewer', platform: 'gemini' }
    ]
  },
  
  worktree: {
    copyIgnoredFiles: ['.env', '.env.local', 'config.local.js'],
    copyFromPath: '.',
    overwriteStrategy: 'skip' // or 'overwrite' or 'backup'
  }
};
```

### CLI Interactive Setup ⚠️ (Partially Implemented)

**STATUS**: The `crewchief setup` command is fully implemented and can be run manually. However, automatic invocation on first run of `crewchief` is NOT yet implemented - it only shows an error message suggesting to run setup.

#### Configuration Wizard - IMPLEMENTED ✅

```bash
$ crewchief setup

Welcome to CrewChief Setup!

? Repository type: (standard/monorepo)
? Main branch name: (main)
? Files to copy to new worktrees (comma-separated): (.env, .env.local)
? Update LLM guide files (e.g., CLAUDE.md) with instructions on using crewchief to spawn agents? (y/n)

Configuration saved to crewchief.config.js
```

The setup wizard creates a minimal configuration focused on essential settings. Advanced options like evaluation thresholds and quality checks can be added manually to the config file.

### Competition Mode ⚠️ (Partially Implemented)

**STATUS**: Competition commands exist but evaluation metrics and scoring are basic. The infrastructure for running multiple agents on the same task is in place, but automatic evaluation and winner selection need more work.

#### Available Commands

- `crewchief competition start <description> <agentIds...>` - Create a new competition
- `crewchief competition assign <competitionId>` - Assign task to all competition agents
- `crewchief competition evaluate <competitionId>` - Evaluate competition runs and pick winner
- `crewchief competition finalize <competitionId>` - Evaluate and attempt auto-merge winner
- `crewchief competition compare <worktrees...>` - Compare worktrees by diff vs main

### Environment Prerequisites ✅ (Implemented)

#### Doctor Command

- `crewchief doctor` or `crewchief prereq` - Check environment prerequisites
  - Verifies Node.js version
  - Checks for git installation
  - Detects iTerm2 availability (macOS)
  - Validates pnpm installation
  - Provides helpful installation instructions for missing components

---

## Current Limitations

### Platform Requirements

- **macOS with iTerm2** required for agent orchestration
- Maproom requires PostgreSQL database

### Known Issues

- Agent close command is mock implementation
- Competition evaluation metrics are basic
- No automatic quality checks before merge
- Setup wizard doesn't auto-run on first use

## Success Metrics

- **Setup Time**: < 5 minutes from install to first agent running ✅
- **Command Simplicity**: Single command for complex workflows ✅
- **Agent Efficiency**: Faster than manual agent coordination ✅
- **Worktree Management**: Simplified git operations ✅
- **Search Performance**: Fast semantic code search via Maproom ✅

## Technical Architecture

### Technology Stack

- **TypeScript**: Main CLI implementation
- **Rust**: Maproom binary for performance-critical indexing/search
- **PostgreSQL**: Database for code index storage
- **Tree-sitter**: Code parsing for semantic understanding
- **iTerm2 Python API**: Terminal automation on macOS
- **Commander.js**: CLI framework
- **Zod**: Configuration schema validation

### Error Handling

- Graceful worktree cleanup on failure ✅
- Comprehensive logging for debugging ✅
- Configuration validation with helpful errors ✅
- iTerm2 required for agent features ✅

### Security Considerations

- Sandboxed worktree operations ✅
- Git-ignored files handled separately ✅
- No hardcoded credentials in codebase ✅
- Database credentials via environment variables ✅
