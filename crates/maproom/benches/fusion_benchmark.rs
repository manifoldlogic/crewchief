//! Benchmarks for fusion strategies.
//!
//! Compares performance characteristics of BasicWeightedFusion and RRFFusion
//! across various scenarios:
//! - Different numbers of result sets (2, 3, 4)
//! - Different result set sizes (10, 100, 1000)
//! - Different chunk overlap patterns
//!
//! Run with:
//! ```bash
//! cargo bench --bench fusion_benchmark
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use maproom::search::executor_types::{RankedResult, RankedResults, SearchSource};
use maproom::search::fusion::{BasicWeightedFusion, FusionWeights, RRFFusion, ScoreFusion};

/// Generate synthetic ranked results for benchmarking.
fn generate_results(
    source: SearchSource,
    num_results: usize,
    start_chunk_id: i64,
) -> RankedResults {
    let results: Vec<RankedResult> = (0..num_results)
        .map(|i| {
            let chunk_id = start_chunk_id + i as i64;
            let score = 1.0 - (i as f32 / num_results as f32); // Descending scores
            let rank = i + 1;
            RankedResult::new(chunk_id, score, rank)
        })
        .collect();

    RankedResults::new(results, source)
}

/// Generate overlapping results (some chunks appear in multiple sources).
fn generate_overlapping_results(
    sources: &[SearchSource],
    num_results_per_source: usize,
    overlap_percentage: f32,
) -> Vec<RankedResults> {
    let mut all_results = Vec::new();
    let overlap_count = (num_results_per_source as f32 * overlap_percentage) as usize;

    for (idx, &source) in sources.iter().enumerate() {
        let mut results = Vec::new();

        // Add overlapping chunks (common across sources)
        for i in 0..overlap_count {
            let chunk_id = i as i64;
            let score = 1.0 - (i as f32 / num_results_per_source as f32);
            let rank = i + 1;
            results.push(RankedResult::new(chunk_id, score, rank));
        }

        // Add unique chunks for this source
        let unique_start = overlap_count;
        let unique_count = num_results_per_source - overlap_count;
        for i in 0..unique_count {
            let chunk_id = 1000 * (idx as i64 + 1) + i as i64;
            let score = 1.0 - ((unique_start + i) as f32 / num_results_per_source as f32);
            let rank = unique_start + i + 1;
            results.push(RankedResult::new(chunk_id, score, rank));
        }

        all_results.push(RankedResults::new(results, source));
    }

    all_results
}

/// Benchmark weighted fusion with varying result set sizes.
fn bench_weighted_fusion_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("weighted_fusion_by_size");
    let fusion = BasicWeightedFusion::new();
    let weights = FusionWeights::default();

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64 * 2)); // 2 sources
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let fts_results = generate_results(SearchSource::FTS, size, 0);
            let vector_results = generate_results(SearchSource::Vector, size, 1000);

            b.iter(|| {
                let results = vec![fts_results.clone(), vector_results.clone()];
                black_box(fusion.fuse(results, &weights, 10))
            });
        });
    }
    group.finish();
}

/// Benchmark RRF fusion with varying result set sizes.
fn bench_rrf_fusion_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("rrf_fusion_by_size");
    let fusion = RRFFusion::default();
    let weights = FusionWeights::default();

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64 * 2)); // 2 sources
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let fts_results = generate_results(SearchSource::FTS, size, 0);
            let vector_results = generate_results(SearchSource::Vector, size, 1000);

            b.iter(|| {
                let results = vec![fts_results.clone(), vector_results.clone()];
                black_box(fusion.fuse(results, &weights, 10))
            });
        });
    }
    group.finish();
}

/// Benchmark weighted fusion with varying number of sources.
fn bench_weighted_fusion_sources(c: &mut Criterion) {
    let mut group = c.benchmark_group("weighted_fusion_by_sources");
    let fusion = BasicWeightedFusion::new();
    let weights = FusionWeights::default();
    let size = 100;

    for num_sources in [2, 3, 4].iter() {
        group.throughput(Throughput::Elements(size as u64 * (*num_sources as u64)));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_sources),
            num_sources,
            |b, &num_sources| {
                let sources = [
                    SearchSource::FTS,
                    SearchSource::Vector,
                    SearchSource::Graph,
                    SearchSource::Signals,
                ];

                let result_sets: Vec<RankedResults> = (0..num_sources)
                    .map(|i| generate_results(sources[i], size, i as i64 * 1000))
                    .collect();

                b.iter(|| {
                    let results = result_sets.clone();
                    black_box(fusion.fuse(results, &weights, 10))
                });
            },
        );
    }
    group.finish();
}

/// Benchmark RRF fusion with varying number of sources.
fn bench_rrf_fusion_sources(c: &mut Criterion) {
    let mut group = c.benchmark_group("rrf_fusion_by_sources");
    let fusion = RRFFusion::default();
    let weights = FusionWeights::default();
    let size = 100;

    for num_sources in [2, 3, 4].iter() {
        group.throughput(Throughput::Elements(size as u64 * (*num_sources as u64)));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_sources),
            num_sources,
            |b, &num_sources| {
                let sources = [
                    SearchSource::FTS,
                    SearchSource::Vector,
                    SearchSource::Graph,
                    SearchSource::Signals,
                ];

                let result_sets: Vec<RankedResults> = (0..num_sources)
                    .map(|i| generate_results(sources[i], size, i as i64 * 1000))
                    .collect();

                b.iter(|| {
                    let results = result_sets.clone();
                    black_box(fusion.fuse(results, &weights, 10))
                });
            },
        );
    }
    group.finish();
}

/// Direct comparison: Weighted vs RRF fusion with same input.
fn bench_fusion_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("fusion_strategy_comparison");
    let weighted = BasicWeightedFusion::new();
    let rrf = RRFFusion::default();
    let weights = FusionWeights::default();

    // Test with realistic scenario: 4 sources, 100 results each, 30% overlap
    let sources = [
        SearchSource::FTS,
        SearchSource::Vector,
        SearchSource::Graph,
        SearchSource::Signals,
    ];
    let result_sets = generate_overlapping_results(&sources, 100, 0.3);

    group.bench_function("weighted_fusion", |b| {
        b.iter(|| {
            let results = result_sets.clone();
            black_box(weighted.fuse(results, &weights, 10))
        });
    });

    group.bench_function("rrf_fusion", |b| {
        b.iter(|| {
            let results = result_sets.clone();
            black_box(rrf.fuse(results, &weights, 10))
        });
    });

    group.finish();
}

/// Benchmark RRF with different k parameters.
fn bench_rrf_k_parameter(c: &mut Criterion) {
    let mut group = c.benchmark_group("rrf_k_parameter");
    let weights = FusionWeights::default();
    let size = 100;

    let fts_results = generate_results(SearchSource::FTS, size, 0);
    let vector_results = generate_results(SearchSource::Vector, size, 1000);

    for k in [10.0, 60.0, 100.0, 200.0].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(k), k, |b, &k| {
            let fusion = RRFFusion::new(k);
            b.iter(|| {
                let results = vec![fts_results.clone(), vector_results.clone()];
                black_box(fusion.fuse(results, &weights, 10))
            });
        });
    }
    group.finish();
}

/// Benchmark with varying overlap percentages.
fn bench_overlap_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("fusion_overlap_impact");
    let weighted = BasicWeightedFusion::new();
    let rrf = RRFFusion::default();
    let weights = FusionWeights::default();

    let sources = [SearchSource::FTS, SearchSource::Vector];

    for overlap in [0.0, 0.3, 0.5, 0.8, 1.0].iter() {
        let result_sets = generate_overlapping_results(&sources, 100, *overlap);

        group.bench_with_input(
            BenchmarkId::new("weighted", format!("{}%", (overlap * 100.0) as i32)),
            overlap,
            |b, _| {
                b.iter(|| {
                    let results = result_sets.clone();
                    black_box(weighted.fuse(results, &weights, 10))
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("rrf", format!("{}%", (overlap * 100.0) as i32)),
            overlap,
            |b, _| {
                b.iter(|| {
                    let results = result_sets.clone();
                    black_box(rrf.fuse(results, &weights, 10))
                });
            },
        );
    }
    group.finish();
}

/// Benchmark memory allocation patterns.
fn bench_fusion_with_limits(c: &mut Criterion) {
    let mut group = c.benchmark_group("fusion_with_limits");
    let weighted = BasicWeightedFusion::new();
    let rrf = RRFFusion::default();
    let weights = FusionWeights::default();

    let size = 1000;
    let fts_results = generate_results(SearchSource::FTS, size, 0);
    let vector_results = generate_results(SearchSource::Vector, size, 1000);
    let graph_results = generate_results(SearchSource::Graph, size, 2000);
    let signal_results = generate_results(SearchSource::Signals, size, 3000);

    for limit in [10, 50, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::new("weighted", limit), limit, |b, &limit| {
            b.iter(|| {
                let results = vec![
                    fts_results.clone(),
                    vector_results.clone(),
                    graph_results.clone(),
                    signal_results.clone(),
                ];
                black_box(weighted.fuse(results, &weights, limit))
            });
        });

        group.bench_with_input(BenchmarkId::new("rrf", limit), limit, |b, &limit| {
            b.iter(|| {
                let results = vec![
                    fts_results.clone(),
                    vector_results.clone(),
                    graph_results.clone(),
                    signal_results.clone(),
                ];
                black_box(rrf.fuse(results, &weights, limit))
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_weighted_fusion_size,
    bench_rrf_fusion_size,
    bench_weighted_fusion_sources,
    bench_rrf_fusion_sources,
    bench_fusion_comparison,
    bench_rrf_k_parameter,
    bench_overlap_impact,
    bench_fusion_with_limits,
);
criterion_main!(benches);
