# CrewChief Maproom

Semantic code search powered by embeddings and PostgreSQL.

## Features

- **🔍 Semantic Code Search** - Find code by concept, not just keywords
- **🎯 Multi-Provider Embeddings** - Choose Ollama (free), OpenAI, or Google Vertex AI
- **⚡ Zero-Config Setup** - Auto-detects Ollama, works out of the box
- **🗃️ PostgreSQL Storage** - Reliable vector storage with pgvector
- **🔄 Incremental Indexing** - Fast updates for changed files
- **🌐 MCP Integration** - Works with Claude, Cursor, and other AI tools

## Quick Start (Zero Config)

### 1. Install Ollama (Free, Local)
```bash
curl -sSL https://ollama.ai/install.sh | sh
ollama pull nomic-embed-text
```

### 2. Index Your Repository
```bash
crewchief maproom scan --generate-embeddings
```

### 3. Search Your Code
```bash
crewchief maproom search "authentication middleware"
```

**That's it!** No API keys, no configuration, no costs.

---

## Embedding Providers

Maproom supports three embedding providers:

| Provider | Cost | Setup | Dimensions | Best For |
|----------|------|-------|------------|----------|
| **Ollama** | Free | Easy | 768 | Local dev, privacy, cost |
| **OpenAI** | ~$0.0001/1K | Easy | 1536 | Proven quality |
| **Google Vertex AI** | ~$0.00025/1K | Medium | 768 | Enterprise, compliance |

See [Provider Comparison](docs/providers/comparison.md) for detailed breakdown.

### Ollama (Recommended for Most Users)

**Advantages:**
- ✅ Completely free
- ✅ Works offline
- ✅ Zero configuration
- ✅ Fast (local processing)
- ✅ Complete privacy (data never leaves your machine)

**Setup:** See [Quick Start](#quick-start-zero-config) above

### OpenAI

**Advantages:**
- ✅ Proven embedding quality
- ✅ Simple API setup
- ✅ Reliable cloud service

**Setup:**
```bash
export OPENAI_API_KEY="sk-proj-..."
export EMBEDDING_PROVIDER=openai
crewchief maproom scan --generate-embeddings
```

See [OpenAI Setup Guide](docs/providers/openai-setup.md)

### Google Vertex AI

**Advantages:**
- ✅ Enterprise compliance (HIPAA, SOC2)
- ✅ Regional data residency
- ✅ GCP integration

**Setup:**
```bash
export GOOGLE_PROJECT_ID="your-project"
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/key.json"
export EMBEDDING_PROVIDER=google
crewchief maproom scan --generate-embeddings
```

See [Google Setup Guide](docs/providers/google-vertex-ai-setup.md)

---

## Configuration

Maproom auto-detects your provider:

1. Checks `EMBEDDING_PROVIDER` env var (explicit)
2. Detects Ollama on localhost:11434
3. Falls back to OpenAI if `OPENAI_API_KEY` present
4. Falls back to Google if `GOOGLE_PROJECT_ID` present

### Explicit Provider Selection
```bash
export EMBEDDING_PROVIDER=ollama  # or openai, google
```

### Mixed Embeddings

You can use multiple providers simultaneously! The database stores 768-dim and 1536-dim embeddings in separate columns. Search automatically uses COALESCE to prefer 768-dim embeddings when both exist.

**Migration:** See [Migration Guide](docs/guides/provider-migration.md)

---

## FAQ

### What embedding dimensions does Maproom use?

- **Ollama**: 768 dimensions (nomic-embed-text model)
- **Google**: 768 dimensions (textembedding-gecko model)
- **OpenAI**: 1536 dimensions (text-embedding-3-small model)

### Can I switch providers without re-indexing?

Yes! Existing embeddings are preserved. New embeddings go in separate columns. See [Migration Guide](docs/guides/provider-migration.md).

### Which provider should I use?

- **Start with Ollama** - Free, fast, and private
- **Use OpenAI** - If you're already an OpenAI customer
- **Use Google** - If you need compliance certifications or GCP integration

See [Provider Comparison](docs/providers/comparison.md) for detailed guidance.

### Do I need a GPU for Ollama?

No, but it helps. Ollama works on CPU (slower) or GPU (faster).

### How much does it cost?

- **Ollama**: $0 (free)
- **OpenAI**: ~$5 per 100K chunks
- **Google**: ~$12.50 per 100K chunks

### Can I use this offline?

Yes with Ollama! It runs entirely locally with no internet required.

---

## For Existing Users

**⚠️ Notice for existing Maproom users:**

If you already have OpenAI embeddings:
- Your existing embeddings are **preserved**
- New embeddings use separate columns
- Search works across both embedding types
- No re-indexing required

See [Migration Guide](docs/guides/provider-migration.md) for details.

---

## Documentation

- [Provider Comparison](docs/providers/comparison.md)
- [Ollama Setup](docs/providers/ollama-setup.md)
- [OpenAI Setup](docs/providers/openai-setup.md)
- [Google Vertex AI Setup](docs/providers/google-vertex-ai-setup.md)
- [Migration Guide](docs/guides/provider-migration.md)
- [MCP Integration](docs/mcp/README.md)

---

## Installation & Setup

### Database Setup

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

### Required
- `DATABASE_URL` - PostgreSQL connection string

### Provider-Specific (pick one)

**Ollama (default):**
- No environment variables required! Just install and run Ollama.

**OpenAI:**
- `OPENAI_API_KEY` - OpenAI API key for embedding generation
- `EMBEDDING_PROVIDER=openai` (optional if API key present)

**Google Vertex AI:**
- `GOOGLE_PROJECT_ID` - Your GCP project ID
- `GOOGLE_APPLICATION_CREDENTIALS` - Path to service account JSON key
- `EMBEDDING_PROVIDER=google` (optional if project ID present)

### Optional Configuration
- `EMBEDDING_PROVIDER` - Explicit provider selection: `ollama`, `openai`, or `google`
- `EMBEDDING_MODEL` - Model name override
- `EMBEDDING_DIMENSION` - Embedding dimension override
- `EMBEDDING_CACHE_SIZE` - LRU cache size (default: 10000)
- `EMBEDDING_CACHE_TTL` - Cache TTL in seconds (default: 3600)
- `EMBEDDING_BATCH_SIZE` - API batch size (default: 100)

### Configuration File

Create a `.env` at the repo root or in `crates/maproom/` by copying `.env.example`:

```bash
cp crates/maproom/.env.example .env
# or
cp crates/maproom/.env.example crates/maproom/.env
```

**Example configurations:**

```bash
# Ollama (default - no configuration needed!)
DATABASE_URL=postgres://postgres:postgres@localhost:5432/maproom

# OpenAI
DATABASE_URL=postgres://postgres:postgres@localhost:5432/maproom
OPENAI_API_KEY=sk-proj-...
EMBEDDING_PROVIDER=openai

# Google Vertex AI
DATABASE_URL=postgres://postgres:postgres@localhost:5432/maproom
GOOGLE_PROJECT_ID=your-project-id
GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json
EMBEDDING_PROVIDER=google
```

### Database Configuration

The `DATABASE_URL` environment variable must point to a running PostgreSQL instance with the required extensions installed.

**Format:**
```
DATABASE_URL=postgresql://username:password@hostname:port/database
```

**Local Development (PostgreSQL on host machine):**
```bash
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/crewchief"
```

**Docker/Devcontainer (PostgreSQL in separate container):**

When running in Docker or a devcontainer, PostgreSQL typically runs in a separate container with hostname `postgres`:
```bash
export DATABASE_URL="postgresql://postgres:postgres@postgres:5432/crewchief"
```

**Common Configuration Issues:**

1. **Connection Refused Error**: If you see "Connection refused (os error 111)", the database is unreachable. Common causes:
   - Wrong hostname: Use `postgres` instead of `localhost` in Docker/devcontainer environments
   - PostgreSQL not running: Verify with `docker ps` or `pg_isready`
   - Wrong port: Ensure PostgreSQL is listening on the specified port

2. **Authentication Failed**: Check username and password are correct

3. **Database Does Not Exist**: Create the database first:
   ```bash
   createdb crewchief
   # or in Docker:
   docker exec -it postgres psql -U postgres -c "CREATE DATABASE crewchief;"
   ```

4. **Missing Schema**: After connecting, run migrations:
   ```bash
   cargo run -p crewchief-maproom -- db migrate
   ```

**Validation:**

The `watch` command validates the database connection on startup and provides helpful error messages if misconfigured. You'll see:
- ✅ Success: "Database connection validated successfully"
- ❌ Failure: Clear error with the DATABASE_URL being used (password sanitized) and troubleshooting steps

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

**Google Cloud Integration Tests:**

Google Vertex AI provider integration tests require real GCP credentials and are marked with `#[ignore]` to prevent accidental execution. These tests validate end-to-end functionality with Google Cloud services.

Prerequisites:
- GCP project with Vertex AI API enabled
- Service account with `roles/aiplatform.user` IAM role
- Service account JSON key file

```bash
# Set up environment
export GCP_INTEGRATION_TESTS=1
export GOOGLE_PROJECT_ID=your-test-project-id
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account-key.json

# Run Google integration tests
cargo test --test google_provider_integration -- --ignored

# Run specific Google test
cargo test --test google_provider_integration test_google_provider_single_embed -- --ignored
```

For detailed setup instructions, see [docs/development/integration-testing.md](docs/development/integration-testing.md).

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
