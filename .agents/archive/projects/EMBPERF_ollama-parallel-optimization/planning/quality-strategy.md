# Quality Strategy: Ollama Parallel Embedding Optimization

## Testing Philosophy

This project modifies core embedding infrastructure that affects all embedding generation. Testing focuses on:
1. **Correctness** - Embeddings are identical regardless of parallelization strategy
2. **Reliability** - No data loss under failure conditions
3. **Performance** - Measurable throughput improvements

---

## Critical Test Areas

### 1. Correctness Tests (Must Pass)

**Order Preservation**
```rust
#[tokio::test]
async fn test_batch_preserves_order() {
    // Input: ["a", "b", "c", "d", "e"]
    // Verify: embeddings[0] is for "a", embeddings[4] is for "e"
    // Test with various batch/concurrency combinations
}
```

**Embedding Equivalence**
```rust
#[tokio::test]
async fn test_parallel_produces_same_embeddings() {
    // Same text should produce same embedding
    // regardless of batch size or concurrency
    let text = "test content";
    let single = provider.embed(text).await?;
    let batched = provider.embed_batch(vec![text]).await?[0];
    assert_eq!(single, batched);
}
```

**Dimension Consistency**
```rust
#[tokio::test]
async fn test_all_embeddings_correct_dimension() {
    // All embeddings must be 768 dimensions
    // Regardless of batch size
    let embeddings = provider.embed_batch(texts).await?;
    for emb in embeddings {
        assert_eq!(emb.len(), 768);
    }
}
```

### 2. Reliability Tests (Must Pass)

**Partial Failure Handling**
```rust
#[tokio::test]
async fn test_handles_partial_batch_failure() {
    // If one sub-batch fails, others should still succeed
    // Failed items should be retried or reported
}
```

**Empty Input**
```rust
#[tokio::test]
async fn test_empty_batch_returns_empty() {
    let result = provider.embed_batch(vec![]).await?;
    assert!(result.is_empty());
}
```

**Configuration Validation**
```rust
#[test]
fn test_invalid_config_rejected() {
    // sub_batch_size: 0 → error
    // max_concurrency: 0 → error
    // sub_batch_size > 1000 → error
}
```

### 3. Performance Tests (Should Improve)

**Throughput Benchmark**
```rust
#[tokio::test]
#[ignore] // Requires running Ollama
async fn bench_throughput_comparison() {
    let texts = generate_test_texts(1000);

    // Baseline: sequential single-text
    let start = Instant::now();
    for text in &texts {
        provider.embed(text.clone()).await?;
    }
    let sequential_time = start.elapsed();

    // Optimized: parallel batched
    let start = Instant::now();
    provider.embed_batch(texts).await?;
    let parallel_time = start.elapsed();

    // Should be at least 5x faster
    assert!(sequential_time > parallel_time * 5);
}
```

---

## Test Categories

### Unit Tests (Fast, No External Dependencies)

| Test | File | Purpose |
|------|------|---------|
| Config validation | `config.rs` | ParallelConfig bounds checking |
| Request serialization | `ollama.rs` | Batch JSON format correct |
| Response deserialization | `ollama.rs` | Handles array embeddings |
| Sub-batch splitting | `ollama.rs` | Correct chunking logic |
| Result merging | `ollama.rs` | Order preserved after parallel |

### Integration Tests (Requires Mock or Ollama)

| Test | Purpose | Mock/Real |
|------|---------|-----------|
| Batch API call | Correct HTTP request format | Mock |
| Parallel execution | Concurrent requests work | Mock |
| Retry logic | Handles transient failures | Mock |
| Full pipeline | End-to-end embedding | Real Ollama |

### Performance Tests (Requires Real Ollama)

| Test | Purpose | Criteria |
|------|---------|----------|
| Throughput | Measure texts/sec | Report only |
| Scaling | Verify concurrency helps | Linear-ish scaling |
| Memory | No leaks under load | Stable RSS |

---

## Test Implementation Plan

### Phase 1: Unit Tests (Ticket EMBPERF-1001)

Add to `ollama.rs`:
```rust
#[cfg(test)]
mod tests {
    // Existing tests...

    #[test]
    fn test_batch_request_serialization() { }

    #[test]
    fn test_batch_response_deserialization() { }

    #[test]
    fn test_sub_batch_splitting() { }

    #[test]
    fn test_result_merge_ordering() { }
}
```

### Phase 2: Integration Tests (Ticket EMBPERF-2001)

Add `tests/ollama_parallel_test.rs`:
```rust
#[tokio::test]
#[ignore] // Requires Ollama
async fn test_batch_api_integration() { }

#[tokio::test]
#[ignore]
async fn test_parallel_correctness() { }

#[tokio::test]
#[ignore]
async fn test_error_recovery() { }
```

### Phase 3: Benchmarks (Ticket EMBPERF-2002)

Add `benches/ollama_parallel_bench.rs`:
```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_batch_sizes(c: &mut Criterion) {
    // Compare batch sizes: 25, 50, 100, 128
}

fn bench_concurrency(c: &mut Criterion) {
    // Compare concurrency: 4, 8, 16, 24
}

fn bench_combined(c: &mut Criterion) {
    // Matrix of batch × concurrency
}
```

---

## Acceptance Criteria

### Functional Requirements

- [ ] Batch API produces correct embeddings
- [ ] Order is preserved in all configurations
- [ ] Empty input returns empty output
- [ ] Invalid config is rejected
- [ ] Partial failures don't lose successful results

### Performance Requirements

- [ ] Batch-only: ≥5x improvement over sequential
- [ ] Parallel+batch: ≥10x improvement over sequential
- [ ] No memory leaks under sustained load
- [ ] Timeout handling prevents hanging

### Compatibility Requirements

- [ ] Existing tests pass unchanged
- [ ] `MAPROOM_EMBEDDING_PARALLEL_ENABLED=false` gives old behavior
- [ ] Works with Ollama 0.2.0+

---

## Test Data

### Standard Test Corpus

```rust
fn generate_test_texts(n: usize) -> Vec<String> {
    (0..n).map(|i| format!(
        "Test chunk {} with some code: fn test_{}() {{ println!(\"hello\"); }}",
        i, i
    )).collect()
}
```

### Edge Cases

| Case | Input | Expected |
|------|-------|----------|
| Empty | `[]` | `[]` |
| Single | `["one"]` | `[vec![...]]` |
| Exact batch | 50 texts, batch=50 | 1 request |
| One over | 51 texts, batch=50 | 2 requests |
| Very long text | 10K chars | Success or clear error |
| Unicode | `["日本語", "emoji: 🚀"]` | Valid embeddings |

---

## CI Integration

### GitHub Actions

```yaml
# In .github/workflows/test.yml
test-rust:
  # ... existing config ...

  - name: Run Ollama tests (mock)
    run: cargo test --features sqlite ollama
    working-directory: crates/maproom

  # Optional: Real Ollama tests (manual trigger)
  # - name: Run Ollama integration tests
  #   if: github.event_name == 'workflow_dispatch'
  #   run: |
  #     ollama serve &
  #     sleep 5
  #     ollama pull nomic-embed-text
  #     cargo test --features sqlite ollama_integration -- --ignored
```

---

## Risk Mitigation

| Risk | Test Coverage |
|------|---------------|
| Order corruption | `test_batch_preserves_order` |
| Dimension mismatch | `test_all_embeddings_correct_dimension` |
| Memory leak | Manual observation during bench |
| Timeout hang | `test_timeout_handling` |
| Config ignored | `test_config_respected` |
