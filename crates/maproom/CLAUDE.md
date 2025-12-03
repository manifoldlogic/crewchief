# CLAUDE.md - Maproom Indexer

Working with the Rust indexer at `/crates/maproom`.

## Directory Structure

```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Library exports
├── cli/             # CLI commands
├── db/              # Database operations (SQLite)
├── embedding/       # Multi-provider embeddings (Ollama, OpenAI, Google)
├── indexer/         # File scanning and indexing
├── search/          # Search (FTS, vector, hybrid)
├── context/         # Context assembly
├── incremental/     # Incremental updates
├── cache/           # LRU caching
└── metrics/         # Prometheus metrics
```

## TypeScript Synchronization

Types in `src/daemon/types.rs` must stay in sync with TypeScript:

| Rust (this crate) | TypeScript (daemon-client) |
|-------------------|---------------------------|
| `src/daemon/types.rs::SearchParams` | `src/client.ts::SearchParams` |
| `src/daemon/types.rs::ContextParams` | `src/client.ts::ContextParams` |
| `src/context/types.rs::ContextBundle` | `src/client.ts::RustContextBundle` |

**Rust is the source of truth.** When modifying daemon RPC types:
1. Update Rust struct first
2. Update corresponding TypeScript interface in `packages/daemon-client/src/client.ts`

## Daemon Mode

`crewchief-maproom serve` runs as a long-lived daemon communicating via JSON-RPC over stdio. The daemon is spawned by the TypeScript daemon-client package (used by maproom-mcp and vscode-maproom).

```bash
# Start daemon manually (for debugging)
cargo run --bin crewchief-maproom -- serve

# Send JSON-RPC request (one per line)
{"jsonrpc":"2.0","method":"ping","id":1}
```

## Database

Maproom uses SQLite for storage. By default, the database is created at:
```
~/.maproom/maproom.db
```

Override with `MAPROOM_DATABASE_URL`:
```bash
MAPROOM_DATABASE_URL="sqlite:///tmp/maproom.db"
```

## Development

```bash
# Build
cargo build --release --bin crewchief-maproom

# Cross-platform builds (all platforms)
./scripts/build-and-package.sh

# Run commands
cargo run --bin crewchief-maproom -- db migrate
cargo run --bin crewchief-maproom -- scan --path /path/to/repo --repo myrepo --worktree main
cargo run --bin crewchief-maproom -- status --repo myrepo
cargo run --bin crewchief-maproom -- search --query "function name" --repo myrepo
cargo run --bin crewchief-maproom -- upsert --paths /path/to/file.rs --repo myrepo --worktree main --root /path/to/repo --commit HEAD

# Generate embeddings (requires embedding provider)
cargo run --bin crewchief-maproom -- generate-embeddings --repo myrepo

# Test
cargo test -p crewchief-maproom
cargo test -p crewchief-maproom -- --nocapture

# E2E validation
./scripts/test_sqlite_e2e.sh

# Code quality
cargo clippy -p crewchief-maproom
cargo fmt
```

## Cleanup Stale Worktrees

Remove worktrees that no longer exist on disk (reduces search result duplication):

```bash
# Preview what will be deleted (dry-run, default behavior)
cargo run --bin crewchief-maproom -- db cleanup-stale

# Actually delete stale worktrees
cargo run --bin crewchief-maproom -- db cleanup-stale --confirm

# Show detailed information
cargo run --bin crewchief-maproom -- db cleanup-stale --verbose
```

**Exit codes:**
- `0` - Success (cleanup completed or dry-run)
- `1` - Error (database connection failed)
- `2` - No stale worktrees found

## Quick Start

```bash
# 1. Initialize database
cargo run --bin crewchief-maproom -- db migrate

# 2. Index a repository
cargo run --bin crewchief-maproom -- scan --path /path/to/repo --repo myrepo --worktree main

# 3. Check status
cargo run --bin crewchief-maproom -- status --repo myrepo

# 4. Search (FTS - no embeddings required)
cargo run --bin crewchief-maproom -- search --query "authentication" --repo myrepo --mode fts

# 5. Generate embeddings (optional, for semantic search)
cargo run --bin crewchief-maproom -- generate-embeddings --repo myrepo

# 6. Hybrid search (FTS + vector)
cargo run --bin crewchief-maproom -- search --query "authentication" --repo myrepo --mode hybrid

# 7. Get context for a code chunk
cargo run --bin crewchief-maproom -- context --chunk-id 12345 --callers --callees --json
```

## Context Command

Retrieve contextually relevant code around a specific chunk (callers, callees, tests, imports).

### Basic Usage

```bash
# Get context for chunk ID from search results
cargo run --bin crewchief-maproom -- context --chunk-id 12345

# Output as JSON
cargo run --bin crewchief-maproom -- context --chunk-id 12345 --json
```

### Expand Options

```bash
# Include callers and callees
cargo run --bin crewchief-maproom -- context --chunk-id 12345 --callers --callees

# Include tests and documentation
cargo run --bin crewchief-maproom -- context --chunk-id 12345 --tests --docs

# Custom budget and depth
cargo run --bin crewchief-maproom -- context --chunk-id 12345 --budget 4000 --max-depth 3

# All options
cargo run --bin crewchief-maproom -- context --chunk-id 12345 \
  --callers --callees --tests --docs --config \
  --routes --hooks --jsx-parents --jsx-children \
  --budget 6000 --max-depth 2 --json
```

### Daemon Context Method

The daemon also exposes context via JSON-RPC:

```json
{
  "jsonrpc": "2.0",
  "method": "context",
  "params": {
    "chunk_id": "12345",
    "budget_tokens": 6000,
    "expand": {
      "callers": true,
      "callees": true,
      "tests": true,
      "max_depth": 2
    }
  },
  "id": 1
}
```

### Error Codes

| Code | Meaning |
|------|---------|
| -32000 | Chunk not found |
| -32001 | File not found on disk |
| -32002 | Budget exceeded |
| -32602 | Invalid parameters |

## Key Dependencies

- **tokio** - Async runtime
- **anyhow/thiserror** - Error handling
- **tracing** - Logging
- **rusqlite + sqlite-vec** - Database with vector extension
- **tree-sitter** - Parsing (TypeScript, Rust, Python, Go, JavaScript, Markdown)
- **reqwest + async-trait** - Embedding providers
- **tiktoken-rs** - Token counting

## Search Modes

1. **FTS** - SQLite FTS5 keyword matching with BM25 ranking
2. **Vector** - sqlite-vec cosine similarity (requires embeddings)
3. **Hybrid** - Reciprocal Rank Fusion (FTS + vector)

Scoring: text relevance, vector similarity, recency, symbol importance, chunk type.

## Database Schema

SQLite schema in `src/db/sqlite/schema.rs`:
- `repos`, `worktrees`, `files`, `chunks`, `chunk_edges`
- `code_embeddings` - Deduplicated embeddings by blob_sha
- `chunks_fts` - FTS5 virtual table for full-text search
- `vec_code` - sqlite-vec virtual table for vector search

## Binary Output

Built to `../../packages/cli/bin/<platform>/crewchief-maproom`:
- Platforms: darwin-arm64, darwin-x64, linux-x64, linux-arm64, win32-x64

## Environment Variables

```bash
# Database location (default: ~/.maproom/maproom.db)
MAPROOM_DATABASE_URL="sqlite:///path/to/maproom.db"

# Embedding provider: ollama, openai, or google
MAPROOM_EMBEDDING_PROVIDER=ollama
MAPROOM_EMBEDDING_MODEL=mxbai-embed-large

# Logging
RUST_LOG=info              # info, debug, trace
RUST_BACKTRACE=1

# OpenAI provider
OPENAI_API_KEY=sk-...

# Google provider
GOOGLE_PROJECT_ID=...
GOOGLE_APPLICATION_CREDENTIALS=...

# Ollama (default - no configuration needed if running locally)
OLLAMA_URL=http://localhost:11434  # optional, auto-detected
```

Config: `~/.config/crewchief/maproom.json`

## SQLite Module Structure

```
src/db/sqlite/
├── mod.rs          # SqliteStore implementation
├── schema.rs       # Schema DDL (repos, worktrees, files, chunks, edges)
├── migrations.rs   # Migration system (versioned, idempotent)
├── embeddings.rs   # Embedding storage and sync to vec_code
├── vector.rs       # Vector search via sqlite-vec
├── fts.rs          # FTS5 search with rank normalization
├── hybrid.rs       # Hybrid search (RRF) + semantic ranking
└── graph.rs        # Graph traversal (recursive CTEs)
```

## Features

- FTS5 full-text search with rank normalization
- sqlite-vec vector similarity search (1536-dim)
- Hybrid search (Reciprocal Rank Fusion)
- Semantic ranking (kind multipliers, exact match boost)
- Embedding deduplication by blob_sha
- Graph traversal (caller/callee, imports, extends)
- Graceful degradation if sqlite-vec extension missing
- WAL mode for concurrent reads

## Graph Traversal

```rust
// Find all chunks that call a function (transitive)
store.find_callers(chunk_id, Some(3)).await?;  // max depth 3

// Find all chunks called by a function
store.find_callees(chunk_id, Some(3)).await?;

// Find import relationships
store.find_imports(chunk_id, ImportDirection::Incoming, None).await?;
```

Uses recursive CTEs with cycle detection. Default depth=3, hard max=10.

## File Watching

Maproom uses git-based polling for file change detection in the incremental indexer.

### How It Works

- Polls `git status --porcelain` at configurable intervals (default: 3 seconds)
- Compares state between polls to detect changes
- Emits FileEvent (Modified, Deleted, Renamed) for downstream processing
- Automatically respects `.gitignore` patterns

### Configuration

```rust
WatcherConfig {
    poll_interval_ms: 3000,    // Polling interval in milliseconds
    include_untracked: true,   // Watch untracked files (respects .gitignore)
    detect_renames: true,      // Detect file renames via git
    git_timeout_ms: 10000,     // Timeout for git command
}
```

### Why Git Polling?

The previous `notify`-based approach caused "too many open files" (EMFILE) errors on large repositories because it created file descriptors for every watched directory. Git polling:

- Uses zero file descriptors for directory watching
- Automatically respects `.gitignore`
- Works consistently across platforms
- Trades instant detection for 2-5s latency (acceptable for dev workflows)

### Requirements

- Git must be installed and in PATH
- Must be run in a git repository
- Returns error for non-git directories

### Module Structure

```
src/incremental/
├── mod.rs           # Module exports
├── events.rs        # FileEvent and IndexingEvent types
├── watcher.rs       # FileWatcher (uses git polling internally)
├── git_state.rs     # GitState parsing and diffing
├── git_poller.rs    # GitPoller async loop
└── worktree_watcher.rs  # Multi-worktree coordination
```

## Supported Embedding Dimensions

Maproom supports multiple embedding dimensions through dimension-specific vector tables:

| Dimension | Table Name | Providers | Models |
|-----------|-----------|-----------|--------|
| 768 | `vec_code_768` | Ollama | nomic-embed-text (legacy) |
| 1024 | `vec_code_1024` | Ollama | mxbai-embed-large (default) |
| 1536 | `vec_code` | OpenAI, Google | text-embedding-3-small, textembedding-gecko |

**Configuration:**
```bash
# For 1024-dim (mxbai-embed-large, default)
# No configuration needed - this is the default!

# For 768-dim (nomic-embed-text, legacy)
export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
export MAPROOM_EMBEDDING_DIMENSION=768
```

**Adding new dimensions:**

To add support for a new embedding dimension (e.g., 512 or 2048), follow this pattern:

1. **Add migration** - Create new migration in `src/db/sqlite/migrations.rs`:
   ```rust
   Migration {
       version: N,
       name: "add_vec_code_DIMENSION",
       up: r#"
       CREATE VIRTUAL TABLE vec_code_DIMENSION USING vec0(
           embedding float[DIMENSION]
       );
       "#,
       down: "DROP TABLE IF EXISTS vec_code_DIMENSION;",
   }
   ```

2. **Update vector.rs** - Add dimension to `SUPPORTED_DIMENSIONS` and `get_vec_table_name()`:
   ```rust
   const SUPPORTED_DIMENSIONS: &[usize] = &[768, 1024, 1536, DIMENSION];

   fn get_vec_table_name(dimension: usize) -> Result<&'static str> {
       match dimension {
           DIMENSION => Ok("vec_code_DIMENSION"),
           // ... existing cases
       }
   }
   ```

3. **Update embeddings.rs** - Add case to `sync_to_vec_table()` dimension routing:
   ```rust
   match dimension {
       DIMENSION => sync_to_vec_table_impl(conn, "vec_code_DIMENSION", blob_sha, embedding)?,
       // ... existing cases
   }
   ```

**Why multiple tables?** sqlite-vec virtual tables have fixed dimensions at table creation time. Supporting multiple dimensions requires separate tables, which provides automatic dimension isolation during vector search.

## Known Limitations

- Single-user only (no multi-process concurrent writes)
- No database encryption
- sqlite-vec extension must be compiled in (statically linked)
