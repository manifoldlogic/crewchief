# Architecture: MCP Server Simplification

## Solution Overview

Transform `@crewchief/maproom-mcp` from a Docker orchestration tool into a single-purpose MCP server with no subcommands.

```
npx @crewchief/maproom-mcp   →   Runs MCP server (stdio)
```

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│  MCP Client (VS Code / Cursor / Claude Code)                    │
│        ↓ spawns process, stdio (JSON-RPC)                       │
├─────────────────────────────────────────────────────────────────┤
│  CLI Entry Point (cli.cjs) - ~50 lines                          │
│  • Auto-detect database URL                                     │
│  • Set environment                                              │
│  • Import and run MCP server                                    │
├─────────────────────────────────────────────────────────────────┤
│  MCP Server (index.ts) - TypeScript                             │
│  • JSON-RPC over stdio                                          │
│  • Tool handlers: search, open, scan, upsert, context, status   │
│  • Spawns Rust daemon on first request (lazy init)              │
├─────────────────────────────────────────────────────────────────┤
│  Rust Daemon (crewchief-maproom serve)                          │
│  • Long-running subprocess                                      │
│  • High-performance search/indexing                             │
│  • JSON-RPC over stdio to MCP server                            │
├─────────────────────────────────────────────────────────────────┤
│  PostgreSQL + pgvector (ONLY CONTAINER)                         │
│  • Must be running before MCP server starts                     │
└─────────────────────────────────────────────────────────────────┘

NOT containers (run directly on host/devcontainer):
• MCP server - spawned by MCP client via npx
• Rust daemon - spawned by MCP server as subprocess
• Ollama - user's responsibility if using local embeddings
```

## Technology Choices

### Keep (Unchanged)
- **TypeScript MCP Server** (`src/index.ts`) - Core functionality works
- **Rust Daemon** - Performance-critical operations
- **PostgreSQL + pgvector** - Semantic search database
- **JSON-RPC over stdio** - MCP protocol

### Remove
- **Docker Compose orchestration** - Not MCP server's responsibility
- **Ollama container management** - Unusably slow, user can configure manually
- **Setup/scan/watch subcommands** - Rust daemon handles these
- **Complex configuration detection** - Simple env var hierarchy

### Simplify
- **CLI** - From 1,971 lines to ~50 lines
- **Dependencies** - Remove chokidar and Docker utilities

## Database Connection Strategy

Three-tier hierarchy with simple fallback:

```javascript
function resolveDatabase() {
  // 1. Explicit override (always wins)
  if (process.env.MAPROOM_DATABASE_URL) {
    return process.env.MAPROOM_DATABASE_URL
  }

  // 2. DevContainer detection
  if (process.env.IN_DEVCONTAINER === 'true') {
    return 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'
  }

  // 3. Default localhost (VSCode extension port)
  return 'postgresql://maproom:maproom@localhost:5433/maproom'
}
```

### Why This Works

| Scenario | `MAPROOM_DATABASE_URL` | `IN_DEVCONTAINER` | Result |
|----------|------------------------|-------------------|--------|
| Custom setup | `postgresql://...` | any | Uses explicit URL |
| DevContainer | not set | `true` | Uses container network |
| Local + VSCode ext | not set | not set | Uses localhost:5433 |

## Component Responsibilities

### MCP Server (`@crewchief/maproom-mcp`)
- **Single purpose**: Run MCP JSON-RPC server
- **Input**: MCP client spawns via npx
- **Output**: JSON-RPC responses over stdio
- **Requirement**: Database must exist

### VSCode Extension (`vscode-maproom`)
- **Docker management**: PostgreSQL container only
- **MCP config writing**: Generate correct editor config
- **User interface**: Status bar, commands

#### Required Extension Changes

The extension currently manages three Docker services (postgres, ollama, maproom-mcp). This must be simplified:

**File: `packages/vscode-maproom/config/docker-compose.yml`**
- Remove `ollama` service definition
- Remove `maproom-mcp` service definition
- Remove `ollama-models` volume
- Remove `maproom-logs` volume
- Keep only `postgres` service and `maproom-data` volume

**File: `packages/vscode-maproom/src/docker/manager.ts`**
- Update `ensureServicesRunning()` to only start `postgres` service
- Remove provider-conditional Ollama startup logic
- Simplify health checking to PostgreSQL only

**File: `packages/vscode-maproom/src/config/mcp-writer.ts`**
- Update `buildEnvironment()` to include:
  - `MAPROOM_DATABASE_URL`: Always include connection string
  - `MAPROOM_EMBEDDING_PROVIDER`: Pass the provider selection
  - Provider-specific keys (existing)

### Rust Binary (`crewchief-maproom`)
- **Database migrations**: `crewchief-maproom db migrate`
- **Indexing**: scan, watch, upsert commands
- **Search**: High-performance queries
- **Daemon mode**: Long-running subprocess for MCP server

## MCP Configuration Examples

### VS Code / Cursor
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

### Claude Code
```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"],
      "env": {
        "MAPROOM_DATABASE_URL": "postgresql://maproom:maproom@localhost:5433/maproom",
        "MAPROOM_EMBEDDING_PROVIDER": "openai",
        "OPENAI_API_KEY": "${OPENAI_API_KEY}"
      }
    }
  }
}
```

### DevContainer (auto-detects database)
```json
{
  "servers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"],
      "env": {
        "MAPROOM_EMBEDDING_PROVIDER": "openai",
        "OPENAI_API_KEY": "${env:OPENAI_API_KEY}"
      }
    }
  }
}
```

## Embedding Provider Support

| Provider | Configuration | Performance | Notes |
|----------|---------------|-------------|-------|
| OpenAI | `OPENAI_API_KEY` | Fast (ms) | Recommended |
| Google | `GOOGLE_APPLICATION_CREDENTIALS` | Fast (ms) | Alternative |
| Ollama | Local installation | Slow (minutes) | Not orchestrated, user-managed |

## Performance Considerations

### Daemon Architecture (Unchanged)
- MCP server spawns Rust daemon on first request
- Daemon stays alive for subsequent requests
- 20-50x faster than spawning per request (225ms vs 160-400ms)

### Connection Pooling
- PostgreSQL connections managed by Rust daemon
- No connection overhead per MCP tool call

## Long-term Maintainability

### What's Removed (Less to Maintain)
- 1,920 lines of Docker orchestration code
- Container health checking
- Multi-stage Docker builds
- Ollama model pulling
- Complex configuration merging

### What Remains (Focused)
- ~50 line CLI (trivial)
- MCP server tool handlers (core functionality)
- Daemon client (performance layer)

## Error Handling

### Missing Database Connection
If `MAPROOM_DATABASE_URL` is not set and auto-detection fails, the daemon throws:
```
Error: MAPROOM_DATABASE_URL environment variable is required for daemon operation
```
This is the desired behavior - fail fast with a clear message.

### Database Not Running
If the database is not reachable, the MCP server will start but tool calls will fail with connection errors. This allows the MCP client to show meaningful error messages.

## Migration Path

### For VSCode Extension Users
No change required - extension already manages Docker and writes MCP config.

The extension will be updated to:
1. Manage only PostgreSQL container (not Ollama or MCP server)
2. Write correct MCP config with `MAPROOM_DATABASE_URL` and `MAPROOM_EMBEDDING_PROVIDER`

### For CLI Users (Non-VSCode)

**IMPORTANT: Breaking change in v3.0.0**

The `setup`, `scan`, and `watch` subcommands have been removed. Users must set up the database manually.

**Step 1: Start PostgreSQL with pgvector**
```bash
docker run -d --name maproom-postgres \
  -e POSTGRES_USER=maproom \
  -e POSTGRES_PASSWORD=maproom \
  -e POSTGRES_DB=maproom \
  -p 5433:5432 \
  pgvector/pgvector:pg16
```

**Step 2: Run database migrations**
```bash
# Install the CLI if not already installed
npm install -g @crewchief/cli

# Run migrations
crewchief-maproom db migrate
```

**Step 3: Configure your MCP client**

Add to your MCP configuration file:
```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"],
      "env": {
        "MAPROOM_DATABASE_URL": "postgresql://maproom:maproom@localhost:5433/maproom",
        "MAPROOM_EMBEDDING_PROVIDER": "openai",
        "OPENAI_API_KEY": "${OPENAI_API_KEY}"
      }
    }
  }
}
```

**Step 4: Index your codebase**
```bash
crewchief-maproom scan /path/to/your/repo
```

### DevContainer Users

DevContainers that set `IN_DEVCONTAINER=true` will auto-detect the database hostname. If this doesn't work for your setup, explicitly set `MAPROOM_DATABASE_URL` in your MCP config.

**Override mechanism**: `MAPROOM_DATABASE_URL` always takes precedence over auto-detection.

### Breaking Change Summary
- **Version**: 3.0.0
- **Removed**: `setup`, `scan`, `watch` subcommands
- **Removed**: Docker orchestration
- **Removed**: Ollama container management
- **Requirement**: Database must exist before MCP server starts
- **Action**: CLI users must manually set up PostgreSQL
