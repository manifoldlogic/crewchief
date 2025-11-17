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
