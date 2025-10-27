# @crewchief/maproom-mcp

Maproom MCP server with local LLM embeddings - **zero configuration required**.

Add one line to your `.mcp.json` and get semantic code search powered by local AI. No API keys, no cloud services, no complex setup.

## Features

✨ **Zero Configuration** - Works out of the box with Docker
🔒 **100% Local** - No API keys, no cloud dependencies, complete privacy
🚀 **Fast Hybrid Search** - Vector similarity + full-text search with PostgreSQL
🤖 **Local LLM** - Ollama with nomic-embed-text (768-dimensional embeddings)
📦 **Fully Containerized** - Everything runs in Docker, isolated and clean
🌳 **Multi-Language** - Tree-sitter parsing for TypeScript, JavaScript, Rust, and more

## Quick Start

Add this to your `.mcp.json` configuration file:

```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"]
    }
  }
}
```

**That's it!** No other configuration needed.

### Where to find `.mcp.json`:

- **Claude Desktop (macOS)**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Claude Desktop (Windows)**: `%APPDATA%\Claude\claude_desktop_config.json`
- **Cursor**: `.cursor/mcp.json` in your project root

## System Requirements

- **Docker Desktop 4.x+** ([Install Docker](https://docs.docker.com/get-docker/))
- **4-8 GB RAM** available for Docker
- **5 GB disk space** (images + model + database)
- **Supported OS**: macOS, Linux, Windows with WSL2

Verify Docker is running:
```bash
docker --version
docker compose version
```

## What to Expect

### First Run (2-5 minutes)
The first time you use Maproom, it will:
1. Download Docker images (~1.5 GB compressed)
2. Download the nomic-embed-text model (~275 MB)
3. Initialize PostgreSQL database with pgvector
4. Start all three services (postgres, ollama, maproom-mcp)

Progress indicators will show each step. This happens once.

### Subsequent Runs (10-20 seconds)
After the first run, startup is fast:
- Images and model are cached
- Services start from Docker cache
- Database persists between sessions

## Environment Variables

Customize behavior with environment variables in your `.mcp.json`:

```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"],
      "env": {
        "LOG_LEVEL": "debug",
        "EMBEDDING_MODEL": "nomic-embed-text"
      }
    }
  }
}
```

Available variables:
- `LOG_LEVEL` - Logging verbosity: `error`, `warn`, `info`, `debug` (default: `info`)
- `EMBEDDING_MODEL` - Ollama model to use (default: `nomic-embed-text`)
- `EMBEDDING_DIMENSION` - Vector dimensions (default: `768`)

## Data Persistence

All data is stored in Docker volumes:
- `maproom-data` - PostgreSQL database (indexed code + embeddings)
- `ollama-models` - Downloaded Ollama models (~275 MB)
- `maproom-logs` - MCP server logs
- `maproom-init-sql` - Database initialization script

Your indexed code and embeddings persist between sessions. To completely reset:

```bash
docker volume rm maproom-data ollama-models maproom-logs maproom-init-sql
```

## Troubleshooting

### Docker is not running

**Error**: `Cannot connect to the Docker daemon`

**Solution**: Start Docker Desktop or the Docker service

```bash
# macOS
open -a Docker

# Linux (systemd)
sudo systemctl start docker
```

### Port already in use

**Error**: `port is already allocated`

**Solution**: Change the port or stop the conflicting service

```bash
# Use different port
MAPROOM_PORT=8080 npx @crewchief/maproom-mcp start

# Or find and stop the conflicting service
lsof -i :3000
```

### Services fail to start

**Check service logs**:

```bash
npx @crewchief/maproom-mcp logs
```

**Common issues**:
- Insufficient memory: Docker Desktop needs at least 4GB RAM
- Disk space: Embedding model requires ~500MB, database grows with indexed code
- Network issues: Ensure Docker can pull images from Docker Hub

### Reset everything

If services are in a bad state, reset and restart:

```bash
npx @crewchief/maproom-mcp stop
docker compose -f <path-to-config>/docker-compose.yml down -v
npx @crewchief/maproom-mcp start
```

## Architecture

The stack consists of three Docker services orchestrated automatically:

1. **PostgreSQL 16** (`pgvector/pgvector:pg16`)
   - Vector database with pgvector extension
   - Stores code chunks, embeddings, and relationships
   - Hybrid search combining full-text (tsvector) and vector similarity (ivfflat)

2. **Ollama** (`ollama/ollama:latest`)
   - Local LLM inference server
   - Runs nomic-embed-text model for 768-dimensional embeddings
   - Completely offline, no API keys or cloud dependencies

3. **Maproom MCP Server** (TypeScript + Node.js)
   - MCP server implementation following Model Context Protocol
   - Communicates via stdio with Claude/Cursor
   - Provides tools: `search`, `open`, `context`, `upsert`, `status`
   - Calls Rust indexer binary for code parsing and indexing

## Documentation

For more information:
- [Full Documentation](https://github.com/your-org/crewchief/tree/main/packages/maproom-mcp)
- [MCP Protocol](https://modelcontextprotocol.io)
- [Ollama Models](https://ollama.com/library)
- [pgvector](https://github.com/pgvector/pgvector)

## License

MIT - see [LICENSE](./LICENSE) file for details
