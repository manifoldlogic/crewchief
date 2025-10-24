# Ticket: PERF_OPT-3001: Parallel Indexing

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
Implement multi-threaded parallel indexing with batch processing, pipeline stages, and work stealing to achieve ≥150 files/min indexing throughput target.

## Background
PERF_OPT_ANALYSIS.md (lines 34-38) identifies parallel processing as essential: "Multi-core machines underutilized. I/O bound operations benefit from concurrency. Batch processing reduces overhead. Pipeline parallelism for indexing."

Current indexing is single-threaded, leaving CPU cores idle. Industry research shows 35-57% improvements from parallelization. The target is ≥150 files/min (PERF_OPT_PLAN.md line 122).

The architecture document (PERF_OPT_ARCHITECTURE.md lines 44-66) provides a detailed parallel indexing design with thread pools and batch processing.

## Acceptance Criteria
- [ ] Multi-threaded parsing implemented with configurable worker count
- [ ] Batch processing implemented to reduce overhead
- [ ] Pipeline stages created (read → parse → embed → store)
- [ ] Work stealing implemented for load balancing
- [ ] CPU utilization >70% during indexing
- [ ] Indexing throughput ≥150 files/min achieved
- [ ] No thread contention or deadlocks
- [ ] Memory usage bounded (doesn't grow with parallelism)

## Technical Requirements

### Parallel Indexer Architecture
Implement `ParallelIndexer` (PERF_OPT_ARCHITECTURE.md lines 46-65):

```rust
pub struct ParallelIndexer {
    thread_pool: ThreadPool,
    chunk_size: usize,
}

impl ParallelIndexer {
    pub fn index_files(&self, files: Vec<PathBuf>) -> Result<Stats> {
        let chunks: Vec<Vec<PathBuf>> = files
            .chunks(self.chunk_size)
            .map(|c| c.to_vec())
            .collect();

        let results: Vec<ChunkResult> = chunks
            .par_iter()
            .map(|batch| self.process_batch(batch))
            .collect();

        Ok(self.aggregate_stats(results))
    }
}
```

### Pipeline Stages
Implement pipelined processing:
1. **Read Stage**: Read file contents from disk
2. **Parse Stage**: Parse with tree-sitter
3. **Extract Stage**: Extract chunks and symbols
4. **Embed Stage**: Generate embeddings (if enabled)
5. **Store Stage**: Batch insert to database

Use channels to connect stages:
```rust
let (read_tx, read_rx) = crossbeam::channel::bounded(100);
let (parse_tx, parse_rx) = crossbeam::channel::bounded(100);
let (extract_tx, extract_rx) = crossbeam::channel::bounded(100);
```

### Work Stealing
Use rayon's work-stealing thread pool:
```rust
use rayon::prelude::*;

files.par_iter()
    .with_max_len(50)  // Batch size
    .map(|file| self.process_file(file))
    .collect()
```

### Batch Processing
Batch database operations:
- Batch INSERT for chunks (50-100 per batch)
- Batch INSERT for edges
- Use `unnest()` for array parameters
- Transaction per batch, not per file

### Configuration
Add to config (PERF_OPT_ARCHITECTURE.md lines 191-193):
```yaml
indexing:
  parallel_workers: 8
  batch_size: 50
  max_file_size: 10485760  # 10MB
```

### Error Handling
- Continue processing on single file errors
- Collect errors, don't fail entire batch
- Log problematic files
- Return aggregate statistics with error count

### Memory Management
- Limit in-flight files with bounded channels
- Drop parsed ASTs after extraction
- Pool buffers for file reading
- Clear embeddings after storage

## Implementation Notes

### Thread Pool Setup
```rust
use rayon::ThreadPoolBuilder;

pub fn create_indexer(workers: usize) -> ParallelIndexer {
    let pool = ThreadPoolBuilder::new()
        .num_threads(workers)
        .thread_name(|i| format!("indexer-{}", i))
        .build()
        .unwrap();

    ParallelIndexer::new(pool)
}
```

### Performance Monitoring
Instrument with metrics:
- Files processed per second
- Per-stage timing
- Thread utilization
- Queue depths
- Error rates

### Testing Strategy
Test with various configurations:
- Single-threaded (baseline)
- 2, 4, 8, 16 threads
- Different batch sizes (10, 50, 100)
- Small vs large files
- I/O bound vs CPU bound workloads

### Profiling
Use benchmarks from PERF_OPT-1001 to measure:
- Throughput improvement
- CPU utilization
- Memory usage
- Contention points

### Known Challenges
- Tree-sitter parsers may not be thread-safe (verify)
- Database connection pooling required
- Embedding generation may be bottleneck
- File I/O may saturate before CPU

## Dependencies
- **PERF_OPT-1001** - Requires benchmark suite to measure improvements
- **PERF_OPT-1002** - Requires bottleneck analysis to confirm parallelization is beneficial
- Existing indexing implementation to parallelize
- rayon crate for data parallelism
- crossbeam crate for channels

## Risk Assessment
- **Risk**: Thread contention may reduce benefits
  - **Mitigation**: Use lock-free data structures, measure with benchmarks
- **Risk**: Memory usage may grow with thread count
  - **Mitigation**: Bounded channels, monitor memory, limit in-flight work
- **Risk**: Database connection pool may be exhausted
  - **Mitigation**: Configure pool size ≥ worker count, use PERF_OPT-2002 pool settings
- **Risk**: Parallel overhead may negate benefits for small files
  - **Mitigation**: Dynamic batching, fall back to serial for small workloads

## Files/Packages Affected
- `crates/maproom/Cargo.toml` - Add rayon, crossbeam dependencies
- `crates/maproom/src/indexer/parallel.rs` - New parallel indexer module
- `crates/maproom/src/indexer/pipeline.rs` - New pipeline stages module
- `crates/maproom/src/indexer/mod.rs` - Update to use parallel indexer
- `crates/maproom/src/config.rs` - Add parallelism configuration
- `crates/maproom/benches/indexing.rs` - Update benchmarks to test parallel indexing
