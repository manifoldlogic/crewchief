//! Deduplication performance benchmarks (SRCHDUP-4001).
//!
//! Measures deduplication latency across various result set sizes.
//!
//! # Performance Targets
//!
//! | Result Count | Target Latency |
//! |--------------|----------------|
//! | 100          | <1ms           |
//! | 1000         | <10ms          |
//! | 10000        | <100ms         |
//!
//! # Running
//!
//! ```bash
//! # Run dedup benchmarks
//! cargo bench --bench dedup_bench
//!
//! # With specific filter
//! cargo bench --bench dedup_bench -- deduplicate
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use maproom::search::dedup::{deduplicate, DeduplicationConfig};
use maproom::search::executor_types::SearchSource;
use maproom::search::results::ChunkSearchResult;
use std::collections::HashMap;

/// Generate test results with a mix of duplicates.
///
/// # Arguments
/// * `count` - Total number of results to generate
/// * `unique_count` - Number of unique identities (rest are duplicates)
fn generate_test_results(count: usize, unique_count: usize) -> Vec<ChunkSearchResult> {
    (0..count)
        .map(|i| {
            let identity_idx = i % unique_count;
            let score = 0.95 - (i as f32 * 0.0001); // Slight variation in scores

            ChunkSearchResult {
                chunk_id: i as i64,
                file_id: (identity_idx % 100) as i64,
                relpath: format!(
                    "src/module_{}/file_{}.rs",
                    identity_idx / 10,
                    identity_idx % 10
                ),
                symbol_name: Some(format!("function_{}", identity_idx)),
                kind: "function".to_string(),
                start_line: 10,
                end_line: 50,
                preview: format!("fn function_{}() {{ ... }}", identity_idx),
                score,
                source_scores: HashMap::from([(SearchSource::FTS, score)]),
            }
        })
        .collect()
}

/// Generate results with no duplicates (worst case for deduplication).
fn generate_unique_results(count: usize) -> Vec<ChunkSearchResult> {
    generate_test_results(count, count)
}

/// Generate results with high duplication (best case for deduplication).
fn generate_highly_duplicated_results(count: usize) -> Vec<ChunkSearchResult> {
    generate_test_results(count, count / 10) // 10x duplication factor
}

fn bench_deduplicate(c: &mut Criterion) {
    let config = DeduplicationConfig::default();

    let mut group = c.benchmark_group("deduplicate");

    // Benchmark typical result sizes
    for (count, unique) in [(100, 50), (1000, 500), (10000, 5000)] {
        let results = generate_test_results(count, unique);

        group.bench_with_input(
            BenchmarkId::new("typical", count),
            &results,
            |b, results| {
                b.iter(|| deduplicate(black_box(results.clone()), black_box(&config)));
            },
        );
    }

    group.finish();
}

fn bench_deduplicate_unique(c: &mut Criterion) {
    let config = DeduplicationConfig::default();

    let mut group = c.benchmark_group("deduplicate_unique");

    // All unique results (worst case - no deduplication possible)
    for count in [100, 1000, 10000] {
        let results = generate_unique_results(count);

        group.bench_with_input(
            BenchmarkId::new("all_unique", count),
            &results,
            |b, results| {
                b.iter(|| deduplicate(black_box(results.clone()), black_box(&config)));
            },
        );
    }

    group.finish();
}

fn bench_deduplicate_heavy(c: &mut Criterion) {
    let config = DeduplicationConfig::default();

    let mut group = c.benchmark_group("deduplicate_heavy");

    // High duplication factor (best case for dedup - many duplicates)
    for count in [100, 1000, 10000] {
        let results = generate_highly_duplicated_results(count);

        group.bench_with_input(
            BenchmarkId::new("heavy_duplication", count),
            &results,
            |b, results| {
                b.iter(|| deduplicate(black_box(results.clone()), black_box(&config)));
            },
        );
    }

    group.finish();
}

fn bench_deduplicate_disabled(c: &mut Criterion) {
    let config = DeduplicationConfig {
        enabled: false,
        ..Default::default()
    };

    let mut group = c.benchmark_group("deduplicate_disabled");

    // Disabled dedup should be O(1)
    for count in [100, 1000, 10000] {
        let results = generate_test_results(count, count / 2);

        group.bench_with_input(
            BenchmarkId::new("disabled", count),
            &results,
            |b, results| {
                b.iter(|| deduplicate(black_box(results.clone()), black_box(&config)));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_deduplicate,
    bench_deduplicate_unique,
    bench_deduplicate_heavy,
    bench_deduplicate_disabled,
);
criterion_main!(benches);
