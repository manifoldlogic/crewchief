# CLAUDE.md

Guidance for Claude Code when working with this repository.

## Project Overview

CrewChief is a CLI tool combining:
- **Git worktree management** - Create, list, and manage git worktrees
- **Semantic code search (Maproom)** - Index and search code using SQLite and tree-sitter

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

Each major component has its own CLAUDE.md or README with detailed development guidance:

- **`/packages/cli/CLAUDE.md`** - TypeScript CLI development
- **`/packages/daemon-client/README.md`** - Daemon client library for JSON-RPC communication
- **`/packages/maproom-mcp/CLAUDE.md`** - MCP server and Docker setup
- **`/crates/maproom/CLAUDE.md`** - Rust indexer implementation
- **`.agents/CLAUDE.md`** - Project workflow and ticket system
- **`.github/CLAUDE.md`** - CI/CD and GitHub Actions
- **`.devcontainer/CLAUDE.md`** - Development container setup

**When working in a specific component, read that component's CLAUDE.md or README first.**

### Daemon Client (packages/daemon-client)

TypeScript library for communicating with the `crewchief-maproom` daemon via JSON-RPC 2.0. Provides:

- **20-50x performance improvement** over process spawning (225ms vs 160-400ms)
- **Auto-restart** with exponential backoff and circuit breaker
- **Connection pooling** with graceful degradation
- **Type-safe** API with comprehensive error handling

See [packages/daemon-client/README.md](packages/daemon-client/README.md) for complete API documentation, migration guide, and troubleshooting.

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

## Language Server MCP Tools

This codebase has language server MCP tools for Rust and TypeScript. These provide semantic code understanding powered by rust-analyzer and typescript-language-server.

### When to Use Language Servers

**Use Language Server tools** for:
- Finding symbol definitions (structs, functions, types, classes)
- Finding all references to a symbol across the codebase
- Getting type information and documentation at a position
- Finding compiler warnings and errors in a file
- Renaming symbols safely across all usages

**Use `Grep/Glob`** for:
- Text searches that aren't symbol-based
- Finding TODOs, FIXMEs, or comment patterns
- Searching in non-code files

### Available Tools

Both language servers provide identical tools:

| Tool | Purpose | Parameters |
|------|---------|------------|
| `definition` | Find where a symbol is defined | `{ symbolName: "MyClass" }` |
| `references` | Find all usages of a symbol | `{ symbolName: "MyFunction" }` |
| `hover` | Get type info at a position | `{ filePath, line, column }` |
| `diagnostics` | Get warnings/errors in a file | `{ filePath }` |
| `rename_symbol` | Rename across codebase | `{ filePath, line, column, newName }` |

### Which Server to Use

| Language Server | Use For | File Patterns | Status |
|-----------------|---------|---------------|--------|
| `mcp__rust-language-server__*` | Rust code | `crates/**/*.rs` | ✅ Working |
| `mcp__ts-language-server__*` | TypeScript/JavaScript | `packages/**/*.ts`, `*.tsx`, `*.js` | ⚠️ Known issues |

> **TypeScript Language Server Limitations** ([GitHub issues](https://github.com/isaacphi/mcp-language-server/issues?q=typescript)):
> - **Slow startup**: May take 10+ seconds to initialize
> - **Diagnostics unreliable**: TypeScript doesn't support `textDocument/diagnostic` method; relies on async notifications
> - **References partial**: Class references may not be found (methods work better)
> - **May hang**: Some operations timeout without response
>
> **Workarounds**: Use `Grep` for TypeScript code searches, or use the built-in TypeScript compiler (`pnpm tsc --noEmit`) for diagnostics.

### Quick Workflow

```
# Rust: Find a struct definition
mcp__rust-language-server__definition({ symbolName: "SqliteStore" })

# TypeScript: Find a class definition
mcp__ts-language-server__definition({ symbolName: "DaemonClient" })

# Rust: Find all usages of a type
mcp__rust-language-server__references({ symbolName: "ContextBundle" })

# TypeScript: Find all usages of an interface
mcp__ts-language-server__references({ symbolName: "SearchParams" })

# Rust: Check for compiler warnings
mcp__rust-language-server__diagnostics({ filePath: "crates/maproom/src/main.rs" })

# TypeScript: Check for type errors
mcp__ts-language-server__diagnostics({ filePath: "packages/daemon-client/src/client.ts" })

# Get type info at cursor (works the same for both)
mcp__rust-language-server__hover({ filePath: "...", line: 100, column: 15 })
mcp__ts-language-server__hover({ filePath: "...", line: 50, column: 10 })
```

### Tips

- **Symbol names**: Use the simple name (e.g., `BasicContextAssembler`, `DaemonClient`), not the full path
- **Definition vs References**: Use `definition` to jump to implementation, `references` to find all callers/usages
- **Diagnostics**: Great for finding unused variables, dead code, and type errors before running tests
- **Hover**: Useful for understanding complex types or checking documentation
- **Choose the right server**: Use rust-language-server for `.rs` files, ts-language-server for `.ts`/`.js` files

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

### Documentation vs Web Search Tools

**Ref** — Use for official API/library/framework documentation lookup
- `ref_search_documentation` — Search technical docs with full sentence queries (e.g., "Stripe API create subscription endpoint")
- `ref_read_url` — Read a specific documentation URL

**Exa** — Use for code examples, implementation patterns, and general web content
- `get_code_context_exa` — Find code snippets and examples from GitHub, Stack Overflow, etc.
- `web_search_exa` — General web search for current information
- `crawling_exa` — Extract content from a specific URL
- `company_research_exa` — Research companies
- `linkedin_search_exa` — Search LinkedIn profiles/companies
- `deep_researcher_start` / `deep_researcher_check` — Complex multi-source research

### Quick Decision Guide

| Need | Tool |
|------|------|
| API parameters, library syntax, official docs | Ref |
| "How do I implement X" code examples | Exa (`get_code_context_exa`) |
| Current info, news, general questions | Exa (`web_search_exa`) |
| Read a non-docs URL | Exa (`crawling_exa`) |

**Rule**: Ref for authoritative documentation. Exa for code examples and web content. Prefer both over built-in Web Search for better token efficiency.

## Architecture

### High-Level Structure

```
CrewChief/
├── packages/
│   ├── cli/           # TypeScript CLI (worktree management, agent orchestration)
│   ├── daemon-client/ # TypeScript daemon client (JSON-RPC communication)
│   └── maproom-mcp/   # MCP server (wraps Rust indexer)
├── crates/
│   └── maproom/       # Rust indexer (tree-sitter, embeddings, search)
├── .agents/           # Project planning and work tickets
├── .github/           # CI/CD workflows
├── .devcontainer/     # Development container config
└── docs/              # Permanent documentation
```

### Database

Maproom uses SQLite for storage:
- Default location: `~/.maproom/maproom.db`
- Override with: `MAPROOM_DATABASE_URL="sqlite:///path/to/db"`
- Zero-config setup: database auto-created on first use
- Details: `crates/maproom/CLAUDE.md`

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

### New libraries and tools
- When choosing a new library or tool, check if an alternative is already in the codebase, and whether it meets the project's needs, or if there is a reason to use the new library or tool.
- Prefer latest stable versions compatible with the project's dependencies.
- Be pragmatic rather than theoretical or ideological in choosing libraries and tools.
- For major decisions about libraries and tools, call out top choices, and the reasoning behind them, during the planning phase. If the user does not provide a decision, move forward with your top choice based on your own analysis and pragmatism. 

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
  /review-project → validate project quality
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
