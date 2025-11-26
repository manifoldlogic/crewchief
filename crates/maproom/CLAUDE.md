# CLAUDE.md - Maproom Indexer

Working with the Rust indexer at `/crates/maproom`.

## Directory Structure

```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Library exports
├── cli/             # CLI commands
├── db/              # Database operations
├── embedding/       # Multi-provider embeddings (Ollama, OpenAI, Google)
├── indexer/         # File scanning and indexing
├── search/          # Search (FTS, vector, hybrid)
├── context/         # Context assembly
├── incremental/     # Incremental updates
├── cache/           # LRU caching
└── metrics/         # Prometheus metrics
```

## Development

```bash
# Build
cargo build --release --bin crewchief-maproom

# Cross-platform builds (all platforms)
./scripts/build-and-package.sh

# Run commands
cargo run --bin crewchief-maproom -- db
cargo run --bin crewchief-maproom -- scan /path/to/repo
cargo run --bin crewchief-maproom -- search "query"
cargo run --bin crewchief-maproom -- context <chunk-id>

# Watch command (unified file and branch watching)
cargo run --bin crewchief-maproom -- watch
# Auto-detects current branch, watches for file changes and branch switches
# Emits NDJSON events to stdout (including branch_switched events)

# Test
cargo test
cargo test -- --nocapture

# Benchmark
cargo bench

# Code quality
cargo clippy
cargo fmt
```

## Key Dependencies

- **tokio** - Async runtime
- **anyhow/thiserror** - Error handling
- **tracing** - Logging
- **tokio-postgres + pgvector** - Database
- **tree-sitter** - Parsing (TypeScript, Rust, Python, Go, JavaScript, Markdown)
- **reqwest + async-trait** - Embedding providers
- **rayon** - Parallelism
- **tiktoken-rs** - Token counting

## Search Modes

1. **FTS** - PostgreSQL tsvector keyword matching
2. **Vector** - pgvector cosine similarity
3. **Hybrid** - Reciprocal Rank Fusion (FTS + vector)

Scoring: text relevance, vector similarity, recency, symbol importance, chunk type.

## Database Schema

See `migrations/`:
- `repos`, `worktrees`, `files`, `chunks`, `chunk_relationships`
- Vector indexes (ivfflat/hnsw)
- GIN indexes for FTS

## Migrations

Migrations 0000-0017: Original maproom schema
Migrations 0018-0020: BLOBSHA/BRANCHX integration (added SCHMAFIX project)

**Migration 0018** (add_blob_sha): Adds blob_sha TEXT column to chunks for content-addressed storage
**Migration 0019** (create_code_embeddings): Creates deduplicated embeddings table with HNSW index
**Migration 0020** (add_worktree_tracking): Adds worktree_ids JSONB column and worktree_index_state table

**Adding New Migrations**:
1. Create SQL file in `crates/maproom/migrations/NNNN_description.sql`
2. Update `src/db/queries.rs` migrations array with `include_str!`
3. Use `IF NOT EXISTS` for idempotency
4. Set `concurrent = false` for transaction safety (default)
5. Write integration tests in `tests/migration_integration.rs`

See `migrations/CLAUDE.md` for detailed migration guidelines.

## Binary Output

Built to `../../packages/cli/bin/<platform>/crewchief-maproom`:
- Platforms: darwin-arm64, darwin-x64, linux-x64, linux-arm64, win32-x64
- OpenSSL vendored for portability

## Environment Variables

```bash
MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5432/maproom
MAPROOM_EMBEDDING_PROVIDER=ollama  # ollama, openai, or google
MAPROOM_EMBEDDING_MODEL=nomic-embed-text
RUST_LOG=info              # info, debug, trace
RUST_BACKTRACE=1
OPENAI_API_KEY=sk-...      # If using OpenAI provider
GOOGLE_PROJECT_ID=...      # If using Google provider
GOOGLE_APPLICATION_CREDENTIALS=... # If using Google provider
```

Config: `~/.config/crewchief/maproom.json`

## Watch Command (Unified)

The `watch` command provides unified file and branch watching:

```bash
maproom watch
```

**Features:**
- Auto-detects the current branch
- Watches for file changes (incremental indexing)
- Detects branch switches and automatically re-indexes
- Emits NDJSON events to stdout for integration with tools

**NDJSON Events:**
- `branch_switched`: Emitted when a branch switch is detected
  - Includes: old/new branch names, old/new worktree IDs, timestamp
  - Used by VSCode extension to update UI and refresh context

**Migration from separate commands:**

Before (required two separate commands):
```bash
maproom watch --repo myproject --worktree main
# Plus separately running branch-watch in another terminal
```

After (single unified command):
```bash
maproom watch
# Handles both file watching and branch detection automatically
```

**Note:** The `--worktree` flag is deprecated but still supported with a warning. Branch auto-detection is now automatic.

## SQLite Backend

The SQLite backend provides zero-config semantic search without PostgreSQL. Enable with `--features sqlite`.

### Features
- FTS5 full-text search with rank normalization
- sqlite-vec vector similarity search (1536-dim)
- Hybrid search (Reciprocal Rank Fusion)
- Semantic ranking (kind multipliers, exact match boost)
- Embedding deduplication by blob_sha
- Graph traversal (caller/callee, imports, extends)
- Graceful degradation if sqlite-vec extension missing
- WAL mode for concurrent reads

### Development

```bash
# Build with SQLite
cargo build --features sqlite

# Test SQLite backend
cargo test --features sqlite --lib db::sqlite
cargo test --features sqlite --test sqlite_integration

# All SQLite tests (98 unit + 14 integration)
cargo test --features sqlite db::sqlite
cargo test --features sqlite --test sqlite_integration --test sqlite_store
```

### SQLite Module Structure

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

### Search Pipeline

1. **FTS5 Search**: Keyword matching with BM25 ranking
2. **Vector Search**: Cosine similarity via sqlite-vec (if available)
3. **Hybrid Fusion**: RRF combines FTS + vector ranks
4. **Semantic Ranking**: Kind multipliers (function=1.2, variable=0.8), exact match boost

### Graph Traversal

```rust
// Find all chunks that call a function (transitive)
store.find_callers(chunk_id, Some(3)).await?;  // max depth 3

// Find all chunks called by a function
store.find_callees(chunk_id, Some(3)).await?;

// Find import relationships
store.find_imports(chunk_id, ImportDirection::Incoming, None).await?;
```

Uses recursive CTEs with cycle detection. Default depth=3, hard max=10.

### Known Limitations
- 1536-dim embeddings only (OpenAI/Vertex compatible)
- 768-dim (Ollama nomic-embed-text) requires config change (deferred)
- Single-user only (no multi-process concurrent writes)
- No database encryption
- sqlite-vec extension must be compiled in (statically linked)
