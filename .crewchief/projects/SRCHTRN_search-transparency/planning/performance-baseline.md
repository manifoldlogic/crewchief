# Performance Baseline - SRCHTRN Project

**Date**: 2025-12-13
**Measured By**: Claude (general agent)
**Commit**: 7047dc6bc13e2ea1a8c1f2b6443eeddbd7f52717

## Purpose

This baseline establishes current search performance metrics before implementing the Search Transparency project. These measurements will be used in Phase 2 to validate that metadata assembly overhead remains <10ms and that overall p95 latency stays <100ms.

## Test Setup

- **Repository**: crewchief (workspace)
- **Indexed chunks**: 23,232
- **Embedding provider**: None (FTS-only mode, embeddings failed due to dimension mismatch)
- **Search mode**: FTS (full-text search only)
- **Query count**: 76 representative queries
- **Result limit**: 10 results per query
- **Test workload**: See [test-workload.txt](test-workload.txt)

## Latency Metrics

### Percentiles (wall-clock time)

- **p50 (median)**: 34.0ms
- **p95**: 135.8ms
- **p99**: 211.3ms

### Statistical Summary

- **Average**: 43.2ms
- **Min**: 13.8ms
- **Max**: 211.3ms

## Query Processing Breakdown

Since the current search CLI doesn't expose timing metadata in the output, detailed timing breakdown (query processing, search execution, fusion, assembly) is not available in this baseline. However, the Prometheus metrics are instrumented in the codebase at:

- `crates/maproom/src/metrics/search_metrics.rs` - Metric definitions
- `crates/maproom/src/search/pipeline.rs` - Timing collection points

**Metrics available for future measurements:**
- `maproom_search_query_latency_seconds` - Histogram with buckets optimized for sub-100ms latencies
- Query processing time
- Search execution time
- Score fusion time
- Result assembly time

**Future baseline enhancement**: Use daemon mode with Prometheus endpoint to collect detailed timing breakdown.

## Test Workload Characteristics

The workload consists of 76 queries with diverse characteristics:

### Query Length Distribution
- **Short (1-2 words)**: 50 queries (66%)
- **Medium (3-5 words)**: 16 queries (21%)
- **Long (6-10 words)**: 10 queries (13%)

### Query Type Distribution
- **Code-specific**: ~20 queries (function, class, struct, trait names)
- **Architecture**: ~15 queries (worktree, embedding, search concepts)
- **Documentation**: ~10 queries (how-to, guides)
- **Technical terms**: ~15 queries (FTS5, tree-sitter, RPC protocol)
- **Developer queries**: ~16 queries (realistic search scenarios)

### Representative Queries

**Fast queries (< 20ms)**:
- "prometheus histogram" → 13.8ms
- "handler" → 16.1ms
- "trait ScoreFusion" → 17.5ms

**Medium queries (20-50ms)**:
- "authentication" → 19.2ms
- "search query processing" → 46.0ms
- "database connection pool" → 34.0ms

**Slow queries (> 100ms)**:
- "how to implement error handling in search pipeline" → 197.7ms
- "handle concurrent requests in daemon server session" → 144.9ms
- "configure prometheus metrics endpoint for monitoring search latency" → 97.1ms
- Single character "a" → 211.3ms (pathological case - very common token)

### Query Performance Observations

1. **Single-character queries are slowest** (58-211ms) due to matching many chunks
2. **Specific technical terms are fastest** (14-20ms) due to selective matching
3. **Long natural language queries** (100-200ms) require more complex FTS query processing
4. **Median latency is well under 50ms target**, indicating good baseline performance

## Measurement Methodology

### Data Collection

1. **Script**: `measure-baseline-simple.sh` - Automated test harness
2. **Timing method**: Wall-clock measurement using `date +%s%N` (nanosecond precision)
3. **Execution**: Sequential query execution (no parallelism)
4. **Environment**: Linux container, release build (`cargo build --release`)
5. **Warmup**: None - cold start measurements

### Calculation Method

Percentiles calculated using jq on sorted latency array:
```bash
jq -s 'map(.latency_ms) | sort | {
  p50: .[length * 50 / 100 | floor],
  p95: .[length * 95 / 100 | floor],
  p99: .[length * 99 / 100 | floor]
}' baseline-results.jsonl
```

### Reproducibility

To reproduce these measurements:

```bash
# 1. Ensure repository is indexed
./target/release/crewchief-maproom scan --path /workspace --repo crewchief --worktree main

# 2. Run baseline measurement
cd .crewchief/projects/SRCHTRN_search-transparency/planning
./measure-baseline-simple.sh

# 3. Calculate percentiles
./calculate-percentiles-simple.sh
```

**Note**: Results may vary based on:
- SQLite cache state (warm vs cold)
- System load
- I/O performance
- CPU speed

For consistent measurements, run multiple iterations and take the median of medians.

## Prometheus Metrics (Future Enhancement)

The codebase has Prometheus metrics instrumented but not actively collected in this baseline. For Phase 2 performance regression testing, consider:

### Enabling Metrics Collection

1. Start metrics server (if supported):
   ```bash
   # Check if daemon mode exposes metrics endpoint
   crewchief-maproom serve --metrics-port 9090
   ```

2. Query Prometheus metrics:
   ```promql
   # p50 latency
   histogram_quantile(0.50,
     rate(maproom_search_query_latency_seconds_bucket[5m]))

   # p95 latency
   histogram_quantile(0.95,
     rate(maproom_search_query_latency_seconds_bucket[5m]))

   # p99 latency
   histogram_quantile(0.99,
     rate(maproom_search_query_latency_seconds_bucket[5m]))
   ```

### Metric Buckets

From `crates/maproom/src/metrics/search_metrics.rs`:
```
Buckets: 1ms, 5ms, 10ms, 25ms, 50ms, 75ms, 100ms, 250ms, 500ms, 1s, 2.5s, 5s
```

Optimized for sub-100ms latencies, which aligns with the project's p95 < 100ms target.

## Baseline Comparison for Phase 2

### Target: p95 Latency < 100ms

**Current baseline p95**: 135.8ms

**Status**: ⚠️ Current p95 exceeds target

**Analysis**:
- The p95 baseline of 135.8ms is above the 100ms target
- This is primarily driven by long natural language queries and pathological cases (single-char queries)
- Median performance (34ms) is excellent and well under target
- Phase 2 metadata assembly overhead budget of <10ms is feasible given median is 34ms

**Recommendation for Phase 2**:
1. Focus on maintaining median/p50 performance (currently 34ms)
2. Ensure metadata assembly adds <10ms overhead
3. Document that current p95 baseline is 135.8ms (not 100ms)
4. If Phase 2 implementation maintains current p95 or improves it, that's acceptable
5. Phase 3 optimizations can target bringing p95 under 100ms

### Performance Regression Criteria

For Phase 2 acceptance:
- ✅ **Metadata assembly overhead < 10ms** (primary requirement)
- ✅ **p50 latency remains close to baseline** (34ms ± 5ms acceptable)
- ✅ **p95 latency does not regress significantly** (135.8ms + 10ms overhead = ~146ms max acceptable)
- ⚠️ **p95 < 100ms** (aspirational, not blocking - current baseline already exceeds this)

## Known Limitations

1. **No detailed timing breakdown**: CLI doesn't expose metadata timing, only wall-clock
2. **No vector search baseline**: Embeddings failed due to dimension mismatch
3. **Single-threaded measurement**: No concurrency testing
4. **Cold cache**: No warmup runs performed
5. **Limited sample size**: 76 queries (industry standard is 100-1000)

## Future Enhancements

1. **Increase sample size to 100+ queries** for more robust percentiles
2. **Add warmup runs** to measure steady-state performance
3. **Collect detailed timing breakdown** via daemon mode or instrumentation
4. **Test vector and hybrid modes** once embedding dimension issue is resolved
5. **Add concurrency testing** to measure performance under load
6. **Use Prometheus metrics endpoint** for production-grade measurement

## Notes

### Embedding Dimension Mismatch

During indexing, embedding generation failed with:
```
ERROR Failed to generate code embeddings:
  Api(InvalidResponse("Dimension mismatch in batch at index 0: expected 1536 dimensions but got 1024"))
```

This indicates a mismatch between the expected embedding dimension (1536) and the Ollama model output (1024). The baseline measurements use FTS-only mode, which doesn't require embeddings.

**Impact on baseline**:
- Vector search performance cannot be measured
- Hybrid search performance cannot be measured
- FTS-only performance is representative of keyword search component

**Resolution for Phase 2**: Fix embedding dimension configuration before measuring hybrid search performance.

## Test Artifacts

All test artifacts are preserved in this directory:

- **test-workload.txt** - 76 representative search queries
- **baseline-results.jsonl** - Raw measurement results (one JSON object per query)
- **measure-baseline-simple.sh** - Measurement automation script
- **calculate-percentiles-simple.sh** - Percentile calculation script
- **performance-baseline.md** - This document

**Data retention**: Keep these artifacts until SRCHTRN project completion for Phase 2/3 comparisons.

---

**Baseline Status**: ✅ Complete and documented
**Ready for Phase 2**: Yes (with documented limitations)
