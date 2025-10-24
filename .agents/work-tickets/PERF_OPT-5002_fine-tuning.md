# Ticket: PERF_OPT-5002: Fine Tuning

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- performance-engineer
- database-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Fine-tune system parameters including connection pooling, batch sizes, buffer sizes, and timeout values to achieve all performance targets: ≥150 files/min indexing, <50ms p95 search, <120ms p95 context, <500MB memory, >60% cache hit rate.

## Background
PERF_OPT_PLAN.md (lines 109-113) identifies final tuning parameters: connection pooling, batch sizes, buffer sizes, and timeout values. This is the final optimization phase where we tune all parameters based on insights from previous tickets.

All previous optimizations (database, parallelization, caching, memory) are in place. This ticket focuses on finding optimal configuration values through systematic testing and tuning.

## Acceptance Criteria
- [ ] All performance targets met (PERF_OPT_PLAN.md lines 121-126):
  - [ ] Indexing: ≥150 files/min
  - [ ] Search p95: <50ms
  - [ ] Context p95: <120ms
  - [ ] Memory: <500MB
  - [ ] Cache hit: >60%
- [ ] Connection pooling optimized
- [ ] Batch sizes tuned for indexing and queries
- [ ] Buffer sizes optimized for I/O
- [ ] Timeout values configured appropriately
- [ ] No performance regressions from baseline
- [ ] Configuration documented with rationale
- [ ] Load testing validates sustained performance

## Technical Requirements

### Connection Pooling
Implement optimized connection pool (PERF_OPT_ARCHITECTURE.md lines 143-161):

```rust
pub struct ConnectionPool {
    pool: bb8::Pool<PostgresConnectionManager>,
}

impl ConnectionPool {
    pub fn new(size: u32) -> Self {
        let manager = PostgresConnectionManager::new(config);
        let pool = bb8::Pool::builder()
            .max_size(size)
            .min_idle(Some(size / 4))
            .max_lifetime(Some(Duration::from_secs(30 * 60)))
            .idle_timeout(Some(Duration::from_secs(10 * 60)))
            .build(manager)
            .await?;

        Self { pool }
    }
}
```

Tune parameters:
- `max_size`: Number of connections (start: 20, tune based on load)
- `min_idle`: Minimum idle connections (max_size / 4)
- `max_lifetime`: Connection lifetime (30 minutes)
- `idle_timeout`: Idle connection timeout (10 minutes)
- `connection_timeout`: Time to wait for connection (5 seconds)

Configuration (PERF_OPT_ARCHITECTURE.md lines 195-198):
```yaml
database:
  pool_size: 20
  statement_timeout: 5000
  work_mem: "256MB"
```

### Batch Sizes
Tune batch sizes for different operations:

1. **Indexing batches** (PERF_OPT_ARCHITECTURE.md line 192):
```yaml
indexing:
  parallel_workers: 8
  batch_size: 50
  max_file_size: 10485760  # 10MB
```

Test batch sizes: 10, 25, 50, 100, 200
- Smaller: Less memory, more overhead
- Larger: More memory, less overhead
- Optimal: Balance throughput and memory

2. **Database INSERT batches**:
```rust
const CHUNK_INSERT_BATCH_SIZE: usize = 100;
const EDGE_INSERT_BATCH_SIZE: usize = 500;
```

3. **Search result batches**:
```rust
const SEARCH_BATCH_SIZE: usize = 50;
const MAX_RESULTS: usize = 100;
```

### Buffer Sizes
Optimize buffer sizes for I/O:

1. **File reading buffer**:
```rust
const FILE_READ_BUFFER_SIZE: usize = 64 * 1024;  // 64KB
```
Test: 4KB, 16KB, 64KB, 256KB

2. **Network buffer**:
```rust
const DB_BUFFER_SIZE: usize = 32 * 1024;  // 32KB
```

3. **Parse buffer**:
```rust
const PARSE_BUFFER_SIZE: usize = 1024 * 1024;  // 1MB
```

### Timeout Values
Configure appropriate timeouts:

1. **Database timeouts**:
```rust
statement_timeout: 5000ms  // Query timeout
lock_timeout: 1000ms       // Lock wait timeout
idle_in_transaction_session_timeout: 30000ms
```

2. **HTTP timeouts** (for future MCP server):
```rust
connect_timeout: 5s
request_timeout: 30s
idle_timeout: 60s
```

3. **Cache TTL** (from PERF_OPT-4002):
```rust
query_ttl: 3600s      // 1 hour
embedding_ttl: 86400s // 24 hours
context_ttl: 1800s    // 30 minutes
```

### Thread Pool Configuration
Tune thread pools:

1. **Tokio runtime**:
```rust
worker_threads: 8  // Number of CPU cores
max_blocking_threads: 16  // For spawn_blocking
thread_stack_size: 2MB
```

2. **Rayon pool** (from PERF_OPT-3001):
```rust
parallel_workers: 8  // Number of CPU cores
```

### Database Configuration
Tune PostgreSQL settings:

```sql
-- Memory settings
shared_buffers = '2GB'                 -- 25% of system RAM
effective_cache_size = '6GB'           -- 50% of system RAM
work_mem = '256MB'                     -- Per-operation memory
maintenance_work_mem = '512MB'         -- For VACUUM, CREATE INDEX

-- Performance settings
random_page_cost = 1.1                 -- SSD optimization
effective_io_concurrency = 200         -- Parallel I/O
max_worker_processes = 8               -- Background workers
max_parallel_workers_per_gather = 4   -- Parallel query workers
max_parallel_workers = 8               -- Total parallel workers

-- Connection settings
max_connections = 100
```

### Load Testing Configuration
Define load testing scenarios:

1. **Indexing load**:
   - 10,000 files, measure throughput
   - Target: ≥150 files/min

2. **Search load**:
   - 1,000 queries, measure latency distribution
   - Target: p95 <50ms

3. **Context load**:
   - 1,000 context assemblies, measure latency
   - Target: p95 <120ms

4. **Memory load**:
   - Index 100k chunks, measure peak memory
   - Target: <500MB

5. **Sustained load**:
   - 24-hour run with mixed operations
   - No memory leaks, consistent performance

## Implementation Notes

### Tuning Methodology
1. **Baseline**: Measure performance with default settings
2. **Single-parameter**: Vary one parameter, measure impact
3. **Grid search**: Test parameter combinations
4. **Validation**: Verify best configuration with load tests
5. **Documentation**: Document optimal values and rationale

### Automated Tuning
Create tuning script:
```bash
#!/bin/bash
# scripts/tune-performance.sh

for batch_size in 10 25 50 100; do
  for workers in 4 8 16; do
    echo "Testing batch_size=$batch_size workers=$workers"
    cargo bench --bench indexing -- --batch-size $batch_size --workers $workers
  done
done
```

### Configuration Template
Create `maproom.config.yaml` template:
```yaml
# Maproom Performance Configuration
# Generated by PERF_OPT-5002 tuning

performance:
  indexing:
    parallel_workers: 8        # Tuned for 8-core CPU
    batch_size: 50             # Optimal throughput/memory balance
    max_file_size: 10485760    # 10MB limit

  database:
    pool_size: 20              # Handles concurrent operations
    statement_timeout: 5000    # 5s query timeout
    work_mem: "256MB"          # Per-operation memory

  cache:
    query_cache_size: 100      # Recent queries
    embedding_cache_size: 1000 # Generated embeddings
    ttl_seconds: 3600          # 1 hour default

  runtime:
    worker_threads: 8          # Tokio workers
    max_blocking_threads: 16   # For blocking ops
    thread_stack_size: 2097152 # 2MB

  buffers:
    file_read_buffer: 65536    # 64KB
    db_buffer: 32768           # 32KB
    parse_buffer: 1048576      # 1MB
```

### Monitoring Integration
Verify targets with metrics from PERF_OPT-1001:
```rust
pub fn verify_targets(metrics: &PerformanceMetrics) -> Result<()> {
    assert!(metrics.indexing_rate >= 150.0, "Indexing rate below target");
    assert!(metrics.search_p95 < 50.0, "Search p95 above target");
    assert!(metrics.context_p95 < 120.0, "Context p95 above target");
    assert!(metrics.memory_mb < 500.0, "Memory usage above target");
    assert!(metrics.cache_hit_rate > 0.6, "Cache hit rate below target");
    Ok(())
}
```

### Regression Testing
Add regression test:
```rust
#[test]
fn test_performance_targets() {
    let metrics = run_benchmark_suite();
    verify_targets(&metrics).expect("Performance targets not met");
}
```

### Documentation
Create `docs/PERFORMANCE_TUNING.md`:
- Explain each parameter
- Document tuning process
- Provide tuning guidelines
- List optimal values for common scenarios

## Dependencies
- **PERF_OPT-1001** - Requires benchmark suite for tuning measurements
- **PERF_OPT-1002** - Requires bottleneck identification
- **PERF_OPT-2001** - Database indices must be optimized
- **PERF_OPT-2002** - Queries must be tuned
- **PERF_OPT-3001** - Parallel indexing must be implemented
- **PERF_OPT-3002** - Concurrent operations must be implemented
- **PERF_OPT-4001** - Caches must be implemented
- **PERF_OPT-4002** - Cache management must be implemented
- **PERF_OPT-5001** - Memory optimizations must be implemented

## Risk Assessment
- **Risk**: Aggressive tuning may work for benchmarks but not production
  - **Mitigation**: Test with realistic workloads, validate with load testing
- **Risk**: Optimal values may be hardware-dependent
  - **Mitigation**: Document tuning process, provide guidelines for different hardware
- **Risk**: Configuration complexity may confuse users
  - **Mitigation**: Provide sensible defaults, document tuning guide
- **Risk**: Performance targets may not be achievable
  - **Mitigation**: Identify bottlenecks, adjust targets if needed, document limitations

## Files/Packages Affected
- `crates/maproom/maproom.config.yaml` - New default configuration file
- `crates/maproom/src/config.rs` - Load and validate configuration
- `crates/maproom/src/tuning/mod.rs` - New tuning utilities module
- `scripts/tune-performance.sh` - New automated tuning script
- `scripts/load-test.sh` - New load testing script
- `docs/PERFORMANCE_TUNING.md` - New tuning documentation
- `crates/maproom/tests/performance_targets.rs` - New regression test
- `crates/maproom/benches/tuning.rs` - New tuning benchmark
