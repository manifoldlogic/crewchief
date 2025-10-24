//! Benchmarks for importance scoring performance.
//!
//! Target: Scoring should complete within 10ms for 1000 chunks.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use crewchief_maproom::context::importance::{ChunkMetadata, ImportanceScorer, Relationship, ScoringConfig};
use crewchief_maproom::context::graph::EdgeType;

fn create_benchmark_chunk(id: i64, relpath: &str) -> ChunkMetadata {
    ChunkMetadata {
        id,
        relpath: relpath.to_string(),
        importance_score: Some(1.5),
        recency_score: Some(0.8),
        churn_score: Some(0.3),
    }
}

fn create_benchmark_relationship(edge_type: EdgeType, distance: u32) -> Relationship {
    Relationship {
        edge_type,
        distance,
    }
}

fn bench_single_score(c: &mut Criterion) {
    let scorer = ImportanceScorer::new();
    let target = create_benchmark_chunk(1000, "src/target.rs");
    let chunk = create_benchmark_chunk(1, "src/chunk.rs");
    let relationship = create_benchmark_relationship(EdgeType::Calls, 1);

    c.bench_function("score_single_chunk", |b| {
        b.iter(|| {
            scorer.score(
                black_box(&chunk),
                black_box(&relationship),
                black_box(&target),
            )
        })
    });
}

fn bench_batch_scoring(c: &mut Criterion) {
    let scorer = ImportanceScorer::new();
    let target = create_benchmark_chunk(1000, "src/target.rs");

    let mut group = c.benchmark_group("batch_scoring");

    for size in [10, 100, 1000].iter() {
        // Create test data
        let chunks: Vec<ChunkMetadata> = (0..*size)
            .map(|i| create_benchmark_chunk(i as i64, &format!("src/file_{}.rs", i)))
            .collect();

        let relationships: Vec<Relationship> = (0..*size)
            .map(|i| {
                create_benchmark_relationship(
                    EdgeType::Calls,
                    (i % 5) as u32, // Distance 0-4
                )
            })
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                for (chunk, rel) in chunks.iter().zip(relationships.iter()) {
                    black_box(scorer.score(black_box(chunk), black_box(rel), black_box(&target)));
                }
            })
        });
    }

    group.finish();
}

fn bench_different_relationship_types(c: &mut Criterion) {
    let scorer = ImportanceScorer::new();
    let target = create_benchmark_chunk(1000, "src/target.rs");
    let chunk = create_benchmark_chunk(1, "src/target.rs");

    let mut group = c.benchmark_group("relationship_types");

    for edge_type in [
        EdgeType::TestOf,
        EdgeType::Calls,
        EdgeType::Imports,
        EdgeType::Exports,
        EdgeType::CalledBy,
        EdgeType::RouteOf,
    ]
    .iter()
    {
        let relationship = create_benchmark_relationship(edge_type.clone(), 1);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:?}", edge_type)),
            edge_type,
            |b, _| {
                b.iter(|| {
                    scorer.score(
                        black_box(&chunk),
                        black_box(&relationship),
                        black_box(&target),
                    )
                })
            },
        );
    }

    group.finish();
}

fn bench_distance_variations(c: &mut Criterion) {
    let scorer = ImportanceScorer::new();
    let target = create_benchmark_chunk(1000, "src/target.rs");
    let chunk = create_benchmark_chunk(1, "src/chunk.rs");

    let mut group = c.benchmark_group("distance_variations");

    for distance in [0, 1, 2, 3, 5, 10].iter() {
        let relationship = create_benchmark_relationship(EdgeType::Calls, *distance);

        group.bench_with_input(
            BenchmarkId::from_parameter(distance),
            distance,
            |b, _| {
                b.iter(|| {
                    scorer.score(
                        black_box(&chunk),
                        black_box(&relationship),
                        black_box(&target),
                    )
                })
            },
        );
    }

    group.finish();
}

fn bench_directory_bonus_checks(c: &mut Criterion) {
    let scorer = ImportanceScorer::new();
    let target = create_benchmark_chunk(1000, "src/modules/api/handlers/user.rs");
    let relationship = create_benchmark_relationship(EdgeType::Calls, 1);

    let mut group = c.benchmark_group("directory_checks");

    // Same directory
    let same_dir = create_benchmark_chunk(1, "src/modules/api/handlers/auth.rs");
    group.bench_function("same_directory", |b| {
        b.iter(|| {
            scorer.score(
                black_box(&same_dir),
                black_box(&relationship),
                black_box(&target),
            )
        })
    });

    // Different directory
    let diff_dir = create_benchmark_chunk(2, "src/utils/helpers.rs");
    group.bench_function("different_directory", |b| {
        b.iter(|| {
            scorer.score(
                black_box(&diff_dir),
                black_box(&relationship),
                black_box(&target),
            )
        })
    });

    group.finish();
}

fn bench_metadata_variations(c: &mut Criterion) {
    let scorer = ImportanceScorer::new();
    let target = create_benchmark_chunk(1000, "src/target.rs");
    let relationship = create_benchmark_relationship(EdgeType::Calls, 1);

    let mut group = c.benchmark_group("metadata_variations");

    // All metadata present
    let full_metadata = ChunkMetadata {
        id: 1,
        relpath: "src/chunk.rs".to_string(),
        importance_score: Some(2.5),
        recency_score: Some(0.9),
        churn_score: Some(0.2),
    };
    group.bench_function("full_metadata", |b| {
        b.iter(|| {
            scorer.score(
                black_box(&full_metadata),
                black_box(&relationship),
                black_box(&target),
            )
        })
    });

    // No metadata
    let no_metadata = ChunkMetadata {
        id: 2,
        relpath: "src/chunk.rs".to_string(),
        importance_score: None,
        recency_score: None,
        churn_score: None,
    };
    group.bench_function("no_metadata", |b| {
        b.iter(|| {
            scorer.score(
                black_box(&no_metadata),
                black_box(&relationship),
                black_box(&target),
            )
        })
    });

    group.finish();
}

fn bench_custom_config(c: &mut Criterion) {
    let custom_config = ScoringConfig::new()
        .with_base_score(2.0)
        .with_decay_factor(0.8)
        .with_relationship_weight(EdgeType::TestOf, 2.5)
        .with_directory_bonus(1.5)
        .with_recency(false)
        .with_churn(false);

    let scorer = ImportanceScorer::with_config(custom_config);
    let target = create_benchmark_chunk(1000, "src/target.rs");
    let chunk = create_benchmark_chunk(1, "src/chunk.rs");
    let relationship = create_benchmark_relationship(EdgeType::TestOf, 1);

    c.bench_function("custom_config", |b| {
        b.iter(|| {
            scorer.score(
                black_box(&chunk),
                black_box(&relationship),
                black_box(&target),
            )
        })
    });
}

fn bench_realistic_context_assembly(c: &mut Criterion) {
    // Simulate a realistic context assembly scenario:
    // - 1 target chunk
    // - 50 related chunks at various distances
    // - Mixed relationship types
    // - Mixed directory locations

    let scorer = ImportanceScorer::new();
    let target = create_benchmark_chunk(1000, "src/api/user_handler.rs");

    let chunks: Vec<ChunkMetadata> = vec![
        // Tests (10)
        (0..10)
            .map(|i| create_benchmark_chunk(i, &format!("src/api/user_handler.test.rs#{}", i)))
            .collect::<Vec<_>>(),
        // Same directory chunks (15)
        (10..25)
            .map(|i| create_benchmark_chunk(i, &format!("src/api/helper_{}.rs", i)))
            .collect::<Vec<_>>(),
        // Different directory chunks (25)
        (25..50)
            .map(|i| create_benchmark_chunk(i, &format!("src/utils/util_{}.rs", i)))
            .collect::<Vec<_>>(),
    ]
    .into_iter()
    .flatten()
    .collect();

    let relationships: Vec<Relationship> = vec![
        // Tests
        (0..10)
            .map(|_| create_benchmark_relationship(EdgeType::TestOf, 1))
            .collect::<Vec<_>>(),
        // Same directory calls
        (10..25)
            .map(|i| create_benchmark_relationship(EdgeType::Calls, (i % 3) as u32))
            .collect::<Vec<_>>(),
        // Different directory imports
        (25..50)
            .map(|i| create_benchmark_relationship(EdgeType::Imports, (i % 4) as u32))
            .collect::<Vec<_>>(),
    ]
    .into_iter()
    .flatten()
    .collect();

    c.bench_function("realistic_assembly_50_chunks", |b| {
        b.iter(|| {
            for (chunk, rel) in chunks.iter().zip(relationships.iter()) {
                black_box(scorer.score(black_box(chunk), black_box(rel), black_box(&target)));
            }
        })
    });
}

// Performance target verification
fn bench_1000_chunks_performance_target(c: &mut Criterion) {
    let scorer = ImportanceScorer::new();
    let target = create_benchmark_chunk(1000, "src/target.rs");

    // Generate 1000 chunks with realistic variety
    let chunks: Vec<ChunkMetadata> = (0..1000)
        .map(|i| {
            ChunkMetadata {
                id: i as i64,
                relpath: format!("src/module_{}/file_{}.rs", i / 100, i % 100),
                importance_score: Some(1.0 + (i % 10) as f64 * 0.2),
                recency_score: Some(0.5 + (i % 5) as f64 * 0.1),
                churn_score: Some((i % 7) as f64 * 0.1),
            }
        })
        .collect();

    let relationships: Vec<Relationship> = (0..1000)
        .map(|i| {
            let edge_type = match i % 6 {
                0 => EdgeType::TestOf,
                1 => EdgeType::Calls,
                2 => EdgeType::Imports,
                3 => EdgeType::Exports,
                4 => EdgeType::CalledBy,
                _ => EdgeType::RouteOf,
            };
            create_benchmark_relationship(edge_type, (i % 5) as u32)
        })
        .collect();

    c.bench_function("TARGET_1000_chunks_under_10ms", |b| {
        b.iter(|| {
            for (chunk, rel) in chunks.iter().zip(relationships.iter()) {
                black_box(scorer.score(black_box(chunk), black_box(rel), black_box(&target)));
            }
        })
    });
}

criterion_group!(
    benches,
    bench_single_score,
    bench_batch_scoring,
    bench_different_relationship_types,
    bench_distance_variations,
    bench_directory_bonus_checks,
    bench_metadata_variations,
    bench_custom_config,
    bench_realistic_context_assembly,
    bench_1000_chunks_performance_target,
);

criterion_main!(benches);
