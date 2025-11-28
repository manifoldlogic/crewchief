# @crewchief/maproom-mcp

MCP server for semantic code search powered by SQLite and your choice of embedding provider.

## Breaking Changes (v3.0.0)

> **Major version update**: v3.0.0 simplifies the package to a single-purpose MCP server.

### What Changed

- **Removed**: `setup`, `scan`, `watch` CLI subcommands
- **Removed**: Docker orchestration (container management)
- **New**: Database must exist before MCP server starts
- **New**: MCP server runs on host via `npx`, not in a container

### Migration Guide

**VSCode Extension Users**: No changes required. The extension handles database setup.

**CLI Users**: See [Migration from v2.x](#migration-from-v2x) below.

## Features

- **Fast Hybrid Search** - Vector similarity + full-text search with SQLite
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
        "MAPROOM_DATABASE_URL": "sqlite:///Users/you/.maproom/maproom.db",
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
| `MAPROOM_DATABASE_URL` | SQLite database URL | Auto-detected¹ |
| `MAPROOM_EMBEDDING_PROVIDER` | `openai`, `google`, or `ollama` | Yes |
| `OPENAI_API_KEY` | OpenAI API key | If provider=openai |
| `GOOGLE_APPLICATION_CREDENTIALS` | Google credentials path | If provider=google |

¹ Auto-detection: `MAPROOM_DATABASE_URL` > `~/.maproom/maproom.db`

## MCP Tools

- **search** - Semantic search with FTS, vector, or hybrid modes
- **open** - Get code by file path with line ranges
- **context** - Related chunks (imports, callers, tests)
- **status** - Index statistics
- **scan** - Full repository indexing
- **upsert** - Update specific files

## Migration from v2.x

If you were using the CLI commands (`setup`, `scan`, `watch`), follow these steps:

### Step 1: Create Index

```bash
# Install the CLI if needed
npm install -g @crewchief/cli

# Scan your repository (creates database at ~/.maproom/maproom.db)
crewchief-maproom scan /path/to/your/repo
```

### Step 2: Configure MCP Client

Add the MCP configuration above to your editor.

## Database Connection

The MCP server auto-detects the database URL:

1. **Explicit**: Uses `MAPROOM_DATABASE_URL` if set (must be `sqlite://...`)
2. **Default**: Uses `~/.maproom/maproom.db`

## Requirements

- Node.js 18+
- SQLite database (auto-created on first scan)
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
