# Maproom

Semantic code search powered by embeddings and SQLite.

## Features

- **Semantic Code Search** - Find code by concept, not just keywords
- **Multi-Provider Embeddings** - Choose Ollama (free), OpenAI, or Google Vertex AI
- **Zero-Config Setup** - SQLite database auto-created, no external services required
- **SQLite Storage** - Portable, zero-config database with FTS5 and sqlite-vec
- **Incremental Indexing** - Fast updates for changed files
- **MCP Integration** - Works with Claude, Cursor, and other AI tools

## Quick Start (Zero Config)

### 1. Install Ollama (Free, Local)
```bash
curl -sSL https://ollama.ai/install.sh | sh
ollama pull nomic-embed-text
```

### 2. Index Your Repository
```bash
maproom scan --generate-embeddings
```

### 3. Search Your Code
```bash
maproom search "authentication middleware"
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
export MAPROOM_EMBEDDING_PROVIDER=openai
maproom scan --generate-embeddings
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
export MAPROOM_EMBEDDING_PROVIDER=google
maproom scan --generate-embeddings
```

See [Google Setup Guide](docs/providers/google-vertex-ai-setup.md)

---

## Configuration

Maproom auto-detects your provider:

1. Checks `MAPROOM_EMBEDDING_PROVIDER` env var (explicit)
2. Detects Ollama on localhost:11434
3. Falls back to OpenAI if `OPENAI_API_KEY` present
4. Falls back to Google if `GOOGLE_PROJECT_ID` present

### Explicit Provider Selection
```bash
export MAPROOM_EMBEDDING_PROVIDER=ollama  # or openai, google
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

## Installation

### Via cargo (recommended)

```bash
cargo install maproom
```

### Build from source

```bash
git clone https://github.com/manifoldlogic/crewchief.git
cd crewchief
cargo build --release -p maproom
```

The binary will be at `target/release/maproom`.

---

## Database Setup

Maproom uses SQLite by default. The database is automatically created at `~/.maproom/maproom.db`.

```bash
# Initialize database (auto-creates if needed)
maproom db migrate

# Or specify a custom location
export MAPROOM_DATABASE_URL="sqlite:///path/to/maproom.db"
maproom db migrate
```

## Usage

### Indexing Code

```bash
maproom scan \
  --repo crewchief \
  --worktree radar \
  --path /path/to/worktree \
  --commit $(git rev-parse HEAD)
```

### Generating Embeddings

After indexing, generate vector embeddings for semantic search:

```bash
# Generate embeddings for all chunks (incremental mode - only NULL embeddings)
maproom generate-embeddings

# Test with a small sample first
maproom generate-embeddings --sample 100

# Dry run to see what would happen without writing to database
maproom generate-embeddings --dry-run --sample 100

# Force regeneration of all embeddings
maproom generate-embeddings --force

# Limit cost to prevent overspending
maproom generate-embeddings --max-cost 5.0

# Custom batch size and delay
maproom generate-embeddings --batch-size 50 --batch-delay 200
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
maproom search \
  --repo crewchief \
  --worktree radar \
  --query "authentication" \
  --k 10
```

## Environment Variables

### Database (Optional)
- `MAPROOM_DATABASE_URL` - SQLite database location (default: `~/.maproom/maproom.db`)
  - Example: `MAPROOM_DATABASE_URL="sqlite:///tmp/maproom.db"`

### Provider-Specific (pick one)

**Ollama (default):**
- No environment variables required! Just install and run Ollama.
- `OLLAMA_URL` - Optional, defaults to `http://localhost:11434`

**OpenAI:**
- `OPENAI_API_KEY` - OpenAI API key
- `MAPROOM_EMBEDDING_PROVIDER=openai` (optional if API key present)

**Google Vertex AI:**
- `GOOGLE_PROJECT_ID` - GCP project ID
- `GOOGLE_APPLICATION_CREDENTIALS` - Service account key path
- `MAPROOM_EMBEDDING_PROVIDER=google` (optional if project ID present)

### Optional Configuration
- `MAPROOM_EMBEDDING_PROVIDER` - Explicit provider selection: `ollama`, `openai`, or `google`
- `MAPROOM_EMBEDDING_MODEL` - Model name override
- `MAPROOM_EMBEDDING_BATCH_SIZE` - API batch size (default: 100)
- `MAPROOM_EMBEDDING_CACHE_SIZE` - LRU cache size (default: 10000)
- `MAPROOM_EMBEDDING_CACHE_TTL` - Cache TTL in seconds (default: 3600)

### Configuration File

Create a `.env` at the repo root or in `crates/maproom/`:

```bash
# Ollama (default - no configuration needed!)
# Database auto-created at ~/.maproom/maproom.db

# OpenAI
OPENAI_API_KEY=sk-proj-...
MAPROOM_EMBEDDING_PROVIDER=openai

# Google Vertex AI
GOOGLE_PROJECT_ID=your-project-id
GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json
MAPROOM_EMBEDDING_PROVIDER=google

# Custom database location (optional)
MAPROOM_DATABASE_URL="sqlite:///path/to/maproom.db"
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

> **Note:** Integration test commands require a repository checkout (`git clone`). They are not available when installed via `cargo install maproom` because integration tests are not included in the published crate.

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

> **Note:** Integration test commands require a repository checkout (`git clone`). They are not available when installed via `cargo install maproom` because integration tests are not included in the published crate.

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
- Sufficient disk space for temporary test repositories

The tests will automatically:
- Create temporary in-memory SQLite databases
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
- All tests use in-memory SQLite databases to avoid conflicts
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

## Language Support

Maproom indexes and extracts semantic information from the following languages:

| Language | File Extensions | Extracted Constructs |
|----------|----------------|---------------------|
| TypeScript | `.ts`, `.tsx` | Functions, classes, interfaces, types, imports, exports |
| JavaScript | `.js`, `.jsx` | Functions, classes, imports, exports |
| Rust | `.rs` | Functions, structs, enums, traits, impls, modules |
| Python | `.py` | Functions, classes, imports, decorators |
| Go | `.go` | Functions, types, interfaces, structs |
| Ruby | `.rb` | Methods, classes, modules, constants |
| C | `.c` | Functions, structs, enums, typedefs, variables, `#include` directives |
| Markdown | `.md` | Headings, sections |
| JSON | `.json` | Keys, structure |
| YAML | `.yml`, `.yaml` | Keys, structure |
| TOML | `.toml` | Keys, structure |

### C Language (MLLANG-1005)

C language support indexes `.c` source files only. Supported constructs:
- Functions (with parameters, return types, doc comments)
- Structs (with field metadata)
- Enums (with enumerator lists)
- Typedefs
- Global variables
- `#include` directives (system and local headers, aggregated into imports)
- Static function detection and storage class metadata

**Current Limitation**: Header files (`.h`) are not indexed. The `.h` file extension is ambiguous between C and C++, and correctly handling headers requires content-based disambiguation. Header file support will be added in MLLANG-1004 (C++ language support), which will handle both C and C++ headers.

**Workaround**: Focus searches on `.c` implementation files, which contain function definitions, struct definitions, and other constructs that Maproom extracts.

## Known Limitations

- Single-user only (no multi-process concurrent writes)
- No database encryption
- sqlite-vec extension must be compiled in (statically linked)
- C header files (`.h`) are not indexed — header support deferred to MLLANG-1004 (C++ language support)
