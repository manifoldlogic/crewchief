# Maproom Integration Tests

This directory contains comprehensive integration tests for the Maproom embedding infrastructure, covering Phase 1 of the HYBRID_SEARCH project.

## Test Organization

### Integration Tests (`tests/integration/`)

#### `embedding_service_test.rs`
Tests for the EmbeddingService component:
- ✅ Embedding generation for simple, complex, and documentation chunks
- ✅ Batch processing with 100+ chunks
- ✅ Retry logic with exponential backoff
- ✅ Cost tracking accuracy
- ✅ Cache integration
- ✅ Error handling

**Run with:**
```bash
cargo test --test embedding_service_test
```

**Run with real API (requires OPENAI_API_KEY):**
```bash
cargo test --test embedding_service_test -- --ignored
```

#### `embedding_cache_test.rs`
Tests for the EmbeddingCache component:
- ✅ Cache hit/miss tracking
- ✅ LRU eviction policy validation
- ✅ Concurrent access stress tests (50+ concurrent tasks)
- ✅ Cache hit rate over 1000 operations (target >80%)
- ✅ TTL and expiration behavior
- ✅ Thread-safety verification

**Run with:**
```bash
cargo test --test embedding_cache_test
```

#### `vector_db_test.rs`
Tests for vector database operations:
- ✅ Schema validation (vector columns, types, dimensions)
- ✅ IVF-Flat index creation and configuration
- ✅ Query planner analysis (EXPLAIN ANALYZE)
- ✅ Performance benchmarks (p95 latency <100ms)
- ✅ Cosine distance operator (<=>)
- ✅ Index usage verification

**Run with (requires MAPROOM_DATABASE_URL):**
```bash
cargo test --test vector_db_test -- --ignored
```

### Test Fixtures (`tests/fixtures/`)

#### `embedding_test_data.json`
Sample chunk data for testing:
- Function chunks (TypeScript, Rust)
- Class definitions
- Documentation snippets
- Test scenarios for simple/complex/documentation chunks
- Performance benchmarks

#### `realistic_queries.txt`
Realistic query sequences simulating:
- Development workflows (high locality)
- Code review patterns
- Documentation updates
- Refactoring workflows
- Testing workflows
- Debugging sessions
- CI/CD pipeline inspection

Used for cache hit rate testing with target >80% hit rate over 1000 operations.

## Running Tests

### All Tests (without external dependencies)
```bash
cargo test
```

### Integration Tests with Real API
```bash
export OPENAI_API_KEY=sk-...
cargo test -- --ignored
```

### Database Integration Tests (Postgres/pgvector)
```bash
export MAPROOM_TEST_PG_URL=postgres://maproom:maproom@localhost:5432/maproom_test
cargo test -p maproom --features postgres --lib db::postgres -- --ignored --test-threads=1
cargo test -p maproom --features postgres --test store_parity -- --ignored --test-threads=1
cargo test -p maproom --features postgres --test pg_real_dedup -- --ignored --test-threads=1
```

### Performance Tests with Output
```bash
cargo test --test embedding_cache_test test_cache_hit_rate_realistic_sequence -- --nocapture
```

### All Integration Tests
```bash
cargo test --test integration
```

## Test Requirements

### Environment Variables

- `OPENAI_API_KEY` (optional): Required for real API tests (marked with `#[ignore]`)
- `MAPROOM_TEST_PG_URL` (optional): Required for the `#[ignore]`'d Postgres suites (skipped when unset)
- `RUST_LOG` (optional): Set to `debug` for verbose logging

### Database Setup

For the Postgres suites, ensure PostgreSQL with pgvector is running:

```bash
# Using Docker
docker run -d \
  --name maproom-test-db \
  -e POSTGRES_USER=maproom \
  -e POSTGRES_PASSWORD=maproom \
  -e POSTGRES_DB=maproom_test \
  -p 5432:5432 \
  pgvector/pgvector:pg16

# Set MAPROOM_TEST_PG_URL
export MAPROOM_TEST_PG_URL=postgres://maproom:maproom@localhost:5432/maproom_test
```

### Running Migrations

Migrations run automatically in database tests via `db::migrate(&client)`.

## Test Coverage

### Acceptance Criteria Coverage

From ticket HYBRID_SEARCH-1901:

| Criteria | Test File | Status |
|----------|-----------|--------|
| ✅ Embedding generation tests (simple, complex, doc chunks) | `embedding_service_test.rs` | Complete |
| ✅ Batch processing (100 chunks) | `embedding_service_test.rs` | Complete |
| ✅ Retry logic with exponential backoff | `embedding_service_test.rs` | Complete |
| ✅ Cache hit rate >80% (1000 operations) | `embedding_cache_test.rs` | Complete |
| ✅ Cache eviction (LRU behavior) | `embedding_cache_test.rs` | Complete |
| ✅ Cost tracking accuracy (within 1%) | `embedding_service_test.rs` | Complete |
| ✅ Budget warning thresholds | `embedding_service_test.rs` | Complete |
| ✅ Vector columns exist and populated | `vector_db_test.rs` | Complete |
| ✅ IVF-Flat indices used by planner | `vector_db_test.rs` | Complete |
| ✅ Vector query p95 latency <100ms | `vector_db_test.rs` | Complete |
| ✅ Integration tests pass in CI | All tests | Complete |

## Performance Benchmarks

### Cache Performance
- **Target**: >80% hit rate over 1000 operations
- **Test**: `test_cache_hit_rate_realistic_sequence`
- **Result**: Achieves >80% with realistic query patterns

### Vector Query Performance
- **Target**: <100ms p95 latency
- **Test**: `test_vector_query_latency`
- **Configuration**: ivfflat.probes = 10

### Concurrent Access
- **Test**: 50 concurrent readers + 25 concurrent writers
- **Result**: No race conditions, cache integrity maintained

### Cost Tracking
- **Accuracy**: Within 1% of expected costs
- **Model**: text-embedding-3-small ($0.02 per 1M tokens)

## CI Configuration

Tests are designed to work in CI environments:

1. **Unit tests** run without external dependencies
2. **Integration tests** marked with `#[ignore]` require API key or database
3. **Mock clients** used for most deterministic testing
4. **Timeout handling** for long-running tests

### GitHub Actions

The Postgres/pgvector CI job already exists — see the `test-postgres` job in
`.github/workflows/test.yml`. It runs a `pgvector/pgvector:pg16` service
container (with a `pg_isready` health check), exports
`MAPROOM_TEST_PG_URL=postgres://maproom:maproom@localhost:5432/maproom_test`,
and invokes the canonical per-suite commands:

```bash
cargo test -p maproom --features postgres --lib db::postgres -- --ignored --test-threads=1
cargo test -p maproom --features postgres --test store_parity -- --ignored --test-threads=1
cargo test -p maproom --features postgres --test pg_real_dedup -- --ignored --test-threads=1
```

Do not duplicate the service definition here or in `test_config.yml`; the
workflow file is the single source of truth.

## Troubleshooting

### Tests Fail with "connection refused"
- Ensure PostgreSQL is running
- Check MAPROOM_TEST_PG_URL is correct
- Verify pgvector extension is installed

### Tests Fail with "API key not found"
- Set OPENAI_API_KEY environment variable
- Or run without `--ignored` flag to skip real API tests

### Cache hit rate below 80%
- Check that realistic_queries.txt has appropriate patterns
- Verify cache size is sufficient (100+ entries)
- Review query distribution (should have high locality)

### Vector query timeout
- Ensure database has data (run indexing first)
- Check ivfflat.probes setting (default: 10)
- Verify indices exist with `\d+ maproom.chunks` in psql

## Contributing

When adding new tests:

1. Follow existing patterns and naming conventions
2. Use `#[ignore]` for tests requiring external dependencies
3. Document test purpose and requirements
4. Add acceptance criteria mapping to this README
5. Ensure tests are deterministic and reproducible
6. Handle both success and failure paths

## References

- Ticket: `.crewchief/work-tickets/HYBRID_SEARCH-1901_test-embedding-infrastructure.md`
- Architecture: `.crewchief/archive/projects/HYBRID_SEARCH_hybrid-retrieval-system/planning/HYBRID_SEARCH_ARCHITECTURE.md`
- Implementation tickets:
  - HYBRID_SEARCH-1001: Embedding Service Setup
  - HYBRID_SEARCH-1002: Database Vector Preparation
  - HYBRID_SEARCH-1003: Embedding Generation Pipeline
