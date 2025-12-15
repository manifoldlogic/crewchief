//! Graph quality-weighted scoring performance benchmarks (SRCHREL-2004).
//!
//! Benchmarks the enhanced graph executor with quality-weighted edge scoring.
//!
//! # Performance Targets
//!
//! | Metric | Target | Acceptable | Critical |
//! |--------|--------|------------|----------|
//! | Graph executor p50 | <20ms | <25ms | >30ms |
//! | Graph executor p95 | <30ms | <35ms | >40ms |
//! | Graph executor p99 | <50ms | <60ms | >80ms |
//!
//! # Running
//!
//! ```bash
//! # Run graph quality benchmarks
//! cargo bench --bench graph_quality_benchmark
//!
//! # Compare with baseline
//! cargo bench --bench graph_quality_benchmark -- --save-baseline quality_weighted
//! ```
//!
//! # Note
//!
//! These benchmarks use synthetic scoring calculations to measure the
//! computational overhead of quality-weighted edge scoring vs legacy scoring.
//! For real-world database benchmarks, use the integration test suite with
//! MAPROOM_DATABASE_URL set.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;

/// Simulates legacy graph importance scoring (count-based).
///
/// This represents the original algorithm that counts edges by type
/// without quality weighting.
fn legacy_graph_score(callers: i32, importers: i32, tests: i32) -> f64 {
    // Original formula: LOG(2 + callers) * 0.3 + LOG(2 + importers) * 0.2 + LOG(2 + tests) * 0.1
    (2.0 + callers as f64).ln() * 0.3
        + (2.0 + importers as f64).ln() * 0.2
        + (2.0 + tests as f64).ln() * 0.1
}

/// Edge quality weights for quality-weighted scoring.
#[derive(Debug, Clone)]
struct EdgeQualityWeights {
    production_code: f32,
    test_code: f32,
    calls: f32,
}

impl Default for EdgeQualityWeights {
    fn default() -> Self {
        Self {
            production_code: 1.0,
            test_code: 0.5,
            calls: 1.0,
        }
    }
}

/// Represents an edge for quality-weighted scoring.
#[derive(Debug, Clone)]
struct EdgeInfo {
    edge_type: EdgeType,
    is_from_test: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EdgeType {
    Calls,
    Imports,
    TestOf,
}

/// Simulates quality-weighted graph importance scoring.
///
/// This represents the enhanced algorithm that weights edges based on:
/// - Edge type (calls vs imports)
/// - Source context (production vs test code)
fn quality_weighted_graph_score(edges: &[EdgeInfo], weights: &EdgeQualityWeights) -> f64 {
    let mut quality_sum = 0.0;

    for edge in edges {
        // Type weight: calls get configurable weight, others get 1.0
        let type_weight = if edge.edge_type == EdgeType::Calls {
            weights.calls as f64
        } else {
            1.0
        };

        // Context weight: test code vs production code
        let context_weight = if edge.is_from_test {
            weights.test_code as f64
        } else {
            weights.production_code as f64
        };

        quality_sum += type_weight * context_weight;
    }

    // Apply logarithmic scaling
    (1.0 + quality_sum).ln()
}

/// Generate test edges for benchmarking.
fn generate_edges(count: usize, test_ratio: f32) -> Vec<EdgeInfo> {
    (0..count)
        .map(|i| {
            let is_from_test = (i as f32 / count as f32) < test_ratio;
            let edge_type = match i % 3 {
                0 => EdgeType::Calls,
                1 => EdgeType::Imports,
                _ => EdgeType::TestOf,
            };
            EdgeInfo {
                edge_type,
                is_from_test,
            }
        })
        .collect()
}

/// Benchmark: Compare legacy vs quality-weighted scoring overhead.
fn bench_scoring_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("scoring_overhead");
    group.measurement_time(Duration::from_secs(10));

    // Test with varying edge counts
    for edge_count in [10, 50, 100, 500] {
        let edges = generate_edges(edge_count, 0.3);
        let weights = EdgeQualityWeights::default();

        // Legacy scoring (simple calculation)
        group.bench_with_input(
            BenchmarkId::new("legacy", edge_count),
            &edge_count,
            |b, &count| {
                b.iter(|| {
                    // Simulate legacy: just count by type
                    let callers = count / 3;
                    let importers = count / 3;
                    let tests = count - callers - importers;
                    black_box(legacy_graph_score(
                        callers as i32,
                        importers as i32,
                        tests as i32,
                    ))
                });
            },
        );

        // Quality-weighted scoring (edge-by-edge calculation)
        group.bench_with_input(
            BenchmarkId::new("quality_weighted", edge_count),
            &edges,
            |b, edges| {
                b.iter(|| black_box(quality_weighted_graph_score(edges, &weights)));
            },
        );
    }

    group.finish();
}

/// Benchmark: Quality-weighted scoring with different weight configurations.
fn bench_weight_variations(c: &mut Criterion) {
    let mut group = c.benchmark_group("weight_variations");
    group.measurement_time(Duration::from_secs(10));

    let edges = generate_edges(100, 0.3);

    // Default weights (production=1.0, test=0.5, calls=1.0)
    let default_weights = EdgeQualityWeights::default();
    group.bench_function("default_weights", |b| {
        b.iter(|| black_box(quality_weighted_graph_score(&edges, &default_weights)));
    });

    // Heavy test penalty (test=0.2)
    let heavy_penalty = EdgeQualityWeights {
        production_code: 1.0,
        test_code: 0.2,
        calls: 1.0,
    };
    group.bench_function("heavy_test_penalty", |b| {
        b.iter(|| black_box(quality_weighted_graph_score(&edges, &heavy_penalty)));
    });

    // Boosted calls (calls=2.0)
    let boosted_calls = EdgeQualityWeights {
        production_code: 1.0,
        test_code: 0.5,
        calls: 2.0,
    };
    group.bench_function("boosted_calls", |b| {
        b.iter(|| black_box(quality_weighted_graph_score(&edges, &boosted_calls)));
    });

    group.finish();
}

/// Benchmark: Test code ratio impact on scoring.
fn bench_test_ratio_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("test_ratio_impact");
    group.measurement_time(Duration::from_secs(10));

    let weights = EdgeQualityWeights::default();

    // Test with different test code ratios
    for test_ratio in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let edges = generate_edges(100, test_ratio);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}%_test", (test_ratio * 100.0) as i32)),
            &edges,
            |b, edges| {
                b.iter(|| black_box(quality_weighted_graph_score(edges, &weights)));
            },
        );
    }

    group.finish();
}

/// Benchmark: Batch scoring for multiple chunks (simulates real query).
fn bench_batch_chunk_scoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_chunk_scoring");
    group.measurement_time(Duration::from_secs(10));

    let weights = EdgeQualityWeights::default();

    // Simulate scoring multiple chunks (as in a search result)
    for chunk_count in [10, 50, 100, 500] {
        // Each chunk has 10-50 edges
        let chunk_edges: Vec<Vec<EdgeInfo>> = (0..chunk_count)
            .map(|i| generate_edges(10 + (i % 41), 0.3))
            .collect();

        // Legacy batch scoring
        group.bench_with_input(
            BenchmarkId::new("legacy_batch", chunk_count),
            &chunk_edges,
            |b, chunks| {
                b.iter(|| {
                    for edges in chunks {
                        let callers = edges.len() / 3;
                        let importers = edges.len() / 3;
                        let tests = edges.len() - callers - importers;
                        black_box(legacy_graph_score(
                            callers as i32,
                            importers as i32,
                            tests as i32,
                        ));
                    }
                });
            },
        );

        // Quality-weighted batch scoring
        group.bench_with_input(
            BenchmarkId::new("quality_batch", chunk_count),
            &chunk_edges,
            |b, chunks| {
                b.iter(|| {
                    for edges in chunks {
                        black_box(quality_weighted_graph_score(edges, &weights));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Simulated full graph executor query.
///
/// This simulates the complete graph executor workflow:
/// 1. Fetch edges from database (simulated)
/// 2. Calculate quality weights
/// 3. Aggregate scores
/// 4. Sort and limit results
fn bench_full_executor_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_executor_simulation");
    group.measurement_time(Duration::from_secs(10));

    let weights = EdgeQualityWeights::default();

    // Simulate different codebase sizes
    for (size_name, chunk_count, avg_edges) in [
        ("small_repo", 100, 15),
        ("medium_repo", 500, 25),
        ("large_repo", 1000, 40),
        ("crewchief_sized", 2000, 30),
    ] {
        // Pre-generate all chunk edges
        let chunk_edges: Vec<Vec<EdgeInfo>> = (0..chunk_count)
            .map(|i| generate_edges(avg_edges + (i % 20), 0.25))
            .collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(size_name),
            &chunk_edges,
            |b, chunks| {
                b.iter(|| {
                    // Step 1: Calculate scores for all chunks
                    let mut scores: Vec<(usize, f64)> = chunks
                        .iter()
                        .enumerate()
                        .map(|(id, edges)| (id, quality_weighted_graph_score(edges, &weights)))
                        .collect();

                    // Step 2: Sort by score descending
                    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

                    // Step 3: Take top 10 (limit)
                    let results: Vec<_> = scores.into_iter().take(10).collect();

                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Fusion weight calculation overhead.
fn bench_fusion_weight_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("fusion_weight_calculation");

    // Simulate fusion weight override calculation
    group.bench_function("no_override", |b| {
        b.iter(|| {
            // Default weights - no calculation needed
            let weights = (0.4f32, 0.35, 0.1, 0.1, 0.05);
            black_box(weights)
        });
    });

    group.bench_function("with_override_renormalization", |b| {
        b.iter(|| {
            // Override graph to 0.2, renormalize others
            let graph_override = 0.2f32;
            let original = (0.4f32, 0.35, 0.1, 0.1, 0.05);
            let non_graph_sum = original.0 + original.1 + original.3 + original.4;
            let remaining = 1.0 - graph_override;
            let scale = remaining / non_graph_sum;

            let weights = (
                original.0 * scale,
                original.1 * scale,
                graph_override,
                original.3 * scale,
                original.4 * scale,
            );
            black_box(weights)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_scoring_overhead,
    bench_weight_variations,
    bench_test_ratio_impact,
    bench_batch_chunk_scoring,
    bench_full_executor_simulation,
    bench_fusion_weight_calculation,
);

criterion_main!(benches);
