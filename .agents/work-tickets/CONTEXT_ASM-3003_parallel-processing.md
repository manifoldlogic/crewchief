# Ticket: CONTEXT_ASM-3003: Parallel Processing

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
Implement parallel processing optimizations for context assembly to achieve sub-120ms p95 assembly times. This includes concurrent chunk loading, parallel relationship queries, async file reading, and pipeline optimization.

## Background
As context assembly matures in Phase 3, performance optimization becomes critical. The current sequential loading approach creates bottlenecks when assembling context from multiple chunks and their relationships. By implementing parallel loading patterns using tokio's async runtime, we can significantly reduce assembly latency while maintaining correctness. This work is foundational for meeting production performance targets.

## Acceptance Criteria
- [ ] Concurrent chunk loading implemented using tokio::join!
- [ ] Parallel relationship queries (callers, callees, tests) working correctly
- [ ] Async file reading implemented with tokio::fs
- [ ] Pipeline optimization for streaming assembly complete
- [ ] p95 assembly time < 120ms verified through benchmarks
- [ ] Early termination on budget exhaustion working in parallel context
- [ ] No data races or ordering issues in parallel operations

## Technical Requirements
- Use tokio::join! for parallel loading operations
- Load primary chunk, tests, callers, and callees concurrently
- Replace synchronous file I/O with tokio::fs async operations
- Implement pipeline optimization for streaming context assembly
- Maintain correctness guarantees despite concurrent operations
- Handle errors gracefully in parallel contexts (don't fail entire assembly if one item fails)
- Ensure thread safety for shared state (budgets, caches)
- Benchmark before and after to validate performance improvements

## Implementation Notes

### Parallel Loading Architecture (from CONTEXT_ASM_ARCHITECTURE.md, lines 244-250)
```rust
// Load all context components concurrently
let (primary, tests, callers, callees) = tokio::join!(
    self.load_primary(chunk_id),
    self.load_tests(chunk_id),
    self.load_callers(chunk_id),
    self.load_callees(chunk_id)
);
```

### Performance Optimization Strategy (from CONTEXT_ASM_ARCHITECTURE.md, lines 232-242)
- **Streaming**: Stream content from files, progressive assembly, early termination on budget exhaustion
- **Caching**: Cache assembled bundles by (chunk_id, options_hash), cache graph traversals, cache token counts
- **Parallel Loading**: Concurrent loading of all relationship types

### Key Considerations
1. **Error Handling**: Use Result types and handle partial failures gracefully (continue assembly even if some relationships fail to load)
2. **Budget Tracking**: Ensure token budgets are thread-safe and correctly updated across parallel operations
3. **Ordering**: Maintain deterministic output despite parallel loading (sort or use stable collection operations)
4. **Early Termination**: Implement cancellation-safe early termination when budget is exhausted
5. **Benchmarking**: Use criterion.rs for before/after performance comparisons

### Implementation Phases
1. **Phase 1**: Convert file I/O to async (tokio::fs)
2. **Phase 2**: Implement parallel chunk loading with tokio::join!
3. **Phase 3**: Parallelize relationship queries at database level
4. **Phase 4**: Pipeline optimization for streaming assembly
5. **Phase 5**: Benchmark and tune for p95 < 120ms target

## Dependencies
- **CONTEXT_ASM-3001**: Query optimization (provides performance baseline and optimized queries to parallelize)
- **Tokio runtime**: Must be properly configured for async operations
- **Criterion.rs**: For comprehensive benchmarking

## Risk Assessment
- **Risk**: Data races or incorrect ordering in parallel operations
  - **Mitigation**: Use Rust's type system (Arc, Mutex, RwLock) appropriately, write concurrent tests, use tokio's testing utilities

- **Risk**: Diminishing returns from parallelization (too much overhead)
  - **Mitigation**: Benchmark at each phase, only parallelize operations that show measurable benefit

- **Risk**: Increased complexity making debugging harder
  - **Mitigation**: Comprehensive logging with tracing spans, use tokio-console for async debugging

- **Risk**: Budget exhaustion not working correctly in parallel context
  - **Mitigation**: Atomic budget operations, integration tests for budget limits with parallel loading

## Files/Packages Affected
- `crates/maproom/src/context/assembler.rs` - Core parallel loading implementation
- `crates/maproom/src/context/graph.rs` - Parallel relationship queries
- `crates/maproom/src/context/pipeline.rs` - Pipeline optimization (new file)
- `crates/maproom/src/context/budget.rs` - Thread-safe budget tracking
- `crates/maproom/benches/context_assembly_bench.rs` - Performance benchmarks
- `crates/maproom/tests/context_parallel_test.rs` - Concurrent correctness tests (new file)
- `Cargo.toml` - Ensure tokio features enabled (fs, sync, macros)
