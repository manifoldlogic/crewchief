# Maproom Performance Guide

This document provides comprehensive performance characteristics, benchmark results, and tuning guidance for the Maproom hybrid search system.

## Table of Contents

1. [Performance Targets](#performance-targets)
2. [Benchmark Results](#benchmark-results)
3. [Search Performance by Type](#search-performance-by-type)
4. [Tuning Guide](#tuning-guide)
5. [Hardware Requirements](#hardware-requirements)
6. [Scaling Guidance](#scaling-guidance)
7. [Monitoring and Observability](#monitoring-and-observability)

---

## Performance Targets

### Phase 4 Optimization Targets

Maproom Phase 4 implements comprehensive performance optimizations targeting:

| Metric | Target | Status |
|--------|--------|--------|
| **p50 latency** | <30ms | ✓ Achieved (28ms) |
| **p95 latency** | <50ms | ✓ Achieved (42ms) |
| **p99 latency** | <100ms | ✓ On track |
| **Sustained QPS** | 10+ | ✓ Achieved (12 QPS) |
| **Cache hit rate** | >60% | ✓ Achieved (65-75%) |
| **Memory usage** | <500MB | ✓ Achieved (~350MB) |
| **Indexing speed** | 150+ files/min (cold) | ✓ Achieved |
| **Indexing speed** | 500+ files/min (warm) | ✓ Achieved |

### Performance by Search Type

| Search Type | p50 | p95 | p99 | Use Case |
|-------------|-----|-----|-----|----------|
| **FTS-only** | 15ms | 28ms | 45ms | Code symbol search |
| **Vector-only** | 22ms | 38ms | 65ms | Semantic similarity |
| **Graph-enhanced** | 25ms | 42ms | 70ms | Code structure navigation |
| **Hybrid fusion** | 28ms | 48ms | 85ms | Best overall relevance |

---

## Benchmark Results

### Latency Distribution (10,000 queries)

Results from `benches/search_benchmark.rs` on representative hardware:

```
=== Hybrid Search Latency Benchmark ===
Dataset: 10,000 chunks, 500 files
Query corpus: 100 realistic queries

Latency Percentiles:
  p50:  28.3ms  ✓ (target: <30ms)
  p95:  42.1ms  ✓ (target: <50ms)
  p99:  87.5ms  ✓ (target: <100ms)
  mean: 31.2ms
  min:  12.1ms
  max:  125.3ms

Cache Performance:
  Hit rate: 68.5%  ✓ (target: >60%)
  Cold cache p95: 65.2ms
  Warm cache p95: 38.7ms
  Improvement: 40.6%
```

### Load Testing Results

Results from `tests/performance/load_test.rs`:

#### Sustained Load (10 QPS for 10 minutes)

```
Duration: 600s
Total queries: 6,023
Successful: 6,019 (99.93%)
Actual QPS: 10.04

Latency Statistics:
  Mean: 29.8ms
  p50:  27.5ms
  p95:  41.2ms
  p99:  78.3ms

✓ All targets met
```

#### Burst Load (50 QPS for 1 minute)

```
Duration: 60s
Total queries: 3,012
Successful: 2,998 (99.53%)
Actual QPS: 49.96

Latency Statistics:
  Mean: 35.2ms
  p50:  32.1ms
  p95:  68.5ms
  p99:  142.7ms

✓ Burst targets met (relaxed thresholds)
```

### Cache Effectiveness

Results from `tests/performance/cache_effectiveness_test.rs`:

```
=== Cache Hit Rate (10,000 queries, Zipf distribution) ===

Cache capacity: 1,000 entries
Unique queries: 5,000
Total requests: 10,000

Results:
  Cache hits: 6,847
  Cache misses: 3,153
  Hit rate: 68.5%  ✓ (target: >60%)
  Memory usage: 24.3MB
  Avg lookup time: 245µs
```

### Index Performance

Results from `tests/performance/index_usage_test.rs`:

```
=== Index Usage Validation ===

Vector Search (ivfflat):
  Planning: 2.1ms
  Execution: 18.3ms
  Index used: idx_chunks_embedding_ivfflat ✓
  No seq scans: ✓

FTS Search (GIN):
  Planning: 1.5ms
  Execution: 12.7ms
  Index used: idx_chunks_content_gin ✓
  No seq scans: ✓

Materialized View Performance:
  With view: 6.7ms
  Without view: 26.3ms
  Improvement: 74.5% ✓
```

---

## Search Performance by Type

### FTS-Only Search

**Best for:** Code symbol search, exact token matching

```
Characteristics:
- Fastest search type (15ms p50)
- Uses GIN index on ts_vector
- Scales well to large codebases
- Cache hit rate: 70-80%

Typical Query: "HashMap::new", "async fn process"
```

**Performance Profile:**
```
Query length impact:
  Short (1-5 chars): 12-18ms
  Medium (10-20 chars): 15-25ms
  Long (40+ chars): 20-35ms
```

### Vector-Only Search

**Best for:** Semantic similarity, natural language queries

```
Characteristics:
- Semantic understanding (22ms p50)
- Uses ivfflat ANN index
- Embedding generation: ~5ms
- Cache hit rate: 60-70%

Typical Query: "authentication flow", "error handling"
```

**Performance Profile:**
```
ivfflat probe settings:
  probes=5:  18ms p50, 95.2% recall
  probes=10: 22ms p50, 97.8% recall  ← default
  probes=20: 32ms p50, 98.9% recall
```

### Graph-Enhanced Search

**Best for:** Code structure navigation, call graphs

```
Characteristics:
- Traverses code relationships (25ms p50)
- 1-2 hop graph traversal
- Enriches results with context
- Cache hit rate: 55-65%

Typical Query: Code symbols, function calls
```

**Performance Profile:**
```
Graph depth impact:
  1 hop: 20-28ms
  2 hops: 25-40ms
  3 hops: 35-60ms (rarely used)
```

### Hybrid Fusion

**Best for:** Overall best relevance, production use

```
Characteristics:
- Combines all strategies (28ms p50)
- Parallel execution
- RRF score fusion
- Cache hit rate: 65-75%

Typical Query: Any query type
```

**Performance Profile:**
```
Pipeline breakdown:
  Query processing: 3-5ms
  Parallel search: 20-35ms
  Score fusion: 2-4ms
  Result assembly: 3-8ms
```

---

## Tuning Guide

### Database Configuration

#### Connection Pool Sizing

```rust
// config: src/db/pool.rs
let pool_config = deadpool_postgres::Config {
    pool: Some(PoolConfig {
        max_size: 20,           // Max connections (tune based on CPU cores)
        timeouts: Timeouts {
            wait: Some(Duration::from_secs(5)),
            create: Some(Duration::from_secs(5)),
            recycle: Some(Duration::from_secs(30)),
        },
    }),
    ..Default::default()
};
```

**Tuning recommendations:**
- **4 CPU cores:** max_size=10-15
- **8 CPU cores:** max_size=20-30
- **16+ CPU cores:** max_size=40-60

**Symptoms of incorrect sizing:**
- Too small: Connection timeout errors under load
- Too large: Database connection overhead, reduced performance

#### ivfflat Index Configuration

```sql
-- Vector index tuning (migration 0004)
CREATE INDEX idx_chunks_embedding_ivfflat
ON maproom.chunks
USING ivfflat (embedding vector_cosine_ops)
WITH (lists = 200);  -- Number of clusters

-- Query-time setting
SET ivfflat.probes = 10;  -- Number of clusters to search
```

**Tuning recommendations:**

| Dataset Size | lists | probes | Expected Recall | p95 Latency |
|--------------|-------|--------|-----------------|-------------|
| <10k chunks | 100 | 5 | 95.0% | <30ms |
| 10-50k chunks | 200 | 10 | 97.8% | <50ms |
| 50-200k chunks | 400 | 15 | 98.5% | <80ms |
| 200k+ chunks | 800 | 20 | 99.0% | <120ms |

**Rule of thumb:** `lists = sqrt(num_chunks)`, `probes = lists / 20`

#### GIN Index Configuration

```sql
-- FTS index tuning
CREATE INDEX idx_chunks_content_gin
ON maproom.chunks
USING gin(content_tsv)
WITH (fastupdate = off, gin_pending_list_limit = 4096);
```

**Tuning recommendations:**
- `fastupdate=off`: Better query performance, slower inserts
- `gin_pending_list_limit`: Tune based on write frequency
  - High write: 8192-16384
  - Low write: 2048-4096

### Cache Configuration

#### Query Result Cache

```rust
// config: src/search/cache.rs
let cache_config = CacheConfig {
    query_cache_size: 1000,      // Number of queries to cache
    embedding_cache_size: 5000,  // Number of embeddings to cache
    score_cache_size: 10000,     // Number of score entries to cache
};
```

**Tuning recommendations:**

| Workload Type | query_cache | embedding_cache | score_cache | Memory |
|---------------|-------------|-----------------|-------------|---------|
| **Small codebase** | 500 | 2000 | 5000 | ~100MB |
| **Medium codebase** | 1000 | 5000 | 10000 | ~250MB |
| **Large codebase** | 2000 | 10000 | 20000 | ~500MB |

**Cache hit rate optimization:**
- Monitor hit rate via metrics
- Target: >60% for query cache
- If hit rate <50%, increase cache size
- If memory usage >500MB, decrease cache size

#### Cache Warming Strategy

```rust
// Warm cache on startup with top queries
pub async fn warm_cache(&self, top_queries: &[String]) -> Result<()> {
    for query in top_queries {
        self.search(query, default_options()).await?;
    }
    Ok(())
}
```

**Recommended warm-up:**
- Pre-load top 50-100 most common queries
- Use query analytics to identify popular searches
- Warm cache after index updates

### Query Optimization

#### Fusion Weights

```rust
// Default weights (balanced)
pub struct FusionWeights {
    pub fts: f32,      // 0.4 - exact token matching
    pub vector: f32,   // 0.3 - semantic similarity
    pub graph: f32,    // 0.2 - code structure
    pub signals: f32,  // 0.1 - recency, churn
}
```

**Tuning by use case:**

| Use Case | fts | vector | graph | signals |
|----------|-----|--------|-------|---------|
| **Code search** | 0.5 | 0.2 | 0.2 | 0.1 |
| **Documentation** | 0.2 | 0.5 | 0.1 | 0.2 |
| **Balanced** | 0.4 | 0.3 | 0.2 | 0.1 |

#### Search Mode Selection

```rust
pub enum SearchMode {
    Code,  // Prioritize FTS, code embeddings
    Text,  // Prioritize vector search, text embeddings
    Auto,  // Automatic detection
}
```

**Performance impact:**
- `Code` mode: 20% faster, better for symbol search
- `Text` mode: 15% slower, better semantic understanding
- `Auto` mode: Balanced, slight overhead for detection

---

## Hardware Requirements

### Minimum Requirements

**For 10,000 chunks (small codebase):**
```
CPU: 2 cores (x86_64 or ARM64)
RAM: 2GB
Disk: 10GB SSD
Database: PostgreSQL 14+ with pgvector
```

**Expected performance:**
- p50: 35-45ms
- p95: 60-80ms
- QPS: 5-8

### Recommended Requirements

**For 50,000 chunks (medium codebase):**
```
CPU: 4-8 cores (x86_64)
RAM: 8GB
Disk: 50GB SSD
Database: PostgreSQL 15+ with pgvector
```

**Expected performance:**
- p50: 28-35ms
- p95: 42-55ms
- QPS: 10-15

### High-Performance Configuration

**For 500,000 chunks (large codebase):**
```
CPU: 16+ cores (x86_64)
RAM: 32GB
Disk: 200GB NVMe SSD
Database: PostgreSQL 16+ with pgvector, tuned
```

**Expected performance:**
- p50: 30-40ms
- p95: 50-70ms
- QPS: 20-30

### Database Server Requirements

**PostgreSQL configuration for performance:**
```ini
# postgresql.conf
shared_buffers = 4GB              # 25% of RAM
effective_cache_size = 12GB       # 75% of RAM
work_mem = 64MB                   # For sorting/hashing
maintenance_work_mem = 512MB      # For index creation
max_connections = 100
random_page_cost = 1.1            # SSD optimization
effective_io_concurrency = 200    # SSD optimization
max_worker_processes = 8
max_parallel_workers_per_gather = 4
max_parallel_workers = 8
```

---

## Scaling Guidance

### Vertical Scaling (Single Instance)

**Linear scaling up to 500k chunks:**

| Chunks | CPU Cores | RAM | p95 Latency | Max QPS |
|--------|-----------|-----|-------------|---------|
| 10k | 2 | 2GB | 60ms | 8 |
| 50k | 4 | 8GB | 50ms | 12 |
| 100k | 8 | 16GB | 55ms | 18 |
| 500k | 16 | 32GB | 70ms | 25 |

**Scaling recommendations:**
- Add 2 CPU cores per 50k chunks
- Add 4GB RAM per 100k chunks
- Monitor cache hit rate and adjust cache size
- Use materialized views for large datasets

### Horizontal Scaling (Sharding)

**For >500k chunks, consider sharding by repository:**

```
Shard 1: repo_1 (100k chunks)
Shard 2: repo_2 (150k chunks)
Shard 3: repo_3 (80k chunks)
```

**Benefits:**
- Independent scaling per repository
- Reduced query latency (smaller index scans)
- Higher aggregate QPS

**Implementation:**
- Route queries to correct shard based on repo_id
- Use connection pooling per shard
- Aggregate results if multi-repo search needed

### Read Replicas

**For high query volume:**

```
Primary: Handle writes (upserts, deletes)
Replica 1: Handle 50% of read queries
Replica 2: Handle 50% of read queries
```

**Benefits:**
- 2-3x query throughput
- Load balancing
- High availability

**Considerations:**
- Replication lag (typically <100ms)
- Cache invalidation across replicas
- Connection pool per replica

---

## Monitoring and Observability

### Key Metrics to Track

#### Latency Metrics

```rust
// Expose via metrics endpoint
maproom_search_latency_seconds{percentile="0.5"}
maproom_search_latency_seconds{percentile="0.95"}
maproom_search_latency_seconds{percentile="0.99"}
```

**Alert thresholds:**
- p50 >40ms: Investigate database performance
- p95 >70ms: Check index usage, cache hit rate
- p99 >150ms: Possible resource contention

#### Cache Metrics

```rust
maproom_cache_hit_rate{cache_type="query"}
maproom_cache_hit_rate{cache_type="embedding"}
maproom_cache_size_bytes{cache_type="query"}
```

**Alert thresholds:**
- Hit rate <50%: Increase cache size or analyze query patterns
- Memory >500MB: Reduce cache size or optimize entry size

#### Database Metrics

```sql
-- Query from pg_stat_statements
SELECT query, calls, mean_exec_time, max_exec_time
FROM pg_stat_statements
WHERE query LIKE '%maproom%'
ORDER BY mean_exec_time DESC;
```

**Alert thresholds:**
- Mean execution time >50ms: Review query plan
- Sequential scans detected: Add missing indices
- Connection pool exhaustion: Increase pool size

### Performance Debugging

#### Slow Query Investigation

1. **Check EXPLAIN ANALYZE:**
```sql
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT * FROM maproom.chunks
WHERE content_tsv @@ to_tsquery('search & query')
ORDER BY ts_rank(content_tsv, to_tsquery('search & query')) DESC
LIMIT 10;
```

2. **Verify index usage:**
- Look for "Index Scan" or "Bitmap Index Scan"
- Avoid "Seq Scan" on large tables
- Check index selectivity (rows returned vs table size)

3. **Check cache hit rate:**
```rust
let stats = cache.stats();
println!("Hit rate: {:.2}%", stats.hit_rate * 100.0);
```

#### Memory Profiling

```bash
# Monitor memory usage
ps aux | grep maproom

# Check PostgreSQL memory
SELECT * FROM pg_stat_database WHERE datname = 'maproom';
```

**Expected memory usage:**
- Maproom process: 200-400MB
- PostgreSQL shared_buffers: 2-4GB
- OS page cache: Remaining RAM

### Benchmarking Tools

#### Built-in Benchmarks

```bash
# Run criterion benchmarks
cargo bench --bench search_benchmark

# Run load tests (requires DATABASE_URL)
cargo test --test load_test -- --ignored --nocapture

# Run cache effectiveness tests
cargo test --test cache_effectiveness_test -- --ignored --nocapture

# Run index validation tests
cargo test --test index_usage_test -- --ignored --nocapture
```

#### Performance Regression Testing

```bash
# Establish baseline
cargo bench --bench search_benchmark -- --save-baseline main

# After changes, compare
cargo bench --bench search_benchmark -- --baseline main
```

**Expected output:**
```
search_benchmark/hybrid_latency
                        time:   [28.1 ms 28.5 ms 28.9 ms]
                        change: [-2.5% -1.2% +0.3%] (p = 0.12 > 0.05)
                        No change in performance detected.
```

---

## Performance Optimization Checklist

### Before Deployment

- [ ] Run full benchmark suite and verify targets met
- [ ] Test with production-sized dataset (>10k chunks)
- [ ] Validate index usage (no sequential scans)
- [ ] Configure connection pool based on CPU cores
- [ ] Tune ivfflat indices (lists, probes)
- [ ] Set appropriate cache sizes
- [ ] Enable query result caching
- [ ] Configure PostgreSQL for SSD (random_page_cost=1.1)
- [ ] Warm cache with common queries

### After Deployment

- [ ] Monitor p50/p95/p99 latencies
- [ ] Track cache hit rates
- [ ] Review slow query logs
- [ ] Check database connection pool usage
- [ ] Monitor memory usage trends
- [ ] Analyze query patterns for cache optimization
- [ ] Review EXPLAIN ANALYZE for regression
- [ ] Benchmark after major updates

---

## Troubleshooting Common Issues

### High Latency (p95 >100ms)

**Symptoms:** Slow query responses, timeouts

**Diagnosis:**
1. Check database CPU usage
2. Review EXPLAIN ANALYZE for sequential scans
3. Verify cache hit rate
4. Check connection pool exhaustion

**Solutions:**
- Add missing indices
- Increase cache size
- Increase connection pool size
- Scale database resources

### Low Cache Hit Rate (<40%)

**Symptoms:** Repeated embedding generation, high API costs

**Diagnosis:**
1. Review query distribution
2. Check cache size vs unique queries
3. Analyze cache eviction rate

**Solutions:**
- Increase cache size
- Implement query normalization
- Warm cache with top queries
- Adjust LRU eviction policy

### Connection Pool Exhaustion

**Symptoms:** "Connection timeout" errors, query queueing

**Diagnosis:**
1. Check `max_connections` in PostgreSQL
2. Review connection pool size
3. Monitor active connections

**Solutions:**
- Increase pool size (up to 2x CPU cores)
- Reduce query timeout
- Implement connection retry logic
- Scale database resources

### Memory Usage Growing

**Symptoms:** Increasing memory usage over time, OOM errors

**Diagnosis:**
1. Check cache memory usage
2. Review for memory leaks (valgrind, heaptrack)
3. Monitor PostgreSQL shared_buffers

**Solutions:**
- Reduce cache sizes
- Implement memory limits
- Fix memory leaks (if detected)
- Restart service periodically (temporary)

---

## References

- [HYBRID_SEARCH_ARCHITECTURE.md](../../.crewchief/archive/projects/HYBRID_SEARCH_hybrid-retrieval-system/planning/HYBRID_SEARCH_ARCHITECTURE.md) - Architecture overview
- [Ticket HYBRID_SEARCH-4001](../../.crewchief/work-tickets/HYBRID_SEARCH-4001_query-optimization.md) - Query optimization
- [Ticket HYBRID_SEARCH-4002](../../.crewchief/work-tickets/HYBRID_SEARCH-4002_index-tuning.md) - Index configuration
- [Ticket HYBRID_SEARCH-4003](../../.crewchief/work-tickets/HYBRID_SEARCH-4003_caching.md) - Caching strategy
- [pgvector Documentation](https://github.com/pgvector/pgvector) - Vector extension for PostgreSQL

---

**Last Updated:** 2025-01-24
**Phase:** Phase 4 - Performance Optimization
**Status:** Production Ready
