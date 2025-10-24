# Ticket: HYBRID_SEARCH-2901: Test Search Pipeline

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- database-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Create comprehensive integration and validation tests for the Phase 2 hybrid search pipeline, verifying query processing, parallel execution, search type functionality, and score fusion.

## Background
Phase 2 implementation introduces the foundational search pipeline with query processing, parallel search execution (FTS, vector, graph), and basic score fusion. Before advancing to Phase 3 optimization and learned fusion, we need rigorous testing to ensure:
- All search types return relevant results
- Parallel execution meets <100ms latency target
- Score fusion produces properly ranked results
- API endpoints are functional and stable

This testing validates the acceptance criteria from the Phase 2 plan and ensures the pipeline is production-ready before adding advanced features.

## Acceptance Criteria
- [ ] Query processing tests validate tokenization, mode detection, expansion, and embedding generation
- [ ] Parallel execution tests confirm all search types (FTS, vector, graph, signals) complete in <100ms combined
- [ ] Search type tests verify each search method returns relevant, non-empty results
- [ ] Integration tests demonstrate end-to-end search functionality with proper result deduplication
- [ ] API endpoint tests confirm correct responses and error handling
- [ ] All tests pass consistently in CI environment
- [ ] Test coverage report shows >80% coverage for search pipeline code
- [ ] Performance benchmarks documented with timing breakdowns

## Technical Requirements
- Use Rust integration testing framework (`tests/integration/`)
- Mock embedding service for deterministic tests
- Test database with pre-seeded fixture data
- Parallel execution timing with tokio instrumentation
- Result validation against expected rankings
- Error scenario coverage (malformed queries, timeouts, empty results)
- Performance profiling with criterion or similar
- Test fixtures for various query types (code queries, text queries, mixed queries)

## Implementation Notes

### Test Structure
Create three main test files:
1. **`query_processor_test.rs`**: Unit/integration tests for QueryProcessor
   - Test tokenization on various inputs (code syntax, natural language, special chars)
   - Validate SearchMode detection (Code/Text/Auto)
   - Test query expansion with mock expander
   - Verify embedding generation integration

2. **`parallel_execution_test.rs`**: Performance and concurrency tests
   - Measure individual search type latencies (FTS, vector, graph, signals)
   - Verify tokio::join! parallel execution
   - Test under various load conditions
   - Validate timeout handling
   - Benchmark against <100ms target

3. **`search_pipeline_test.rs`**: End-to-end integration tests
   - Full search pipeline execution
   - Result fusion validation
   - Deduplication verification
   - Ranking correctness
   - API endpoint integration tests
   - Error handling scenarios

### Test Data Setup
```rust
// Fixture setup helper
async fn setup_test_database() -> Result<TestDatabase> {
    // Create test database with known data
    // Seed with representative code chunks
    // Include embeddings for vector search
    // Populate graph edges for graph search
    // Set recency/churn scores
}
```

### Architecture References
From `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md`:

- **Query Processor** (lines 44-81): Test tokenization, expansion, embedding, and mode detection
- **Search Executors** (lines 119-205): Validate FTS, vector, and graph queries
- **Score Fusion** (lines 207-260): Test RRF and weighted fusion algorithms
- **Query Pipeline** (lines 296-340): End-to-end pipeline integration

### Key Test Cases

**1. Query Processing**
```rust
#[tokio::test]
async fn test_query_tokenization() {
    // Test: "authentication flow" -> tokens
    // Test: "fn main() {}" -> code mode detection
    // Test: "how to handle errors?" -> text mode detection
}

#[tokio::test]
async fn test_embedding_generation() {
    // Test: embedding cache hits
    // Test: embedding generation on cache miss
    // Test: embedding dimension correctness
}
```

**2. Parallel Execution**
```rust
#[tokio::test]
async fn test_parallel_search_latency() {
    let start = Instant::now();
    let (fts, vector, graph, signals) = tokio::join!(
        executor.fts_search(&query, 100),
        executor.vector_search(&query, 100),
        executor.graph_search(&query, 100),
        executor.signal_search(&query)
    );
    let duration = start.elapsed();
    assert!(duration < Duration::from_millis(100));
}
```

**3. Search Type Validation**
```rust
#[tokio::test]
async fn test_fts_returns_results() {
    let results = executor.fts_search("authentication", 20).await?;
    assert!(!results.is_empty());
    assert!(results[0].score > 0.0);
}

#[tokio::test]
async fn test_vector_semantic_similarity() {
    let results = executor.vector_search("login system", 20).await?;
    // Should return chunks about authentication even without exact match
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_graph_prioritizes_connected_code() {
    let results = executor.graph_search("exported function", 20).await?;
    // Should prioritize functions with high in-degree (many callers)
    assert!(results[0].graph_score > results[1].graph_score);
}
```

**4. Integration Tests**
```rust
#[tokio::test]
async fn test_end_to_end_search() {
    let pipeline = SearchPipeline::new(config).await?;
    let results = pipeline.search("authentication", SearchOptions::default()).await?;

    assert!(!results.results.is_empty());
    assert!(results.results.len() <= 20); // Default limit
    assert!(is_sorted_by_score(&results.results));
}

#[tokio::test]
async fn test_result_deduplication() {
    // Query that might return same chunk from multiple sources
    let results = pipeline.search("common term", opts).await?;
    let unique_ids: HashSet<_> = results.results.iter().map(|r| r.chunk_id).collect();
    assert_eq!(unique_ids.len(), results.results.len());
}

#[tokio::test]
async fn test_api_endpoint() {
    let response = client.post("/api/search")
        .json(&json!({"query": "test", "limit": 10}))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let body: SearchResponse = response.json().await?;
    assert!(!body.results.is_empty());
}
```

**5. Error Handling**
```rust
#[tokio::test]
async fn test_malformed_query_handling() {
    let result = pipeline.search("", opts).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_timeout_handling() {
    // Test with artificially slow executor
    let result = pipeline.search_with_timeout(query, Duration::from_millis(10)).await;
    // Should handle gracefully
}
```

### Performance Benchmarks
Document timing breakdowns:
- Query processing time
- FTS query time
- Vector query time
- Graph query time
- Signal query time
- Fusion time
- Total end-to-end time

### Mocking Strategy
- Mock EmbeddingClient for deterministic embeddings
- Use testcontainers for PostgreSQL database
- Mock external dependencies
- Real tokio runtime for concurrency testing

## Dependencies
- HYBRID_SEARCH-2001 (Query Processing Pipeline) - Must be implemented
- HYBRID_SEARCH-2002 (Parallel Search Execution) - Must be implemented
- HYBRID_SEARCH-2003 (Score Fusion & API Integration) - Must be implemented

## Risk Assessment
- **Risk**: Tests may be flaky due to timing dependencies in parallel execution tests
  - **Mitigation**: Use multiple runs, statistical analysis, and reasonable tolerance margins (e.g., 95th percentile < 100ms)

- **Risk**: Test database setup may be complex and slow down test suite
  - **Mitigation**: Use in-memory PostgreSQL or testcontainers with caching, optimize fixture data size

- **Risk**: Mocked embeddings may not reflect real-world behavior
  - **Mitigation**: Include integration tests with real embedding service in separate test suite (marked as #[ignore] for CI)

- **Risk**: Test coverage may reveal missing edge cases in implementation
  - **Mitigation**: This is actually desired - document findings and create follow-up tickets for fixes

## Files/Packages Affected
- **New Files**:
  - `crates/maproom/tests/integration/query_processor_test.rs`
  - `crates/maproom/tests/integration/parallel_execution_test.rs`
  - `crates/maproom/tests/integration/search_pipeline_test.rs`
  - `crates/maproom/tests/fixtures/test_data.sql`
  - `crates/maproom/tests/helpers/mod.rs` (test utilities)

- **Modified Files**:
  - `crates/maproom/Cargo.toml` (add test dependencies)
  - `crates/maproom/tests/common/mod.rs` (shared test utilities)
