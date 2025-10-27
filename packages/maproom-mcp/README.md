# @crewchief/maproom-mcp

Maproom MCP server with local LLM embeddings - zero configuration required.

This package provides a fully containerized semantic code search service powered by:
- **PostgreSQL 16** with pgvector extension for hybrid search
- **Ollama** with nomic-embed-text model for local embeddings (768 dimensions)
- **Maproom** indexer and MCP server built in Rust

## Features

- Zero configuration - everything runs in Docker containers
- Local LLM embeddings - no API keys or cloud services required
- Hybrid search - combines vector similarity and full-text search
- Tree-sitter powered code parsing for TypeScript, JavaScript, Rust, and more
- MCP (Model Context Protocol) server for AI assistant integration

## Prerequisites

You must have **Docker** installed and running:

- **macOS/Windows**: [Docker Desktop 4.x or later](https://www.docker.com/products/docker-desktop/)
- **Linux**: Docker Engine with Docker Compose v2 plugin

Verify Docker is running:
```bash
docker --version
docker compose version
```

## Installation

No installation required! Run directly with npx:

```bash
npx @crewchief/maproom-mcp
```

Or install globally:

```bash
npm install -g @crewchief/maproom-mcp
maproom-mcp
```

## Quick Start

### 1. Start the Maproom stack

```bash
npx @crewchief/maproom-mcp start
```

This will:
- Pull required Docker images (PostgreSQL, Ollama, Maproom)
- Download the nomic-embed-text embedding model (~274MB)
- Initialize the PostgreSQL database with pgvector schema
- Start all services in the background

**Note**: First run may take 5-10 minutes to download the embedding model.

### 2. Check service status

```bash
npx @crewchief/maproom-mcp status
```

### 3. View logs

```bash
npx @crewchief/maproom-mcp logs
```

### 4. Stop the stack

```bash
npx @crewchief/maproom-mcp stop
```

## Configuration

### Environment Variables

You can customize the stack behavior using environment variables:

- `MAPROOM_PORT` - MCP server port (default: 3000)
- `OLLAMA_PORT` - Ollama API port (default: 11434)
- `HOST_WORKSPACE` - Path to workspace directory to index (default: /workspace)
- `RUST_LOG` - Log level for Maproom service (default: info)

Example:

```bash
MAPROOM_PORT=8080 npx @crewchief/maproom-mcp start
```

### Persistent Data

All data is stored in Docker volumes:
- `maproom-data` - PostgreSQL database (indexed code, embeddings)
- `ollama-models` - Downloaded Ollama models
- `maproom-config` - Maproom configuration

To reset all data:

```bash
npx @crewchief/maproom-mcp stop
docker volume rm maproom-data ollama-models maproom-config
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

The stack consists of three services:

1. **PostgreSQL** (pgvector/pgvector:pg16)
   - Vector database with pgvector extension
   - Stores code chunks, embeddings, and relationships
   - Hybrid search combining vector similarity and full-text search

2. **Ollama** (ollama/ollama:latest)
   - Local LLM inference server
   - Runs nomic-embed-text model for 768-dimension embeddings
   - No API keys or cloud dependencies

3. **Maproom** (custom Rust binary)
   - Code indexer using tree-sitter for parsing
   - MCP server for AI assistant integration
   - Handles search queries and context assembly

## Documentation

For more information:
- [Full Documentation](https://github.com/your-org/crewchief/tree/main/packages/maproom-mcp)
- [MCP Protocol](https://modelcontextprotocol.io)
- [Ollama Models](https://ollama.com/library)
- [pgvector](https://github.com/pgvector/pgvector)

## License

MIT - see [LICENSE](./LICENSE) file for details
