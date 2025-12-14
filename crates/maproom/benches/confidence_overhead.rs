//! Benchmarks for confidence computation overhead.
//!
//! Validates that confidence computation meets performance targets:
//! - Per-result computation: <1ms
//! - Batch of 20 results: <5ms
//!
//! Run with:
//! ```bash
//! cargo bench --bench confidence_overhead
//! ```

use crewchief_maproom::search::confidence::compute_result_confidence;
use crewchief_maproom::search::executor_types::SearchSource;
use crewchief_maproom::search::fusion::FusedResult;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;

/// Generate realistic FusedResult for benchmarking.
fn make_realistic_result(chunk_id: i64, score: f32, source_count: usize) -> FusedResult {
    let mut source_scores = HashMap::new();

    // Add sources based on count
    if source_count >= 1 {
        source_scores.insert(SearchSource::FTS, score * 0.9);
    }
    if source_count >= 2 {
        source_scores.insert(SearchSource::Vector, score * 0.85);
    }
    if source_count >= 3 {
        source_scores.insert(SearchSource::Graph, score * 0.7);
    }
    if source_count >= 4 {
        source_scores.insert(SearchSource::Signals, score * 0.6);
    }

    FusedResult::with_exact_match(chunk_id, score, source_scores, Some(3.0))
}

/// Generate a batch of realistic search results.
fn generate_result_batch(count: usize) -> Vec<FusedResult> {
    (0..count)
        .map(|i| {
            let chunk_id = i as i64;
            let score = 1.0 - (i as f32 / count as f32); // Descending scores
            let source_count = (i % 4) + 1; // Vary source count 1-4
            make_realistic_result(chunk_id, score, source_count)
        })
        .collect()
}

/// Benchmark per-result confidence computation.
///
/// Target: <1ms per result
fn bench_per_result(c: &mut Criterion) {
    let results = generate_result_batch(20);

    c.bench_function("confidence_per_result", |b| {
        b.iter(|| {
            let confidence = compute_result_confidence(
                black_box(&results[0]),
                black_box(&results),
                black_box(0),
                black_box(Some(3.0)),
            );
            black_box(confidence);
        });
    });
}

/// Benchmark batch computation for 20 results.
///
/// Target: <5ms total
fn bench_batch_20(c: &mut Criterion) {
    let results = generate_result_batch(20);

    c.bench_function("confidence_batch_20", |b| {
        b.iter(|| {
            let confidences: Vec<_> = results
                .iter()
                .enumerate()
                .map(|(i, result)| {
                    compute_result_confidence(
                        black_box(result),
                        black_box(&results),
                        black_box(i),
                        black_box(result.exact_match_multiplier),
                    )
                })
                .collect();
            black_box(confidences);
        });
    });
}

/// Benchmark varying batch sizes.
fn bench_batch_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("confidence_batch_sizes");

    for size in [5, 10, 20, 50, 100].iter() {
        let results = generate_result_batch(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let confidences: Vec<_> = results
                    .iter()
                    .enumerate()
                    .map(|(i, result)| {
                        compute_result_confidence(
                            black_box(result),
                            black_box(&results),
                            black_box(i),
                            black_box(result.exact_match_multiplier),
                        )
                    })
                    .collect();
                black_box(confidences);
            });
        });
    }

    group.finish();
}

/// Benchmark edge cases.
fn bench_edge_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("confidence_edge_cases");

    // Single result (no next result for gap)
    let single_result = vec![make_realistic_result(1, 0.95, 3)];
    group.bench_function("single_result", |b| {
        b.iter(|| {
            let confidence = compute_result_confidence(
                black_box(&single_result[0]),
                black_box(&single_result),
                black_box(0),
                black_box(Some(3.0)),
            );
            black_box(confidence);
        });
    });

    // Last result in list (gap = 0.0)
    let results = generate_result_batch(20);
    group.bench_function("last_result", |b| {
        b.iter(|| {
            let confidence = compute_result_confidence(
                black_box(&results[19]),
                black_box(&results),
                black_box(19),
                black_box(Some(3.0)),
            );
            black_box(confidence);
        });
    });

    // Empty source scores
    let empty_sources = HashMap::new();
    let empty_result = FusedResult::new(1, 0.95, empty_sources);
    let empty_batch = vec![empty_result];
    group.bench_function("empty_sources", |b| {
        b.iter(|| {
            let confidence = compute_result_confidence(
                black_box(&empty_batch[0]),
                black_box(&empty_batch),
                black_box(0),
                black_box(None),
            );
            black_box(confidence);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_per_result,
    bench_batch_20,
    bench_batch_sizes,
    bench_edge_cases
);
criterion_main!(benches);
