# Performance Tuning Guide

This guide explains how to tune Maproom's performance parameters to achieve optimal performance for your specific workload and hardware configuration.

## Table of Contents

1. [Performance Targets](#performance-targets)
2. [Configuration Overview](#configuration-overview)
3. [Tuning Methodology](#tuning-methodology)
4. [Parameter Reference](#parameter-reference)
5. [Hardware-Specific Guidelines](#hardware-specific-guidelines)
6. [Monitoring and Validation](#monitoring-and-validation)
7. [Common Issues](#common-issues)

## Performance Targets

The default configuration is tuned to meet these targets (from PERF_OPT_PLAN.md):

- **Indexing**: ≥150 files/min (cold cache), ≥500 files/min (warm cache)
- **Search p95**: <50ms
- **Context p95**: <120ms
- **Memory**: <500MB
- **Cache hit rate**: >60%

## Configuration Overview

Maproom's performance is controlled by five main configuration sections:

### 1. Indexing Configuration (`indexing`)

Controls parallel file processing and batch sizes:

```yaml
indexing:
  parallel_workers: 8        # Number of parallel indexing workers
  batch_size: 50             # Files per batch
  max_file_size: 10485760    # 10MB maximum file size
  chunk_insert_batch_size: 100   # Database INSERT batch size
  edge_insert_batch_size: 500    # Edge INSERT batch size
```

### 2. Database Configuration (`database`)

Controls connection pooling and query timeouts:

```yaml
database:
  pool_size: 20                    # Maximum connections
  connection_timeout_ms: 5000      # Connection acquisition timeout
  statement_timeout_ms: 5000       # Query timeout
  lock_timeout_ms: 1000            # Lock wait timeout
  idle_in_transaction_timeout_ms: 30000  # Idle transaction timeout
  work_mem: "256MB"                # PostgreSQL per-operation memory
  max_connection_lifetime_secs: 1800  # 30 minutes
  idle_connection_timeout_secs: 600   # 10 minutes
```

### 3. Runtime Configuration (`runtime`)

Controls thread pools and async runtime:

```yaml
runtime:
  worker_threads: 8              # Tokio worker threads
  max_blocking_threads: 16       # Blocking operation threads
  thread_stack_size: 2097152     # 2MB thread stack
  enable_thread_names: true      # Enable thread naming for debugging
```

### 4. Buffer Configuration (`buffers`)

Controls I/O buffer sizes:

```yaml
buffers:
  file_read_buffer: 65536        # 64KB file read buffer
  db_buffer: 32768               # 32KB database buffer
  parse_buffer: 1048576          # 1MB parse buffer
  buffer_pool_size: 100          # Maximum pooled buffers
```

### 5. Cache Configuration (`cache`)

Controls multi-layer caching:

```yaml
cache:
  l1_query:
    max_entries: 100
    ttl_seconds: 3600            # 1 hour
    enabled: true
  l2_embedding:
    max_entries: 1000
    ttl_seconds: 86400           # 24 hours
    enabled: true
  l3_context:
    max_entries: 500
    ttl_seconds: 1800            # 30 minutes
    enabled: true
  parse_tree:
    max_entries: 200
    ttl_seconds: 0               # Never expire
    enabled: true
```

## Tuning Methodology

Follow this systematic approach to tune performance:

### 1. Establish Baseline

First, measure current performance with default settings:

```bash
# Run all benchmarks
cargo bench --bench indexing
cargo bench --bench search_benchmark
cargo bench --bench context_assembly_bench

# Run load tests
cargo test --test load_test -- --ignored --nocapture

# Validate performance targets
cargo test --test performance_targets -- --ignored --nocapture
```

Record baseline metrics:
- Indexing throughput (files/min)
- Search p50/p95/p99 latency
- Context assembly p50/p95/p99 latency
- Memory usage (peak and average)
- Cache hit rates (L1/L2/L3)

### 2. Single-Parameter Tuning

Vary one parameter at a time and measure impact:

#### Example: Tuning Parallel Workers

```bash
# Test different worker counts
for workers in 4 8 12 16; do
  echo "Testing with $workers workers"
  # Update config or use environment variable
  INDEXING_PARALLEL_WORKERS=$workers cargo bench --bench indexing
done
```

Record results in a table:

| Workers | Files/Min | Memory MB | CPU % |
|---------|-----------|-----------|-------|
| 4       | 120       | 300       | 50%   |
| 8       | 180       | 350       | 75%   |
| 12      | 195       | 450       | 85%   |
| 16      | 200       | 550       | 90%   |

**Analysis**: 8 workers provide optimal throughput/memory tradeoff for this hardware.

### 3. Parameter Interactions

Test combinations of related parameters:

```bash
# Test batch size × worker combinations
for batch in 25 50 100; do
  for workers in 4 8 16; do
    echo "batch=$batch workers=$workers"
    # Run benchmark with these settings
  done
done
```

### 4. Load Testing Validation

After tuning, validate with realistic workloads:

```bash
# Run sustained load test
cargo test --test load_test test_sustained_load -- --ignored --nocapture

# Run burst load test
cargo test --test load_test test_burst_load -- --ignored --nocapture
```

### 5. Regression Testing

Compare tuned configuration against baseline:

```bash
# Save baseline
cargo bench --bench indexing -- --save-baseline before

# Apply tuning changes
# Edit config file or set environment variables

# Compare against baseline
cargo bench --bench indexing -- --baseline before
```

## Parameter Reference

### Indexing Parameters

#### `parallel_workers` (default: 8)

Number of parallel workers for file indexing.

**Impact**:
- **Higher**: More parallel processing, higher throughput, more memory
- **Lower**: Less parallelism, lower throughput, less memory

**Tuning guidelines**:
- Start with number of CPU cores (e.g., 8 for 8-core CPU)
- Monitor CPU utilization (target 70-80%)
- Watch for memory pressure (stay under 500MB target)
- Test values: `num_cpus`, `num_cpus * 1.5`, `num_cpus * 2`

**System-specific**:
- **8-core CPU**: 8-12 workers optimal
- **4-core CPU**: 4-6 workers optimal
- **16-core CPU**: 12-16 workers optimal

#### `batch_size` (default: 50)

Number of files processed in each batch.

**Impact**:
- **Larger batches**: Less overhead, more memory, higher latency per batch
- **Smaller batches**: More overhead, less memory, lower latency per batch

**Tuning guidelines**:
- Balance throughput and memory usage
- Test values: 10, 25, 50, 100, 200
- Monitor memory usage during indexing
- Watch for database connection pool exhaustion

**System-specific**:
- **Low memory (<4GB)**: 25-50
- **Medium memory (4-8GB)**: 50-100
- **High memory (>8GB)**: 100-200

#### `chunk_insert_batch_size` (default: 100)

Number of chunks inserted in a single database batch.

**Impact**:
- **Larger**: Fewer round trips, better throughput, more memory
- **Smaller**: More round trips, lower throughput, less memory

**Tuning guidelines**:
- Test values: 50, 100, 200, 500
- Monitor database connection time
- Watch for transaction timeout errors
- Consider network latency to database

#### `edge_insert_batch_size` (default: 500)

Number of edges inserted in a single database batch.

**Impact**: Same as `chunk_insert_batch_size` but edges are smaller objects.

**Tuning guidelines**:
- Can be larger than chunk batch since edges are smaller
- Test values: 200, 500, 1000
- Monitor database transaction time

### Database Parameters

#### `pool_size` (default: 20)

Maximum number of database connections in pool.

**Impact**:
- **Larger**: More concurrent queries, higher database overhead
- **Smaller**: Connection exhaustion risk, query queuing

**Tuning guidelines**:
- Monitor connection pool utilization (<80% healthy)
- Test values: 10, 20, 30, 50
- Watch PostgreSQL `max_connections` setting
- Consider concurrent search queries + indexing operations

**Calculation**:
```
pool_size = (concurrent_searches * 2) + (indexing_workers * 2) + margin
Example: (5 * 2) + (8 * 2) + 10 = 36
```

#### `statement_timeout_ms` (default: 5000)

Maximum time for a single query to execute.

**Impact**:
- **Higher**: Complex queries succeed, slow queries not caught
- **Lower**: Fast failure, risk of legitimate query timeout

**Tuning guidelines**:
- Set based on p99 latency of your slowest query type
- Test values: 1000, 3000, 5000, 10000
- Monitor query timeout errors
- Use EXPLAIN ANALYZE to profile slow queries

#### `work_mem` (default: "256MB")

PostgreSQL per-operation memory allocation.

**Impact**:
- **Higher**: Faster sorts and hashes, more memory per connection
- **Lower**: More disk spills, slower operations

**Tuning guidelines**:
- Formula: `work_mem = total_ram / (max_connections * 3)`
- Example: 16GB / (100 * 3) = ~54MB per connection
- Monitor PostgreSQL temp file usage
- Test values: "64MB", "128MB", "256MB", "512MB"

### Runtime Parameters

#### `worker_threads` (default: 8)

Number of Tokio async worker threads.

**Impact**:
- **More threads**: Better async concurrency, more context switching
- **Fewer threads**: Less overhead, potential async bottlenecks

**Tuning guidelines**:
- Set to number of CPU cores
- Monitor CPU utilization per core
- Watch for thread contention
- Test values: `num_cpus`, `num_cpus + 2`, `num_cpus * 2`

#### `max_blocking_threads` (default: 16)

Maximum threads for blocking operations (spawn_blocking).

**Impact**:
- **More threads**: More concurrent blocking ops, more memory
- **Fewer threads**: Blocking operation queuing

**Tuning guidelines**:
- Set to 2× worker_threads
- Monitor blocking operation wait times
- Test values: `worker_threads`, `worker_threads * 2`, `worker_threads * 3`

### Buffer Parameters

#### `file_read_buffer` (default: 64KB)

Buffer size for file reading operations.

**Impact**:
- **Larger**: Fewer read syscalls, more memory per file
- **Smaller**: More syscalls, less memory

**Tuning guidelines**:
- Test values: 4KB, 16KB, 64KB, 256KB
- Monitor I/O wait time
- Consider typical file sizes
- SSD vs HDD: Larger buffers help HDDs more

**System-specific**:
- **SSD**: 64KB-128KB optimal
- **HDD**: 128KB-256KB optimal
- **Network storage**: 256KB+ may help

#### `parse_buffer` (default: 1MB)

Buffer size for parser intermediate operations.

**Impact**:
- **Larger**: Can parse larger constructs, more memory
- **Smaller**: May need multiple allocations for large files

**Tuning guidelines**:
- Set based on largest file you'll parse
- Test values: 512KB, 1MB, 2MB
- Monitor parser memory allocations

### Cache Parameters

#### `l1_query.max_entries` (default: 100)

Number of search results to cache.

**Impact**:
- **More entries**: Higher hit rate, more memory
- **Fewer entries**: Lower memory, more cache misses

**Tuning guidelines**:
- Monitor cache hit rate (target >60%)
- Consider query uniqueness (how many unique queries?)
- Test values: 50, 100, 200, 500
- Watch memory usage (each entry ~5-50KB)

**Memory calculation**:
```
L1 memory ≈ max_entries × avg_result_size
Example: 100 × 20KB = 2MB
```

#### `l1_query.ttl_seconds` (default: 3600)

How long to cache search results.

**Impact**:
- **Longer**: Higher hit rate, stale results risk
- **Shorter**: Fresher results, lower hit rate

**Tuning guidelines**:
- Consider index update frequency
- Balance freshness vs performance
- Test values: 300 (5min), 1800 (30min), 3600 (1hr)
- Monitor cache expiration rate

#### `l2_embedding.max_entries` (default: 1000)

Number of embedding vectors to cache.

**Impact**:
- **More entries**: Fewer embedding API calls, more memory
- **Fewer entries**: More API calls, less memory

**Tuning guidelines**:
- Embeddings are expensive to generate (API cost + latency)
- Each embedding ~6KB (1536 dimensions × 4 bytes)
- Test values: 500, 1000, 5000, 10000
- Monitor embedding cache hit rate

**Memory calculation**:
```
L2 memory ≈ max_entries × dimension × 4 bytes
Example: 1000 × 1536 × 4 = 6.1MB
```

#### `l3_context.max_entries` (default: 500)

Number of context bundles to cache.

**Impact**:
- **More entries**: Faster context assembly, more memory
- **Fewer entries**: More assembly operations, less memory

**Tuning guidelines**:
- Context bundles vary in size (5-50KB)
- Monitor context cache hit rate
- Test values: 100, 500, 1000
- Consider how often contexts are reused

## Hardware-Specific Guidelines

### Low-End System (4 cores, 4GB RAM, HDD)

Optimize for memory and I/O:

```yaml
indexing:
  parallel_workers: 4
  batch_size: 25
  chunk_insert_batch_size: 50
  edge_insert_batch_size: 200

database:
  pool_size: 10
  work_mem: "64MB"

runtime:
  worker_threads: 4
  max_blocking_threads: 8

buffers:
  file_read_buffer: 131072    # 128KB for HDD
  parse_buffer: 524288        # 512KB
  buffer_pool_size: 50

cache:
  l1_query:
    max_entries: 50
  l2_embedding:
    max_entries: 500
  l3_context:
    max_entries: 100
```

**Expected performance**:
- Indexing: 80-100 files/min
- Search p95: 60-80ms
- Memory: 250-300MB

### Mid-Range System (8 cores, 8GB RAM, SSD)

Default configuration (already optimized):

**Expected performance**:
- Indexing: 150-200 files/min
- Search p95: 40-50ms
- Memory: 350-450MB

### High-End System (16 cores, 32GB RAM, NVMe SSD)

Maximize throughput:

```yaml
indexing:
  parallel_workers: 16
  batch_size: 100
  chunk_insert_batch_size: 200
  edge_insert_batch_size: 1000

database:
  pool_size: 40
  work_mem: "512MB"

runtime:
  worker_threads: 16
  max_blocking_threads: 32

buffers:
  file_read_buffer: 131072    # 128KB
  parse_buffer: 2097152       # 2MB
  buffer_pool_size: 200

cache:
  l1_query:
    max_entries: 500
  l2_embedding:
    max_entries: 10000
  l3_context:
    max_entries: 2000
```

**Expected performance**:
- Indexing: 300-500 files/min
- Search p95: 20-30ms
- Memory: 500-800MB

### Cloud Environment (Variable CPU, Network Storage)

Optimize for network I/O and variable resources:

```yaml
indexing:
  parallel_workers: 8  # Conservative for shared CPU
  batch_size: 50
  chunk_insert_batch_size: 200  # Larger batches for network DB
  edge_insert_batch_size: 1000

database:
  pool_size: 30
  connection_timeout_ms: 10000  # Higher for network latency
  statement_timeout_ms: 10000   # Higher for network latency
  work_mem: "256MB"

buffers:
  file_read_buffer: 262144      # 256KB for network storage
  db_buffer: 65536              # 64KB for network DB
```

## Monitoring and Validation

### Performance Metrics to Track

1. **Indexing Metrics**:
   - Files per minute throughput
   - Chunks created per file
   - Parse time distribution (p50/p95/p99)
   - Database insertion time
   - Memory usage during indexing

2. **Search Metrics**:
   - Query latency distribution (p50/p95/p99)
   - Queries per second (QPS)
   - Cache hit rates per layer
   - Database query time
   - Result fusion time

3. **System Metrics**:
   - CPU utilization (per core and overall)
   - Memory usage (RSS, heap, stack)
   - Database connections (active, idle, waiting)
   - I/O wait time
   - Network latency (if remote database)

### Validation Commands

```bash
# Validate all performance targets
cargo test --test performance_targets -- --ignored --nocapture

# Run specific benchmark with metrics
cargo bench --bench indexing -- --verbose

# Profile memory usage
cargo bench --bench memory -- --profile-time=60

# Load test with monitoring
cargo test --test load_test -- --ignored --nocapture --test-threads=1
```

### PostgreSQL Monitoring Queries

```sql
-- Check connection pool usage
SELECT count(*), state
FROM pg_stat_activity
WHERE datname = 'your_database'
GROUP BY state;

-- Check slow queries
SELECT query, mean_exec_time, calls
FROM pg_stat_statements
WHERE query LIKE '%maproom%'
ORDER BY mean_exec_time DESC
LIMIT 10;

-- Check cache hit ratio
SELECT
  sum(heap_blks_read) as heap_read,
  sum(heap_blks_hit) as heap_hit,
  sum(heap_blks_hit) / (sum(heap_blks_hit) + sum(heap_blks_read)) as ratio
FROM pg_statio_user_tables
WHERE schemaname = 'maproom';
```

## Common Issues

### Issue: High Memory Usage

**Symptoms**: Memory usage exceeds 500MB target

**Diagnosis**:
```bash
# Check memory breakdown
cargo test --test performance_tests test_memory_breakdown -- --ignored

# Profile heap allocations
cargo bench --bench memory -- --profile-heap
```

**Solutions**:
1. Reduce `parallel_workers` (less concurrent work)
2. Reduce `batch_size` (smaller working set)
3. Reduce cache sizes (`max_entries`)
4. Reduce `buffer_pool_size`

### Issue: Slow Indexing (<150 files/min)

**Symptoms**: Indexing throughput below target

**Diagnosis**:
```bash
# Profile indexing
cargo flamegraph --bench indexing

# Check CPU utilization
htop  # or top
```

**Solutions**:
1. Increase `parallel_workers` (if CPU <70%)
2. Increase `batch_size` (reduce overhead)
3. Increase `chunk_insert_batch_size` (fewer DB round trips)
4. Check database connection pool (may be exhausted)
5. Profile parser (may be slow for certain file types)

### Issue: Search Latency Too High (p95 >50ms)

**Symptoms**: Search p95 latency exceeds target

**Diagnosis**:
```bash
# Profile search queries
cargo bench --bench search_benchmark -- --profile

# Check database query plans
# In PostgreSQL:
EXPLAIN ANALYZE SELECT ...;
```

**Solutions**:
1. Check cache hit rates (should be >60%)
2. Increase cache sizes for higher hit rate
3. Check database indices (should cover common queries)
4. Reduce `max_candidates_per_method` (less work per query)
5. Increase `work_mem` for PostgreSQL (faster sorts)
6. Consider materialized views for expensive joins

### Issue: Connection Pool Exhaustion

**Symptoms**: "Failed to acquire database connection" errors

**Diagnosis**:
```bash
# Monitor pool stats
# Add logging to see pool utilization
```

**Solutions**:
1. Increase `pool_size` (more concurrent connections)
2. Reduce `parallel_workers` (fewer concurrent operations)
3. Reduce `connection_timeout_ms` (fail faster)
4. Check for connection leaks (connections not returned)
5. Increase PostgreSQL `max_connections`

### Issue: Cache Hit Rate Too Low (<60%)

**Symptoms**: Cache hit rate below target

**Diagnosis**:
```bash
# Check cache statistics
cargo test --test cache_effectiveness -- --ignored
```

**Solutions**:
1. Increase cache sizes (`max_entries`)
2. Increase TTL values (cache longer)
3. Analyze query patterns (may be too unique)
4. Consider cache warming strategies
5. Check cache eviction rate (may be too aggressive)

## Advanced Tuning

### Automated Tuning Script

```bash
#!/bin/bash
# scripts/auto-tune.sh

echo "Automated Performance Tuning"
echo "==========================="

# Test different worker counts
for workers in 4 8 12 16; do
  export INDEXING_PARALLEL_WORKERS=$workers
  echo "Testing workers=$workers"
  cargo bench --bench indexing -- --quiet | grep "time:"
done

# Find optimal batch size
for batch in 25 50 100 200; do
  export INDEXING_BATCH_SIZE=$batch
  echo "Testing batch=$batch"
  cargo bench --bench indexing -- --quiet | grep "time:"
done

# Validate final configuration
cargo test --test performance_targets -- --ignored
```

### Grid Search Tuning

For more sophisticated tuning, use grid search:

```python
# scripts/grid_search.py
import subprocess
import itertools

# Parameter ranges
workers = [4, 8, 12, 16]
batches = [25, 50, 100, 200]
pools = [10, 20, 30, 40]

results = []
for w, b, p in itertools.product(workers, batches, pools):
    # Set environment variables
    env = {
        'INDEXING_PARALLEL_WORKERS': str(w),
        'INDEXING_BATCH_SIZE': str(b),
        'DATABASE_POOL_SIZE': str(p)
    }

    # Run benchmark
    result = subprocess.run(
        ['cargo', 'bench', '--bench', 'indexing'],
        env=env, capture_output=True
    )

    # Parse and record results
    # ...
    results.append((w, b, p, throughput, memory))

# Find optimal combination
best = max(results, key=lambda x: x[3])  # Max throughput
print(f"Optimal: workers={best[0]}, batch={best[1]}, pool={best[2]}")
```

## Conclusion

Performance tuning is an iterative process:

1. **Measure** baseline performance
2. **Identify** bottlenecks through profiling
3. **Tune** one parameter at a time
4. **Validate** improvements with benchmarks
5. **Monitor** in production
6. **Iterate** as workload changes

Use this guide as a starting point, but always validate tuning decisions with your specific workload, data, and hardware configuration.

For more information:
- [PERF_OPT_PLAN.md](../../.agents/archive/projects/PERF_OPT_performance-optimization/planning/PERF_OPT_PLAN.md) - Performance optimization plan
- [PERF_OPT_ARCHITECTURE.md](../../.agents/archive/projects/PERF_OPT_performance-optimization/planning/PERF_OPT_ARCHITECTURE.md) - Architecture decisions
- [Performance Tests](../tests/performance/) - Test suite documentation
- [Benchmarks](../benches/) - Benchmark suite documentation
