//! Ollama Parallel Embedding Benchmarks (EMBPERF-3001)
//!
//! Comprehensive benchmark suite measuring performance improvements from:
//! - EMBPERF-1001: Batch API support
//! - EMBPERF-2001: Parallel processing with configurable concurrency
//!
//! # Benchmarks
//!
//! 1. **Sequential Baseline**: Single-text requests (pre-optimization baseline)
//! 2. **Batch Sizes**: Compare batch sizes (25, 50, 100, 128)
//! 3. **Concurrency Levels**: Compare parallel configs (4, 8, 16)
//! 4. **Combined Matrix**: Optimal batch size × concurrency combinations
//!
//! # Running
//!
//! ```bash
//! # Prerequisites: Start Ollama with nomic-embed-text
//! ollama serve &
//! ollama pull nomic-embed-text
//!
//! # Run all benchmarks
//! cargo bench --bench ollama_parallel_bench
//!
//! # Run specific benchmark group
//! cargo bench --bench ollama_parallel_bench -- sequential
//! cargo bench --bench ollama_parallel_bench -- batch_sizes
//! cargo bench --bench ollama_parallel_bench -- concurrency
//! cargo bench --bench ollama_parallel_bench -- combined
//!
//! # Save baseline for comparison
//! cargo bench --bench ollama_parallel_bench -- --save-baseline before_parallel
//! # ... make changes ...
//! cargo bench --bench ollama_parallel_bench -- --baseline before_parallel
//! ```
//!
//! # Environment Variables
//!
//! - `MAPROOM_EMBEDDING_PARALLEL_ENABLED`: Enable/disable parallel processing (default: true)
//! - `MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE`: Sub-batch size (default: 50)
//! - `MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY`: Max concurrency (default: 8)
//!
//! # Requirements
//!
//! - Ollama running at localhost:11434
//! - nomic-embed-text model pulled
//! - Tests marked `#[ignore]` - run manually when Ollama available

use maproom::embedding::config::ParallelConfig;
use maproom::embedding::ollama::OllamaProvider;
use maproom::embedding::provider::EmbeddingProvider;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

/// Generate realistic test texts for benchmarking
///
/// Creates texts that resemble code chunks with varying content
/// to avoid any caching effects and ensure representative performance.
fn generate_test_texts(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| {
            format!(
                "Test chunk {} with code: fn test_{}() {{ println!(\"Processing item {}\"); let result = compute_value({}); return result; }}",
                i, i, i, i
            )
        })
        .collect()
}

/// Benchmark: Sequential baseline (single-text requests)
///
/// This simulates the "before" state - making individual API calls
/// for each text without batching. This is the baseline for measuring
/// improvement from batch and parallel optimizations.
fn bench_sequential_baseline(c: &mut Criterion) {
    // Check if Ollama is available - skip if not
    let rt = tokio::runtime::Runtime::new().unwrap();
    let provider = match rt.block_on(async {
        let p = OllamaProvider::default_config().ok()?;
        // Test connectivity
        p.embed("test".to_string()).await.ok()?;
        Some(p)
    }) {
        Some(p) => p,
        None => {
            eprintln!("Skipping sequential_baseline: Ollama not available");
            return;
        }
    };

    let mut group = c.benchmark_group("sequential_baseline");
    group.sample_size(10); // Fewer samples for slow sequential processing
    group.measurement_time(Duration::from_secs(30));

    for size in [10, 25, 50] {
        let texts = generate_test_texts(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &texts, |b, texts| {
            b.to_async(&rt).iter(|| async {
                let p = provider.clone();
                let mut results = Vec::with_capacity(texts.len());
                for text in texts {
                    let embedding = p.embed(black_box(text.clone())).await.unwrap();
                    results.push(embedding);
                }
                black_box(results)
            });
        });
    }

    group.finish();
}

/// Benchmark: Batch API performance (single batch, no parallelism)
///
/// Tests different batch sizes to find optimal batching without parallelism.
/// Compares: 25, 50, 100, 128 texts per batch.
fn bench_batch_sizes(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create provider with parallel disabled (batch-only mode)
    let provider = match rt.block_on(async {
        let config = ParallelConfig {
            enabled: false,       // Batch only, no parallelism
            sub_batch_size: 1000, // Large enough to fit entire batch
            max_concurrency: 1,
        };
        let p = OllamaProvider::new_with_config(
            OllamaProvider::DEFAULT_ENDPOINT.to_string(),
            OllamaProvider::DEFAULT_MODEL.to_string(),
            768,
            config,
        )
        .ok()?;

        // Test connectivity
        p.embed("test".to_string()).await.ok()?;
        Some(p)
    }) {
        Some(p) => p,
        None => {
            eprintln!("Skipping batch_sizes: Ollama not available");
            return;
        }
    };

    let mut group = c.benchmark_group("batch_sizes");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(30));

    for size in [25, 50, 100, 128] {
        let texts = generate_test_texts(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &texts, |b, texts| {
            b.to_async(&rt).iter(|| async {
                let p = provider.clone();
                let embeddings = p.embed_batch(black_box(texts.clone())).await.unwrap();
                black_box(embeddings)
            });
        });
    }

    group.finish();
}

/// Benchmark: Concurrency levels (parallel processing)
///
/// Tests different concurrency levels with fixed batch size.
/// Measures impact of: 4, 8, 16 concurrent requests.
fn bench_concurrency_levels(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrency_levels");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(30));

    let batch_size = 200; // Large enough to benefit from parallelism
    let texts = generate_test_texts(batch_size);
    group.throughput(Throughput::Elements(batch_size as u64));

    for concurrency in [4, 8, 16] {
        // Create provider with specific concurrency level
        let provider = match rt.block_on(async {
            let config = ParallelConfig {
                enabled: true,
                sub_batch_size: 50, // Standard sub-batch size
                max_concurrency: concurrency,
            };
            let p = OllamaProvider::new_with_config(
                OllamaProvider::DEFAULT_ENDPOINT.to_string(),
                OllamaProvider::DEFAULT_MODEL.to_string(),
                768,
                config,
            )
            .ok()?;

            // Test connectivity
            p.embed("test".to_string()).await.ok()?;
            Some(p)
        }) {
            Some(p) => p,
            None => {
                eprintln!(
                    "Skipping concurrency level {}: Ollama not available",
                    concurrency
                );
                continue;
            }
        };

        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            &texts,
            |b, texts| {
                b.to_async(&rt).iter(|| async {
                    let p = provider.clone();
                    let embeddings = p.embed_batch(black_box(texts.clone())).await.unwrap();
                    black_box(embeddings)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Combined matrix (batch size × concurrency)
///
/// Tests combinations of batch sizes and concurrency levels to identify
/// optimal configuration for different hardware profiles.
fn bench_combined(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("combined_matrix");
    group.sample_size(15);
    group.measurement_time(Duration::from_secs(30));

    let total_texts = 200;
    let texts = generate_test_texts(total_texts);
    group.throughput(Throughput::Elements(total_texts as u64));

    // Test matrix: (sub_batch_size, max_concurrency)
    let configurations = [
        (25, 4),
        (25, 8),
        (50, 4),
        (50, 8),
        (50, 16),
        (100, 4),
        (100, 8),
    ];

    for (sub_batch_size, max_concurrency) in configurations {
        let config_name = format!("batch{}_con{}", sub_batch_size, max_concurrency);

        let provider = match rt.block_on(async {
            let config = ParallelConfig {
                enabled: true,
                sub_batch_size,
                max_concurrency,
            };
            let p = OllamaProvider::new_with_config(
                OllamaProvider::DEFAULT_ENDPOINT.to_string(),
                OllamaProvider::DEFAULT_MODEL.to_string(),
                768,
                config,
            )
            .ok()?;

            // Test connectivity
            p.embed("test".to_string()).await.ok()?;
            Some(p)
        }) {
            Some(p) => p,
            None => {
                eprintln!("Skipping {}: Ollama not available", config_name);
                continue;
            }
        };

        group.bench_with_input(
            BenchmarkId::new("config", &config_name),
            &texts,
            |b, texts| {
                b.to_async(&rt).iter(|| async {
                    let p = provider.clone();
                    let embeddings = p.embed_batch(black_box(texts.clone())).await.unwrap();
                    black_box(embeddings)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_sequential_baseline,
    bench_batch_sizes,
    bench_concurrency_levels,
    bench_combined
);
criterion_main!(benches);
