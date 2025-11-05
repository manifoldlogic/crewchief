# CLAUDE.md

Guidance for Claude Code when working with this repository.

## Project Overview

CrewChief is a CLI tool combining:
- **Git worktree management** - Create, list, and manage git worktrees
- **Semantic code search (Maproom)** - Index and search code using PostgreSQL and tree-sitter

## Quick Start

```bash
# Install and build everything
pnpm install
pnpm build

# Testing
pnpm test
pnpm test:watch

# Code quality
pnpm lint
pnpm format
```

## Component-Specific Documentation

Each major component has its own CLAUDE.md with detailed development guidance:

- **`/packages/cli/CLAUDE.md`** - TypeScript CLI development
- **`/packages/maproom-mcp/CLAUDE.md`** - MCP server and Docker setup
- **`/crates/maproom/CLAUDE.md`** - Rust indexer implementation
- **`.agents/CLAUDE.md`** - Project workflow and ticket system
- **`.github/CLAUDE.md`** - CI/CD and GitHub Actions
- **`.devcontainer/CLAUDE.md`** - Development container setup

**When working in a specific component, read that component's CLAUDE.md first.**

## Maproom Semantic Search

This codebase is indexed! Use maproom MCP tools for semantic search.

### When to Use Maproom

**Use `mcp__maproom__search`** for:
- Finding code by concept: "authentication flow", "message handling"
- Exploring architecture: "main classes", "entry points"
- Understanding relationships: "what calls sendMessage"
- Navigating unfamiliar code

**Use `Grep/Glob`** for:
- Exact text matches
- Known filename patterns
- Simple string searches

### Quick Maproom Workflow

1. **Check status**: `mcp__maproom__status({ repo: "crewchief" })`
2. **Search**: `mcp__maproom__search({ repo: "crewchief", query: "agent spawn" })`
3. **Get code**: `mcp__maproom__open({ relpath: "path", worktree: "main" })`
4. **Get context**: `mcp__maproom__context({ chunk_id: "uuid" })`
5. **Update**: `mcp__maproom__upsert({ repo, worktree, root, commit: "HEAD", paths: [...] })`

**Tips**: Use concepts not keywords. If no results, check `status` first. Use `debug: true` to understand rankings.

## Documentation

### `.agents/` - Work in Flight
Project planning, active work tickets, and execution tracking. Agents write here.
- Project planning documents
- Active work tickets
- Progress tracking
- Implementation notes

### `docs/` - Permanent Documentation
Long-term codebase documentation. Read by both agents and humans.
- Architecture documentation
- How-to guides
- API references
- Technical specifications

**Rule**: Agents document active work in `.agents/`, finalized knowledge goes in `docs/`.

## Architecture

### High-Level Structure

```
CrewChief/
├── packages/
│   ├── cli/           # TypeScript CLI (worktree management, agent orchestration)
│   └── maproom-mcp/   # MCP server (wraps Rust indexer)
├── crates/
│   └── maproom/       # Rust indexer (tree-sitter, embeddings, search)
├── .agents/           # Project planning and work tickets
├── .github/           # CI/CD workflows
├── .devcontainer/     # Development container config
└── docs/              # Permanent documentation
```

### Database

Single PostgreSQL instance: `maproom-postgres:5432/maproom`
- Connection: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
- Managed via `packages/maproom-mcp/config/docker-compose.yml`
- Details: `docs/architecture/DATABASE_ARCHITECTURE.md`

## Development Practices

### TypeScript
- ESM modules (import/export)
- Vitest for testing
- Trailing commas enforced (pre-commit)
- Build to `dist/`

### Rust
- Tokio async runtime
- anyhow/thiserror for errors
- Binaries in `packages/cli/bin/<platform>/`

## Safety Rules

**CRITICAL**: File operations must stay within current worktree.

**Verify before ANY file operation**:
```bash
git rev-parse --show-toplevel  # Get worktree root
```

**Never modify**:
- System directories (`/usr/`, `/etc/`, `/System/`)
- Home directory files outside worktree (`~/.bashrc`, `~/.gitconfig`)
- Other repositories or worktrees
- `.git` directory
- Paths outside current worktree

**If external modification seems needed**: STOP, explain what/why, suggest alternatives, wait for approval.

## Project Workflow

CrewChief uses a structured ticket-based workflow for planning and execution. See [.agents/README.md](.agents/README.md) for full directory structure and conventions.

### Slash Commands (`.claude/commands/`)

- **`/create-project [description]`** - Create project planning documents (analysis, architecture, quality strategy, plan)
- **`/create-project-tickets [PROJECT_SLUG]`** - Generate individual tickets from project plan
- **`/review-tickets [PROJECT_SLUG]`** - Review created tickets for quality and completeness
- **`/work-on-project [PROJECT_SLUG]`** - Execute all tickets for a project sequentially
- **`/single-ticket [ticket-id]`** - Complete, verify, and commit a single ticket

### Ticket Workflow Agents (`.claude/agents/ticket-workflow/`)

Each ticket progresses through these agents:

1. **ticket-creator** - Creates standardized work tickets from requirements
2. **[implementation agent]** - Specialized agent completes the work (e.g., database-engineer, rust-indexer-engineer)
3. **unit-test-runner** - Executes tests and reports results (no fixes)
4. **verify-ticket** - Verifies all acceptance criteria are met
5. **commit-ticket** - Creates Conventional Commit with proper formatting

### Workflow Sequence

```
Planning Phase:
  /create-project → analysis, architecture, quality-strategy, plan
  /create-project-tickets → individual ticket files
  /review-tickets → validate ticket quality

Execution Phase (per ticket):
  Implementation → Tests → Verification → Commit

  If tests fail: return to implementation
  If verification fails: return to implementation
  If verification passes: commit and move to next ticket
```

### Project Organization

```
.agents/projects/{SLUG}_{descriptive-name}/
├── README.md           # Project overview
├── planning/           # Strategic docs
│   ├── analysis.md
│   ├── architecture.md
│   ├── plan.md
│   └── quality-strategy.md
└── tickets/            # Work tickets
    ├── {SLUG}-1001_description.md
    └── ...
```

**Active projects**: `.agents/projects/`
**Completed projects**: `.agents/archive/projects/`

See [.agents/README.md](.agents/README.md) for:
- Project lifecycle details
- Naming conventions
- Agent capabilities
- Archive process
