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

## Binary Output

Built to `../../packages/cli/bin/<platform>/crewchief-maproom`:
- Platforms: darwin-arm64, darwin-x64, linux-x64, linux-arm64, win32-x64
- OpenSSL vendored for portability

## Environment Variables

```bash
DATABASE_URL=postgresql://maproom:maproom@localhost:5432/maproom
RUST_LOG=info              # info, debug, trace
RUST_BACKTRACE=1
OPENAI_API_KEY=sk-...      # If using OpenAI
```

Config: `~/.config/crewchief/maproom.json`
