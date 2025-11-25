# @crewchief/maproom-mcp

MCP server for semantic code search powered by PostgreSQL, pgvector, and your choice of embedding provider.

## Breaking Changes (v3.0.0)

> **Major version update**: v3.0.0 simplifies the package to a single-purpose MCP server.

### What Changed

- **Removed**: `setup`, `scan`, `watch` CLI subcommands
- **Removed**: Docker orchestration (PostgreSQL, Ollama container management)
- **Removed**: Container-based MCP server
- **New**: Database must exist before MCP server starts
- **New**: MCP server runs on host via `npx`, not in a container

### Migration Guide

**VSCode Extension Users**: No changes required. The extension handles database setup.

**CLI Users**: See [Migration from v2.x](#migration-from-v2x) below.

## Features

- **Fast Hybrid Search** - Vector similarity + full-text search with PostgreSQL
- **Semantic Ranking** - Implementations rank higher than tests or docs
- **Choice of Providers** - OpenAI (recommended), Google Vertex AI, or Ollama
- **Multi-Language** - Tree-sitter parsing for TypeScript, JavaScript, Rust, and more

## Usage

### With VSCode/Cursor Extension (Recommended)

Install the [Maproom extension](https://marketplace.visualstudio.com/items?itemName=crewchief.maproom) which handles everything automatically.

### Manual MCP Configuration

Add to your editor's MCP configuration:

```json
{
  "servers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"],
      "env": {
        "MAPROOM_DATABASE_URL": "postgresql://maproom:maproom@localhost:5433/maproom",
        "MAPROOM_EMBEDDING_PROVIDER": "openai",
        "OPENAI_API_KEY": "${env:OPENAI_API_KEY}"
      }
    }
  }
}
```

## Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `MAPROOM_DATABASE_URL` | PostgreSQL connection string | Auto-detected¹ |
| `MAPROOM_EMBEDDING_PROVIDER` | `openai`, `google`, or `ollama` | Yes |
| `OPENAI_API_KEY` | OpenAI API key | If provider=openai |
| `GOOGLE_APPLICATION_CREDENTIALS` | Google credentials path | If provider=google |

¹ Auto-detection: `MAPROOM_DATABASE_URL` > `IN_DEVCONTAINER` > `localhost:5433`

## MCP Tools

- **search** - Semantic search with FTS, vector, or hybrid modes
- **open** - Get code by file path with line ranges
- **context** - Related chunks (imports, callers, tests)
- **status** - Index statistics
- **scan** - Full repository indexing
- **upsert** - Update specific files

## Migration from v2.x

If you were using the CLI commands (`setup`, `scan`, `watch`), follow these steps:

### Step 1: Start PostgreSQL

```bash
docker run -d --name maproom-postgres \
  -e POSTGRES_USER=maproom \
  -e POSTGRES_PASSWORD=maproom \
  -e POSTGRES_DB=maproom \
  -p 5433:5432 \
  pgvector/pgvector:pg16
```

### Step 2: Run Migrations

```bash
# Install the CLI if needed
npm install -g @crewchief/cli

# Run database migrations
crewchief-maproom db migrate
```

### Step 3: Configure MCP Client

Add the MCP configuration above to your editor.

### Step 4: Index Your Codebase

```bash
crewchief-maproom scan /path/to/your/repo
```

## Database Connection

The MCP server auto-detects the database URL:

1. **Explicit**: Uses `MAPROOM_DATABASE_URL` if set
2. **DevContainer**: Uses `maproom-postgres:5432` if `IN_DEVCONTAINER=true`
3. **Default**: Uses `localhost:5433` (VSCode extension port)

## Requirements

- Node.js 18+
- PostgreSQL with pgvector extension
- One of: OpenAI API key, Google credentials, or Ollama installation

## Semantic Ranking

Search results prioritize implementations over tests and documentation:

```
Query: "authenticate"

Before: 1. Documentation: "Auth Guide" ← Not what you want
After:  1. Function: authenticate()   ← Found immediately!
```

Enable debug mode to see score breakdowns:

```typescript
const results = await search({ query: 'authenticate', debug: true })
```

## Support

- [GitHub Issues](https://github.com/danielbushman/crewchief/issues)
- [Documentation](https://github.com/danielbushman/crewchief/tree/main/packages/maproom-mcp)

## License

MIT
