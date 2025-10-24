# Ticket: PERF_OPT-1002: Identify Bottlenecks

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
Profile Maproom with flamegraphs, analyze database queries, track memory allocations, and analyze I/O patterns to identify specific performance bottlenecks that need optimization.

## Background
After establishing baseline metrics in PERF_OPT-1001, we need to identify where the bottlenecks are before optimizing. PERF_OPT_ANALYSIS.md (lines 46-50) emphasizes the importance of profiling before optimizing: "Measure first, optimize second. Focus on hotspots. Consider Amdahl's Law."

Current state (PERF_OPT_ANALYSIS.md lines 19-24):
- No performance benchmarks
- No query optimization
- No caching strategy
- No parallel processing
- Basic indices only

## Acceptance Criteria
- [ ] Bottlenecks identified with specific functions/queries causing slowdowns
- [ ] Flamegraphs generated showing CPU hotspots
- [ ] Database queries analyzed with EXPLAIN ANALYZE
- [ ] Memory allocation patterns tracked and documented
- [ ] I/O bottlenecks identified (file reads, database calls)
- [ ] Prioritized list of optimization opportunities created

## Technical Requirements
- Generate CPU flamegraphs using cargo-flamegraph or perf
- Run EXPLAIN ANALYZE on all critical queries
- Track memory allocations with heaptrack or valgrind
- Measure I/O patterns with strace or equivalent
- Profile async operations with tokio-console
- Document top 10 hotspots by time spent
- Measure database query patterns and frequency
- Identify sequential scans in query plans
- Track lock contention and async blocking
- Analyze thread utilization patterns

## Implementation Notes

### CPU Profiling
Use puffin profiling integration (PERF_OPT_ARCHITECTURE.md lines 178-183) to instrument hot paths:
```rust
#[cfg(feature = "profiling")]
pub fn profile_operation<T>(name: &str, op: impl FnOnce() -> T) -> T {
    puffin::profile_scope!(name);
    op()
}
```

Generate flamegraphs for:
- Full indexing run
- Search queries (various patterns)
- Context assembly operations
- Graph traversal

### Database Query Analysis
Run EXPLAIN ANALYZE on critical queries:
- Chunk search queries
- File lookup queries
- Edge traversal queries
- Statistics computation queries

Look for:
- Sequential scans (should be index scans)
- High cost operations
- Excessive I/O
- Missing statistics

### Memory Profiling
Track allocations during:
- Large repository indexing
- High-frequency search queries
- Context bundle assembly
- Graph operations

Identify:
- Large allocations
- Allocation frequency hotspots
- Memory leaks
- String duplication patterns

### I/O Analysis
Measure:
- File read patterns
- Database query frequency
- Network calls (if any)
- Lock wait times

### Expected Findings
Based on PERF_OPT_ANALYSIS.md (lines 28-44), likely bottlenecks:
- Database: Missing indices, poor query plans
- Parallelism: Single-threaded operations
- Caching: No result caching
- Memory: Repeated allocations, string duplication

### Output
Create `docs/PERFORMANCE_BOTTLENECKS.md` documenting:
1. Top CPU hotspots with % time
2. Slow queries with execution plans
3. Memory allocation patterns
4. I/O bottlenecks
5. Prioritized optimization list

## Dependencies
- **PERF_OPT-1001** - Requires benchmark suite to run profiling tests
- Existing Maproom functionality must be working

## Risk Assessment
- **Risk**: Profiling overhead may give misleading results
  - **Mitigation**: Use sampling profilers, not instrumentation for CPU profiling
- **Risk**: Test workloads may not represent real usage
  - **Mitigation**: Profile with realistic repository sizes and query patterns
- **Risk**: Analysis paralysis - too much data to process
  - **Mitigation**: Focus on top 10 hotspots by time spent, use Amdahl's Law to prioritize

## Files/Packages Affected
- `docs/PERFORMANCE_BOTTLENECKS.md` - New bottleneck analysis document
- `crates/maproom/src/lib.rs` - Add profiling instrumentation
- `crates/maproom/src/indexer.rs` - Add profiling scopes
- `crates/maproom/src/search.rs` - Add profiling scopes
- `crates/maproom/src/database.rs` - Add query logging
- `scripts/profile.sh` - New profiling script
- `scripts/analyze-queries.sql` - New SQL analysis script
