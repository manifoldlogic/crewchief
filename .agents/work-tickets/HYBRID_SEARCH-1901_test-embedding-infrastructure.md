# Ticket: HYBRID_SEARCH-1901: Test Embedding Infrastructure

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- embeddings-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Integration and validation testing for Phase 1 embedding infrastructure, including embedding generation, caching, cost tracking, and vector database operations.

## Background
Phase 1 of the hybrid search system introduces embedding infrastructure as a foundational component. This includes the EmbeddingService, caching layer, cost tracking, and vector database columns with indices. Comprehensive testing is required to validate that all components work correctly together and meet performance requirements before proceeding to Phase 2 (fusion and reranking).

The embedding infrastructure must reliably generate embeddings for all chunk types, maintain high cache hit rates for efficiency, accurately track API costs to manage budget, and leverage optimized vector indices for fast similarity search.

## Acceptance Criteria
- [ ] All embedding generation tests pass for simple, complex, and documentation chunks
- [ ] Batch processing test successfully handles 100 chunks
- [ ] Retry logic properly handles API failures with exponential backoff
- [ ] Cache hit rate exceeds 80% in realistic usage scenarios (1000 operations)
- [ ] Cache eviction policy correctly maintains LRU behavior
- [ ] Cost tracking accurate within 1% for all batch operations
- [ ] Budget warning thresholds trigger at configured levels
- [ ] Vector columns (code_embedding, text_embedding) exist and are populated
- [ ] IVF-Flat indices are used by query planner (verified with EXPLAIN ANALYZE)
- [ ] Vector query performance achieves <100ms at p95 latency
- [ ] Integration tests pass in CI environment

## Technical Requirements
- **Embedding Generation**
  - Test OpenAI API integration with text-embedding-3-small model
  - Validate 1536-dimensional vector output
  - Test batch embedding generation (up to 100 chunks)
  - Verify retry logic with exponential backoff on API errors
  - Test handling of various chunk types (function, class, documentation)

- **Caching Layer**
  - LRU cache with configurable size (10,000 entries default)
  - Test cache hit/miss tracking
  - Verify cache TTL behavior (3600s default)
  - Measure hit rate over realistic query sequences
  - Test thread-safe concurrent access (Arc<RwLock<LruCache>>)

- **Cost Tracking**
  - Track token counts per API call
  - Calculate costs based on pricing model ($0.00002/1K tokens)
  - Aggregate costs across batch operations
  - Test warning thresholds for budget alerts
  - Verify accuracy against OpenAI billing

- **Vector Database**
  - Verify vector columns exist: code_embedding, text_embedding (vector(1536))
  - Test IVF-Flat index creation with 200 lists
  - Validate query planner uses indices (EXPLAIN ANALYZE)
  - Measure p95 latency for vector similarity queries
  - Test cosine distance operator (<=>)

- **Test Framework**
  - Use Rust integration tests in `crates/maproom/tests/integration/`
  - Test database fixtures with realistic data
  - Mock OpenAI API for deterministic testing
  - Performance benchmarks with criterion or custom timing
  - CI-compatible test configuration

## Implementation Notes

### Architecture Reference
The embedding infrastructure is defined in the architecture document:
- **EmbeddingService** (lines 83-117): Cache-backed embedding generation with OpenAI client
- **Configuration** (lines 265-294): Embedding provider, model, cache settings
- **Caching Strategy** (lines 343-379): LRU cache with async access patterns

### Test Structure
Create three main test files:

1. **embedding_service_test.rs**
   - Unit tests for EmbeddingService
   - Mock OpenAI client for deterministic results
   - Test cache behavior, retry logic, error handling
   - Performance benchmarks for embedding generation

2. **embedding_cache_test.rs**
   - Cache hit/miss tracking tests
   - LRU eviction policy validation
   - Concurrent access stress tests
   - Hit rate measurement over realistic sequences

3. **vector_db_test.rs**
   - Database schema validation (columns, types)
   - Index creation and verification
   - Query planner analysis (EXPLAIN ANALYZE)
   - Performance benchmarks for vector queries
   - Cosine distance accuracy tests

### Key Test Scenarios

**Embedding Generation**:
```rust
#[tokio::test]
async fn test_embed_simple_code_chunk() {
    let service = EmbeddingService::new(config);
    let chunk = "fn main() { println!(\"Hello\"); }";
    let embedding = service.embed_text(chunk).await.unwrap();
    assert_eq!(embedding.len(), 1536);
}
```

**Cache Hit Rate**:
```rust
#[tokio::test]
async fn test_cache_hit_rate_realistic() {
    let service = EmbeddingService::new(config);
    let queries = load_realistic_query_sequence(1000);
    let mut hits = 0;

    for query in queries {
        if service.embed_text(&query).await.is_cache_hit() {
            hits += 1;
        }
    }

    let hit_rate = hits as f64 / 1000.0;
    assert!(hit_rate > 0.8, "Cache hit rate {:.2}% below 80%", hit_rate * 100.0);
}
```

**Vector Query Performance**:
```sql
-- Test query with EXPLAIN ANALYZE
EXPLAIN (ANALYZE, BUFFERS)
SELECT id, code_embedding <=> $1::vector AS distance
FROM maproom.chunks
WHERE code_embedding IS NOT NULL
ORDER BY code_embedding <=> $1::vector
LIMIT 20;
```

### Risk Mitigation
- **API Rate Limits**: Use mocked client for most tests; only real API in designated integration test
- **Cost Management**: Limit real API calls; use cached embeddings for repeated tests
- **Performance Variance**: Run performance tests multiple times; use median/p95 rather than mean
- **CI Environment**: Ensure database available in CI; use containerized PostgreSQL if needed

## Dependencies
- **HYBRID_SEARCH-1001**: Embedding service implementation (EmbeddingService, OpenAI client)
- **HYBRID_SEARCH-1002**: Database schema changes (vector columns, indices)
- **HYBRID_SEARCH-1003**: Indexing pipeline integration (embedding generation during indexing)
- **External**: PostgreSQL with pgvector extension
- **External**: OpenAI API access (or mocked client for CI)

## Risk Assessment
- **Risk**: OpenAI API rate limits during testing
  - **Mitigation**: Use mocked client for most tests; only call real API in dedicated integration test with rate limiting

- **Risk**: Performance tests fail due to environment variance
  - **Mitigation**: Run tests multiple times; use percentile metrics (p95) rather than averages; document test environment requirements

- **Risk**: Cache hit rate depends on query distribution
  - **Mitigation**: Create realistic query sequence from actual usage patterns; test multiple scenarios (development, production)

- **Risk**: Vector indices not used by query planner
  - **Mitigation**: Use EXPLAIN ANALYZE to verify index usage; adjust ivfflat parameters (lists, probes) if needed

- **Risk**: Cost tracking accuracy depends on OpenAI response metadata
  - **Mitigation**: Validate against OpenAI billing dashboard; implement tolerance threshold (1%) for floating-point comparison

## Files/Packages Affected
- **New Test Files**:
  - `crates/maproom/tests/integration/embedding_service_test.rs`
  - `crates/maproom/tests/integration/embedding_cache_test.rs`
  - `crates/maproom/tests/integration/vector_db_test.rs`

- **Test Fixtures**:
  - `crates/maproom/tests/fixtures/embedding_test_data.json`
  - `crates/maproom/tests/fixtures/realistic_queries.txt`

- **Configuration**:
  - `crates/maproom/tests/test_config.yml` (test-specific configuration)

- **CI Configuration**:
  - `.github/workflows/test.yml` (may need PostgreSQL service container)

- **Dependencies** (Cargo.toml):
  - May need: `mockito` or `wiremock` for API mocking
  - May need: `criterion` for performance benchmarks
