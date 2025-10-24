# Context Assembly Performance Tuning Guide

Version: 1.0.0
Last Updated: 2025-10-24

## Overview

This guide covers performance optimization techniques for the Context Assembly system, including caching strategies, query optimization, parallel processing, and resource management.

## Performance Targets

**Production Performance Goals:**
- p50 assembly latency: <50ms
- p95 assembly latency: <120ms ✓ **ACHIEVED (8.1ms with parallel assembler)**
- p99 assembly latency: <200ms
- Throughput: >100 assemblies/second
- Cache hit rate: >60%

## Quick Wins

### 1. Use Parallel Assembler

**Impact:** 80% latency reduction (5x speedup)

```rust
// ❌ DON'T: Use basic assembler in production
let assembler = BasicContextAssembler::new_without_cache(pool);

// ✓ DO: Use parallel assembler
use crewchief_maproom::context::ParallelContextAssembler;
use crewchief_maproom::context::cache::CacheConfig;

let assembler = ParallelContextAssembler::new(pool, CacheConfig::default());
```

**Benchmark Results:**
- Sequential p95: 40.4ms
- Parallel p95: 8.1ms
- **5x faster for typical queries**

### 2. Enable Caching

**Impact:** 60-80% hit rate reduces assembly time to <1ms for cached queries

```rust
// ✓ DO: Enable caching with appropriate TTL
let cache_config = CacheConfig {
    enabled: true,
    ttl_seconds: 3600,     // 1 hour for production
    max_entries: 5000,     // Larger cache for better hit rate
    hit_rate_target: 0.70, // 70% hit rate goal
};

let assembler = ParallelContextAssembler::new(pool, cache_config);
```

**Cache Performance:**
- Cache hit: <1ms (PostgreSQL lookup)
- Cache miss: ~8ms (full assembly + cache store)
- Cache write: <2ms (async, doesn't block)

### 3. Optimize Expand Options

**Impact:** 30-50% faster assembly by reducing relationship traversal

```rust
// ❌ DON'T: Use max_depth=3 unnecessarily
let slow_options = ExpandOptions {
    max_depth: 3,  // Traverses 3 levels (expensive)
    ..Default::default()
};

// ✓ DO: Use appropriate depth
let fast_options = ExpandOptions {
    max_depth: 1,  // Only direct relationships (fast)
    tests: true,
    callers: false,  // Skip if not needed
    callees: true,
    ..Default::default()
};
```

**Depth Performance:**
- depth=1: ~5ms (fastest)
- depth=2: ~8ms (recommended default)
- depth=3: ~15ms (use sparingly)

## Database Optimization

### Connection Pool Tuning

```rust
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

let pool = PgPoolOptions::new()
    .max_connections(20)              // ✓ 2-5x concurrent assemblers
    .min_connections(5)               // ✓ Keep warm connections
    .acquire_timeout(Duration::from_secs(3))
    .idle_timeout(Duration::from_secs(600))  // 10 min
    .max_lifetime(Duration::from_secs(3600)) // 1 hour
    .connect(&database_url)
    .await?;
```

**Tuning Guidelines:**
- `max_connections`: Set to 2-5x expected concurrent assembly requests
- `min_connections`: Keep 5-10 connections warm for low-latency starts
- `acquire_timeout`: 3-5 seconds prevents indefinite waiting
- `idle_timeout`: 10-15 minutes balances connection recycling
- `max_lifetime`: 1 hour prevents stale connections

### PostgreSQL Configuration

Add these to your `postgresql.conf`:

```ini
# Connection and performance
max_connections = 100
shared_buffers = 256MB           # 25% of RAM for dedicated DB server
effective_cache_size = 1GB       # 50-75% of RAM
work_mem = 16MB                  # Per-operation memory
maintenance_work_mem = 128MB     # For VACUUM, indexes

# Query planning
random_page_cost = 1.1           # SSD optimization
effective_io_concurrency = 200   # SSD optimization

# Write-ahead log
wal_buffers = 16MB
checkpoint_completion_target = 0.9
```

### Index Optimization

The migration `0008_context_query_optimizations.sql` creates optimal indexes:

```sql
-- Composite index for chunk relationships
CREATE INDEX CONCURRENTLY idx_chunk_edges_traversal
ON maproom.chunk_edges(src_chunk_id, dst_chunk_id, edge_type);

-- Covering index for chunk metadata
CREATE INDEX CONCURRENTLY idx_chunks_context_assembly
ON maproom.chunks(id, file_id, symbol_name, kind, start_line, end_line);

-- Test link optimization
CREATE INDEX CONCURRENTLY idx_test_links_lookup
ON maproom.test_links(tested_chunk_id, test_chunk_id);

-- File worktree lookup
CREATE INDEX CONCURRENTLY idx_files_worktree_lookup
ON maproom.files(worktree_id, relpath);

-- Worktree commit lookup
CREATE INDEX CONCURRENTLY idx_worktrees_commit
ON maproom.worktrees(commit_sha);
```

**Verify indexes are used:**
```sql
EXPLAIN ANALYZE
SELECT c.id, c.symbol_name, c.kind, c.start_line, c.end_line
FROM maproom.chunks c
WHERE c.id = $1;

-- Should show "Index Scan using idx_chunks_context_assembly"
```

### Query Performance Monitoring

Monitor slow queries:

```sql
-- Enable slow query logging
ALTER DATABASE maproom SET log_min_duration_statement = 100;  -- Log queries >100ms

-- Check slow queries
SELECT query, mean_exec_time, calls
FROM pg_stat_statements
WHERE mean_exec_time > 50  -- Queries averaging >50ms
ORDER BY mean_exec_time DESC
LIMIT 20;
```

## Caching Strategies

### Cache Hit Rate Optimization

Monitor and tune cache performance:

```rust
use crewchief_maproom::context::cache::ContextCache;

let cache = ContextCache::new(pool, cache_config);

// Monitor hit rate
let stats = cache.stats();
let hit_rate = stats.hit_rate();

if hit_rate < cache_config.hit_rate_target {
    eprintln!(
        "Cache hit rate {:.2}% below target {:.2}%",
        hit_rate * 100.0,
        cache_config.hit_rate_target * 100.0
    );

    // Increase TTL or max_entries
}
```

### TTL Tuning

**Development:**
```rust
let dev_cache = CacheConfig {
    ttl_seconds: 300,      // 5 min (code changes frequently)
    max_entries: 500,
    hit_rate_target: 0.40, // Lower expectation
    ..Default::default()
};
```

**Production:**
```rust
let prod_cache = CacheConfig {
    ttl_seconds: 7200,     // 2 hours (stable code)
    max_entries: 10000,    // Larger cache
    hit_rate_target: 0.75, // Higher expectation
    ..Default::default()
};
```

### Cache Invalidation

Invalidate cache when code changes:

```rust
// After re-indexing a file
cache.invalidate(chunk_id).await?;

// Or invalidate entire cache
sqlx::query("TRUNCATE TABLE maproom.context_cache")
    .execute(&pool)
    .await?;
```

### Cache Storage Optimization

Monitor cache table size:

```sql
-- Check cache size
SELECT
    pg_size_pretty(pg_total_relation_size('maproom.context_cache')) as total_size,
    count(*) as entry_count,
    avg(pg_column_size(bundle)) as avg_bundle_size
FROM maproom.context_cache;

-- Vacuum regularly
VACUUM ANALYZE maproom.context_cache;
```

## Parallel Processing Optimization

### Thread Pool Configuration

```rust
// Configure tokio runtime
let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(8)           // Match CPU cores
    .thread_name("context-asm")
    .thread_stack_size(3 * 1024 * 1024)  // 3MB stack
    .enable_all()
    .build()?;
```

### Concurrent Assembly Limits

Rate-limit concurrent assemblies:

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

let semaphore = Arc::new(Semaphore::new(10));  // Max 10 concurrent

async fn assemble_with_limit(
    assembler: &ParallelContextAssembler,
    chunk_id: i64,
    semaphore: Arc<Semaphore>,
) -> Result<ContextBundle> {
    let _permit = semaphore.acquire().await?;
    assembler.assemble(chunk_id, 6000, ExpandOptions::default()).await
}
```

## Resource Management

### Memory Optimization

**File Loading:**
```rust
// ✓ DO: Stream large files
let content = tokio::fs::read_to_string(&file_path).await?;

// ❌ DON'T: Load entire file into memory if only need excerpt
// (System already optimizes this via line range extraction)
```

**Budget Management:**
```rust
// ✓ DO: Use appropriate budgets
let budgets = vec![
    2000,  // Quick context
    6000,  // Standard (recommended)
    12000, // Comprehensive
];

// ❌ DON'T: Use excessive budgets unnecessarily
let excessive_budget = 50000;  // Rarely needed, slow
```

### Connection Management

```rust
// ✓ DO: Reuse assemblers
let assembler = ParallelContextAssembler::new(pool, cache_config);

// Process many requests with same assembler
for chunk_id in chunk_ids {
    let bundle = assembler.assemble(chunk_id, 6000, options.clone()).await?;
}

// ❌ DON'T: Create new assembler per request
for chunk_id in chunk_ids {
    let assembler = ParallelContextAssembler::new(pool.clone(), cache_config.clone());
    // Creates new connections, slower
}
```

## Benchmarking

### Running Benchmarks

```bash
# Run performance benchmarks
cargo bench --bench context_assembly_bench

# Save baseline
cargo bench --bench context_assembly_bench -- --save-baseline main

# Compare against baseline
cargo bench --bench context_assembly_bench -- --baseline main

# Specific benchmark
cargo bench --bench context_assembly_bench -- parallel_complex
```

### Custom Benchmarks

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_assembly(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_assembly");

    for depth in [1, 2, 3] {
        group.bench_with_input(
            BenchmarkId::new("depth", depth),
            &depth,
            |b, &depth| {
                b.iter(|| {
                    // Assembly with specified depth
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_assembly);
criterion_main!(benches);
```

## Profiling

### CPU Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile assembly
cargo flamegraph --bench context_assembly_bench -- --bench

# View flamegraph.svg in browser
```

### Query Profiling

```sql
-- Enable query timing
\timing on

-- Analyze specific query
EXPLAIN (ANALYZE, BUFFERS)
WITH RECURSIVE related AS (
    SELECT id, 0 as depth, 1.0 as relevance
    FROM maproom.chunks
    WHERE id = $1

    UNION ALL

    SELECT DISTINCT
        e.dst_chunk_id as id,
        r.depth + 1 as depth,
        r.relevance * 0.7 as relevance
    FROM related r
    JOIN maproom.chunk_edges e ON e.src_chunk_id = r.id
    WHERE r.depth < $2
)
SELECT * FROM related;
```

## Performance Monitoring

### Application Metrics

```rust
use std::time::Instant;

let start = Instant::now();
let bundle = assembler.assemble(chunk_id, 6000, options).await?;
let duration = start.elapsed();

// Log metrics
info!(
    "Assembly completed: chunk_id={}, items={}, tokens={}, duration={:?}, truncated={}",
    chunk_id,
    bundle.items.len(),
    bundle.total_tokens,
    duration,
    bundle.truncated
);

// Track p95 latency
if duration.as_millis() > 120 {
    warn!("Assembly exceeded p95 target: {:?}", duration);
}
```

### Prometheus Metrics (Optional)

```rust
use prometheus::{Histogram, Counter, register_histogram, register_counter};

lazy_static! {
    static ref ASSEMBLY_DURATION: Histogram = register_histogram!(
        "context_assembly_duration_seconds",
        "Context assembly duration in seconds"
    ).unwrap();

    static ref CACHE_HITS: Counter = register_counter!(
        "context_cache_hits_total",
        "Total cache hits"
    ).unwrap();

    static ref CACHE_MISSES: Counter = register_counter!(
        "context_cache_misses_total",
        "Total cache misses"
    ).unwrap();
}

// Track metrics
let timer = ASSEMBLY_DURATION.start_timer();
let bundle = assembler.assemble(chunk_id, 6000, options).await?;
timer.observe_duration();

if cache_hit {
    CACHE_HITS.inc();
} else {
    CACHE_MISSES.inc();
}
```

## Performance Checklist

**Before deploying to production:**

- [ ] Use `ParallelContextAssembler` (not `BasicContextAssembler`)
- [ ] Enable caching with appropriate TTL (3600-7200 seconds)
- [ ] Configure connection pool (max_connections >= 20)
- [ ] Apply database indexes from migration 0008
- [ ] Set `max_depth=2` for standard queries (not 3+)
- [ ] Monitor cache hit rate (target >60%)
- [ ] Configure PostgreSQL shared_buffers and work_mem
- [ ] Run benchmarks to establish baseline
- [ ] Set up query performance monitoring
- [ ] Implement metric tracking (latency, throughput)
- [ ] Test with realistic workload and data size
- [ ] Configure rate limiting for concurrent assemblies

## Troubleshooting

**Slow Queries:**
1. Check `EXPLAIN ANALYZE` output
2. Verify indexes are being used
3. Increase shared_buffers or work_mem
4. Consider reducing max_depth

**Low Cache Hit Rate:**
1. Increase TTL (longer cache lifetime)
2. Increase max_entries (larger cache)
3. Check if queries vary (different options = cache miss)
4. Monitor cache evictions

**High Memory Usage:**
1. Reduce max_connections in pool
2. Lower max_entries in cache
3. Use smaller budgets (reduce context size)
4. Check for connection leaks

**High CPU Usage:**
1. Reduce worker_threads in tokio runtime
2. Limit concurrent assemblies with semaphore
3. Use smaller max_depth (less traversal)
4. Enable caching to reduce repeated work

## See Also

- [API Reference](context_assembly_api.md) - Core types and methods
- [Configuration Guide](context_configuration.md) - Configuration options
- [Custom Strategies](custom_strategies.md) - Strategy optimization
- Benchmark Results: `PERFORMANCE_SUMMARY_CONTEXT_ASM_3003.md`
