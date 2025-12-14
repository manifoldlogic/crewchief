# Ticket: [SRCHREL-1004]: Rust Performance Benchmarks

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
Implement performance benchmarks for relationship expansion to validate the <20ms overhead budget and establish baseline latency measurements.

## Background
The relationship expansion feature has a hard performance constraint: <20ms p95 overhead. Benchmarks must measure baseline search latency, relationship expansion overhead, and validate that 3 concurrent expansions stay within budget. These benchmarks will run in CI to catch performance regressions.

This implements Phase 1 deliverables: performance benchmarks and baseline latency measurement.

## Acceptance Criteria
- [ ] Benchmark suite created at `crates/maproom/benches/search_relationships.rs`
- [ ] Baseline search latency benchmark (without relationships)
- [ ] Relationship expansion overhead benchmark (delta measurement)
- [ ] Graph traversal scaling benchmark (10, 100, 1000 edges per chunk)
- [ ] All benchmarks run successfully with `cargo bench --bench search_relationships`
- [ ] Benchmark results validate <20ms overhead for 3 concurrent expansions at p95
- [ ] Documentation includes interpretation of benchmark results

## Technical Requirements

### Benchmark Suite Structure
Create `crates/maproom/benches/search_relationships.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crewchief_maproom::search::*;

fn baseline_search_latency(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let store = runtime.block_on(setup_benchmark_db());

    c.bench_function("baseline_search_no_relationships", |b| {
        b.iter(|| {
            runtime.block_on(async {
                execute_search(&store, "test query", false, true).await
            })
        })
    });
}

fn relationship_expansion_overhead(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let store = runtime.block_on(setup_benchmark_db());

    c.bench_function("search_with_relationships", |b| {
        b.iter(|| {
            runtime.block_on(async {
                execute_search(&store, "test query", true, true).await
            })
        })
    });
}

fn graph_traversal_scaling(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    for edge_count in [10, 100, 1000] {
        let store = runtime.block_on(setup_db_with_edges(edge_count));

        c.bench_function(&format!("graph_traversal_{}_edges", edge_count), |b| {
            b.iter(|| {
                runtime.block_on(async {
                    find_top_related_chunks(&store, black_box(1), 5).await
                })
            })
        });
    }
}

criterion_group!(
    benches,
    baseline_search_latency,
    relationship_expansion_overhead,
    graph_traversal_scaling
);
criterion_main!(benches);
```

### Benchmark Database Setup
- Create in-memory SQLite database with realistic data
- ~1000 chunks with varying edge counts
- Mix of edge types (import, call, extends)
- Mix of chunk kinds (function, class, test)

### Success Criteria for p95 Overhead
- Measure p95 latency for both benchmarks
- Calculate overhead: `p95_with_relationships - p95_baseline`
- Verify overhead < 20ms
- Document results in benchmark output

## Implementation Notes

Use Criterion.rs for benchmarking:
- Add to `Cargo.toml`: `[dev-dependencies] criterion = "0.5"`
- Configure `[[bench]]` section in Cargo.toml:
```toml
[[bench]]
name = "search_relationships"
harness = false
```

Benchmark interpretation:
- Baseline establishes normal search latency (e.g., 50ms p95)
- Overhead is the delta (e.g., 68ms - 50ms = 18ms overhead)
- Scaling benchmark validates depth-2 traversal remains bounded

Realistic test data:
- Use production database schema
- Populate with representative chunk counts (~1000 chunks)
- Create varied graph structures (linear chains, fan-out, cycles)

CI integration (deferred to Phase 3):
- Phase 1: Manual benchmark execution
- Phase 3: Automated CI benchmark with performance regression detection

## Dependencies
- SRCHREL-1003 (search pipeline integration must be complete to benchmark)

## Risk Assessment
- **Risk**: Benchmark setup overhead dominates actual measurement
  - **Mitigation**: Use `black_box()` to prevent compiler optimization; measure setup separately
- **Risk**: In-memory database doesn't reflect production performance
  - **Mitigation**: Document limitation; Phase 3 can add production DB benchmark
- **Risk**: p95 overhead exceeds 20ms budget
  - **Mitigation**: If sequential exceeds budget, implement parallel traversal in Phase 3

## Files/Packages Affected
- `crates/maproom/benches/search_relationships.rs` (new file)
- `crates/maproom/Cargo.toml` (add benchmark configuration)

## Verification Notes
The verify-ticket agent should check:
- Benchmarks run successfully: `cargo bench --bench search_relationships`
- Benchmark output shows p50, p95, p99 latencies
- Overhead calculation documented in comments or README
- Benchmark validates <20ms overhead target (or documents if exceeded with mitigation plan)
- All three benchmark functions present (baseline, overhead, scaling)
- Benchmark database setup is realistic (not trivial data)
