# Ticket: PERF_OPT-3002: Concurrent Operations

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- performance-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement concurrent search operations using async/await, parallel edge computation, concurrent file I/O, and thread pool tuning to maximize throughput and minimize latency.

## Background
After parallelizing indexing in PERF_OPT-3001, we need to parallelize search and other read operations. The search target is p95 <50ms (PERF_OPT_PLAN.md line 123), which requires concurrent execution of FTS, vector, and graph searches.

The architecture document (PERF_OPT_ARCHITECTURE.md lines 68-78) provides a parallel search strategy using tokio::join! to run multiple search strategies concurrently.

## Acceptance Criteria
- [ ] Async search queries implemented (FTS, vector, graph run concurrently)
- [ ] Parallel edge computation implemented for graph traversal
- [ ] Concurrent file I/O implemented for reading multiple files
- [ ] Thread pool tuned for optimal performance
- [ ] Search p95 latency <50ms achieved
- [ ] Context assembly p95 latency <120ms achieved
- [ ] No blocking operations in async code
- [ ] Proper error handling for concurrent operations

## Technical Requirements

### Parallel Search Implementation
Implement concurrent search (PERF_OPT_ARCHITECTURE.md lines 70-78):

```rust
pub async fn parallel_search(&self, query: ProcessedQuery) -> Results {
    let (fts, vector, graph) = tokio::join!(
        self.fts_search(&query),
        self.vector_search(&query),
        self.graph_search(&query)
    );

    self.fuse_results(vec![fts?, vector?, graph?])
}
```

Benefits:
- All three search strategies run concurrently
- Total latency = max(fts, vector, graph) instead of sum
- Better CPU utilization

### Parallel Edge Computation
For graph traversal, compute edges concurrently:
```rust
pub async fn compute_edges_parallel(&self, chunk_ids: Vec<i64>) -> Vec<Edge> {
    let futures: Vec<_> = chunk_ids
        .into_iter()
        .map(|id| self.get_edges_for_chunk(id))
        .collect();

    let results = futures::future::join_all(futures).await;
    results.into_iter().flatten().collect()
}
```

### Concurrent File I/O
Read multiple files concurrently for context assembly:
```rust
pub async fn read_files_concurrent(&self, paths: Vec<PathBuf>) -> Vec<FileContent> {
    let futures: Vec<_> = paths
        .into_iter()
        .map(|path| tokio::fs::read_to_string(path))
        .collect();

    futures::future::join_all(futures).await
        .into_iter()
        .filter_map(Result::ok)
        .collect()
}
```

### Thread Pool Tuning
Configure tokio runtime:
```rust
#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() {
    // Application code
}
```

Or use runtime builder:
```rust
let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(8)
    .thread_name("maproom-worker")
    .enable_all()
    .build()
    .unwrap();
```

### Database Async Operations
Use tokio-postgres for async database queries:
```rust
pub async fn search_chunks(&self, query: &str) -> Result<Vec<Chunk>> {
    let rows = self.client
        .query("SELECT * FROM maproom.chunks WHERE ...", &[query])
        .await?;

    Ok(rows.into_iter().map(|row| Chunk::from_row(row)).collect())
}
```

### Avoiding Blocking
- Use `spawn_blocking` for CPU-intensive work:
  ```rust
  let result = tokio::task::spawn_blocking(|| {
      // CPU-intensive parsing or computation
  }).await?;
  ```
- Use async file I/O: `tokio::fs`
- Use async database clients: `tokio-postgres`, `sqlx`
- Avoid `std::sync::Mutex` in async code, use `tokio::sync::Mutex`

## Implementation Notes

### Search Pipeline
Concurrent search pipeline:
1. Parse query (sync)
2. Run searches concurrently:
   - FTS search (async DB query)
   - Vector search (async DB query)
   - Graph search (async DB query + edge computation)
3. Fuse results (sync computation)
4. Rank and filter (sync computation)
5. Return results

### Context Assembly Pipeline
Concurrent context assembly:
1. Get chunk IDs from search
2. Fetch chunks concurrently from database
3. Read file contents concurrently
4. Assemble context bundles
5. Return context

### Error Handling
Handle errors in concurrent operations:
```rust
let results = tokio::join!(op1, op2, op3);
match results {
    (Ok(r1), Ok(r2), Ok(r3)) => Ok(combine(r1, r2, r3)),
    _ => Err("One or more operations failed")
}
```

Or use `try_join!` to fail fast:
```rust
let (r1, r2, r3) = tokio::try_join!(op1, op2, op3)?;
```

### Performance Monitoring
Add async instrumentation:
- Use `tracing` crate for async spans
- Use `tokio-console` for runtime analysis
- Monitor task blocking time
- Track concurrent operation count

### Benchmarking
Benchmark improvements from PERF_OPT-1001:
- Sequential vs concurrent search
- Single-threaded vs multi-threaded
- Measure latency percentiles (p50, p95, p99)
- Measure throughput (queries per second)

### Configuration
Add runtime configuration (PERF_OPT_ARCHITECTURE.md lines 191-193):
```yaml
runtime:
  worker_threads: 8
  max_blocking_threads: 16
  thread_stack_size: 2097152  # 2MB
```

## Dependencies
- **PERF_OPT-1001** - Requires benchmark suite to measure improvements
- **PERF_OPT-1002** - Requires bottleneck analysis to identify async opportunities
- **PERF_OPT-3001** - Parallel indexing provides foundation for concurrent patterns
- tokio runtime for async execution
- futures crate for async combinators
- Database connection pooling from PERF_OPT-2002

## Risk Assessment
- **Risk**: Too many concurrent operations may overwhelm database
  - **Mitigation**: Use connection pooling from PERF_OPT-2002, limit concurrency with semaphores
- **Risk**: Blocking operations may starve async runtime
  - **Mitigation**: Use `spawn_blocking` for CPU-intensive work, audit for blocking calls
- **Risk**: Error in one concurrent operation may fail entire request
  - **Mitigation**: Use `join!` instead of `try_join!` where appropriate, handle partial failures
- **Risk**: Async overhead may negate benefits for fast operations
  - **Mitigation**: Benchmark sequential vs concurrent, use concurrent only where beneficial

## Files/Packages Affected
- `crates/maproom/Cargo.toml` - Add tokio, futures dependencies
- `crates/maproom/src/search/concurrent.rs` - New concurrent search module
- `crates/maproom/src/search/mod.rs` - Update to use concurrent search
- `crates/maproom/src/database/async.rs` - New async database operations
- `crates/maproom/src/context/concurrent.rs` - New concurrent context assembly
- `crates/maproom/src/config.rs` - Add runtime configuration
- `crates/maproom/benches/search.rs` - Update benchmarks to test concurrent search
- `crates/maproom/src/main.rs` - Configure tokio runtime
