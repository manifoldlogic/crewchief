# CrewChief CLI

A powerful command-line tool for git worktree management, semantic code search, and AI agent orchestration.

## Installation

```bash
# Check compatibility before installing
npx crewchief doctor

# Install globally
npm install -g crewchief

# Now use directly
crewchief --help
```

(Also works with yarn, pnpm, bun, and other npm-compatible package managers)

## Requirements

- **Node.js >= 18**
- **Git** (for worktree management)
- **PostgreSQL** (for semantic search features - see setup below)
- **macOS with [iTerm2](https://iterm2.com/downloads.html)** (for agent orchestration features)
- **CLI agent tools** (`claude`, `gemini`, etc.) must be installed for agent orchestration

## Quick Start

### Basic Worktree Management

```bash
# Create and switch to a new worktree
crewchief worktree create feature-branch

# List all worktrees
crewchief worktree list

# Switch to an existing worktree (creates if needed)
crewchief worktree use feature-branch

# Merge worktree changes back to source branch
crewchief worktree merge feature-branch

# Clean up worktrees
crewchief worktree clean --all
```

### Semantic Code Search

#### Recommended: Using Maproom MCP

For the best experience with AI assistants like Claude and Cursor, use the Maproom MCP server. This allows AI assistants to search your indexed codebase directly.

See the [maproom-mcp package](https://www.npmjs.com/package/maproom-mcp) for installation and setup instructions.

#### Direct CLI Usage (Requires PostgreSQL)

First, set up your database connection:

```bash
export PG_DATABASE_URL="postgres://user:password@localhost:5432/maproom"
```

Then initialize and use semantic search:

```bash
# Initialize database
crewchief maproom:db

# Index your codebase
crewchief maproom:scan

# Search semantically
crewchief maproom:search "authentication flow"

# Watch for changes and auto-index
crewchief maproom:watch
```

### AI Agent Orchestration (Requires iTerm2)

```bash
# Spawn AI agents with dedicated worktrees
crewchief spawn claude "implement-auth"
crewchief spawn gemini "code-review"

# Spawn multiple agents at once
crewchief spawn claude,gemini "fix-bug"

# List running agents
crewchief agent list

# Send messages to agents
crewchief agent message implement-auth__claude "Add OAuth support"

# Send message to all agents on a task
crewchief agent message implement-auth --all "Update approach"
```

## Configuration

Run the interactive setup wizard on first use:

```bash
crewchief setup
```

This will guide you through:
- Repository type (standard or monorepo)
- Main branch name
- Files to copy to new worktrees (.env files, etc.)
- Whether to update LLM guide files (CLAUDE.md, etc.)

The wizard creates a `crewchief.config.js` file in your project root with your preferences.

**Tip:** Add `.crewchief` to your `.gitignore` file to avoid committing worktree data.

### Manual Configuration

You can also manually create `crewchief.config.js`:

```javascript
export default {
  repository: {
    mainBranch: 'main',
    worktreeBasePath: '.crewchief/worktrees'
  },
  worktree: {
    // Auto-copy .env files to new worktrees
    copyIgnoredFiles: ['.env', '.env.local'],
    copyFromPath: '.',
    overwriteStrategy: 'skip', // 'skip', 'overwrite', or 'backup'
  },
  terminal: {
    backend: 'iterm',
    iterm: {
      sessionName: 'crewchief'
    }
  },
  evaluation: {
    autoMergeThreshold: 0.95,
    requireTestsPass: true,
    requireReview: false
  }
}
```

## Command Reference

All commands below should be prefixed with `crewchief`. For example: `crewchief worktree create feature-branch`

### Worktree Commands

| Command | Description |
|---------|-------------|
| `worktree create <name>` | Create a new worktree |
| `worktree list` | List all worktrees |
| `worktree use <name>` | Switch to worktree (creates if needed) |
| `worktree merge <name>` | Merge worktree changes back |
| `worktree clean` | Remove worktrees |
| `worktree copy-ignored <name>` | Copy .env files to worktree |

### Maproom Commands (Semantic Search)

| Command | Description |
|---------|-------------|
| `maproom:db` | Initialize PostgreSQL database |
| `maproom:scan` | Index your codebase |
| `maproom:search <query>` | Search code semantically |
| `maproom:watch` | Auto-index on file changes |
| `maproom:upsert [files...]` | Update specific files |

**Note:** For AI assistant integration, install [maproom-mcp](https://www.npmjs.com/package/maproom-mcp) instead of using these commands directly.

### Agent Commands (iTerm2 Required)

| Command | Description |
|---------|-------------|
| `spawn <agents> [task]` | Spawn AI agents |
| `agent list` | List running agents |
| `agent message <name> <msg>` | Send message to agent |
| `agent close <id>` | Close agent pane |

### System Commands

| Command | Description |
|---------|-------------|
| `setup` | Interactive configuration wizard |
| `doctor` | Check dependencies |

## PostgreSQL Setup for Semantic Search

The semantic search features require PostgreSQL. Here's a quick setup:

### Option 1: Local PostgreSQL

```bash
# macOS with Homebrew
brew install postgresql@14
brew services start postgresql@14

# Create database
createdb maproom

# Set connection string
export PG_DATABASE_URL="postgres://localhost:5432/maproom"
```

### Option 2: Docker

```bash
docker run -d \
  --name maproom-postgres \
  -e POSTGRES_DB=maproom \
  -e POSTGRES_PASSWORD=password \
  -p 5432:5432 \
  postgres:14

export PG_DATABASE_URL="postgres://postgres:password@localhost:5432/maproom"
```

### Option 3: Cloud Database

Use any PostgreSQL provider (Supabase, Neon, etc.) and set the connection string they provide.

## Supported File Types for Indexing

- TypeScript (.ts, .tsx)
- JavaScript (.js, .jsx)
- Markdown (.md, .mdx)
- JSON (.json)
- YAML (.yaml, .yml)
- TOML (.toml)
- Rust (.rs)

## Alternative Installation Methods

### Run without installing

```bash
npx crewchief --help
```

### Install in a project

```bash
npm install crewchief
# Then run with:
npx crewchief --help
```

## Troubleshooting

### Check system dependencies
```bash
crewchief doctor
```

### Common Issues

**PostgreSQL connection failed**: Ensure PostgreSQL is running and `PG_DATABASE_URL` is set correctly.

**iTerm2 not found**: Agent features require iTerm2 on macOS. Install from [iterm2.com](https://iterm2.com).

**Worktree creation failed**: Ensure you're in a git repository with at least one commit.