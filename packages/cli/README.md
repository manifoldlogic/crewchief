# `crewchief` — Git Worktree Management & Code Indexing CLI

## 🎯 Manage Worktrees. Index Code. Search Semantically

`crewchief` is a CLI tool that simplifies git worktree management and provides powerful semantic code search through the integrated Maproom indexing system.

### Current Features

- **🔀 Git Worktree Management** — Create, list, clean, and navigate git worktrees with simple commands
- **🔍 Semantic Code Search** — Index and search your codebase using PostgreSQL-backed full-text and semantic search
- **📊 Code Intelligence** — Search for functions, classes, and concepts across TypeScript, JavaScript, Rust, Markdown, and JSON files
- **🤖 Agent Infrastructure** — Basic support for spawning AI agents in isolated tmux panes with dedicated worktrees (experimental)
- **📝 Run Tracking** — Track and review agent runs with detailed logging and event capture

### Planned Features (Not Yet Implemented)

- **🤝 Cross-Agent Coordination** — Agents typing into each other's panes and handing off work
- **🏆 Competition & Benchmarking** — Run tournaments between agents to improve quality
- **🧠 Semantic Retrieval (Realm)** — Vector database integration for shared knowledge
- **📈 Advanced Evaluation** — Automated quality scoring and merge decisions

---

## Quick start

```bash
# Install dependencies
pnpm install

# Build the CLI
pnpm build

# Run the CLI
crewchief --help
```

Or run without installing:

```bash
npx crewchief --help
# or
pnpm dlx crewchief --help
```

## Core Commands

### Worktree Management

- `crewchief worktree create <name> [--branch <branch>]` — Create a new worktree and cd into it by default
- `crewchief worktree list` — List all active worktrees
- `crewchief worktree clean [--all] [--stale]` — Remove worktrees
- `crewchief worktree cd <selector> [--print]` — Navigate to a worktree

### Maproom (Semantic Search)

- `crewchief maproom:db` — Initialize/migrate the database
- `crewchief maproom:scan` — Index files into PostgreSQL
- `crewchief maproom:search <query>` — Search indexed code
- `crewchief maproom:upsert` — Update specific files in the index
- `crewchief maproom:watch` — Watch for changes and auto-index

### Agent Management (Experimental)

- `crewchief agent spawn <type> <task>` — Spawn an agent in a tmux pane
- `crewchief agent message <agentId> <message>` — Send message to agent
- `crewchief agent close <agentId>` — Close an agent's pane

### Run Tracking

- `crewchief runs list` — List all agent runs
- `crewchief runs events <runId>` — View run events
- `crewchief runs logs <runId> [--tail N]` — View run logs

### Setup & Configuration

- `crewchief init` — Initialize `.crewchief/` directory
- `crewchief setup` — Interactive configuration wizard
- `crewchief doctor` — Check system dependencies

### Experimental Features

- `crewchief eval run <runId>` — Evaluate a run (limited implementation)
- `crewchief merge run <branch>` — Merge a worktree branch
- `crewchief competition start <description> <agentIds...>` — Start a competition
- `crewchief task assign <agentTypeId> <description>` — Assign a task

## Requirements

- Node.js >= 18 (ESM modules)
- Git (for worktree management)
- PostgreSQL (for Maproom indexing)
- Tmux (optional, for agent features)
- Rust/Cargo (optional, for building Maproom from source)

## Install Options

- Global install: `npm i -g crewchief` (or `pnpm add -g crewchief`)
- Run without install: `npx crewchief@latest` or `pnpm dlx crewchief@latest`

## Maproom Setup

### Database Configuration

Maproom requires PostgreSQL. Set the connection string:

```bash
export PG_DATABASE_URL="postgres://user:password@localhost:5432/maproom"
```

### Initial Setup

```bash
# Initialize database
crewchief maproom:db

# Index your codebase
crewchief maproom:scan

# Search for code
crewchief maproom:search "function that handles authentication"
```

### Supported File Types

- TypeScript (.ts, .tsx)
- JavaScript (.js, .jsx)
- Rust (.rs)
- Markdown (.md, .mdx) - with heading-based chunking
- JSON (.json) - with key-based chunking
- YAML (.yaml, .yml) - with key-based chunking
- TOML (.toml) - with section-based chunking

The Maproom binary (`crewchief-maproom`) is bundled with the package. You can override it with `CREWCHIEF_MAPROOM_BIN=/path/to/binary`.

## Current Limitations

- Agent spawning requires `claude` or `gemini` CLI tools (falls back to mock-agent)
- Competition mode evaluation metrics are basic
- Auto-merge thresholds are not fully implemented
- Cross-agent communication (input injection) is not implemented
- Realm/semantic retrieval features are planned but not implemented
- Benchmarking and tournament features are planned but not implemented
- The main `crewchief` command without arguments does not yet launch a tmux session automatically

## Development

```bash
# Run tests
pnpm test

# Run in development mode
pnpm dev

# Build TypeScript
pnpm build

# Build Maproom (Rust)
cargo build --release --bin crewchief-maproom
```

## Architecture

CrewChief consists of:

1. **CLI Package** (`packages/cli/`) - TypeScript CLI for orchestration
2. **Maproom** (`crates/maproom/`) - Rust-based code indexing and search
3. **Maproom MCP** (`packages/maproom-mcp/`) - Model Context Protocol server for AI assistants

See the [architecture documentation](../context/cli/specification.md) for planned features and design details.
