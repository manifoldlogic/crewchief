# CLAUDE.md - Maproom MCP

Working with the MCP server at `/packages/maproom-mcp`.

## Overview

Single-purpose MCP server for semantic code search. Runs via stdio, expects database to exist.

```bash
# Usage (via MCP client)
npx @crewchief/maproom-mcp
```

## Architecture

```
MCP Client (VS Code / Cursor / Claude Code)
    ↓ spawns process, stdio (JSON-RPC)
CLI Entry Point (cli.cjs) - ~50 lines
    ↓ auto-detects database, imports MCP server
MCP Server (index.ts)
    ↓ spawns daemon on first request
Rust Daemon (crewchief-maproom serve)
    ↓ queries
PostgreSQL + pgvector
```

## Database Connection

Three-tier auto-detection:

1. **Explicit**: `MAPROOM_DATABASE_URL` environment variable
2. **DevContainer**: `IN_DEVCONTAINER=true` → uses `maproom-postgres:5432`
3. **Default**: `localhost:5433` (VSCode extension port)

```bash
# Override example
MAPROOM_DATABASE_URL=postgresql://user:pass@host:5432/db npx @crewchief/maproom-mcp
```

## Directory Structure

```
├── bin/cli.cjs              # CLI entry point (~50 lines)
├── src/
│   ├── index.ts             # MCP server
│   ├── daemon.ts            # Rust binary wrapper
│   └── tools/               # MCP tool handlers
└── tests/                   # Connection tests
```

## Development

```bash
# Build
pnpm build

# Test
pnpm test
```

## MCP Tools

- `search` - Semantic search (FTS/vector/hybrid)
- `open` - Get code with line ranges
- `context` - Related chunks (imports, callers, tests)
- `status` - Index stats
- `scan` - Full repo indexing (via daemon)
- `upsert` - Update specific files
- `explain` - Symbol documentation

## Rust Daemon

Wraps `../../packages/cli/bin/<platform>/crewchief-maproom`:
- Spawned as subprocess by MCP server
- JSON-RPC over stdin/stdout
- Handles all database operations
- 20-50x faster than process-per-request

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MAPROOM_DATABASE_URL` | Database connection string | Auto-detected |
| `IN_DEVCONTAINER` | Set `true` for devcontainer network | Not set |
| `MAPROOM_EMBEDDING_PROVIDER` | `openai`, `google`, or `ollama` | Required |
| `OPENAI_API_KEY` | OpenAI API key (if using openai) | Required for openai |
| `GOOGLE_APPLICATION_CREDENTIALS` | Google credentials (if using google) | Required for google |

## Key Points

- **Single purpose**: Run MCP server only (no setup/scan/watch commands)
- **Database required**: PostgreSQL with pgvector must be running
- **ESM modules**
- **Zod** for MCP validation
- **Pino** for logging
- **pg** for PostgreSQL
