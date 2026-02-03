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

## Clean Ignored Chunks

Delete indexed chunks matching patterns in `.maproomignore`:

```bash
# Preview what will be deleted (dry-run mode)
cargo run --bin crewchief-maproom -- clean-ignored --repo myrepo --worktree main --dry-run

# Actually delete matching chunks
cargo run --bin crewchief-maproom -- clean-ignored --repo myrepo --worktree main
```

**Use cases:**
- After adding new patterns to `.maproomignore`, clean up already-indexed chunks
- Remove noise from search results without a full rescan
- Faster than rescanning (surgical removal vs full re-indexing)

**Example workflow:**
```bash
# 1. Add patterns to .maproomignore
echo "test/**" >> .maproomignore
echo "*.log" >> .maproomignore

# 2. Preview what will be deleted
cargo run --bin crewchief-maproom -- clean-ignored --repo myrepo --worktree main --dry-run

# 3. Delete matching chunks
cargo run --bin crewchief-maproom -- clean-ignored --repo myrepo --worktree main

# 4. Verify with search (previously indexed test files should be gone)
cargo run --bin crewchief-maproom -- search --query "test" --repo myrepo
```

**Exit codes:**
- `0` - Success
- `1` - Error (repo/worktree not found, database error, or invalid patterns)

## Ignore Patterns

Maproom supports custom ignore patterns via `.maproomignore` files to exclude files from indexing without modifying `.gitignore`. This allows you to separate indexing concerns from version control, excluding test fixtures, build artifacts, or other files that should remain in git but don't need to be indexed for code search.

### Usage

Create a `.maproomignore` file in your repository root (same location as `.git/`). Patterns are loaded once when scanning or watching starts.

**Important**: `.maproomignore` must be at the repository root. Subdirectory `.maproomignore` files are not supported.

### Pattern Syntax

Patterns use gitignore-style glob syntax:

- **Glob patterns**: `*.tmp`, `test/**`, `build/`
- **Comments**: Lines starting with `#` are ignored
- **Blank lines**: Ignored
- **Relative paths**: All patterns are relative to the repository root
- **Leading `/`**: Matches only at repository root (e.g., `/README.md` matches root README only)
- **Trailing `/`**: Matches directories only (e.g., `build/` matches `build/` but not `build.txt`)
- **Wildcards**: `*` matches any sequence except `/`, `**` matches any sequence including `/`

**Examples:**
```
test/**           # Exclude entire test directory
*.tmp             # Exclude all .tmp files anywhere
build/            # Exclude build directory (trailing slash = directories only)
/specific.txt     # Exclude specific.txt at repository root only
data/**/large.*   # Exclude files named large.* in any subdirectory of data/
```

### Example .maproomignore File

```
# Example .maproomignore
# Exclude test fixtures from indexing
test-fixtures/**
tests/data/**

# Exclude build artifacts
build/
dist/
target/

# Exclude temporary files
*.tmp
*.bak
*.swp

# Exclude large data files
*.sql
*.csv
data/**

# Exclude log files
*.log
logs/**

# Exclude generated documentation
docs/api/generated/**
```

### Pattern Precedence

Maproom uses **additive** ignore patterns - a file is excluded if it matches **any** of the following:

1. **`.maproomignore` patterns** - Custom patterns for indexing exclusions
2. **`.gitignore` patterns** - Version control ignore patterns (automatically respected)
3. **Default patterns** - Built-in exclusions (e.g., `.git/`, `node_modules/`)

This is an **OR** relationship, not a hierarchy. All three pattern sources are checked independently, and a file matching any source is excluded.

**Important**: There is no pattern negation or allowlist mechanism. Once a file matches any ignore pattern, it cannot be re-included.

### Integration Details

**Scan operation** (`scan` command):
- Patterns loaded via `load_ignore_patterns()` at scan start
- Applied to `WalkBuilder` via `OverrideBuilder`
- Files matching patterns are skipped during directory traversal
- Pattern loading errors fail scan startup (fail-fast behavior)

**Watch operation** (incremental indexer):
- Patterns loaded once at watcher startup
- File events filtered via `should_ignore()` in `event_conversion_task()`
- Ignored files generate no indexing events
- Pattern loading errors fail watcher startup (fail-fast behavior)

**Code references:**
- Pattern loading: `src/indexer/ignore.rs::load_ignore_patterns()`
- Scan integration: `src/indexer/scanner.rs` (WalkBuilder configuration)
- Watch integration: `src/incremental/watcher.rs::event_conversion_task()`
- Ignore checking: `src/incremental/watcher.rs::should_ignore()`

### Limitations

**Repository root only**: `.maproomignore` files in subdirectories are ignored. Only the root-level file is respected. This simplifies pattern resolution and avoids precedence complexity.

**No hot-reload**: Changes to `.maproomignore` require restarting the watcher. Pattern loading happens once at startup. For scan operations, run a new scan after modifying patterns.

**To apply new patterns to already-indexed files**, use the `clean-ignored` command (see "Clean Ignored Chunks" section above) for surgical removal, or re-run `scan` for a full re-index.

**Fail-fast validation**: Invalid glob patterns (e.g., unclosed brackets `[abc`) cause scan or watch startup to fail immediately with a clear error message. There is no graceful degradation or pattern skipping.

**Pattern format**: Only gitignore-style globs are supported. Regular expressions, literal path matching, or other pattern syntaxes are not supported.

**No negation**: Unlike `.gitignore`, there is no `!pattern` negation syntax. All patterns are exclusions only.

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

## Embedding Dimension Configuration

Maproom automatically infers embedding dimensions for known Ollama models:
- `mxbai-embed-large*`: 1024 dimensions (default, matches tags like `:latest`)
- `nomic-embed-text*`: 768 dimensions (matches tags like `:latest`)

To override automatic inference or configure custom models:
```bash
export MAPROOM_EMBEDDING_DIMENSION=512
```

Explicit configuration always takes precedence over inference.

## After Upgrading to Dimension Inference

If you previously experienced dimension mismatch errors:
1. The fix is automatic - no configuration changes needed
2. Existing embeddings are dimension-tagged and remain valid
3. New embeddings will use correct inferred dimensions
4. No regeneration required

Zero-config workflows now work correctly:
```bash
# No environment variables needed for Ollama with standard models
crewchief-maproom generate-embeddings --repo myrepo
# Automatically uses mxbai-embed-large at 1024 dimensions
```

## Google Vertex AI Provider - Parallel Configuration

The Google provider supports parallel batch processing for improved throughput:

**Default Configuration:**
- `sub_batch_size`: 200 (near API limit of 250)
- `max_concurrency`: 16 (cloud API is I/O-bound)
- `enabled`: true

**Environment Variables:**
- `MAPROOM_EMBEDDING_PARALLEL_ENABLED`: Enable/disable (default: true)
- `MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE`: Texts per sub-batch (default: 200)
- `MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY`: Concurrent requests (default: 16)

**Performance:**
- 1000 texts: ~5-8x faster than sequential
- 10,000 texts: ~10-12x faster than sequential
- Throughput limited by API quotas (5M tokens/min)

**Usage:**
See module documentation in `src/embedding/google.rs` for examples.

## Known Limitations

- Single-user only (no multi-process concurrent writes)
- No database encryption
- sqlite-vec extension must be compiled in (statically linked)
