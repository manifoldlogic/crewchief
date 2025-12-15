# Ticket: SRCHREL-2004 - Performance Benchmarking

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- performance-engineer
- verify-ticket
- commit-ticket

## Summary

Benchmark enhanced graph executor on real CrewChief codebase. Measure p50/p95/p99 latencies and validate <35ms p95 target.

## Acceptance Criteria

- [ ] Benchmark on CrewChief repository (real data)
- [ ] Measure graph executor p50, p95, p99 latencies
- [ ] Compare old vs enhanced executor overhead
- [ ] Validate p95 latency <35ms (acceptable), <30ms (target)
- [ ] Measure total search latency p95 <100ms
- [ ] Run EXPLAIN QUERY PLAN to verify index usage
- [ ] Document results in `planning/performance-results.md`
- [ ] If performance exceeds budget, document optimization options

## Technical Requirements

**Benchmark Setup:**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_real_codebase(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let store = runtime.block_on(load_real_database()); // CrewChief DB
    let config = SearchConfig::default_with_quality_enabled();

    c.bench_function("enhanced_graph_executor_real", |b| {
        b.to_async(&runtime).iter(|| async {
            GraphExecutor::execute(
                &store,
                black_box(1),
                black_box(None),
                black_box(10),
                Some(&config),
            ).await
        });
    });
}
```

**Performance Targets:**

| Metric | Target | Acceptable | Critical |
|--------|--------|------------|----------|
| Graph executor p50 | <20ms | <25ms | >30ms |
| Graph executor p95 | <30ms | <35ms | >40ms |
| Graph executor p99 | <50ms | <60ms | >80ms |
| Total search p95 | <100ms | <120ms | >150ms |

## Dependencies

**Prerequisites:**
- SRCHREL-2003 (pipeline integrated)

**Blocks:**
- SRCHREL-2005 (ranking quality evaluation can run in parallel)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Task 2.4, lines 305-310)
