# Ticket: [SRCHREL-3001]: Performance Regression Tests and CI Integration

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- performance-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create automated performance regression tests that validate the <20ms overhead budget and integrate them into CI to catch performance regressions before merge.

## Background
Performance is a hard constraint for relationship expansion. Automated regression tests ensure the <20ms overhead budget is maintained as the codebase evolves. These tests complement the benchmarks from SRCHREL-1004 by adding pass/fail criteria and CI integration.

This implements Phase 3 deliverables: performance regression tests, overhead validation, CI integration.

## Acceptance Criteria
- [ ] Performance regression test suite created (Rust or TypeScript)
- [ ] Test validates p95 overhead <20ms with realistic data
- [ ] Test validates response size <10KB for typical queries
- [ ] Test fails if performance budget exceeded
- [ ] CI workflow updated to run performance tests
- [ ] Performance test documentation includes threshold justification
- [ ] Tests run successfully in CI on pull requests
- [ ] Parallel traversal optimization implemented if sequential exceeds budget

## Technical Requirements

### Rust Performance Test
Create `crates/maproom/tests/performance_regression_test.rs`:

```rust
#[tokio::test]
async fn test_relationship_expansion_overhead_budget() {
    let store = setup_production_like_db().await;

    // Baseline: 10 searches without relationships
    let mut baseline_latencies = Vec::new();
    for _ in 0..10 {
        let start = Instant::now();
        execute_search(&store, "test query", false, true).await;
        baseline_latencies.push(start.elapsed());
    }

    // With relationships: 10 searches with relationships
    let mut relationship_latencies = Vec::new();
    for _ in 0..10 {
        let start = Instant::now();
        execute_search(&store, "test query", true, true).await;
        relationship_latencies.push(start.elapsed());
    }

    // Calculate p95 (9th percentile of 10 samples)
    baseline_latencies.sort();
    relationship_latencies.sort();

    let baseline_p95 = baseline_latencies[8];
    let relationship_p95 = relationship_latencies[8];

    let overhead = relationship_p95.saturating_sub(baseline_p95);

    println!("Baseline p95: {:?}", baseline_p95);
    println!("With relationships p95: {:?}", relationship_p95);
    println!("Overhead: {:?}", overhead);

    // HARD CONSTRAINT: <20ms overhead
    assert!(
        overhead < Duration::from_millis(20),
        "Overhead {:?} exceeds 20ms budget", overhead
    );
}

#[tokio::test]
async fn test_response_size_budget() {
    let store = setup_production_like_db().await;

    let response = execute_search(&store, "common query", true, true).await;

    // Serialize to JSON
    let json = serde_json::to_string(&response).unwrap();
    let size_bytes = json.len();

    println!("Response size: {} bytes", size_bytes);

    // HARD CONSTRAINT: <10KB response
    assert!(
        size_bytes < 10 * 1024,
        "Response size {} bytes exceeds 10KB budget", size_bytes
    );
}

#[tokio::test]
async fn test_max_concurrent_expansions_performance() {
    let store = setup_production_like_db().await;

    // Search with 5+ high-confidence results (cap at 3 expansions)
    let start = Instant::now();
    execute_search(&store, "common term", true, true).await;
    let elapsed = start.elapsed();

    println!("Search with capped expansions: {:?}", elapsed);

    // Even with cap, total overhead should be reasonable
    assert!(
        elapsed < Duration::from_millis(100),
        "Capped search took {:?}, exceeds 100ms", elapsed
    );
}
```

### CI Workflow Integration
Update `.github/workflows/test.yml`:

```yaml
jobs:
  performance-tests:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run performance regression tests
        run: cargo test --test performance_regression_test -- --nocapture

      - name: Fail on performance regression
        if: failure()
        run: |
          echo "Performance regression detected!"
          echo "Overhead exceeds 20ms budget or response size exceeds 10KB."
          exit 1
```

### Parallel Traversal Optimization (Contingency)
If sequential traversal exceeds budget, implement parallel:

```rust
use futures::future::join_all;

const MAX_CONCURRENT_EXPANSIONS: usize = 3;

if options.include_related.unwrap_or(false) {
    let high_conf_results: Vec<_> = results.iter_mut()
        .filter(|r| qualifies(r))
        .take(MAX_CONCURRENT_EXPANSIONS)
        .collect();

    let futures: Vec<_> = high_conf_results.iter()
        .map(|r| find_top_related_chunks(store, r.chunk_id, 5))
        .collect();

    let related_results = join_all(futures).await;

    for (result, related) in high_conf_results.iter_mut().zip(related_results) {
        match related {
            Ok(chunks) => result.related = Some(chunks),
            Err(e) => tracing::warn!("Failed to find related chunks: {}", e),
        }
    }
}
```

## Implementation Notes

Performance test strategy:
- Use production-like database (realistic chunk count, edge density)
- Multiple runs (10+) to account for variance
- p95 instead of average (captures tail latency)
- Document environment (CI runner specs)

CI integration considerations:
- Performance tests must be deterministic (no flaky failures)
- Document acceptable variance (e.g., ±5ms acceptable)
- Provide clear failure messages with actual measurements

Response size testing:
- Serialize full response to JSON
- Measure byte count
- Validate typical queries (10 results, 3 with relationships)

Parallel traversal decision:
- Only implement if sequential exceeds budget
- Benchmark both approaches
- Document trade-offs (database load vs latency)

## Dependencies
- SRCHREL-1004 (benchmarks provide baseline measurements)
- SRCHREL-2003 (integration tests must pass first)

## Risk Assessment
- **Risk**: CI runner performance varies, causing flaky tests
  - **Mitigation**: Use p95 instead of max; allow ±5ms variance; run multiple iterations
- **Risk**: Parallel traversal increases database load unacceptably
  - **Mitigation**: Benchmark database concurrency; cap at 3 concurrent queries

## Files/Packages Affected
- `crates/maproom/tests/performance_regression_test.rs` (new file)
- `.github/workflows/test.yml` (add performance test job)
- `crates/maproom/src/search/pipeline.rs` (if parallel traversal needed)

## Verification Notes
The verify-ticket agent should check:
- Performance tests run successfully: `cargo test --test performance_regression_test -- --nocapture`
- Test output shows baseline, relationship, and overhead measurements
- Tests fail if budget exceeded (validate with artificially slow implementation)
- CI workflow includes performance test job
- Documentation explains thresholds and variance allowance
- If parallel traversal implemented:
  - Benchmark comparison documented (sequential vs parallel)
  - Database load impact measured
  - Trade-offs justified
