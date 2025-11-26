# Architecture: Ollama Parallel Embedding Optimization

## Solution Overview

Optimize the Ollama embedding pipeline to achieve 10-20x throughput improvement through:
1. **Batch API usage** - Send multiple texts per request
2. **Parallel requests** - Multiple concurrent batched requests
3. **Configuration integration** - Use existing ParallelConfig infrastructure
4. **Auto-tuning** - Optional hardware detection for optimal defaults

---

## Architecture Decisions

### Decision 1: Batch-First Architecture

**Choice**: Implement true batching in OllamaProvider using Ollama's array input

**Rationale**:
- Ollama's `/api/embed` accepts `"input": ["text1", "text2", ...]`
- Single request with 50 texts vs 50 requests = ~10x less HTTP overhead
- GPU can process batched matrix operations more efficiently

**Implementation**:
```rust
// Before (current)
struct OllamaRequest {
    model: String,
    input: String,  // Single text
}

// After
struct OllamaRequest {
    model: String,
    input: Vec<String>,  // Batch of texts
}
```

---

### Decision 2: Configurable Parallelism via ParallelConfig

**Choice**: Wire OllamaProvider to use existing ParallelConfig

**Rationale**:
- Infrastructure already exists (config.rs:363-404)
- Environment variables already defined
- Consistent API across providers
- No new configuration surface to maintain

**Implementation**:
```rust
impl OllamaProvider {
    pub fn new_with_config(
        endpoint: String,
        model: String,
        parallel_config: ParallelConfig,
    ) -> Result<Self, EmbeddingError>
}
```

---

### Decision 3: Hybrid Batch + Parallel Strategy

**Choice**: Split large batches into sub-batches, process sub-batches in parallel

**Rationale**:
- Pure batching: Single point of failure, high latency per request
- Pure parallel: HTTP overhead multiplied
- Hybrid: Best of both - reduced overhead AND fault tolerance

**Flow**:
```
Input: 500 texts
  ↓
Split: 10 sub-batches of 50 texts each
  ↓
Parallel: 4 concurrent sub-batch requests
  ↓
Merge: Ordered results
  ↓
Output: 500 embeddings
```

---

### Decision 4: Progressive Enhancement

**Choice**: Keep backward compatibility, add parallel as opt-in then default

**Rationale**:
- Don't break existing deployments
- Allow gradual rollout and testing
- Users can tune for their hardware

**Phases**:
1. Add batch support (transparent improvement)
2. Add parallel support with `ParallelConfig.enabled`
3. Enable parallel by default after validation

---

## Component Design

### Modified: OllamaProvider

```
┌─────────────────────────────────────────────────────────┐
│                    OllamaProvider                        │
├─────────────────────────────────────────────────────────┤
│ Fields:                                                  │
│   - client: Client                                       │
│   - endpoint: String                                     │
│   - model: String                                        │
│   - parallel_config: ParallelConfig  ← NEW              │
│   - semaphore: Arc<Semaphore>                           │
├─────────────────────────────────────────────────────────┤
│ Methods:                                                 │
│   + embed(text) → Vector                                │
│   + embed_batch(texts) → Vec<Vector>    ← MODIFIED     │
│   + embed_batch_raw(texts) → Vec<Vector> ← NEW (batch) │
│   + embed_batch_parallel(texts) → Vec<Vector> ← NEW    │
└─────────────────────────────────────────────────────────┘
```

### Modified: EmbeddingService

```
┌─────────────────────────────────────────────────────────┐
│                   EmbeddingService                       │
├─────────────────────────────────────────────────────────┤
│ Changes:                                                 │
│   - embed_batch() checks ParallelConfig.enabled         │
│   - If enabled: delegates to provider.embed_batch()     │
│   - Provider handles parallel internally                │
└─────────────────────────────────────────────────────────┘
```

### Factory Integration

```
┌─────────────────────────────────────────────────────────┐
│                create_provider_from_env()                │
├─────────────────────────────────────────────────────────┤
│ For Provider::Ollama:                                    │
│   1. Load ParallelConfig from env                       │
│   2. Create OllamaProvider with parallel_config         │
│   3. Return Box<dyn EmbeddingProvider>                  │
└─────────────────────────────────────────────────────────┘
```

---

## Data Flow

### Current Flow (Inefficient)
```
EmbeddingPipeline.process_batch(100 texts)
  → EmbeddingService.embed_batch(100 texts)
    → OllamaProvider.embed_batch(100 texts)
      → for each text: spawn task
        → embed_single(text)  // 1 text per HTTP request
        → HTTP POST {input: "text"}
      → 100 HTTP requests, semaphore limit 10
```

### Optimized Flow
```
EmbeddingPipeline.process_batch(100 texts)
  → EmbeddingService.embed_batch(100 texts)
    → OllamaProvider.embed_batch(100 texts)
      → if parallel_config.enabled:
          → split into sub_batches of 50
          → for each sub_batch: spawn task with semaphore
            → embed_batch_raw(sub_batch)
            → HTTP POST {input: ["t1", "t2", ...]}
          → 2 HTTP requests, concurrent
        else:
          → embed_batch_raw(100 texts)
          → 1 HTTP request
```

---

## Configuration

### Environment Variables (Existing)

| Variable | Default | Description |
|----------|---------|-------------|
| `MAPROOM_EMBEDDING_PARALLEL_ENABLED` | `true` | Enable parallel processing |
| `MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE` | `50` | Texts per HTTP request |
| `MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY` | `8` | Concurrent requests |

### Recommended Settings by Hardware

| Hardware | Sub-Batch Size | Max Concurrency | Expected Throughput |
|----------|----------------|-----------------|---------------------|
| M1/M2 (base) | 50 | 4 | ~300-400 texts/sec |
| M2 Pro | 50 | 8 | ~500-700 texts/sec |
| M2 Max | 100 | 16 | ~800-1200 texts/sec |
| M2 Ultra | 100 | 24 | ~1000-1500 texts/sec |
| NVIDIA RTX 3080 | 100 | 16 | ~1000-1500 texts/sec |

---

## Error Handling

### Batch Failure Strategy

**Partial failure handling**:
```rust
// If a sub-batch fails, retry with exponential backoff
// If still fails, fall back to single-text requests for that batch
// Never lose data - always return error for genuinely failed texts
```

### Timeout Handling

- Increase default timeout from 30s to 60s for batch requests
- Configurable via `MAPROOM_EMBEDDING_REQUEST_TIMEOUT_SECS`
- Add per-text timeout estimation based on batch size

---

## Performance Monitoring

### Metrics to Add

```rust
pub struct BatchMetrics {
    pub batch_size: usize,
    pub concurrency: usize,
    pub total_texts: usize,
    pub total_requests: usize,
    pub avg_request_time_ms: f64,
    pub throughput_texts_per_sec: f64,
}
```

### Logging

```rust
info!("Batch embedding: {} texts in {} sub-batches with concurrency {}",
      total_texts, num_batches, concurrency);
debug!("Sub-batch {} completed in {}ms ({} texts)",
       batch_idx, elapsed_ms, batch_size);
info!("Batch complete: {} texts in {:.2}s ({:.0} texts/sec)",
      total_texts, elapsed_secs, throughput);
```

---

## Testing Strategy

### Benchmark Suite

Create `benches/ollama_parallel_bench.rs`:
1. Single-text baseline
2. Batch-only (various sizes: 25, 50, 100, 128)
3. Parallel-only (various concurrency: 4, 8, 16, 24)
4. Combined batch+parallel (matrix of options)

### Integration Tests

1. `test_batch_preserves_order` - Results match input order
2. `test_partial_failure_recovery` - Handles individual failures
3. `test_config_respected` - Environment variables work
4. `test_backward_compatible` - Old behavior still works

---

## Migration Path

### Phase 1: Batch Support (Non-Breaking)
- Add `embed_batch_raw()` using array input
- `embed_batch()` calls `embed_batch_raw()` internally
- Immediate 5-10x improvement

### Phase 2: Parallel Support (Opt-In)
- Add parallel processing with `ParallelConfig`
- Default `enabled: false` for safety
- Users opt-in via env var

### Phase 3: Enable by Default
- After validation, flip `enabled: true`
- Document recommended settings
- Add auto-tuning hints

---

## File Changes Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `embedding/ollama.rs` | Major | Add batch API, parallel processing |
| `embedding/config.rs` | Minor | Update defaults (sub_batch: 50) |
| `embedding/factory.rs` | Minor | Pass ParallelConfig to Ollama |
| `embedding/service.rs` | None | Provider handles parallel internally |
| `embedding/mod.rs` | None | No changes needed |
| `benches/ollama_parallel_bench.rs` | New | Performance benchmarks |

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Ollama version incompatibility | Medium | Check API version, graceful fallback |
| Memory pressure on large batches | Medium | Cap batch size at 128, configurable |
| Network timeout on large batches | Low | Increase timeout, retry logic |
| Breaking existing deployments | Low | Backward compatible, opt-in parallel |
