# CLAUDE.md - Maproom MCP

Working with the MCP server at `/packages/maproom-mcp`.

## Directory Structure

```
├── bin/cli.cjs              # CLI + Docker orchestration
├── config/
│   ├── docker-compose.yml   # PostgreSQL + pgvector
│   └── init.sql             # Database schema
├── src/
│   ├── index.ts             # MCP server
│   ├── indexer.ts           # Rust binary wrapper
│   └── tools/               # MCP tool handlers
└── tests/                   # Connection tests
```

## Development

```bash
# Build
pnpm build

# Setup provider (one-time)
node bin/cli.cjs setup --provider=ollama
node bin/cli.cjs setup --provider=openai

# Scan/watch
node bin/cli.cjs scan /path/to/repo
node bin/cli.cjs watch /path/to/repo

# Test
pnpm test
```

## Database

PostgreSQL via Docker Compose (`config/docker-compose.yml`):
- **Host**: `maproom-postgres` or `localhost:5432`
- **Database**: `maproom`
- **User/Password**: `maproom/maproom`
- **Connection**: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`

Schema in `config/init.sql`.

## MCP Tools

- `search` - Semantic search (FTS/vector/hybrid)
- `open` - Get code with line ranges
- `context` - Related chunks (imports, callers, tests)
- `status` - Index stats
- `scan` - Full repo indexing
- `upsert` - Update specific files
- `explain` - Symbol documentation

## Rust Binary

Wraps `../../packages/cli/bin/<platform>/crewchief-maproom`:
- Spawned as subprocess
- JSON-RPC over stdin/stdout
- Rebuild only when changing `crates/maproom/`

## Key Points

- **ESM modules**
- **Zod** for MCP validation
- **Pino** for logging
- **pg** for PostgreSQL
- Fallback from Docker network to localhost
