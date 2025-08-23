# CrewChief

A multi-tool CLI for git worktree management, semantic code search, and AI agent orchestration.

## Requirements

**macOS with [iTerm2](https://iterm2.com/downloads.html)**  
> ⚠️ The tmux implementation is incomplete and no longer under development. iTerm2 is required for agent orchestration features.

## What's Working

✅ **Git Worktree Management** - Simplify creating, listing, and navigating git worktrees  
✅ **Semantic Code Search** - Index and search code, docs, and configs using PostgreSQL  
✅ **MCP Integration** - Maproom MCP server for AI assistants (Claude, Cursor)  
✅ **Multi-Format Support** - TypeScript, JavaScript, Rust, Markdown, JSON, YAML, TOML  
✅ **Agent Orchestration** - Spawn AI agents in iTerm2 panes with isolated worktrees  
✅ **Agent Communication** - Send messages to agents with proper text submission (chr(13) for Claude)

## What's In Progress

⚠️ **Competition Mode** - Run multiple agents on the same task and compare results

## Quick Start

```bash
# Install and build
pnpm install
pnpm build

# Set up database
export PG_DATABASE_URL="postgres://user:password@localhost:5432/maproom"
crewchief maproom:db

# Index your code (auto-detects git context)
crewchief maproom:scan
# ✅ Scan completed successfully!
#    Files processed: 150
#    Total chunks: 1234

# Search semantically
crewchief maproom:search "authentication flow"

# Manage worktrees
crewchief worktree create feature-branch
crewchief worktree list
crewchief worktree cd feature-branch

# Auto-copy .env files to new worktrees (configure in crewchief.config.ts)
crewchief worktree copy-ignored feature-branch

# Merge worktree changes back to source branch
crewchief worktree merge feature-branch

# Spawn AI agents in iTerm2 (REQUIRES iTerm2)
crewchief spawn claude "implement-auth"      # Creates worktree and launches Claude
crewchief spawn gemini "code-review"         # Creates worktree and launches Gemini
crewchief agent message claude "Add OAuth support"  # Send task to Claude
crewchief agent list                          # List running agents
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
└── crewchief_context/ # Architecture docs & specifications
```

## Documentation

- [CLI README](packages/cli/README.md) - Detailed command reference
- [Architecture Spec](crewchief_context/cli/specification.md) - Full vision with implementation status
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

See the [specification](crewchief_context/cli/specification.md) for the full roadmap.
