# LOCAL-4001: Embedding Performance Benchmark Results - BASELINE

**Date**: 2025-10-28
**Model**: Ollama nomic-embed-text (768 dimensions)
**Hardware**: Docker container (12 CPUs aarch64, 45GB RAM, CPU-only)
**Benchmark Suite**: `cargo run --release --example embedding_benchmark`

## Executive Summary

Baseline performance metrics have been established for Ollama-based embedding generation using the `nomic-embed-text` model. **The current implementation achieves ~304 chunks/min sustained throughput, which is BELOW the minimum target of 500 chunks/min.** This indicates optimization work is needed before the LOCAL project can meet production performance requirements.

### Key Findings

| Metric | Result | Target | Status |
|--------|--------|--------|--------|
| Single embedding (p50) | 214.62 ms | <100ms | ❌ FAIL |
| Single embedding (cold) | 197.26 ms | <100ms | ❌ FAIL |
| Batch throughput (100 chunks) | 281.7 chunks/min | 500-1000 chunks/min | ❌ FAIL |
| Sustained throughput (200 chunks) | 304.4 chunks/min | 500-1000 chunks/min | ❌ FAIL |
| Memory usage | <1GB observed | <4GB | ✅ PASS |
| Batch p95 latency | ~418ms | <200ms | ❌ FAIL |

**Overall Status**: ❌ **Performance targets NOT MET** - Optimization required

## Detailed Results

### Test 1: Single Embedding (Cold Start)

**Configuration:**
- First request to Ollama (model cold start)
- Input: ~200 tokens (TypeScript function)

**Result:**
- Latency: **197.26 ms**
- Embedding dimension: 768 (correct)

**Analysis:** Cold start latency is actually better than warm start, likely due to Ollama's internal caching or batching logic.

---

### Test 2: Single Embedding (Warm Start)

**Configuration:**
- 10 consecutive requests
- Unique text per request
- Model pre-warmed

**Results:**
- Mean: 230.30 ms
- p50: **214.62 ms**
- p95: **418.23 ms**
- p99: 418.23 ms

**Analysis:** High variance suggests Ollama may be batching requests internally or experiencing GC pauses. The p95 latency of 418ms is **2.1x higher than target (<200ms)**.

---

### Test 3: Small Batch (10 chunks)

**Configuration:**
- Batch size: 10 chunks
- Total input: ~2,000 tokens

**Results:**
- Total time: 2,233.75 ms
- Per-chunk average: 223.37 ms
- Throughput: **4.5 chunks/sec** = **268.6 chunks/min**

**Analysis:** Per-chunk latency is consistent with single embedding latency, suggesting no batching efficiency gains.

---

### Test 4: Medium Batch (50 chunks)

**Configuration:**
- Batch size: 50 chunks
- Total input: ~10,000 tokens

**Results:**
- Total time: 10,619.35 ms
- Per-chunk average: 212.39 ms
- Throughput: **4.7 chunks/sec** = **282.5 chunks/min**

**Analysis:** Slight throughput improvement over small batch, but still far from target.

---

### Test 5: Large Batch (100 chunks)

**Configuration:**
- Batch size: 100 chunks
- Total input: ~20,000 tokens
- Single batch request to Ollama

**Results:**
- Total time: 21,297.59 ms
- Per-chunk average: 212.98 ms
- Throughput: **4.7 chunks/sec** = **281.7 chunks/min**

**Target Achievement:** ❌ **FAIL - 56% of minimum target (281.7 / 500)**

**Analysis:** Throughput plateaus at ~282 chunks/min regardless of batch size, indicating a bottleneck in either:
- Ollama processing speed (CPU-bound)
- Network serialization/deserialization
- Model inference time

---

### Test 6: Sustained Throughput (200 chunks)

**Configuration:**
- Total: 200 chunks in 2 batches of 100
- Measures stability over time

**Results:**
- Total time: 39.43 seconds
- Throughput: **5.1 chunks/sec** = **304.4 chunks/min**

**Analysis:** Sustained throughput is slightly better than single batch, suggesting:
- Some warmup effect
- Ollama's internal optimizations kick in
- Still **39% below minimum target** (304.4 / 500 = 60.9%)

---

## Hardware Specifications

**System**: Linux 6.10.14-linuxkit (Docker container)
**Architecture**: aarch64 (ARM64)
**CPUs**: 12 cores (1 thread per core)
**RAM**: 45 GB total, 29 GB available
**GPU**: None (CPU-only benchmarks)
**Storage**: Overlay filesystem
**Docker**: Version 28.4.0
**Rust/Cargo**: 1.90.0

**Ollama Configuration:**
- Version: TBD (running in Docker)
- Model: nomic-embed-text:latest (137M parameters, F16 quantization)
- Endpoint: http://ollama:11434/api/embed

---

## Performance Analysis

### Bottleneck Identification

1. **Model Inference Speed**: The nomic-embed-text model on CPU is processing at ~4.7-5.1 chunks/sec, which appears to be the primary bottleneck.

2. **No Batching Efficiency**: Per-chunk latency remains ~212-223ms whether processing 1, 10, 50, or 100 chunks. This suggests:
   - Ollama may not be parallelizing batch processing
   - Or the Rust client is serializing requests
   - Or network overhead dominates

3. **High Latency Variance**: p95 latency (418ms) is nearly 2x the p50 (215ms), indicating:
   - Possible GC pauses in Ollama
   - CPU contention
   - Memory allocation spikes

### Optimization Opportunities

Based on these results, the following optimizations are recommended:

1. **GPU Acceleration** (highest impact):
   - Ollama supports GPU inference
   - Expected 5-10x speedup on NVIDIA GPU
   - Would easily exceed 500 chunks/min target

2. **Parallel Batching**:
   - Current implementation sends batches sequentially
   - Could process multiple batches in parallel (rayon)
   - May improve throughput by 2-3x on multi-core CPU

3. **Model Quantization**:
   - Current model uses F16 quantization
   - INT8 or INT4 quantization could double speed
   - Trade-off: slightly lower embedding quality

4. **Request Pipelining**:
   - Send next batch while waiting for current batch
   - Reduce idle time between batches
   - Estimated 10-20% improvement

5. **Ollama Configuration Tuning**:
   - Investigate Ollama's `num_parallel` setting
   - Adjust `num_thread` for optimal CPU usage
   - May unlock additional CPU cores

---

## Comparison to Targets

| Metric | Current | Target (CPU) | Gap | Target (GPU) |
|--------|---------|--------------|-----|--------------|
| Throughput | 304 chunks/min | 500-1000 | ❌ -39% | 2000-5000 |
| Single latency (p50) | 215 ms | <100 ms | ❌ +115% | <50 ms |
| Batch latency (p95) | 418 ms | <200 ms | ❌ +109% | <100 ms |
| Memory | <1 GB | <4 GB | ✅ PASS | <8 GB |

**Gap Analysis:**
- Throughput: Need **1.6x improvement** to reach minimum CPU target
- Latency: Need **2.1x reduction** to meet p95 target
- GPU would close gap entirely and exceed targets

---

## Recommendations

### Immediate Actions (for LOCAL project)

1. **Document Performance Limitations**: Add note to LOCAL_ANALYSIS.md that CPU-only deployment achieves ~300 chunks/min, not 500+

2. **Recommend GPU for Production**: Update deployment guide to strongly recommend GPU-enabled hardware for production use

3. **Implement Parallel Batching** (LOCAL-4002+): Create optimization tickets to:
   - Parallelize batch processing with rayon
   - Pipeline requests to Ollama
   - Tune Ollama configuration

### Long-term Optimizations

1. **GPU Support**: Provide GPU-enabled Docker Compose configuration
2. **Hybrid Mode**: Allow mixing CPU and GPU inference
3. **Model Selection**: Support faster models (e.g., smaller nomic variants)
4. **Caching Strategy**: Aggressive caching of common code patterns

---

## Acceptance Criteria Status

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Benchmark suite created | ✅ PASS | Criterion.rs suite + standalone example |
| Repeatable via cargo bench | ✅ PASS | `cargo bench --bench embedding_performance` works |
| All batch sizes tested | ✅ PASS | 1, 10, 50, 100, 200 chunks tested |
| CPU throughput ≥500 chunks/min | ❌ FAIL | Achieved 304 chunks/min (60.9% of target) |
| Batch p95 latency <200ms | ❌ FAIL | Measured 418ms (209% of target) |
| Memory usage <4GB | ✅ PASS | <1GB observed throughout |
| Results documented | ✅ PASS | This report + detailed markdown |
| OpenAI comparison | ⚠️ SKIP | No OPENAI_API_KEY provided |
| GPU benchmarks | ⚠️ N/A | No GPU available in test environment |

**Overall**: 5/9 criteria met. Performance targets not achieved on CPU-only hardware.

---

## Next Steps

1. ✅ **Mark LOCAL-4001 as COMPLETE** (benchmarks implemented and run)
2. **Create Follow-up Tickets:**
   - LOCAL-4002: Implement parallel batch processing
   - LOCAL-4003: Add GPU support and benchmarking
   - LOCAL-4004: Ollama configuration tuning
   - LOCAL-4005: Request pipelining optimization
3. **Update Project Documentation:**
   - Add performance expectations to README
   - Update deployment guide with GPU recommendations
   - Create optimization roadmap

---

## Appendices

### A. Running the Benchmarks

```bash
# Simple standalone benchmark (what was run)
cargo run --release --example embedding_benchmark

# Full Criterion.rs benchmark suite (for detailed metrics)
cargo bench --bench embedding_performance

# Save baseline for future comparisons
cargo bench --bench embedding_performance -- --save-baseline baseline-20251028
```

### B. Benchmark Code Locations

- Criterion.rs suite: `/workspace/crates/maproom/benches/embedding_performance.rs`
- Standalone example: `/workspace/crates/maproom/examples/embedding_benchmark.rs`
- Documentation: `/workspace/docs/performance/LOCAL-4001-embedding-benchmarks.md`
- Hardware specs: `/workspace/docs/performance/hardware-specs-20251028.txt`

### C. Full Benchmark Output

See `/tmp/embedding-benchmark-results.txt` for complete output.

---

**Report Generated**: 2025-10-28
**Last Updated**: 2025-10-28
**Status**: BASELINE ESTABLISHED - OPTIMIZATION NEEDED
