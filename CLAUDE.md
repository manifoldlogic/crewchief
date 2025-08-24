# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Status

CrewChief is a multi-tool CLI that combines:
- **Git worktree management** - Simplify creating, listing, and managing git worktrees
- **Semantic code search** (Maproom) - Index and search code using PostgreSQL and tree-sitter
- **AI agent orchestration** - Spawn and coordinate AI agents in isolated environments

**Important:** Agent orchestration features require **macOS with iTerm2**. The tmux backend is legacy code that is no longer under active development.

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
# or use the comprehensive build script:
./scripts/build-and-package.sh

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
- Worktree: Automatically detected from current branch
- File types: TypeScript, JavaScript, JSON, Markdown, Rust, YAML, TOML

## Architecture Overview

### Multi-Tool CLI System

CrewChief is a multi-tool CLI for git worktree management, semantic code search, and AI agent orchestration. Key architectural components:

1. **Agent Management** (`packages/cli/src/agents/`)
   - `registry.ts`: Central registry for agent types and capabilities
   - `runner.ts`: Handles agent lifecycle and execution
   - `discovery.ts`: Discovers available agents on the system
   - Each agent runs in its own terminal pane (iTerm2) and git worktree

2. **Message Bus** (`packages/cli/src/bus/`)
   - `message.bus.ts`: Core inter-agent communication infrastructure
   - `jsonl.ts`: JSONL format for message persistence
   - `logFollower.ts`: Real-time log monitoring
   - All agent communications are logged for inspection

3. **Git Worktree Isolation** (`packages/cli/src/git/`)
   - `worktrees.ts`: Manages isolated git worktrees for each agent
   - `merge.ts`: Handles merging agent work back to main branch
   - `copy-ignored-files.ts`: Automatically copies git-ignored files (like .env) to new worktrees
   - Each agent works in `.crewchief/worktrees/<agent-id>/`

4. **Orchestration** (`packages/cli/src/orchestrator/`)
   - `runManager.ts`: Manages agent runs and their lifecycle
   - `scheduler.ts`: Schedules and coordinates agent tasks
   - `competition.ts`: Runs quality competitions between agents (in progress)
   - `autoMerge.ts`: Automatic merge based on evaluation scores

5. **CLI Commands** (`packages/cli/src/cli/`)
   - Each command file (`agent.ts`, `worktree.ts`, `spawn.ts`, etc.) registers subcommands
   - Main entry point is `index.ts` which sets up all commands
   - Uses Commander.js for CLI structure

6. **Terminal Integration** (`packages/cli/src/terminal/` and `packages/cli/src/iterm/`)
   - `factory.ts`: Creates appropriate terminal adapter (iTerm2 preferred, tmux fallback)
   - `iterm.adapter.ts`: iTerm2 integration for macOS
   - `tmux.adapter.ts`: Legacy tmux support (no longer under development)
   - Each agent gets its own terminal pane for isolation

7. **Configuration** (`packages/cli/src/config/`)
   - `loader.ts`: Loads `crewchief.config.ts` from project root
   - `schema.ts`: Zod schema for config validation
   - Configuration controls agent defaults, terminal backend, evaluation thresholds

### Rust Components

1. **Maproom** (`crates/maproom/`)
   - Code indexing and search service
   - Uses PostgreSQL for storage
   - Tree-sitter for parsing TypeScript/JavaScript
   - Provides semantic search capabilities
   - MCP server integration for AI assistants

### Key Design Patterns

- **Agent Isolation**: Each agent operates in its own sandbox (terminal pane + git worktree)
- **Message-Based Communication**: Agents communicate via logged message bus
- **Competition Framework**: Multiple agents can compete on the same task (in progress)
- **Evaluation Pipeline**: Automated evaluation of agent outputs before merging
- **Plugin Architecture**: Extensible agent types through registry pattern
- **Terminal Backend Abstraction**: Support for multiple terminal backends (iTerm2 preferred)

## Working with CrewChief Code

When modifying the CLI:

1. TypeScript code is in `packages/cli/src/`
2. Use ESM modules (import/export syntax)
3. Follow existing patterns in command files for new subcommands
4. Tests use Vitest framework
5. Build outputs to `dist/` directory
6. Linting enforces trailing commas everywhere (runs automatically on commit)
7. Use `pnpm lint` and `pnpm format` to check and fix code style

When working with the Maproom component:

1. Maproom code is in `crates/maproom/`
2. Database migrations in `crates/maproom/migrations/`
3. Uses tokio for async runtime
4. Follow Rust error handling with anyhow/thiserror
5. Binary is built and packaged for multiple platforms in `packages/cli/bin/<platform>/`

## Key Features

### Maproom (Semantic Search)
- **Auto-detection**: Commands automatically detect repo, worktree, path, and commit from git context
- **MCP Integration**: Works as an MCP server for AI assistants (Claude, Cursor)
- **Multi-language Support**: TypeScript, JavaScript, Rust, Markdown, JSON, YAML, TOML
- **Statistics Output**: Detailed indexing statistics (files processed, chunks created, language breakdown)
- **Platform Binaries**: Pre-built binaries for multiple platforms in `packages/cli/bin/<platform>/`

### Worktree Management
- **Quick Commands**: Create, list, use, and merge worktrees with simple commands
- **Ignored Files Copying**: Automatically copy git-ignored files (like .env) to new worktrees
- **Agent Integration**: Each AI agent gets its own isolated worktree
- **Flexible Configuration**: Configure patterns, source paths, and overwrite strategies in `crewchief.config.ts`

### Agent Orchestration (macOS + iTerm2)
- **Multi-agent Spawning**: Launch multiple AI agents simultaneously with smart window splitting
- **Isolated Environments**: Each agent works in its own worktree and terminal pane
- **Message Bus**: Inter-agent communication via logged message bus
- **Competition Mode** (in progress): Run multiple agents on the same task and compare results

### Development Experience
- **ESLint + Prettier**: Automatic code formatting with trailing commas on commit
- **Husky Pre-commit Hooks**: Ensures code quality before commits
- **Comprehensive Build Script**: `./scripts/build-and-package.sh` builds all components for all platforms
- **TypeScript + Vitest**: Modern development stack with fast testing

- CRITICAL SAFETY RULE: File modifications must be strictly confined to the current git worktree or repository working directory.

PROHIBITED: Never modify, create, or delete files in:
- System directories (e.g., /usr/, /etc/, /System/)
- User home directory files outside the current worktree (e.g., ~/.bashrc, ~/.zshrc, ~/.gitconfig)
- Parent directories above the current worktree root
- Other git repositories, projects, or other worktrees of the same repository
- Package manager global directories (e.g., /usr/local/, node_modules outside the project)
- The .git directory itself (whether in .git/ or in a separate worktree location)
- Any absolute paths that lead outside the current worktree

REQUIRED: Before ANY file operation:
1. Verify the target path is within the current worktree using `git rev-parse --show-toplevel`
   (This correctly returns the worktree root, whether in main repo or linked worktree)
2. Use relative paths from the worktree root whenever possible
3. If you believe there's a legitimate need to modify external files, STOP immediately and:
   - Explain WHAT file you want to modify and its full path
   - Explain WHY this seems necessary
   - Suggest alternative approaches that stay within the current worktree
   - Wait for explicit user approval before proceeding

RATIONALE: Modifying files outside the worktree can damage system configurations, break other projects (including other worktrees of the same repo), create security vulnerabilities, or cause data loss. The worktree boundary is a critical safety barrier.