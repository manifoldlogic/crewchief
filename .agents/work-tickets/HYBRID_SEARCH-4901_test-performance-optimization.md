# Ticket: HYBRID_SEARCH-4901: Test Performance Optimization

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- performance-engineer
- integration-tester
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Comprehensive integration and validation testing for Phase 4 performance optimization. Verify that optimized query execution, index configuration, and caching strategies meet latency and throughput targets: p50 <30ms, p95 <50ms, p99 <100ms, and sustained 10+ QPS.

## Background
Phase 4 implements critical performance optimizations including:
- Query optimization via materialized views and index tuning (HYBRID_SEARCH-4001)
- Index configuration for ivfflat, GIN, and partial indices (HYBRID_SEARCH-4002)
- Multi-layer caching for queries, embeddings, and scores (HYBRID_SEARCH-4003)

These optimizations are designed to meet production latency and throughput requirements. Testing must validate that all performance targets are met without regressing search quality (recall/precision).

## Acceptance Criteria
- [ ] p50 latency consistently <30ms across 1000+ queries
- [ ] p95 latency consistently <50ms across 1000+ queries
- [ ] p99 latency consistently <100ms across 1000+ queries
- [ ] Sustained 10+ QPS without performance degradation
- [ ] Cache hit rate >60% on realistic query distributions
- [ ] Memory usage remains <500MB under load
- [ ] No regressions in search quality metrics (recall, precision)
- [ ] All search types (FTS, vector, graph, hybrid) meet latency targets
- [ ] Index usage verified (no sequential scans on large tables)
- [ ] Materialized views demonstrably reduce query time

## Technical Requirements

### 1. Latency Benchmark Suite
- Measure p50, p95, p99 latencies for 1000 queries per search type
- Test with cold cache (first run) and warm cache (subsequent runs)
- Verify materialized views reduce query time vs baseline
- Test all search modes: FTS-only, vector-only, graph-enhanced, hybrid fusion
- Record latency breakdown by pipeline stage (processing, execution, fusion)

### 2. Index Performance Validation
- Use EXPLAIN ANALYZE to verify index usage on all queries
- Confirm no sequential scans on chunks, files, or edges tables
- Test ivfflat probe settings (5, 10, 20) impact on latency/recall tradeoff
- Validate partial indices used for recency/churn filtered queries
- Measure GIN index performance on FTS queries

### 3. Cache Effectiveness Tests
- Measure cache hit rate over 10,000 queries with realistic distribution
- Test cache warming on startup (pre-populate common queries)
- Verify cache invalidation on data updates (upsert operations)
- Monitor memory usage stays <500MB with 10,000 cached queries
- Test LRU eviction behavior under memory pressure

### 4. Load Testing
- Sustained 10 QPS load over 10 minutes
- Burst load testing: measure throughput at 50 QPS
- Concurrent query testing: 100 simultaneous requests
- Verify connection pool handles load without exhaustion
- Test degradation behavior under overload conditions

### 5. Regression Testing
- Compare optimized vs baseline performance on same query set
- Verify recall and precision unchanged (±2% acceptable variance)
- Test memory usage over 1-hour load test (detect memory leaks)
- Validate database statistics remain accurate after load

## Implementation Notes

### Test Infrastructure
```rust
// Performance benchmark framework
pub struct PerformanceBenchmark {
    db_pool: PgPool,
    search_pipeline: SearchPipeline,
    query_corpus: Vec<String>,
    metrics_collector: MetricsCollector,
}

impl PerformanceBenchmark {
    pub async fn run_latency_benchmark(&self) -> BenchmarkResults {
        let mut latencies = Vec::new();

        for query in &self.query_corpus {
            let start = Instant::now();
            let _ = self.search_pipeline.search(query, default_options()).await;
            latencies.push(start.elapsed());
        }

        BenchmarkResults {
            p50: percentile(&latencies, 0.5),
            p95: percentile(&latencies, 0.95),
            p99: percentile(&latencies, 0.99),
            mean: mean(&latencies),
            std_dev: std_dev(&latencies),
        }
    }
}
```

### Test Query Corpus
- 100 common code search queries ("authentication", "database connection", etc.)
- 50 natural language queries ("how to handle errors in rust")
- 50 symbol-specific queries ("HashMap::new", "async fn process")
- Mix of short (1-2 words) and long (5+ words) queries

### Cache Test Scenarios
- Cold start: Empty cache, measure first-query performance
- Warm cache: Re-query same set, expect >90% hit rate
- Realistic distribution: Zipf distribution (20% of queries account for 80% of traffic)
- Cache invalidation: Trigger upsert, verify stale results evicted

### Load Test Configuration
- Tool: Criterion.rs for Rust benchmarks + custom async load generator
- Database: PostgreSQL with realistic dataset (10K+ chunks)
- Metrics: Latency histograms, throughput, error rates, resource usage
- Duration: 10-minute sustained load, 1-minute burst tests

### Architecture Reference
See `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md`:
- Query optimization (lines 404-425): Materialized views for importance scores
- Index configuration (lines 383-402): Partial indices and composite indices
- Caching strategy (lines 343-379): Multi-layer LRU caches
- Performance metrics (lines 453-462): Monitoring and observability

## Dependencies
- **HYBRID_SEARCH-4001**: Query optimization (materialized views, query planning)
- **HYBRID_SEARCH-4002**: Index tuning (ivfflat, GIN, partial indices)
- **HYBRID_SEARCH-4003**: Caching implementation (query cache, embedding cache)

All three Phase 4 implementation tickets must be completed before performance testing can begin.

## Risk Assessment

### Risk: Performance targets not met on realistic hardware
**Mitigation**:
- Test on representative hardware (similar to production)
- Profile slow queries and optimize further if needed
- Document minimum hardware requirements if targets only met on high-spec machines

### Risk: Cache effectiveness lower than expected
**Mitigation**:
- Analyze query distribution to tune cache size
- Implement cache warming for top 100 queries
- Consider query normalization to improve cache hit rate

### Risk: Database index overhead slows writes
**Mitigation**:
- Benchmark upsert performance separately
- Tune index configuration if write performance degrades
- Consider delayed index updates for high-churn scenarios

### Risk: Memory usage exceeds 500MB target
**Mitigation**:
- Profile memory allocations to identify leaks
- Tune cache size parameters (reduce cache_size if needed)
- Implement memory pressure monitoring and adaptive eviction

## Files/Packages Affected

### New Test Files
- `crates/maproom/tests/performance/latency_benchmark.rs` - Latency measurement suite
- `crates/maproom/tests/performance/load_test.rs` - Load and stress testing
- `crates/maproom/tests/performance/cache_effectiveness_test.rs` - Cache hit rate validation
- `crates/maproom/tests/performance/index_usage_test.rs` - Index utilization checks
- `crates/maproom/tests/performance/regression_test.rs` - Quality regression detection

### New Benchmark Files
- `crates/maproom/benches/search_benchmark.rs` - Criterion-based benchmarks
- `crates/maproom/benches/fusion_benchmark.rs` - Score fusion performance
- `crates/maproom/benches/cache_benchmark.rs` - Cache layer performance

### Test Data
- `crates/maproom/tests/fixtures/query_corpus.txt` - Representative query set
- `crates/maproom/tests/fixtures/performance_baseline.json` - Baseline metrics

### Configuration
- Update `crates/maproom/Cargo.toml` to add criterion dev-dependency
- Configure benchmark harness in Cargo.toml

### Documentation
- `crates/maproom/docs/PERFORMANCE.md` - Performance characteristics and tuning guide
- Update README with performance benchmarks and requirements
