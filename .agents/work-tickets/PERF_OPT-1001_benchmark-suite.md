# Ticket: PERF_OPT-1001: Create Benchmark Suite

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- performance-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Create comprehensive benchmark suite to measure indexing throughput, search latency, context assembly timing, and memory usage. Establish baseline metrics for performance optimization work.

## Background
Maproom currently has no performance benchmarks or baseline metrics. Before optimizing the system, we need to establish current performance levels and create automated benchmarks to detect regressions. Industry research shows that proper profiling before optimization is critical (PERF_OPT_ANALYSIS.md lines 46-50).

The benchmark suite must measure the key performance targets:
- Indexing: ≥150 files/min target
- Search p95: <50ms target
- Context p95: <120ms target
- Memory: <500MB target

## Acceptance Criteria
- [ ] Baseline metrics established for indexing, search, context, and memory
- [ ] Profiling infrastructure ready with flamegraph support
- [ ] Regression detection setup to track performance over time
- [ ] Benchmark results documented with current performance levels
- [ ] Automated benchmark suite can be run via `cargo bench`

## Technical Requirements
- Implement indexing throughput tests measuring files/minute
- Create search latency measurements with p50, p95, p99 percentiles
- Build context assembly timing tests
- Add memory profiling to track allocation patterns
- Integrate performance metrics collection system (PERF_OPT_ARCHITECTURE.md lines 167-174)
- Support profiling with puffin integration (PERF_OPT_ARCHITECTURE.md lines 178-183)
- Generate flamegraphs for CPU profiling
- Measure database query performance
- Track I/O patterns and bottlenecks

## Implementation Notes

### Benchmark Infrastructure
Create `crates/maproom/benches/` directory with:
- `indexing.rs` - File indexing throughput tests
- `search.rs` - Search latency measurements
- `context.rs` - Context assembly timing
- `memory.rs` - Memory profiling tests

### Metrics Collection
Implement `PerformanceMetrics` struct (PERF_OPT_ARCHITECTURE.md lines 167-174):
```rust
pub struct PerformanceMetrics {
    indexing_rate: Histogram,
    search_latency: Histogram,
    cache_hit_rate: Gauge,
    memory_usage: Gauge,
    query_throughput: Counter,
}
```

### Profiling Support
Add profiling feature with puffin integration (PERF_OPT_ARCHITECTURE.md lines 178-183) for flamegraph generation.

### Test Data
Use realistic test datasets:
- Small: 100 files (~1MB)
- Medium: 1,000 files (~10MB)
- Large: 10,000 files (~100MB)

### Measurements
Track key metrics from PERF_OPT_PLAN.md (lines 121-126):
- Indexing rate (files/min)
- Search p95 latency (ms)
- Context p95 latency (ms)
- Memory usage (MB)
- Cache hit rate (%)

## Dependencies
- None - this is the first ticket in the performance optimization project
- Requires existing Maproom indexing and search functionality

## Risk Assessment
- **Risk**: Benchmark overhead may affect production code
  - **Mitigation**: Use feature flags to isolate profiling code
- **Risk**: Test data may not represent real-world usage
  - **Mitigation**: Use actual repository samples for benchmarks
- **Risk**: Metrics collection may impact performance
  - **Mitigation**: Make metrics optional with sampling rate configuration

## Files/Packages Affected
- `crates/maproom/Cargo.toml` - Add criterion, puffin dependencies
- `crates/maproom/benches/indexing.rs` - New benchmark file
- `crates/maproom/benches/search.rs` - New benchmark file
- `crates/maproom/benches/context.rs` - New benchmark file
- `crates/maproom/benches/memory.rs` - New benchmark file
- `crates/maproom/src/metrics.rs` - New metrics collection module
- `crates/maproom/src/lib.rs` - Export metrics types
