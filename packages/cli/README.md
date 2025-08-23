# `crewchief` — Git Worktree Management & Code Indexing CLI

## 🎯 Manage Worktrees. Index Code. Orchestrate AI Agents.

`crewchief` is a CLI tool that simplifies git worktree management, provides powerful semantic code search, and orchestrates AI agents in iTerm2.

### Requirements

**macOS with [iTerm2](https://iterm2.com/downloads.html)**  
> ⚠️ The tmux implementation is incomplete and no longer under development. iTerm2 is required for agent orchestration features.

### Current Features

- **🔀 Git Worktree Management** — Create, list, clean, and navigate git worktrees with simple commands
- **🔍 Semantic Code Search** — Index and search your codebase using PostgreSQL-backed full-text and semantic search
- **📊 Code Intelligence** — Search for functions, classes, and concepts across TypeScript, JavaScript, Rust, Markdown, and JSON files
- **🤖 Agent Orchestration** — Spawn AI agents in isolated iTerm2 panes with dedicated worktrees
- **💬 Agent Communication** — Send messages to agents with proper text submission (chr(13) for Claude)
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
- `crewchief worktree clean [selector] [--all] [--stale]` — Remove specific worktree or clean up stale/all worktrees
- `crewchief worktree cd <selector> [--print]` — Navigate to a worktree or print its path
- `crewchief worktree copy-ignored <selector> [--dry-run]` — Copy git-ignored files to an existing worktree
- `crewchief worktree merge <name> [--no-delete]` — Merge changes from a worktree back to its source branch and optionally clean up

### Maproom (Semantic Search)

- `crewchief maproom:db` — Initialize/migrate the database
- `crewchief maproom:scan` — Index files into PostgreSQL (auto-detects repo, worktree, path, and commit)
- `crewchief maproom:search <query>` — Search indexed code semantically
- `crewchief maproom:upsert [files...]` — Update specific files in the index
- `crewchief maproom:watch` — Watch for changes and auto-index (auto-detects context)

### Agent Management (iTerm2 Required)

- `crewchief spawn <agent> [task]` — Spawn an agent in an iTerm2 pane with its own worktree
- `crewchief agent message <agentId> <message>` — Send message to agent (uses chr(13) for Claude)
- `crewchief agent list` — List running agents
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

## Configuration

CrewChief uses a `crewchief.config.js` file for configuration. Key settings include:

- **Worktree Settings**: Configure automatic copying of git-ignored files to new worktrees
- **Agent Defaults**: Set default agent types and platforms
- **Terminal Backend**: iTerm2 is required (tmux is deprecated)
- **Evaluation Thresholds**: Set auto-merge thresholds and quality checks

### Example: Auto-copy .env files to worktrees

```javascript
// crewchief.config.js
export default {
  worktree: {
    copyIgnoredFiles: ['.env', '.env.local', 'config/*.secret'],
    copyFromPath: '.',
    overwriteStrategy: 'skip', // 'skip', 'overwrite', or 'backup'
  },
}
```

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
# or
export DATABASE_URL="postgres://user:password@localhost:5432/maproom"
```

### Initial Setup

```bash
# Initialize database
crewchief maproom:db

# Index your codebase (auto-detects git context)
crewchief maproom:scan
# Scan completes with statistics:
# ✅ Scan completed successfully!
#    Files processed: 150
#    Total chunks: 1234
#    Total size: 2.5 MB

# Search for code semantically
crewchief maproom:search "function that handles authentication"

# Watch for changes and auto-index
crewchief maproom:watch
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
- The main `crewchief` command without arguments launches a tmux session but agent coordination is experimental

## Development

```bash
# Run tests
pnpm test
pnpm test:watch  # Watch mode

# Run in development mode (no build required)
pnpm dev <command> [options]
# or directly:
tsx src/cli/index.ts <command> [options]

# Build TypeScript
pnpm build

# Build everything (TypeScript + Rust binaries)
pnpm build:all

# Build Maproom (Rust) manually
cargo build --release --bin crewchief-maproom
# or use the build script:
./scripts/build-and-package.sh

# Linting and formatting
pnpm lint       # Run ESLint
pnpm lint --fix # Auto-fix issues
pnpm format     # Run Prettier
```

## Architecture

CrewChief consists of:

1. **CLI Package** (`packages/cli/`) - TypeScript CLI for orchestration
2. **Maproom** (`crates/maproom/`) - Rust-based code indexing and search
3. **Maproom MCP** (`packages/maproom-mcp/`) - Model Context Protocol server for AI assistants

## Related Packages

### Maproom MCP Server

The [maproom-mcp](https://www.npmjs.com/package/maproom-mcp) package provides a Model Context Protocol (MCP) server that enables AI assistants like Claude, Cursor, and other MCP-compatible tools to search and navigate your codebase using Maproom's semantic search capabilities.

#### Key Features:
- Semantic code search across your entire codebase
- Direct file access with line range support
- Automatic indexing integration with the CrewChief CLI
- Works with any MCP-compatible AI assistant

#### Installation:
```bash
npm install -g maproom-mcp
```

#### Integration with CrewChief:
The maproom-mcp server uses the same PostgreSQL database and indexing infrastructure as the CrewChief CLI. When you run `crewchief maproom:scan` to index your codebase, that same index becomes available to AI assistants through the MCP server.

For setup instructions and MCP configuration, see the [maproom-mcp documentation](https://www.npmjs.com/package/maproom-mcp).
