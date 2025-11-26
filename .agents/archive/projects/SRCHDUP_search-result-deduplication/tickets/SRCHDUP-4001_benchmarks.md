# Ticket: SRCHDUP-4001: Add dedup benchmarks

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - benchmarks run successfully, all targets met
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Create benchmarks for the deduplication module to measure and validate performance targets. Benchmarks should cover various result set sizes to ensure deduplication adds minimal latency.

## Background

Performance is a key acceptance criterion for this project. The deduplication step must complete in <10ms for 1000 results to avoid perceptible latency. Benchmarks provide evidence that targets are met and establish baseline for future optimization.

**Reference:** plan.md Phase 4, quality-strategy.md "Performance Benchmarks", architecture.md "Benchmarking Targets"

## Acceptance Criteria

- [x] New benchmark file `crates/maproom/benches/dedup_bench.rs` exists
- [x] Benchmark covers 100, 1000, and 10000 result sets
- [x] Benchmark results show <10ms for 1000 results
- [x] Results are documented in benchmark output or README
- [x] `cargo bench dedup` runs without errors

## Technical Requirements

### Benchmark File
```rust
// crates/maproom/benches/dedup_bench.rs

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use maproom::search::dedup::{deduplicate, DeduplicationConfig, ChunkSearchResult};

fn generate_test_results(count: usize, unique_count: usize) -> Vec<ChunkSearchResult> {
    // Generate results where `unique_count` have distinct identities
    // and the rest are duplicates distributed among them
    (0..count)
        .map(|i| {
            let identity_idx = i % unique_count;
            ChunkSearchResult {
                chunk_id: i as i64,
                relpath: format!("src/file_{}.rs", identity_idx),
                symbol_name: Some(format!("func_{}", identity_idx)),
                start_line: 10,
                score: 0.9 - (i as f64 * 0.0001),
                // ... other fields
            }
        })
        .collect()
}

fn bench_deduplicate(c: &mut Criterion) {
    let config = DeduplicationConfig::default();

    let mut group = c.benchmark_group("deduplicate");

    for (count, unique) in [(100, 50), (1000, 500), (10000, 5000)] {
        let results = generate_test_results(count, unique);

        group.bench_with_input(
            BenchmarkId::new("results", count),
            &results,
            |b, results| {
                b.iter(|| deduplicate(results.clone(), &config));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_deduplicate);
criterion_main!(benches);
```

### Cargo.toml Addition
```toml
# In crates/maproom/Cargo.toml

[[bench]]
name = "dedup_bench"
harness = false

[dev-dependencies]
criterion = "0.5"
```

### Performance Targets
| Result Count | Target Latency | Rationale |
|--------------|----------------|-----------|
| 100 | <1ms | Typical search |
| 1000 | <10ms | Heavy search |
| 10000 | <100ms | Stress test |

## Implementation Notes

1. **Check existing benchmarks** - See if there's a benches/ directory already
2. **Use criterion** - Industry standard for Rust benchmarks
3. **Clone input** - Benchmark needs fresh data each iteration
4. **Measure wall time** - criterion handles warmup and statistics

### Running Benchmarks
```bash
cd crates/maproom
cargo bench dedup

# Or with filtering
cargo bench -- deduplicate

# HTML report (if criterion is configured)
open target/criterion/deduplicate/report/index.html
```

### Sample Output Format
```
deduplicate/results/100  time:   [123.45 µs 125.67 µs 127.89 µs]
deduplicate/results/1000 time:   [1.2345 ms 1.2567 ms 1.2789 ms]
deduplicate/results/10000 time: [12.345 ms 12.567 ms 12.789 ms]
```

## Dependencies

- SRCHDUP-2002 (dedup module integrated and working)

## Risk Assessment

- **Risk**: Benchmark results vary by machine
  - **Mitigation**: Focus on order-of-magnitude; exact times less important
- **Risk**: ChunkSearchResult construction differs from production
  - **Mitigation**: Use realistic field sizes and types
- **Risk**: criterion not in dependencies
  - **Mitigation**: Add to dev-dependencies as shown

## Files/Packages Affected

- `crates/maproom/benches/dedup_bench.rs` (NEW)
- `crates/maproom/Cargo.toml` (add bench config and criterion dep)
