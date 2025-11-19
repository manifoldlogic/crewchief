# @crewchief/maproom-mcp

Semantic code search powered by PostgreSQL, pgvector, and your choice of embedding provider.

**Fast semantic search. One setup command. One line config.**

## Features

- ✨ **Choice of Providers** - OpenAI (recommended), Google Vertex AI, or local Ollama
- 🚀 **Fast Hybrid Search** - Vector similarity + full-text search with PostgreSQL
- 🔄 **Auto-Sync** - Watch mode keeps your index up-to-date automatically
- 🌿 **Automatic Branch Detection** ✨ NEW - Auto-index branches on switch (no manual scan needed)
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

#### Automatic Indexing (Recommended) ✨ NEW

Start the branch watcher to automatically index as you switch branches:

```bash
# Set database URL
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"

# Start watcher (Terminal 1)
maproom branch-watch --repo /path/to/your/repo

# Work normally (Terminal 2) - branches auto-index
git checkout feature-auth  # Automatically indexed in <1 minute
```

The watcher runs continuously and indexes branches automatically when you switch. For more details, see the [Automatic Indexing Guide](../../docs/features/automatic-indexing.md).

#### Manual Indexing

Alternatively, manually trigger indexing:

**With OpenAI:**
```bash
MAPROOM_EMBEDDING_PROVIDER=openai npx @crewchief/maproom-mcp scan /path/to/your/repo
```

**With Google Vertex AI:**
```bash
MAPROOM_EMBEDDING_PROVIDER=google npx @crewchief/maproom-mcp scan /path/to/your/repo
```

**With Ollama (local):**
```bash
MAPROOM_EMBEDDING_PROVIDER=ollama npx @crewchief/maproom-mcp scan /path/to/your/repo
```

**Optional: Auto-sync with watch mode**
```bash
MAPROOM_EMBEDDING_PROVIDER=openai npx @crewchief/maproom-mcp watch /path/to/your/repo
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
        "MAPROOM_EMBEDDING_PROVIDER": "openai",
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
        "MAPROOM_EMBEDDING_PROVIDER": "openai",
        "OPENAI_API_KEY": "${OPENAI_API_KEY}"
      }
    }
  }
}
```

For **Google Vertex AI**, use:
```json
"env": {
  "MAPROOM_EMBEDDING_PROVIDER": "google",
  "GOOGLE_PROJECT_ID": "${GOOGLE_PROJECT_ID}",
  "GOOGLE_APPLICATION_CREDENTIALS": "${GOOGLE_APPLICATION_CREDENTIALS}"
}
```

For **Ollama** (local), use:
```json
"env": {
  "MAPROOM_EMBEDDING_PROVIDER": "ollama"
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

## Database Schema

### Core Tables

**chunks table** - Code chunks with worktree tracking
- `chunk_id` - UUID primary key
- `blob_sha` - Content-addressed SHA (links to embeddings)
- `relpath` - File path relative to repository root
- `symbol_name` - Function/class/symbol name
- `content` - Source code text
- `worktree_ids` - **JSONB array** of worktree IDs containing this chunk
- `start_line`, `end_line` - Line range in file
- `created_at`, `updated_at` - Timestamps

**worktree_index_state table** - Tracks last indexed git tree SHA per worktree
- `worktree_id` - Foreign key to worktrees table
- `last_tree_sha` - Git tree SHA from `git rev-parse HEAD^{tree}`
- `last_indexed` - Timestamp of last successful scan
- `chunks_processed` - Cumulative count for monitoring
- `embeddings_generated` - Cost tracking metric

**code_embeddings table** - Cached embeddings for content deduplication
- `blob_sha` - Primary key (content-addressed)
- `embedding` - Vector embedding (pgvector type)
- `model` - Embedding model name
- `dimension` - Vector dimension

### Indexes

**GIN index on worktree_ids** - Enables efficient worktree filtering
```sql
CREATE INDEX idx_chunks_worktree_ids
ON maproom.chunks USING gin(worktree_ids);
```

Supports JSONB operators:
- `WHERE worktree_ids ? '2'` - Find chunks in worktree 2
- `WHERE worktree_ids ?| ARRAY['2', '5']` - Find chunks in any of multiple worktrees

### Branch-Aware Features

**Content deduplication**: Same code across branches shares single embedding (via blob_sha)

**Incremental updates**: Tree SHA comparison enables instant "no changes" detection (<100ms)

**Worktree filtering**: Search code from specific branch/worktree

**See also**: [Branch-Aware Indexing Architecture](/docs/architecture/branch-aware-indexing.md) for complete technical details

---

## Database Connection

The Maproom MCP server uses intelligent connection fallback to detect and connect to the PostgreSQL database automatically.

### Connection Priority

The system tries these methods in order:

1. **MAPROOM_DATABASE_URL** (explicit config) - If set, uses this connection string exactly
   ```bash
   export MAPROOM_DATABASE_URL="postgresql://user:pass@host:port/dbname"
   ```

2. **MAPROOM_DB_HOST** (component override) - If MAPROOM_DATABASE_URL not set, constructs connection from parts
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
- Try setting MAPROOM_DATABASE_URL explicitly:
  ```bash
  export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@127.0.0.1:5433/maproom"
  ```

**Custom database setup:**
If you want to use your own PostgreSQL instance instead of the bundled one:
```bash
export MAPROOM_DATABASE_URL="postgresql://myuser:mypass@myhost:5432/mydb"
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
        "MAPROOM_DATABASE_URL": "postgresql://user:pass@custom-host:5432/mydb",
        "MAPROOM_EMBEDDING_PROVIDER": "openai",
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
  "MAPROOM_EMBEDDING_PROVIDER": "openai",
  "MAPROOM_EMBEDDING_MODEL": "text-embedding-3-large",
  "EMBEDDING_DIMENSION": "3072"
}
```

**Google:**
```json
"env": {
  "MAPROOM_EMBEDDING_PROVIDER": "google",
  "MAPROOM_EMBEDDING_MODEL": "textembedding-gecko@003"
}
```

**Ollama:**
```json
"env": {
  "MAPROOM_EMBEDDING_PROVIDER": "ollama",
  "MAPROOM_EMBEDDING_MODEL": "mxbai-embed-large"
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

## Search Tool - Semantic Code Search

> **New in v2.1.0**: The `search` tool now automatically scopes results to your current git branch, eliminating result duplication and making search results more relevant to your active work.

The `search` MCP tool performs semantic code search across your indexed codebase using hybrid search (vector similarity + full-text search).

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `repo` | string | **Required.** Repository name (must match indexed name) |
| `query` | string | **Required.** Search query (concept or keywords) |
| `worktree` | string \| null \| undefined | **Optional.** Worktree scope:<br/>- `undefined` (default): Auto-detect current branch<br/>- `"branch-name"`: Search specific branch<br/>- `null`: Search all worktrees |
| `limit` | number | **Optional.** Max results (default: 10) |
| `mode` | string | **Optional.** Search mode: `"vector"`, `"fts"`, or `"hybrid"` (default) |
| `debug` | boolean | **Optional.** Include ranking details (default: false) |

### Worktree-Scoped Search (Auto-Detection)

**Default behavior (v2.1.0+)**: When `worktree` parameter is omitted, the search tool automatically detects your current git branch and scopes results to that branch only.

**Example 1: Auto-detection** (recommended)
```typescript
// In feature-auth branch, searches only feature-auth worktree
const results = await mcp__maproom__search({
  repo: "my-repo",
  query: "authentication flow"
})
// Returns: { hits: [...], worktree: "feature-auth", auto_detected: true, mode: "auto" }
```

**Example 2: Explicit worktree override**
```typescript
// In feature-auth branch, but search main worktree instead
const results = await mcp__maproom__search({
  repo: "my-repo",
  query: "authentication flow",
  worktree: "main"
})
// Returns: { hits: [...], worktree: "main", auto_detected: false, mode: "explicit" }
```

**Example 3: Search all worktrees**
```typescript
// Search across all indexed branches
const results = await mcp__maproom__search({
  repo: "my-repo",
  query: "authentication flow",
  worktree: null
})
// Returns: { hits: [...], worktree: null, mode: "all" }
```

### File Type Filtering

Filter search results by file extension to focus on specific languages or file types.

**Single extension:**
```typescript
const result = await mcp__maproom__search({
  repo: 'crewchief',
  query: 'authentication',
  filters: { file_type: 'ts' }
})
// Returns only TypeScript (.ts) files
```

**Multiple extensions:**
```typescript
const result = await mcp__maproom__search({
  repo: 'crewchief',
  query: 'authentication',
  filters: { file_type: 'ts,tsx,js' }
})
// Returns TypeScript or JavaScript files
```

**Common patterns:**
```typescript
// Search only documentation
filters: { file_type: 'md,mdx' }

// Search Rust code
filters: { file_type: 'rs' }

// Search frontend code
filters: { file_type: 'tsx,jsx,vue,svelte' }

// Combine with recency filter
filters: {
  file_type: 'ts,tsx',
  recency_threshold: '7 days'
}
// Returns recent TypeScript files only
```

**Syntax:**
- Comma-separated for multiple extensions
- Case insensitive: `"TS"` same as `"ts"`
- With or without dot: `".ts"` same as `"ts"`
- Maximum 20 extensions per filter

**Error handling:**
- Empty filter (`""`) searches all files (no error)
- Too many extensions (>20) returns error with helpful message
- Invalid input normalized or filtered out gracefully

### Fallback Behavior

When auto-detection is enabled but the current branch is not indexed, the search tool gracefully falls back to the `main` worktree with a helpful hint:

```typescript
// In unindexed feature-xyz branch
const results = await mcp__maproom__search({
  repo: "my-repo",
  query: "authentication"
})

// Returns:
{
  hits: [...],  // Results from 'main' worktree
  worktree: "main",
  mode: "fallback",
  hint: "Current branch 'feature-xyz' is not indexed.\n\n" +
        "To search your current code:\n" +
        "1. Run: mcp__maproom__scan({repo: \"my-repo\", worktree: \"feature-xyz\"})\n\n" +
        "Searching 'main' worktree instead."
}
```

If the `main` worktree is also not indexed, the tool falls back to searching all worktrees.

### Result Metadata

Search results include metadata about worktree resolution:

| Field | Type | Description |
|-------|------|-------------|
| `hits` | array | Search results with content, file paths, and scores |
| `total` | number | Total number of results returned |
| `worktree` | string \| null | Which worktree was searched |
| `auto_detected` | boolean | Was worktree auto-detected from git? |
| `mode` | string | Resolution mode: `"explicit"`, `"auto"`, `"fallback"`, or `"all"` |
| `hint` | string \| undefined | Helpful message when fallback occurs |
| `debug` | object \| undefined | Ranking details (only if `debug: true`) |

### Performance

- **Cache hit rate**: >99% for git branch detection (60s TTL)
- **Search latency**: <10ms with warm cache
- **Memory overhead**: Minimal (<100 KB for LRU caches)

### Troubleshooting

See [Troubleshooting](#troubleshooting) section for common issues.

---

## Open Tool - File Retrieval

The `open` MCP tool retrieves file contents from your indexed codebase with intelligent path resolution and security validation.

###  Multi-Candidate Fallback

When multiple worktrees exist with the same name (common after repeated indexing), the open tool automatically tries each candidate in order:

1. Queries database for all matching worktrees (ordered by most recent ID first)
2. Validates each candidate path against the filesystem
3. Returns content from the first valid worktree found

This gracefully handles database pollution from:
- Repeated indexing from different working directories
- Repository moves or renames
- Stale database entries

### Security Features

**Path Traversal Protection:**
- Validates all relative paths before filesystem access
- Rejects paths containing `../`, absolute paths, or null bytes
- Prevents access outside repository boundaries

**Symlink Validation:**
- Detects symlinks using `fs.lstat()` before reading
- Resolves symlink targets with `fs.realpath()`
- Blocks symlinks pointing outside repository boundaries
- Allows legitimate internal symlinks (e.g., shared configs)

**File Type Checking:**
- Only returns content for regular files
- Directories and special files are rejected
- Ensures `fileExists()` helper validates both readability AND file type

### Error Messages

| Error Message | Meaning | Recommended Action |
|--------------|---------|-------------------|
| `File exists in other worktrees: main, develop` | File not found in specified worktree but exists in others | Check worktree parameter spelling or use suggested worktree |
| `File 'X' not found in worktree 'Y'` | No matching database entry | Ensure repository is indexed and file path is correct |
| `File 'X' not accessible in worktree 'Y'. Tried N candidates...` | Database pollution detected - multiple entries but none valid on disk | Run `maproom db cleanup-stale` to remove stale entries |
| `Path traversal detected: ../../../etc/passwd` | Security violation in input | Use relative paths only, no parent directory references |
| `Path is outside repository boundaries` | Symlink or resolved path escapes repo | Check symlink targets or file paths |
| `Null bytes not allowed in path` | Invalid characters in path parameter | Remove null bytes from file path |

### Troubleshooting

**Issue: "Tried N candidate paths but none exist on disk"**

This indicates database pollution - the database has multiple entries for the same worktree name, but none correspond to valid paths on the filesystem.

**Diagnosis:**
```bash
# Check for duplicate worktree entries
docker exec -it maproom-postgres psql -U maproom -d maproom -c \
  "SELECT w.name, w.abs_path, COUNT(*)
   FROM maproom.worktrees w
   GROUP BY w.name, w.abs_path
   HAVING COUNT(*) > 1;"
```

**Solution:**
```bash
# Clean up stale database entries
maproom db cleanup-stale
```

**Issue: File not found but file definitely exists**

**Diagnosis:**
- Verify the repository is indexed: Check `maproom status` output
- Verify worktree name: The `worktree` parameter must match the database entry exactly
- Check file path: Path must be relative to repository root

**Issue: Symlink outside repository**

**Diagnosis:**
```bash
# Check where symlink points
readlink /path/to/symlink

# Verify it's within repo boundaries
# Should start with repository root path
```

**Solution:**
- Move symlink target inside repository, or
- Access target file directly instead of via symlink

### Path Resolution Flow

```
1. Input Validation
   ├─ Reject path traversal (../)
   ├─ Reject absolute paths (/)
   └─ Reject null bytes (\0)

2. Database Query
   └─ SELECT all matching (worktree, relpath) pairs
      ORDER BY worktree.id DESC

3. Multi-Candidate Validation
   ├─ For each candidate:
   │  ├─ Check filesystem existence
   │  ├─ Validate within repo boundaries
   │  └─ Return if valid
   └─ Error if all candidates fail

4. Security Checks
   ├─ Detect symlinks (fs.lstat)
   ├─ Validate symlink target (validateWithinRepo)
   └─ Verify file type (stats.isFile)

5. Content Retrieval
   └─ Read file with size limit validation
```

---

## Environment Variables

### Provider Configuration

- `MAPROOM_EMBEDDING_PROVIDER`: (Required) One of: `openai`, `cohere`, `google`, `ollama`, `local`
- `MAPROOM_EMBEDDING_MODEL`: (Required) Model name for the provider
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
