# @crewchief/maproom-mcp

Semantic code search powered by PostgreSQL, pgvector, and your choice of embedding provider.

**Fast semantic search. One setup command. One line config.**

## Features

✨ **Choice of Providers** - OpenAI (recommended), Google Vertex AI, or local Ollama
🚀 **Fast Hybrid Search** - Vector similarity + full-text search with PostgreSQL
🔄 **Auto-Sync** - Watch mode keeps your index up-to-date automatically
📦 **Fully Containerized** - Everything runs in Docker, isolated and clean
🌳 **Multi-Language** - Tree-sitter parsing for TypeScript, JavaScript, Rust, and more
🔒 **Privacy Options** - Use local Ollama for 100% private embeddings (no API keys)

## Quick Start

### 1. Run Setup (First Time Only)

**Recommended: OpenAI (fast, low cost)**
```bash
export OPENAI_API_KEY=sk-...
npx @crewchief/maproom-mcp setup --provider=openai
```

**Alternative: Google Vertex AI (fast, low cost)**
```bash
export GOOGLE_PROJECT_ID=my-project
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json
npx @crewchief/maproom-mcp setup --provider=google
```

**Local: Ollama (slower, no API key needed)**
```bash
npx @crewchief/maproom-mcp setup --provider=ollama
```

This will (2-5 minutes on first run):
- Download Docker images
- Download embedding model (Ollama only)
- Initialize PostgreSQL with pgvector
- Validate everything works

### 2. Index Your Codebase

```bash
npx @crewchief/maproom-mcp scan /path/to/your/repo
```

**Optional: Auto-sync with watch mode**
```bash
npx @crewchief/maproom-mcp watch /path/to/your/repo
```

This keeps your index up-to-date as you edit code. Leave it running in a terminal.

### 3. Add to MCP Configuration

**Claude Code** (`.claude/mcp.json` in your project):
```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"],
      "env": {
        "EMBEDDING_PROVIDER": "openai",
        "OPENAI_API_KEY": "${OPENAI_API_KEY}"
      }
    }
  }
}
```

**Cursor** (`.cursor/mcp.json` in your project):
```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"],
      "env": {
        "EMBEDDING_PROVIDER": "openai",
        "OPENAI_API_KEY": "${OPENAI_API_KEY}"
      }
    }
  }
}
```

For **Google Vertex AI**, use:
```json
"env": {
  "EMBEDDING_PROVIDER": "google",
  "GOOGLE_PROJECT_ID": "${GOOGLE_PROJECT_ID}",
  "GOOGLE_APPLICATION_CREDENTIALS": "${GOOGLE_APPLICATION_CREDENTIALS}"
}
```

For **Ollama** (local), just omit the `env` field entirely.

### 4. Restart Your MCP Client

Restart Claude Code or Cursor to connect to Maproom.

**That's it!** Use Maproom tools for semantic code search.

---

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

---

## Provider Comparison

| Provider | Speed | Cost | Setup | Privacy |
|----------|-------|------|-------|---------|
| **OpenAI** | ⚡ Fast | 💵 ~$0.02/1M tokens | API key | ☁️ Cloud |
| **Google** | ⚡ Fast | 💵 Similar to OpenAI | GCP setup | ☁️ Cloud |
| **Ollama** | 🐌 Slow* | 💰 Free | None | 🔒 100% Local |

*Ollama is 5-10x slower without GPU. Requires 8GB+ RAM.

**Recommendation**: Use OpenAI or Google for best performance. Use Ollama only if you need 100% local processing and have good hardware.

---

## Commands

### `setup`
Initial configuration. Required before first use.

```bash
npx @crewchief/maproom-mcp setup --provider=openai
npx @crewchief/maproom-mcp setup --provider=google
npx @crewchief/maproom-mcp setup --provider=ollama
```

### `scan`
Index a repository (run after cloning or major changes).

```bash
npx @crewchief/maproom-mcp scan /path/to/repo
npx @crewchief/maproom-mcp scan .  # Current directory
```

### `watch`
Monitor repository for changes and auto-reindex.

```bash
npx @crewchief/maproom-mcp watch /path/to/repo
npx @crewchief/maproom-mcp watch --debounce=5000  # Custom debounce (ms)
```

Leave running in a terminal. Press Ctrl+C to stop.

---

## Troubleshooting

### "Setup required!" error
Run the setup command with your chosen provider:
```bash
npx @crewchief/maproom-mcp setup --provider=openai
```

### Containers not starting
1. Verify Docker is running: `docker info`
2. Check for port conflicts:
   ```bash
   lsof -i :5433  # PostgreSQL
   lsof -i :11434 # Ollama (if using)
   ```
3. Re-run setup

### Database errors
Reset everything:
```bash
docker compose -f ~/.maproom-mcp/docker-compose.yml down -v
npx @crewchief/maproom-mcp setup --provider=<your-provider>
```

### Slow indexing with Ollama
Ollama is CPU-bound without GPU. Consider:
- Using OpenAI or Google instead (much faster)
- Adding a GPU to your system
- Reducing batch size: `EMBEDDING_BATCH_SIZE=10` (slower but lower memory)

### Enable diagnostic mode
```bash
MAPROOM_MCP_DEBUG=true npx @crewchief/maproom-mcp setup
```

---

## Data Persistence

All data is stored in Docker volumes:
- `maproom-data` - PostgreSQL database (indexed code + embeddings)
- `ollama-models` - Downloaded Ollama models (if using Ollama)
- `maproom-logs` - MCP server logs

Your indexed code persists between sessions. To completely reset:
```bash
docker volume rm maproom-data ollama-models maproom-logs
```

---

## Advanced Configuration

### Custom Database
Override the default database connection:

```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"],
      "env": {
        "DATABASE_URL": "postgresql://user:pass@custom-host:5432/mydb",
        "EMBEDDING_PROVIDER": "openai",
        "OPENAI_API_KEY": "${OPENAI_API_KEY}"
      }
    }
  }
}
```

### Custom Embedding Models

**OpenAI:**
```json
"env": {
  "EMBEDDING_PROVIDER": "openai",
  "EMBEDDING_MODEL": "text-embedding-3-large",
  "EMBEDDING_DIMENSION": "3072"
}
```

**Google:**
```json
"env": {
  "EMBEDDING_PROVIDER": "google",
  "EMBEDDING_MODEL": "textembedding-gecko@003"
}
```

**Ollama:**
```json
"env": {
  "EMBEDDING_PROVIDER": "ollama",
  "EMBEDDING_MODEL": "mxbai-embed-large"
}
```

### Batch Size Tuning

Adjust embedding batch size (default: 50):
```json
"env": {
  "EMBEDDING_BATCH_SIZE": "100"
}
```

Higher = faster but more memory. Lower = slower but less memory.

---

## License

MIT - See LICENSE file for details.
