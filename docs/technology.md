# Project Overview

CrewChief is a TypeScript-based multi-agent orchestration tool that enables AI agents to collaborate on repositories using isolated git worktrees with visual coordination through tmux.

## Commands

### Package Management

- **Use pnpm** - This project uses pnpm as the package manager (not npm or yarn)
- `pnpm install` - Install dependencies
- `pnpm test` - Run tests (currently not configured)

## Architecture & Structure

### Core Components

1. **Orchestrator** - Main control pane for project planning, task distribution, quality evaluation, and worktree merging
2. **Agent Management** - Handles multiple AI agents (Claude, Gemini, custom) working in isolated worktrees
   - Claude agents use `.claude/agents/` for native agent definitions and `.claude/commands/` for reusable commands
   - Gemini agents use `.gemini/agents/` and `.gemini/commands/` for gemini.
   - Codex agents use `.codex/agents/` and `.codex/commands/` for codex.
3. **Tmux Integration** - Visual layout and pane management for real-time agent visibility
4. **Git Worktree Management** - Abstracts complex worktree operations into simple commands

### Key Concepts

- **Worktrees** - Isolated git environments for each agent to work without conflicts
- **Competition Mode** - Compare different agents/configurations on identical tasks
- **Task Distribution** - Automatic assignment and evaluation of work across agents
- **Message Bus** - Bidirectional communication protocol between orchestrator and agents

### Documentation

- All technical documentation is in the `docs/` folder
- `docs/specification.md` contains the full system specification and implementation roadmap
- `docs/project-plan.md` contains the detailed implementation plan

## Development Guidelines

### Code Principles

- Use the docs folder for all documentation and diligently keep it up to date
- Focus on ease of use and simplicity
- Make the terminal interface rich and delightful
- Use colors with meaning not haphazardly

### Implementation Phases

1. **Phase 1**: Core foundation - Git worktree wrapper, tmux session management
2. **Phase 2**: Agent integration - Claude/Gemini CLI patterns, message passing
3. **Phase 3**: Orchestration - Task distribution, quality evaluation, merging
4. **Phase 4**: Polish - Interactive setup wizard, competition features

### Technology Stack

- TypeScript for all implementation
- Commander for CLI framework
- Simple-git for git operations
- Node-pty for terminal control
- Chalk for terminal colors
- Inquirer for interactive prompts
- Zod for schema validation
