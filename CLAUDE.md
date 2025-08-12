# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### TypeScript/Node CLI Package

```bash
# Install dependencies
pnpm install

# Build the TypeScript CLI
pnpm build
# or specifically for the CLI package:
cd packages/cli && pnpm build

# Run tests
pnpm test
pnpm test:watch  # Run tests in watch mode

# Run a single test file
pnpm vitest run tests/agent.int.test.ts

# Lint and format
pnpm lint
pnpm format

# Development (run CLI without building)
pnpm dev
# or use tsx directly:
tsx src/cli/index.ts --help

# Release versions
pnpm release:patch
pnpm release:minor
pnpm release:major
```

### Rust Components (Maproom)

```bash
# Build Maproom binary
cargo build --release --bin crewchief-maproom
# or use the build script:
./scripts/build-maproom.sh

# Run Maproom tests
cargo test

# Run Maproom with specific commands
cargo run --bin crewchief-maproom -- db
cargo run --bin crewchief-maproom -- scan
cargo run --bin crewchief-maproom -- search
```

## Maproom Semantic Search Available

This codebase has maproom semantic search indexed! Use the maproom MCP tools for:

### When to use Maproom vs other search tools:
- **Use `mcp__maproom__search`** for:
  - Finding code by concept: "authentication flow", "message handling", "state management"
  - Exploring architecture: "main classes", "entry points", "core components"
  - Understanding relationships: "what calls sendMessage", "database queries"
  - Navigating unfamiliar code: When you need to understand how something works
  
- **Use `Grep/Glob`** for:
  - Finding exact text matches
  - Quick file lookups when you know the filename pattern
  - Simple string searches

### Maproom Tools Guide:
1. **`mcp__maproom__status`** - Check what's indexed (USE FIRST!)
   - Shows available repos and worktrees
   - Displays index statistics and last update times
   - Helps you understand what's searchable

2. **`mcp__maproom__search`** - Semantic code search
   - Searches across functions, classes, and symbols
   - Returns ranked results with relevance scores
   - Provides hints and suggestions when no results found

3. **`mcp__maproom__open`** - Retrieve specific code sections
   - Opens files from search results
   - Supports line ranges and context lines
   - Use `context` parameter to see surrounding code

4. **`mcp__maproom__upsert`** - Update the index
   - Re-index files after changes
   - Required when working with new code

### Current Index Status:
- Repository: `crewchief`
- Worktree: `claude-code-docs` (actively indexed)
- File types: TypeScript, JavaScript, JSON, Markdown

## Architecture Overview

### Multi-Agent Orchestration System

CrewChief is a CLI tool that orchestrates multiple AI agents working in parallel. Key architectural components:

1. **Agent Management** (`packages/cli/src/agents/`)
   - `registry.ts`: Central registry for agent types and capabilities
   - `runner.ts`: Handles agent lifecycle and execution
   - `discovery.ts`: Discovers available agents on the system
   - Each agent runs in its own tmux pane and git worktree

2. **Message Bus** (`packages/cli/src/bus/`)
   - `message.bus.ts`: Core inter-agent communication infrastructure
   - `jsonl.ts`: JSONL format for message persistence
   - `logFollower.ts`: Real-time log monitoring
   - All agent communications are logged for inspection

3. **Git Worktree Isolation** (`packages/cli/src/git/`)
   - `worktrees.ts`: Manages isolated git worktrees for each agent
   - `merge.ts`: Handles merging agent work back to main branch
   - Each agent works in `.crewchief/worktrees/<agent-id>/`

4. **Orchestration** (`packages/cli/src/orchestrator/`)
   - `runManager.ts`: Manages agent runs and their lifecycle
   - `scheduler.ts`: Schedules and coordinates agent tasks
   - `competition.ts`: Runs quality competitions between agents
   - `autoMerge.ts`: Automatic merge based on evaluation scores

5. **CLI Commands** (`packages/cli/src/cli/`)
   - Each command file (`agent.ts`, `worktree.ts`, etc.) registers subcommands
   - Main entry point is `index.ts` which sets up all commands
   - Uses Commander.js for CLI structure

6. **Tmux Integration** (`packages/cli/src/tmux/`)
   - `tmux.service.ts`: Manages tmux sessions and panes
   - Each agent gets its own tmux pane for isolation

7. **Configuration** (`packages/cli/src/config/`)
   - `loader.ts`: Loads `crewchief.config.ts` from project root
   - `schema.ts`: Zod schema for config validation
   - Configuration controls agent defaults, tmux layout, evaluation thresholds

### Rust Components

1. **Maproom** (`crates/maproom/`)
   - Code indexing and search service
   - Uses PostgreSQL for storage
   - Tree-sitter for parsing TypeScript/JavaScript
   - Provides semantic search capabilities

2. **Opsdeck** (`crates/opsdeck/`)
   - Terminal UI for monitoring agent operations
   - Real-time visualization of agent activities

### Key Design Patterns

- **Agent Isolation**: Each agent operates in its own sandbox (tmux pane + git worktree)
- **Message-Based Communication**: Agents communicate via logged message bus
- **Competition Framework**: Multiple agents can compete on the same task
- **Evaluation Pipeline**: Automated evaluation of agent outputs before merging
- **Plugin Architecture**: Extensible agent types through registry pattern

## Working with CrewChief Code

When modifying the CLI:

1. TypeScript code is in `packages/cli/src/`
2. Use ESM modules (import/export syntax)
3. Follow existing patterns in command files for new subcommands
4. Tests use Vitest framework
5. Build outputs to `dist/` directory

When working with Rust components:

1. Maproom code is in `crates/maproom/`
2. Database migrations in `crates/maproom/migrations/`
3. Use tokio for async runtime
4. Follow Rust error handling with anyhow/thiserror
