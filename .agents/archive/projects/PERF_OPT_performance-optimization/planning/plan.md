# PERF_OPT Plan: Performance Optimization

## Project Overview
Achieve performance targets through systematic optimization: 150 files/min indexing, <50ms p95 search latency, <120ms context assembly.

## Phase 1: Benchmarking (Week 1)

**Agent: performance-engineer**

### Tasks
1. **Create Benchmark Suite**
   - Indexing throughput tests
   - Search latency measurements
   - Context assembly timing
   - Memory profiling

2. **Identify Bottlenecks**
   - Profile with flamegraphs
   - Database query analysis
   - Memory allocation tracking
   - I/O pattern analysis

**Acceptance Criteria:**
- [ ] Baseline metrics established
- [ ] Bottlenecks identified
- [ ] Profiling infrastructure ready
- [ ] Regression detection setup

## Phase 2: Database Optimization (Week 2)

**Agent: database-engineer + performance-engineer**

### Tasks
1. **Index Optimization**
   - Create missing indices
   - Optimize existing indices
   - Add partial indices
   - Implement covering indices

2. **Query Tuning**
   - EXPLAIN ANALYZE all queries
   - Rewrite slow queries
   - Create materialized views
   - Update table statistics

**Acceptance Criteria:**
- [ ] Query time reduced 50%+
- [ ] Index usage verified
- [ ] No sequential scans
- [ ] Statistics updated

## Phase 3: Parallelization (Week 3)

**Agent: rust-indexer-engineer + performance-engineer**

### Tasks
1. **Parallel Indexing**
   - Multi-threaded parsing
   - Batch processing
   - Pipeline stages
   - Work stealing

2. **Concurrent Operations**
   - Async search queries
   - Parallel edge computation
   - Concurrent file I/O
   - Thread pool tuning

**Acceptance Criteria:**
- [ ] CPU utilization >70%
- [ ] Indexing >150 files/min
- [ ] No thread contention
- [ ] Memory usage bounded

## Phase 4: Caching Implementation (Week 4)

**Agent: performance-engineer**

### Tasks
1. **Cache Systems**
   - Query result cache
   - Embedding cache
   - Context bundle cache
   - Parse tree cache

2. **Cache Management**
   - TTL configuration
   - Eviction policies
   - Cache warming
   - Invalidation logic

**Acceptance Criteria:**
- [ ] Cache hit rate >60%
- [ ] Memory usage <500MB
- [ ] No stale data
- [ ] Measurable speedup

## Phase 5: Final Optimization (Week 5)

**Agent: performance-engineer**

### Tasks
1. **Memory Optimization**
   - String interning
   - Vector quantization
   - Buffer pooling
   - Allocation reduction

2. **Fine Tuning**
   - Connection pooling
   - Batch sizes
   - Buffer sizes
   - Timeout values

**Acceptance Criteria:**
- [ ] All targets met
- [ ] Memory usage optimal
- [ ] No regressions
- [ ] Documentation complete

## Success Metrics
- Indexing: ≥150 files/min
- Search p95: <50ms
- Context p95: <120ms
- Memory: <500MB
- Cache hit: >60%

## Testing Strategy
- Automated benchmarks
- Load testing
- Memory leak detection
- Regression testing
- Real-world workloads