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
```

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
MAPROOM_EMBEDDING_MODEL=nomic-embed-text

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

## Known Limitations

- 1536-dim embeddings only (OpenAI/Vertex compatible)
- 768-dim (Ollama nomic-embed-text) requires config change (deferred)
- Single-user only (no multi-process concurrent writes)
- No database encryption
- sqlite-vec extension must be compiled in (statically linked)
