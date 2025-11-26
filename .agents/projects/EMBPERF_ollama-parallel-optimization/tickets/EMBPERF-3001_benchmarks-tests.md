# Ticket: EMBPERF-3001: Benchmarks & Integration Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: Tests MUST be executed - benchmark suite and integration tests.

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive benchmark suite and integration tests to measure performance improvements and ensure correctness across configurations.

## Background
With batch API (EMBPERF-1001) and parallel processing (EMBPERF-2001) complete, we need to:
1. Measure actual performance improvements
2. Validate correctness across all configurations
3. Identify optimal settings for different hardware
4. Establish baseline for future regression detection

This implements Phase 3 (testing portion) from `plan.md`.

## Acceptance Criteria
- [ ] Benchmark suite created at `benches/ollama_parallel_bench.rs`
- [ ] Benchmarks compare: sequential baseline, batch-only, parallel+batch
- [ ] Benchmarks test batch sizes: 25, 50, 100, 128
- [ ] Benchmarks test concurrency levels: 4, 8, 16
- [ ] Integration test suite at `tests/ollama_parallel_test.rs`
- [ ] Tests verify order preservation
- [ ] Tests verify embedding equivalence (single vs batch)
- [ ] Tests verify dimension consistency (768-dim)
- [ ] Tests verify empty input handling
- [ ] Tests verify config validation
- [ ] All benchmarks run successfully
- [ ] Performance improvement documented (actual measurements)

## Technical Requirements

### Benchmark Suite (`benches/ollama_parallel_bench.rs`)

```rust
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn generate_test_texts(n: usize) -> Vec<String> {
    (0..n).map(|i| format!(
        "Test chunk {} with code: fn test_{}() {{ println!(\"hello\"); }}",
        i, i
    )).collect()
}

fn bench_sequential_baseline(c: &mut Criterion) {
    // Measure: sequential single-text requests
    // This is the "before" measurement
}

fn bench_batch_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_sizes");
    for size in [25, 50, 100, 128] {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, &size| {
                // Benchmark batch of `size` texts
            },
        );
    }
    group.finish();
}

fn bench_concurrency_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrency");
    for level in [4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::from_parameter(level),
            &level,
            |b, &level| {
                // Benchmark with concurrency `level`
            },
        );
    }
    group.finish();
}

fn bench_combined(c: &mut Criterion) {
    // Matrix: batch_size × concurrency
    // Find optimal combination
}

criterion_group!(
    benches,
    bench_sequential_baseline,
    bench_batch_sizes,
    bench_concurrency_levels,
    bench_combined
);
criterion_main!(benches);
```

### Integration Tests (`tests/ollama_parallel_test.rs`)

```rust
use maproom::embedding::{OllamaProvider, ParallelConfig};

#[tokio::test]
#[ignore] // Requires Ollama
async fn test_batch_preserves_order() {
    // Verify embeddings[i] corresponds to texts[i]
}

#[tokio::test]
#[ignore]
async fn test_parallel_produces_same_embeddings() {
    // Same text → same embedding, regardless of batch/parallel config
}

#[tokio::test]
#[ignore]
async fn test_all_embeddings_correct_dimension() {
    // All embeddings should be 768-dim for nomic-embed-text
}

#[tokio::test]
#[ignore]
async fn test_empty_batch_returns_empty() {
    // embed_batch([]) → []
}

#[tokio::test]
fn test_config_validation() {
    // sub_batch_size: 0 → error
    // max_concurrency: 0 → error
}

#[tokio::test]
#[ignore]
async fn test_disabled_parallel_uses_single_batch() {
    // enabled: false → single batch request
}
```

## Implementation Notes

### Running Benchmarks
```bash
# Add criterion to dev-dependencies in Cargo.toml
cargo bench --bench ollama_parallel_bench

# With specific hardware settings
MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY=16 cargo bench
```

### Running Integration Tests
```bash
# Start Ollama first
ollama serve &
ollama pull nomic-embed-text

# Run ignored tests
cargo test --test ollama_parallel_test -- --ignored
```

### Criterion Configuration
Add to `Cargo.toml`:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["async_tokio"] }

[[bench]]
name = "ollama_parallel_bench"
harness = false
```

## Dependencies
- EMBPERF-2001 (parallel processing must be complete)
- Ollama running with `nomic-embed-text`
- Criterion crate for benchmarking

## Risk Assessment
- **Risk**: Ollama not available in CI
  - **Mitigation**: Mark integration tests as `#[ignore]`, document manual testing
- **Risk**: Hardware variations affect benchmark results
  - **Mitigation**: Document hardware specs, note results are relative comparisons

## Files/Packages Affected
- New: `crates/maproom/benches/ollama_parallel_bench.rs`
- New: `crates/maproom/tests/ollama_parallel_test.rs`
- Modified: `crates/maproom/Cargo.toml` (add criterion dev-dependency)

## Deliverables

### Performance Report Template
After running benchmarks, document results:

```markdown
# EMBPERF Performance Results

## Hardware
- CPU/GPU: [specs]
- Ollama version: [version]

## Results

### Throughput Comparison
| Configuration | Texts/sec | Improvement |
|---------------|-----------|-------------|
| Sequential baseline | X | 1x |
| Batch only (50) | Y | Yx |
| Parallel + Batch | Z | Zx |

### Optimal Settings
| Hardware | Batch Size | Concurrency | Throughput |
|----------|------------|-------------|------------|
| M2 Max | 100 | 16 | X texts/s |

### Recommendations
- Default batch size: 50
- Default concurrency: 8
- M2 Max users: set MAX_CONCURRENCY=16
```
