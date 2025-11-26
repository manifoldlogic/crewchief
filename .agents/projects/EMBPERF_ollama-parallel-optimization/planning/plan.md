# Plan: Ollama Parallel Embedding Optimization

## Executive Summary

Optimize Ollama embedding throughput from ~50-100 texts/sec to 500-1500 texts/sec through batch API usage and parallel request processing.

**Key insight**: Current implementation sends 1 text per HTTP request. Ollama's API supports batch input, and M2 Max is severely underutilized at 15% GPU.

---

## Phase Overview

| Phase | Focus | Deliverables | Risk |
|-------|-------|--------------|------|
| 0 | Baseline & Validation | Performance baseline, API verification | Low |
| 1 | Batch API | OllamaProvider uses array input | Low |
| 2 | Parallel Processing | Concurrent batched requests | Medium |
| 3 | Final Benchmarking | Measure improvements and tune | Low |

**Total Scope**: 6 tickets across 4 phases

---

## Phase 0: Baseline & API Validation

### Objective
Establish current performance baseline and verify Ollama's batch API works as expected before implementing changes.

### Deliverables

1. **Baseline measurements**:
   - Current throughput (texts/sec) with existing implementation
   - HTTP request count for known batch sizes
   - GPU utilization during embedding generation

2. **API validation**:
   - Verify Ollama `/api/embed` accepts array input: `{"input": ["text1", "text2"]}`
   - Confirm response format: `{"embeddings": [[...], [...]]}`
   - Test batch sizes: 10, 25, 50, 100, 128
   - Measure per-batch latency

3. **Concurrency exploration**:
   - Test parallel requests to current single-text endpoint
   - Measure throughput at concurrency levels: 4, 8, 16, 24
   - Identify diminishing returns threshold

### Tickets
- **EMBPERF-0001**: Establish baseline and validate Ollama batch API

### Expected Output
A brief report with:
- Baseline: X texts/sec (current implementation)
- Batch API: confirmed working / not working
- Optimal batch size: ~N texts (based on latency testing)
- Optimal concurrency: ~N parallel requests (based on throughput testing)

---

## Phase 1: Batch API Support

### Objective
Modify OllamaProvider to send multiple texts per HTTP request using Ollama's batch API.

### Changes

**File: `embedding/ollama.rs`**

1. Change request struct to accept array input:
```rust
#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    input: Vec<String>,  // Was: String
}
```

2. Add batch embedding method:
```rust
async fn embed_batch_raw(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
    // Single HTTP request with all texts
    // Parse array response
}
```

3. Update `embed_batch()` to use batch API:
```rust
async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
    if texts.len() <= MAX_BATCH_SIZE {
        self.embed_batch_raw(texts).await
    } else {
        // Split and process sequentially for now
    }
}
```

### Expected Improvement
- 5-10x throughput increase
- Reduced HTTP overhead

### Tickets
- **EMBPERF-1001**: Implement batch API support in OllamaProvider

---

## Phase 2: Parallel Processing

### Objective
Add concurrent processing of sub-batches using existing ParallelConfig infrastructure.

### Changes

**File: `embedding/ollama.rs`**

1. Accept ParallelConfig in constructor:
```rust
pub fn new_with_config(
    endpoint: String,
    model: String,
    config: ParallelConfig,
) -> Result<Self, EmbeddingError>
```

2. Implement parallel batch processing:
```rust
async fn embed_batch_parallel(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
    let sub_batches = texts.chunks(self.config.sub_batch_size);
    let semaphore = Arc::new(Semaphore::new(self.config.max_concurrency));

    // Spawn concurrent tasks for each sub-batch
    // Collect and merge results in order
}
```

3. Update `embed_batch()` to use parallel when enabled:
```rust
async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
    if self.config.enabled && texts.len() > self.config.sub_batch_size {
        self.embed_batch_parallel(texts).await
    } else {
        self.embed_batch_raw(texts).await
    }
}
```

**File: `embedding/factory.rs`**

4. Pass ParallelConfig to OllamaProvider:
```rust
Provider::Ollama => {
    let parallel_config = ParallelConfig::from_env()?;
    Box::new(OllamaProvider::new_with_config(endpoint, model, parallel_config)?)
}
```

**File: `embedding/config.rs`**

5. Update defaults for Ollama optimization:
```rust
impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sub_batch_size: 50,  // Was: 25
            max_concurrency: 8,  // Was: 4
        }
    }
}
```

### Expected Improvement
- Additional 2-4x throughput (combined 10-20x total)
- Better GPU utilization

### Tickets
- **EMBPERF-2001**: Implement parallel sub-batch processing with ParallelConfig

---

## Phase 3: Benchmarking and Tuning

### Objective
Measure actual performance, establish baselines, and tune defaults.

### Deliverables

1. **Benchmark suite** (`benches/ollama_parallel_bench.rs`):
   - Baseline: sequential single-text
   - Batch-only: various batch sizes
   - Parallel: various concurrency levels
   - Combined: matrix of configurations

2. **Performance documentation**:
   - Recommended settings by hardware
   - Tuning guide
   - Benchmark results

3. **Integration tests**:
   - Order preservation
   - Error recovery
   - Config validation

### Tickets
- **EMBPERF-3001**: Create benchmark suite and integration tests
- **EMBPERF-3002**: Document optimal configurations

---

## Implementation Order

```
EMBPERF-0001: Baseline & API Validation
    ↓
EMBPERF-1001: Batch API Support
    ↓
EMBPERF-2001: Parallel Processing (includes ParallelConfig wiring)
    ↓
EMBPERF-3001: Final Benchmarks & Tests
    ↓
EMBPERF-3002: Documentation
```

---

## Configuration Reference

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MAPROOM_EMBEDDING_PARALLEL_ENABLED` | `true` | Enable parallel processing |
| `MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE` | `50` | Texts per HTTP request |
| `MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY` | `8` | Concurrent requests |

### Recommended Settings

| Hardware | Sub-Batch | Concurrency | Command |
|----------|-----------|-------------|---------|
| M1/M2 base | 50 | 4 | Default is fine |
| M2 Pro | 50 | 8 | Default is fine |
| M2 Max | 100 | 16 | `MAX_CONCURRENCY=16 SUB_BATCH_SIZE=100` |
| M2 Ultra | 100 | 24 | `MAX_CONCURRENCY=24 SUB_BATCH_SIZE=100` |

---

## Risk Mitigation

| Risk | Mitigation | Fallback |
|------|------------|----------|
| Ollama API incompatibility | Check version, test batch format | Fall back to single-text |
| Memory pressure | Cap batch size at 128 | Reduce batch size |
| Timeout on large batches | Increase timeout to 60s | Smaller batches |
| Breaking changes | Keep backward compat | Disable via env var |

---

## Success Metrics

| Metric | Baseline | Target | Measured |
|--------|----------|--------|----------|
| Throughput (texts/sec) | ~50-100 | 500+ | TBD |
| HTTP requests per 100 texts | 100 | 2-4 | TBD |
| 10K chunk time | ~2-3 min | <30 sec | TBD |
| GPU utilization | ~15% | 50%+ | TBD |

---

## Agent Assignments

| Ticket | Agent | Rationale |
|--------|-------|-----------|
| EMBPERF-0001 | technical-researcher | Baseline measurement and API exploration |
| EMBPERF-1001 | rust-indexer-engineer | Core Rust implementation |
| EMBPERF-2001 | rust-indexer-engineer | Core Rust implementation |
| EMBPERF-3001 | integration-tester | Test suite creation |
| EMBPERF-3002 | technical-researcher | Documentation |

---

## Timeline Considerations

**Dependencies**:
- Requires Ollama running locally for integration tests
- Benchmarks require actual M2 Max hardware for meaningful results
- **VECSTORE-1000 (Soft)**: SQLite 768-dim support required for SQLite users

**Blockers**:
- None for PostgreSQL users - all code changes are internal
- **SQLite users blocked by VECSTORE-1000**: Ollama produces 768-dim embeddings, but SQLite only supports 1536-dim until VECSTORE-1000 completes

**Parallelization**:
- Phase 1 must complete before Phase 2
- Phase 3 can partially overlap with Phase 2 (benchmarks need Phase 2)

---

## Project Dependencies

| Project | Ticket | Relationship |
|---------|--------|--------------|
| VECSTORE | VECSTORE-1000 | Soft dependency - SQLite 768-dim support |

**VECSTORE Relationship**:
- EMBPERF produces 768-dim embeddings (Ollama/nomic-embed-text)
- PostgreSQL supports 768-dim ✅
- SQLite only supports 1536-dim ❌ (until VECSTORE-1000)
- Zero-config experience (SQLite + Ollama) requires both projects

**Execution Order**: VECSTORE-1000 should complete first for full cross-backend support, but EMBPERF can proceed independently for PostgreSQL users.
