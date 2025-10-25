# crewchief-maproom

Rust indexer + CLI for Maproom. Stores AST-aware chunks and metadata into Postgres with pgvector.

## Setup

1. Install Postgres with `vector`, `pg_trgm`, and `unaccent` extensions.
2. Create a DB and apply migrations:

```
createdb maproom
psql maproom -f migrations/0001_init.sql
psql maproom -f scripts/analyze.sql
```

Or via CLI:

```
export DATABASE_URL=postgres://USER:PASSWORD@localhost:5432/maproom
cargo run -p crewchief-maproom -- db migrate
```

## Usage

### Indexing Code

```bash
cargo run -p crewchief-maproom -- scan \
  --repo crewchief \
  --worktree radar \
  --path /path/to/worktree \
  --commit $(git rev-parse HEAD)
```

### Generating Embeddings

After indexing, generate vector embeddings for semantic search:

```bash
# Generate embeddings for all chunks (incremental mode - only NULL embeddings)
cargo run -p crewchief-maproom -- generate-embeddings

# Test with a small sample first
cargo run -p crewchief-maproom -- generate-embeddings --sample 100

# Dry run to see what would happen without writing to database
cargo run -p crewchief-maproom -- generate-embeddings --dry-run --sample 100

# Force regeneration of all embeddings
cargo run -p crewchief-maproom -- generate-embeddings --force

# Limit cost to prevent overspending
cargo run -p crewchief-maproom -- generate-embeddings --max-cost 5.0

# Custom batch size and delay
cargo run -p crewchief-maproom -- generate-embeddings --batch-size 50 --batch-delay 200
```

**Options:**
- `--incremental` - Only process chunks with NULL embeddings (default: true)
- `--batch-size` - Number of chunks to process per batch (default: 100)
- `--dry-run` - Don't write embeddings to database (default: false)
- `--sample` - Process only N chunks for testing
- `--batch-delay` - Milliseconds to wait between batches (default: 100)
- `--max-cost` - Maximum cost in USD before stopping
- `--force` - Regenerate all embeddings (overrides --incremental)

### Searching

```bash
# Full-text search
cargo run -p crewchief-maproom -- search \
  --repo crewchief \
  --worktree radar \
  --query "authentication" \
  --k 10
```

## Environment Variables

Required:
- `DATABASE_URL` - PostgreSQL connection string
- `OPENAI_API_KEY` - OpenAI API key for embedding generation

Optional:
- `EMBEDDING_PROVIDER` - Provider to use (default: openai)
- `EMBEDDING_MODEL` - Model name (default: text-embedding-3-small)
- `EMBEDDING_DIMENSION` - Embedding dimension (default: 1536)
- `EMBEDDING_CACHE_SIZE` - LRU cache size (default: 10000)
- `EMBEDDING_CACHE_TTL` - Cache TTL in seconds (default: 3600)
- `EMBEDDING_BATCH_SIZE` - API batch size (default: 100)

Create a `.env` at the repo root or in `crates/maproom/` by copying `.env.example`:

```bash
cp crates/maproom/.env.example .env
# or
cp crates/maproom/.env.example crates/maproom/.env
```

Then edit the configuration:

```
DATABASE_URL=postgres://<your_username>:<your_password>@localhost:5432/maproom
OPENAI_API_KEY=sk-...
```

## Testing

### Running Tests

Maproom includes comprehensive test suites for all major components.

**Unit Tests:**
```bash
# Run all tests
cargo test

# Run tests for a specific module
cargo test --lib incremental

# Run with output
cargo test -- --nocapture
```

**Integration Tests:**
```bash
# Run all integration tests
cargo test --test '*'

# Run specific integration test suite
cargo test --test incremental_scenarios
cargo test --test concurrent_updates
cargo test --test batch_processing
cargo test --test failure_recovery

# Run integration tests in the integration directory
cargo test --test integration
```

**Extended Performance Tests:**

Some tests are marked with `#[ignore]` to avoid running during regular test execution. These include large-scale batch tests (5000+ files):

```bash
# Run ignored tests (performance/stress tests)
cargo test -- --ignored

# Run ALL tests including ignored ones
cargo test -- --include-ignored
```

### Test Requirements

Integration tests require:
- PostgreSQL running locally (default: `localhost:5432`)
- Database credentials configured in `DATABASE_URL` environment variable
- Sufficient disk space for temporary test repositories

The tests will automatically:
- Create temporary test databases with unique names
- Clean up test data after execution
- Use temporary directories for file system operations

### Test Coverage by Component

**Incremental Indexing Tests:**
- `incremental_scenarios.rs` - File operations (create, modify, delete, rename)
- `concurrent_updates.rs` - Concurrent operations and race conditions
- `batch_processing.rs` - Large batch operations (1000+ files)
- `failure_recovery.rs` - Error handling and recovery scenarios

**Search and MCP Tests:**
- `mcp_integration_test.rs` - MCP tool interface validation
- `search_*_test.rs` - Search pipeline and fusion tests

**Production Readiness:**
- `monitoring_test.rs` - Metrics and monitoring
- `production_readiness_test.rs` - Production deployment validation
- `config_management_test.rs` - Configuration handling

### Continuous Integration

Tests are designed to run in CI environments:
- All tests use unique database names to avoid conflicts
- Temporary directories are automatically cleaned up
- Tests have appropriate timeouts for deadlock detection
- Database schema is created automatically per test

### Performance Benchmarks

Performance benchmarks are tracked in batch processing tests:
- Throughput: Expected >= 10 files/sec for standard operations
- Latency: Average time per file tracked and reported
- Memory: Tests verify system handles large batches without OOM

Run benchmarks separately:
```bash
cargo bench
```
