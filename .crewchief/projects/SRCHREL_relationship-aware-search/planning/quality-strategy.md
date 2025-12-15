# Quality Strategy: Relationship-Aware Search Ranking

## Testing Philosophy

**Confidence over Coverage:** Test critical paths and edge cases that could break ranking quality or performance. Don't chase 100% coverage—focus on tests that give confidence.

**Pragmatic Validation:** Use real queries and databases for integration tests. Manual evaluation for ranking quality (automated tests can't judge "better" ranking).

**Performance as Quality:** Latency is a feature. Benchmarks are quality gates, not afterthoughts.

## Test Types

### Unit Tests

**Scope:**
- Edge quality computation (all edge types, source kinds)
- Test chunk detection heuristic
- Configuration loading and validation
- Logarithmic scaling with quality weights

**Tools:**
- Rust: `cargo test`
- Test framework: built-in `#[test]` + `rstest` for parameterized tests

**Coverage Target:**
- Edge quality scorer: 100% (small, critical module)
- Configuration: 90% (focus on validation logic)
- Overall: 70% (pragmatic, not ceremonial)

**Example Tests:**
```rust
#[rstest]
#[case("calls", "function", 1.0)]  // Production code call
#[case("calls", "test", 0.5)]      // Test code call
#[case("extends", "class", 1.5)]   // Inheritance boost
#[case("imports", "function", 0.8)] // Import edge
fn test_edge_quality_computation(
    #[case] edge_type: &str,
    #[case] source_kind: &str,
    #[case] expected: f32,
) {
    let weights = EdgeQualityWeights::default();
    let quality = compute_edge_quality(edge_type, source_kind, &weights);
    assert!((quality - expected).abs() < 0.01);
}
```

### Integration Tests

**Scope:**
- Enhanced graph executor with real database
- Quality-weighted scores vs old scores (comparison)
- Feature flag toggle (enable/disable)
- Fusion weight override
- End-to-end search with enhanced ranking

**Approach:**
- Use in-memory SQLite database
- Populate with synthetic graph data (production + test code edges)
- Compare old vs enhanced executor results

**Test Cases:**
```rust
#[tokio::test]
async fn test_enhanced_executor_boosts_production_code() {
    let store = setup_test_db().await;

    // Chunk A: 10 production code callers
    // Chunk B: 10 test code callers
    // Expected: A ranks higher than B despite same edge count

    let results = GraphExecutor::execute_enhanced(&store, repo_id, None, 10, &config).await?;

    let chunk_a_rank = results.iter().position(|r| r.chunk_id == chunk_a_id);
    let chunk_b_rank = results.iter().position(|r| r.chunk_id == chunk_b_id);

    assert!(chunk_a_rank < chunk_b_rank, "Production code should rank higher");
}
```

### Performance Tests

**Scope:**
- Graph executor latency (<30ms p95)
- Total search latency (<100ms p95)
- Response size (ensure reasonable)
- Database query efficiency (no full table scans)

**Tools:**
- Rust: `criterion` benchmarks
- Database: `EXPLAIN QUERY PLAN`

**Benchmark Targets:**
| Metric | Target | Alert Threshold | Critical Threshold |
|--------|--------|-----------------|-------------------|
| Graph executor (enhanced) | <30ms p95 | >35ms | >40ms (rollback) |
| Total search latency | <100ms p95 | >120ms | >150ms |
| SQL query rows scanned | <10K | >50K | >100K |
| Graph executor overhead | <+10ms | >+15ms | >+20ms |

**Baseline Establishment (Phase 2):**
Before enabling quality scoring in production:
1. Run with `enable_quality=false` for 1 week
2. Record baseline metrics:
   - Graph executor p50/p95/p99
   - Total search latency p50/p95/p99
   - Error rate baseline
3. Use baseline for comparison when `enable_quality=true` enabled

**Performance Regression Tests:**
```rust
#[test]
fn test_no_excessive_overhead() {
    let baseline = benchmark_old_executor();  // ~20ms p95
    let enhanced = benchmark_enhanced_executor();

    let overhead = enhanced - baseline;
    assert!(overhead < 15_000, "Overhead exceeds 15ms: {}μs", overhead);
}
```

**Example Benchmark:**
```rust
fn bench_enhanced_graph_executor(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let store = runtime.block_on(setup_large_db(100_000)); // 100K chunks

    c.bench_function("enhanced_graph_executor", |b| {
        b.iter(|| {
            runtime.block_on(async {
                GraphExecutor::execute_enhanced(&store, repo_id, None, 10, &config).await
            })
        });
    });
}
```

### Ranking Quality Tests

**Scope (Manual Evaluation):**
- 50+ representative queries
- Compare rankings: old vs enhanced
- Subjective quality: "Is top result architecturally important?"

**Test Queries:**
```
1. "authentication handler"
   Expected: Core auth handler (with many callers) ranks #1

2. "validate token"
   Expected: Main validator (not utility function) ranks #1

3. "BaseController"
   Expected: Base class (with many implementations) ranks #1

4. "logger"
   Expected: Active logger (not deprecated) ranks higher

5. "database connection"
   Expected: Connection pool (core component) ranks #1
```

**Evaluation Criteria (Quantitative):**
- **Improved:** Architecturally central code moves from position >3 to position ≤2
- **Same:** Ranking unchanged (position stays ≤3 or stays >3)
- **Worse:** Central code moves from position ≤2 to position >3

**Success Threshold:**
- **Primary Target:** ≥32/50 queries improved (64%)
- **Acceptable:** ≥30/50 queries improved (60%)
- **Failure:** <30/50 improved OR >2/50 degraded (>4% worse)

**Specific Example:**
```
Query: "authentication handler"

Old Ranking:
1. validateTokenFormat (score: 0.85) - utility, 2 callers
2. isTokenValid (score: 0.78) - helper, 1 caller
3. TokenValidator (score: 0.82) - main class, 15 callers

Enhanced Ranking:
1. TokenValidator (score: 0.89) - IMPROVED (position 3 → 1)
2. validateTokenFormat (score: 0.85) - same utility
3. isTokenValid (score: 0.78) - same helper

Result: IMPROVED (central code now #1)
```

## Critical Paths

The following paths MUST be tested:

### 1. Edge Quality Computation
**Why Critical:** Core logic that determines all importance scores
**Tests:**
- All edge types (calls, imports, extends, implements, test_of)
- All source kinds (production, test, edge cases)
- Weight multiplication correctness

### 2. Feature Flag Toggle
**Why Critical:** Rollback mechanism if issues arise
**Tests:**
- Flag=false: Old behavior (no regressions)
- Flag=true: New behavior (enhanced scoring)
- Flag changes take effect without code changes

### 3. Performance Budget
**Why Critical:** Latency regressions break user experience
**Tests:**
- Benchmark on large repository (100K chunks, 500K edges)
- Measure p50, p95, p99 latencies
- Alert if >30ms p95

### 4. Configuration Validation
**Why Critical:** Invalid config should fail fast, not at runtime
**Tests:**
- Valid config loads successfully
- Invalid weights rejected (negative, too large)
- Missing config uses defaults

### 5. Ranking Quality (Manual)
**Why Critical:** Primary value proposition—better rankings
**Tests:**
- Test queries show important code ranks higher
- No regressions on already-good queries

### 6. SQL Query Efficiency
**Why Critical:** Inefficient query kills performance
**Tests:**
- EXPLAIN shows index usage
- No full table scans
- Query plan stable across database sizes

## Test Data Strategy

### Synthetic Graph Data

**For Unit/Integration Tests:**
```rust
// Create test scenario
async fn create_test_scenario(store: &SqliteStore) -> (i64, i64) {
    // Chunk A: Important (10 production callers)
    let chunk_a = create_chunk(store, "AuthHandler", "function").await;
    for _ in 0..10 {
        let caller = create_chunk(store, "ProductionCode", "function").await;
        create_edge(store, caller, chunk_a, "calls").await;
    }

    // Chunk B: Peripheral (10 test callers)
    let chunk_b = create_chunk(store, "formatDate", "function").await;
    for _ in 0..10 {
        let test = create_chunk(store, "test_utils", "test").await;
        create_edge(store, test, chunk_b, "calls").await;
    }

    (chunk_a, chunk_b)
}
```

### Real Database Testing

**For Performance/Ranking Tests:**
- Use actual crewchief repository database
- Index codebase in test environment
- Run queries against real data
- **Benefit:** Realistic edge distribution, actual performance

## Quality Gates

Before ticket verification:

### Code Quality
- [ ] `cargo test` passes (all unit tests)
- [ ] `cargo clippy` clean (no warnings)
- [ ] `cargo fmt` applied (consistent formatting)

### Functional Quality
- [ ] Integration tests pass (enhanced executor)
- [ ] Feature flag toggle works
- [ ] Configuration loads and validates

### Performance Quality
- [ ] Benchmarks meet targets (<30ms p95 graph executor)
- [ ] No performance regressions vs baseline
- [ ] SQL query uses indexes (EXPLAIN verified)

### Ranking Quality (Phase 2+)
- [ ] Test queries show improvement (≥60% better)
- [ ] No significant regressions (≤5% worse)
- [ ] Manual evaluation complete (50+ queries)

## Regression Prevention

### Performance Regression Tests

**Automated:**
```rust
#[test]
fn test_no_latency_regression() {
    let old_latency = benchmark_old_executor();  // Baseline
    let new_latency = benchmark_enhanced_executor();

    let overhead = new_latency - old_latency;
    assert!(overhead < 10_000, "Overhead exceeds 10ms budget: {}μs", overhead);
}
```

**CI Integration:**
- Run benchmarks on every PR
- Compare against main branch baseline
- Fail if regression >20% (configurable threshold)

### Ranking Regression Tests

**Automated (Snapshot Testing):**
```rust
#[test]
fn test_ranking_stability() {
    let queries = load_test_queries();  // 50 representative queries

    for query in queries {
        let results = search_enhanced(query);
        let snapshot = load_snapshot(query.id);

        // Top 3 results should be stable
        assert_eq!(results[0].chunk_id, snapshot.top3[0]);
        assert_eq!(results[1].chunk_id, snapshot.top3[1]);
        assert_eq!(results[2].chunk_id, snapshot.top3[2]);
    }
}
```

## Monitoring After Deploy

### Metrics to Track

**Performance (Required):**
```prometheus
# Latency
histogram_quantile(0.50, graph_executor_latency_seconds{mode="quality_weighted"})
histogram_quantile(0.95, graph_executor_latency_seconds{mode="quality_weighted"})
histogram_quantile(0.99, graph_executor_latency_seconds{mode="quality_weighted"})
histogram_quantile(0.95, search_total_latency_seconds)

# Overhead comparison
histogram_quantile(0.95, graph_executor_latency_seconds{mode="quality_weighted"})
- histogram_quantile(0.95, graph_executor_latency_seconds{mode="legacy"})
```

**Scoring Distribution (Optional):**
```prometheus
# Verify quality weights are applied
graph_score_average{mode="quality_weighted"}
graph_score_average{mode="legacy"}

# Check edge quality distribution
edge_quality_sum{edge_type="calls", source_kind="production"} /
edge_quality_count{edge_type="calls", source_kind="production"}

edge_quality_sum{edge_type="calls", source_kind="test"} /
edge_quality_count{edge_type="calls", source_kind="test"}
```

**Feature Flag Usage (Operational):**
```prometheus
graph_executor_requests_total{mode="quality_weighted"}
graph_executor_requests_total{mode="legacy"}
```

### Alert Thresholds (Specific Values)

**Performance Degradation (WARNING):**
```yaml
- alert: GraphExecutorSlowWarning
  expr: histogram_quantile(0.95, graph_executor_latency_seconds{mode="quality_weighted"}) > 0.035
  for: 5m
  severity: warning
  annotations:
    summary: "Graph executor p95 latency exceeds 35ms (target: 30ms)"
    description: "Current: {{ $value }}s, investigate SQL query performance"
    runbook: "Check EXPLAIN QUERY PLAN, review database size, consider adding indexes"
```

**Performance Degradation (CRITICAL - Consider Rollback):**
```yaml
- alert: GraphExecutorSlowCritical
  expr: histogram_quantile(0.95, graph_executor_latency_seconds{mode="quality_weighted"}) > 0.040
  for: 10m
  severity: critical
  annotations:
    summary: "Graph executor p95 latency exceeds 40ms (critical threshold)"
    description: "Current: {{ $value }}s, CONSIDER ROLLBACK"
    action: "Review metrics, if sustained >40ms, execute rollback procedure"
```

**Total Search Latency Degradation:**
```yaml
- alert: SearchLatencyCritical
  expr: histogram_quantile(0.95, search_total_latency_seconds) > 0.120
  for: 10m
  severity: critical
  annotations:
    summary: "Total search p95 latency exceeds 120ms (alert threshold)"
    description: "Current: {{ $value }}s, investigate all executors"
```

**Error Rate Increase:**
```yaml
- alert: GraphExecutorErrorRate
  expr: rate(graph_executor_errors_total[5m]) > 0.01
  for: 5m
  severity: warning
  annotations:
    summary: "Graph executor error rate >1%"
    description: "Rate: {{ $value }}, check logs for SQL errors"
```

**Feature Flag Unexpectedly Disabled:**
```yaml
- alert: EnhancedScoringDisabled
  expr: |
    graph_executor_requests_total{mode="quality_weighted"}
    / (graph_executor_requests_total{mode="quality_weighted"}
       + graph_executor_requests_total{mode="legacy"}) < 0.01
  for: 10m
  severity: warning
  annotations:
    summary: "Enhanced graph scoring is not being used (<1% of requests)"
    description: "Check config: enable_quality_scoring should be true"
```

### Baseline Values (To Be Established in Phase 2)

**Current System (Before Enhancement):**
- Graph executor p50: TBD (measure in Phase 2)
- Graph executor p95: ~20ms (estimated from architecture doc)
- Graph executor p99: TBD
- Total search p95: TBD
- Error rate: TBD

**Target After Enhancement:**
- Graph executor p50: <20ms
- Graph executor p95: <35ms (acceptable: 30ms, critical: 40ms)
- Graph executor p99: <50ms
- Total search p95: <100ms
- Error rate: <0.5% (same as baseline)

**How to Establish Baseline:**
1. Deploy Phase 1 code with `enable_quality=false`
2. Run in production for 1 week
3. Record p50/p95/p99 latencies from metrics
4. Use these as baseline for comparison when flag enabled

## Testing Anti-Patterns to Avoid

**Don't:**
- ❌ Chase 100% code coverage (waste of time)
- ❌ Test implementation details (brittle tests)
- ❌ Skip performance tests (latency is critical)
- ❌ Use only mocked data (miss real-world edge cases)
- ❌ Manual-only ranking validation (not scalable)

**Do:**
- ✅ Test critical paths thoroughly
- ✅ Test behavior, not implementation
- ✅ Benchmark early and often
- ✅ Mix synthetic + real database tests
- ✅ Automate snapshot tests for ranking stability
