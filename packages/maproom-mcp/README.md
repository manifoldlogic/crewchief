# @crewchief/maproom-mcp

Semantic code search powered by PostgreSQL, pgvector, and your choice of embedding provider.

**Fast semantic search. One setup command. One line config.**

## Features

- ✨ **Choice of Providers** - OpenAI (recommended), Google Vertex AI, or local Ollama
- 🚀 **Fast Hybrid Search** - Vector similarity + full-text search with PostgreSQL
- 🔄 **Auto-Sync** - Watch mode keeps your index up-to-date automatically
- 📦 **Fully Containerized** - Everything runs in Docker, isolated and clean
- 🌳 **Multi-Language** - Tree-sitter parsing for TypeScript, JavaScript, Rust, and more
- 🔒 **Privacy Options** - Use local Ollama for 100% private embeddings (no API keys)

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

**With OpenAI:**
```bash
EMBEDDING_PROVIDER=openai npx @crewchief/maproom-mcp scan /path/to/your/repo
```

**With Google Vertex AI:**
```bash
EMBEDDING_PROVIDER=google npx @crewchief/maproom-mcp scan /path/to/your/repo
```

**With Ollama (local):**
```bash
EMBEDDING_PROVIDER=ollama npx @crewchief/maproom-mcp scan /path/to/your/repo
```

**Optional: Auto-sync with watch mode**
```bash
EMBEDDING_PROVIDER=openai npx @crewchief/maproom-mcp watch /path/to/your/repo
```

This keeps your index up-to-date as you edit code. Leave it running in a terminal.

### 3. Add to MCP Configuration

**Claude Code** (`.claude/mcp.json` in your project):
```json
{
  "mcpServers": {
    "maproom": {
      "command": "docker",
      "args": [
        "exec",
        "-i",
        "maproom-mcp",
        "node",
        "/app/dist/index.js"
      ],
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
      "command": "docker",
      "args": [
        "exec",
        "-i",
        "maproom-mcp",
        "node",
        "/app/dist/index.js"
      ],
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

For **Ollama** (local), use:
```json
"env": {
  "EMBEDDING_PROVIDER": "ollama"
}
```

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

## Progress Indicators

The `scan` command now shows real-time progress during indexing, making it easy to track what's happening without slowing down performance.

### Scan Command Progress

When you run `scan`, you'll see:

```text
🔍 Scanning worktree: main @ abc12345
   Repository: my-repo
   Path: /path/to/repo

Processing: 45/100 files (45%)
✅ Completed in 8.3s

📊 Scan Summary:
   Files processed: 100
   Total chunks: 847
   Total size: 2.14 MB
```

**Features:**
- Real-time progress updates (throttled to every 200-500ms to avoid console flooding)
- File and chunk counts as indexing progresses
- Completion timing prominently displayed
- Works in both TTY (interactive terminal) and non-TTY (CI/logging) environments

**Default Directory Behavior:**
You don't need to specify `.` for the current directory - it's the default:

```bash
# These are equivalent:
npx @crewchief/maproom-mcp scan
npx @crewchief/maproom-mcp scan .
npx @crewchief/maproom-mcp scan /path/to/repo  # Or specify a path
```

### Verbose Mode

For more detailed output during debugging:

```bash
npx @crewchief/maproom-mcp scan --verbose
```

Currently shows the same output as default mode, but reserved for future detailed diagnostics.

### Performance

Progress tracking adds minimal overhead (<5%) through:
- Atomic counters for thread-safe updates
- Smart throttling (200ms minimum between updates)
- Efficient TTY detection

---

## Troubleshooting

### "Connection refused" errors to localhost:11434

**Problem:** OpenAI or Cohere provider attempting to connect to local Ollama endpoint.

**Solution:** This was a bug in earlier versions (< 1.2.0). Update to the latest version where provider-aware endpoint validation prevents this issue:

```bash
npx @crewchief/maproom-mcp@latest setup --provider=openai
```

The fix ensures cloud providers only use their official endpoints, preventing cross-provider endpoint pollution.

### Custom endpoint not used

**Problem:** Set `EMBEDDING_API_ENDPOINT` but provider uses default.

**Solution:** Ensure the endpoint domain matches your provider:
- **OpenAI**: Must contain "openai.com"
- **Cohere**: Must contain "cohere"
- **Ollama/Local**: Any endpoint accepted
- **Google**: Ignores `EMBEDDING_API_ENDPOINT` (uses region-based endpoint)

Example of correct custom endpoint:
```bash
# ✅ Correct: OpenAI custom endpoint (contains "openai.com")
export EMBEDDING_API_ENDPOINT=https://api.openai.com/v1/embeddings

# ❌ Wrong: Ollama endpoint for OpenAI provider (ignored)
export EMBEDDING_API_ENDPOINT=http://localhost:11434
```

### Database "column updated_at does not exist" errors

**Problem:** Missing column in database schema.

**Solution:** Run database migrations. The maproom binary automatically applies migrations on startup:

```bash
npx @crewchief/maproom-mcp setup --provider=<your-provider>
```

Or manually apply migrations by restarting containers:
```bash
docker compose -f ~/.maproom-mcp/docker-compose.yml restart
```

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

## Database Connection

The Maproom MCP server uses intelligent connection fallback to detect and connect to the PostgreSQL database automatically.

### Connection Priority

The system tries these methods in order:

1. **DATABASE_URL** (explicit config) - If set, uses this connection string exactly
   ```bash
   export DATABASE_URL="postgresql://user:pass@host:port/dbname"
   ```

2. **MAPROOM_DB_HOST** (component override) - If DATABASE_URL not set, constructs connection from parts
   ```bash
   export MAPROOM_DB_HOST="custom-host"
   export MAPROOM_DB_PORT="5432"  # optional, defaults to 5432
   ```

3. **maproom-postgres** (auto-detection) - Attempts to connect to maproom-postgres hostname
   - Works automatically in Docker environments
   - No configuration needed if maproom-postgres container is running (default)

4. **localhost:5433** (fallback) - Development fallback for local testing
   - Useful for local postgres instances on non-standard port

### Troubleshooting Connection Issues

**Can't connect to database:**
1. Verify maproom-postgres is running:
   ```bash
   docker ps | grep maproom-postgres
   ```

2. Start if needed:
   ```bash
   docker compose -f ~/.maproom-mcp/docker-compose.yml up -d
   ```

3. Check logs:
   ```bash
   docker logs maproom-postgres
   ```

**Connection refused:**
- Verify port 5432 (internal) or 5433 (host) is not blocked
- Check network connectivity:
  ```bash
  docker network inspect maproom-network
  ```

**Hostname not found:**
- Verify you're in correct Docker network
- Try setting DATABASE_URL explicitly:
  ```bash
  export DATABASE_URL="postgresql://maproom:maproom@127.0.0.1:5433/maproom"
  ```

**Custom database setup:**
If you want to use your own PostgreSQL instance instead of the bundled one:
```bash
export DATABASE_URL="postgresql://myuser:mypass@myhost:5432/mydb"
npx @crewchief/maproom-mcp scan /path/to/code
```

---

## Advanced Configuration

### Custom Database
Override the default database connection:

```json
{
  "mcpServers": {
    "maproom": {
      "command": "docker",
      "args": [
        "exec",
        "-i",
        "maproom-mcp",
        "node",
        "/app/dist/index.js"
      ],
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

## Environment Variables

### Provider Configuration

- `EMBEDDING_PROVIDER`: (Required) One of: `openai`, `cohere`, `google`, `ollama`, `local`
- `EMBEDDING_MODEL`: (Required) Model name for the provider
- `EMBEDDING_DIMENSION`: (Required) Vector dimension for embeddings
- `EMBEDDING_API_ENDPOINT`: (Optional) Custom endpoint override

### Endpoint Configuration

**Cloud Providers (OpenAI, Cohere):**
- Use official endpoints by default (https://api.openai.com/v1/embeddings, etc.)
- `EMBEDDING_API_ENDPOINT` only used if domain matches provider
- Example: Setting `EMBEDDING_API_ENDPOINT=http://localhost:11434` for OpenAI is ignored

**Ollama:**
- Defaults to `http://localhost:11434/api/embed`
- Set `EMBEDDING_API_ENDPOINT` for custom Ollama server location

**Google Vertex AI:**
- Endpoint constructed from `GOOGLE_VERTEX_REGION` (e.g., `us-west1`)
- `EMBEDDING_API_ENDPOINT` is ignored

**Local Provider:**
- Requires `EMBEDDING_API_ENDPOINT` to be set explicitly

### Environment Variable Precedence

1. Explicit configuration in code (if applicable)
2. `EMBEDDING_API_ENDPOINT` environment variable (validated by provider)
3. Provider-specific default endpoint

### API Keys

- `OPENAI_API_KEY`: For OpenAI provider
- `COHERE_API_KEY`: For Cohere provider
- `GOOGLE_APPLICATION_CREDENTIALS`: For Google Vertex AI

---

## License

MIT - See LICENSE file for details.
