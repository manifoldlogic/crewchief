# CrewChief

A multi-tool CLI for git worktree management and semantic code search.

## What's Working

✅ **Git Worktree Management** - Simplify creating, listing, and navigating git worktrees  
✅ **Semantic Code Search** - Index and search code, docs, and configs using PostgreSQL  
✅ **MCP Integration** - Maproom MCP server for AI assistants (Claude, Cursor)  
✅ **Multi-Format Support** - TypeScript, JavaScript, Rust, Markdown, JSON, YAML, TOML

## What's In Progress

⚠️ **Agent Orchestration** - Spawn AI agents in tmux panes with isolated worktrees  
⚠️ **Competition Mode** - Run multiple agents on the same task and compare results

## Quick Start

```bash
# Install and build
pnpm install
pnpm build

# Set up database
export PG_DATABASE_URL="postgres://user:password@localhost:5432/maproom"
crewchief maproom:db

# Index your code
crewchief maproom:scan

# Search semantically
crewchief maproom:search "authentication flow"

# Manage worktrees
crewchief worktree create feature-branch
crewchief worktree list
crewchief worktree cd feature-branch
```

## Project Structure

```
crewchief/
├── packages/
│   ├── cli/           # Main TypeScript CLI
│   └── maproom-mcp/   # MCP server for AI assistants
├── crates/
│   ├── maproom/       # Rust indexing engine
│   └── opsdeck/       # Terminal UI (planned)
└── context/           # Architecture docs & specifications
```

## Documentation

- [CLI README](packages/cli/README.md) - Detailed command reference
- [Architecture Spec](context/cli/specification.md) - Full vision with implementation status
- [Testing Report](TESTING_REPORT.md) - Features that need verification

## Requirements

- Node.js >= 18
- PostgreSQL (for Maproom)
- Git
- Tmux (optional, for agent features)

## Contributing

This project is actively being developed. Key areas that need work:

1. Completing the agent orchestration features
2. Implementing evaluation metrics for competition mode
3. Building the Realm semantic retrieval system
4. Creating the main `crewchief` tmux session launcher

See the [specification](context/cli/specification.md) for the full roadmap.