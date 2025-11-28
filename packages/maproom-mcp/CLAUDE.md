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
- `context` - Context assembly via daemon (imports, callers, tests, React components)
- `status` - Index stats
- `scan` - Full repo indexing (via daemon)
- `upsert` - Update specific files
- `explain` - Symbol documentation

## Context Tool

Retrieves contextually relevant code around a specific code chunk. Uses the Rust daemon for assembly (20-50x faster than previous PostgreSQL implementation).

### Usage

```typescript
const result = await server.callTool('context', {
  chunk_id: '12345',
  budget_tokens: 6000,  // default: 6000, range: 1000-20000
  expand: {
    callers: true,      // functions that call this chunk
    callees: true,      // functions called by this chunk
    tests: true,        // related test files
    docs: false,        // documentation chunks
    config: false,      // configuration files
    max_depth: 2,       // relationship traversal depth (1-10)
    routes: false,      // route handlers
    hooks: true,        // React hooks
    jsx_parents: true,  // parent React components
    jsx_children: true, // child React components
  },
})
```

### Response Format

```typescript
{
  items: ContextItem[],    // Array of context chunks
  total_tokens: number,    // Tokens used
  budget_tokens: number,   // Budget from request
  budget_remaining: number,// Remaining budget
  truncated: boolean,      // True if budget exceeded
  metadata: {
    chunk_id: number,
    expand_options: object,
  }
}
```

### Troubleshooting

| Error | Cause | Solution |
|-------|-------|----------|
| `DAEMON_START_FAILED` | Daemon binary not found | Ensure crewchief-maproom is installed |
| `CHUNK_NOT_FOUND` | Invalid chunk_id | Use search tool to find valid chunk IDs |
| `CONTEXT_TIMEOUT` | Request took too long | Reduce budget_tokens or check database |
| `INVALID_PARAMS` | Bad parameters | Check chunk_id is positive integer |

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
