# Analysis: MCP Server Simplification

## Problem Definition

The `@crewchief/maproom-mcp` package has evolved into an overly complex Docker orchestration tool when its core purpose should be simple: provide an MCP server that connects to a database.

### Current State Problems

1. **Over-engineered CLI** (1,971 lines)
   - Docker Compose orchestration for multiple containers
   - Setup, scan, watch subcommands
   - Ollama container management
   - Complex configuration detection

2. **Wrong Responsibility Boundaries**
   - MCP server shouldn't manage infrastructure
   - Duplicates VSCode extension's Docker management
   - Creates confusion about who owns what

3. **Ollama Performance Reality**
   - Local embedding generation is prohibitively slow
   - Users report waiting minutes for simple operations
   - OpenAI/Google embeddings complete in milliseconds
   - Ollama container management adds complexity for unusable functionality

4. **Configuration Complexity**
   - Multiple ways to configure database connection
   - Environment detection logic that's hard to debug
   - DevContainer vs local vs Docker detection

## Existing Industry Solutions

### Standard MCP Server Pattern
MCP servers are designed to be simple, single-purpose tools:
- Receive JSON-RPC over stdio
- Perform operations
- Return results

Infrastructure concerns (databases, services) are external. The MCP client spawns the server process; the server doesn't manage infrastructure.

### Example: Reference MCP Servers
```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem"],
      "env": {}
    }
  }
}
```
No setup commands. No subcommands. Just runs.

## Current Project State

### What Works
- MCP server (`src/index.ts`) - Tool handlers function correctly
- Rust daemon spawning (`src/daemon.ts`) - Performance architecture is sound
- Database operations - PostgreSQL queries work

### What's Unnecessary
- `bin/cli.cjs` - 2,000 lines of Docker orchestration
- `src/config-manager.ts` - Complex configuration management
- `src/utils/docker-detection.ts` - Environment detection logic
- `config/docker-compose.yml` - Multi-container orchestration
- `config/Dockerfile.mcp-server` - MCP server containerization
- `config/Dockerfile.combined` - Combined container builds
- `config/init.sql` - Schema (Rust handles this via `db migrate`)

### Dependencies to Remove
- `chokidar` - File watching (Rust daemon handles this)
- Various Docker-related utilities

## Research Findings

### User Experience Flow

**Current (Complex)**:
```bash
npx @crewchief/maproom-mcp setup --provider=openai
# Creates containers, runs migrations, configures everything
npx @crewchief/maproom-mcp  # Finally runs MCP server
```

**Desired (Simple)**:
```bash
# Database exists (VSCode extension or manual docker-compose)
npx @crewchief/maproom-mcp  # Runs MCP server
```

### Editor Configuration Reality

All major MCP clients use essentially the same pattern:
- VS Code: `servers.maproom`
- Claude Code: `mcpServers.maproom`
- Cursor: Same as VS Code

The config format is identical except for the root key. Environment variables handle all customization:
- `MAPROOM_DATABASE_URL` - Database connection
- `MAPROOM_EMBEDDING_PROVIDER` - openai, google, or ollama
- `OPENAI_API_KEY` / `GOOGLE_APPLICATION_CREDENTIALS` - Provider auth

### DevContainer Support

DevContainers set `IN_DEVCONTAINER=true` automatically. This single environment variable enables different database hostname detection:
- DevContainer: `maproom-postgres:5432`
- Local: `localhost:5433`

## Key Insights

1. **Single Responsibility**: MCP server should only serve MCP protocol
2. **Infrastructure is External**: VSCode extension handles Docker for those users
3. **CLI Users Have Options**: `docker run postgres` + `crewchief-maproom db migrate`
4. **Ollama is Unusable**: Remove orchestration, keep as user-managed option
5. **50 Lines, Not 2000**: The CLI should be trivial
