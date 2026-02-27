//! Memory Optimization Benchmarks
//!
//! Benchmarks for PERF_OPT-5001 memory optimization features:
//! - String interning
//! - Vector quantization
//! - Buffer pooling
//! - Memory metrics tracking
//!
//! # Performance Targets
//!
//! - String interning: >80% hit rate on typical paths
//! - Vector quantization: 75% memory reduction (f32 → i8)
//! - Buffer pooling: >90% reuse rate
//! - Memory usage: <500MB for 100k chunks
//!
//! # Running
//!
//! ```bash
//! # Run all memory optimization benchmarks
//! cargo bench --bench memory_optimization_bench
//!
//! # Run specific benchmark
//! cargo bench --bench memory_optimization_bench -- interner
//! cargo bench --bench memory_optimization_bench -- quantization
//! cargo bench --bench memory_optimization_bench -- pool
//! ```

use maproom::memory::{
    dequantize_embedding, get_memory_metrics, quantize_embedding, BufferPool, StringInterner,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;

/// Benchmark string interning performance
fn bench_string_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_interning");

    // Typical file paths in a codebase
    let paths = vec![
        "src/main.rs",
        "src/lib.rs",
        "src/utils/mod.rs",
        "src/utils/helpers.rs",
        "tests/integration.rs",
        "benches/benchmark.rs",
        "Cargo.toml",
        "README.md",
    ];

    // Benchmark intern performance with cold cache
    group.bench_function("intern_cold", |b| {
        b.iter(|| {
            let interner = StringInterner::new();
            for path in &paths {
                black_box(interner.intern(path));
            }
        });
    });

    // Benchmark intern performance with hot cache (repeated paths)
    group.bench_function("intern_hot", |b| {
        let interner = StringInterner::new();
        b.iter(|| {
            for path in &paths {
                black_box(interner.intern(path));
            }
        });
    });

    // Benchmark intern performance with realistic duplication (100k paths, 1k unique)
    group.bench_function("intern_realistic_100k", |b| {
        let unique_paths: Vec<String> = (0..1000)
            .map(|i| format!("src/module_{}/file_{}.rs", i / 10, i % 10))
            .collect();

        b.iter(|| {
            let interner = StringInterner::new();
            for i in 0..100_000 {
                let path = &unique_paths[i % 1000];
                black_box(interner.intern(path));
            }
        });
    });

    // Benchmark memory savings
    group.bench_function("memory_savings", |b| {
        let interner = StringInterner::new();

        // Simulate 100k chunks, each with a file path
        // Typical codebase has ~1k unique files
        let unique_paths: Vec<String> = (0..1000)
            .map(|i| format!("src/module_{}/component_{}.rs", i / 20, i % 20))
            .collect();

        b.iter(|| {
            for i in 0..100_000 {
                let path = &unique_paths[i % 1000];
                black_box(interner.intern(path));
            }

            let stats = interner.stats();
            black_box((
                stats.unique_strings,
                stats.hit_rate(),
                stats.deduplication_ratio(),
            ))
        });
    });

    group.finish();
}

/// Benchmark vector quantization performance
fn bench_vector_quantization(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_quantization");

    // Standard OpenAI embedding dimension
    let embedding_dim = 1536;

    // Create test embedding
    let embedding: Vec<f32> = (0..embedding_dim)
        .map(|i| (i as f32 / embedding_dim as f32) * 2.0 - 1.0)
        .collect();

    // Benchmark quantization
    group.bench_function("quantize", |b| {
        b.iter(|| {
            let quantized = quantize_embedding(black_box(&embedding));
            black_box(quantized)
        });
    });

    // Benchmark dequantization
    let quantized = quantize_embedding(&embedding);
    group.bench_function("dequantize", |b| {
        b.iter(|| {
            let dequantized = dequantize_embedding(black_box(&quantized));
            black_box(dequantized)
        });
    });

    // Benchmark roundtrip (quantize + dequantize)
    group.bench_function("roundtrip", |b| {
        b.iter(|| {
            let quantized = quantize_embedding(black_box(&embedding));
            let dequantized = dequantize_embedding(&quantized);
            black_box(dequantized)
        });
    });

    // Benchmark memory savings for 100k embeddings
    group.throughput(Throughput::Elements(100_000));
    group.bench_function("memory_savings_100k", |b| {
        b.iter(|| {
            // Simulate quantizing 100k embeddings
            let mut total_f32_bytes = 0;
            let mut total_i8_bytes = 0;

            for _ in 0..100_000 {
                let quantized = quantize_embedding(&embedding);
                total_f32_bytes += embedding.len() * std::mem::size_of::<f32>();
                total_i8_bytes += quantized.len() * std::mem::size_of::<i8>();
            }

            let savings_mb = (total_f32_bytes - total_i8_bytes) as f64 / (1024.0 * 1024.0);
            black_box(savings_mb)
        });
    });

    group.finish();
}

/// Benchmark buffer pooling performance
fn bench_buffer_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_pool");

    let buffer_size = 64 * 1024; // 64KB
    let pool_size = 10;

    // Benchmark acquire with cold pool (allocates)
    group.bench_function("acquire_cold", |b| {
        b.iter(|| {
            let pool = BufferPool::new(buffer_size, pool_size);
            let buffer = pool.acquire();
            black_box(buffer)
        });
    });

    // Benchmark acquire with hot pool (reuses)
    group.bench_function("acquire_hot", |b| {
        let pool = Arc::new(BufferPool::new(buffer_size, pool_size));

        // Pre-populate pool
        {
            let buffers: Vec<_> = (0..pool_size).map(|_| pool.acquire()).collect();
            drop(buffers);
        }

        b.iter(|| {
            let buffer = pool.acquire();
            black_box(buffer)
        });
    });

    // Benchmark realistic file reading pattern
    group.bench_function("file_reading_pattern", |b| {
        let pool = Arc::new(BufferPool::new(buffer_size, pool_size));

        b.iter(|| {
            // Simulate reading 100 files
            for _ in 0..100 {
                let mut buffer = pool.acquire();
                // Simulate file read
                buffer.extend_from_slice(&vec![0u8; 1024]);
                // Buffer returns to pool on drop
            }

            let stats = pool.stats();
            black_box((stats.reuse_rate(), stats.allocations, stats.reuses))
        });
    });

    // Benchmark pool stats overhead
    group.bench_function("stats_overhead", |b| {
        let pool = BufferPool::new(buffer_size, pool_size);
        b.iter(|| {
            let stats = pool.stats();
            black_box(stats)
        });
    });

    group.finish();
}

/// Benchmark memory metrics tracking
fn bench_memory_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_metrics");

    // Benchmark allocation tracking
    group.bench_function("record_allocation", |b| {
        let metrics = get_memory_metrics();
        b.iter(|| {
            metrics.record_allocation(black_box("indexer"), black_box(1024));
        });
    });

    // Benchmark deallocation tracking
    group.bench_function("record_deallocation", |b| {
        let metrics = get_memory_metrics();
        b.iter(|| {
            metrics.record_deallocation(black_box("indexer"), black_box(1024));
        });
    });

    // Benchmark snapshot creation
    group.bench_function("snapshot", |b| {
        let metrics = get_memory_metrics();
        // Pre-populate with some data
        for i in 0..100 {
            metrics.record_allocation(&format!("component_{}", i % 10), 1024);
        }

        b.iter(|| {
            let snapshot = metrics.snapshot();
            black_box(snapshot)
        });
    });

    // Benchmark realistic tracking workload
    group.bench_function("realistic_tracking", |b| {
        let metrics = get_memory_metrics();

        b.iter(|| {
            // Simulate indexing 1000 chunks
            for _ in 0..1000 {
                metrics.record_allocation("indexer", 8192);
            }

            // Simulate freeing half
            for _ in 0..500 {
                metrics.record_deallocation("indexer", 8192);
            }

            let current = metrics.current_mb();
            black_box(current)
        });
    });

    group.finish();
}

/// Benchmark combined optimizations
fn bench_combined_optimizations(c: &mut Criterion) {
    let mut group = c.benchmark_group("combined_optimizations");

    // Simulate realistic workflow with all optimizations
    group.bench_function("realistic_100k_chunks", |b| {
        b.iter(|| {
            let metrics = get_memory_metrics();
            let interner = Arc::new(StringInterner::new());
            let pool = Arc::new(BufferPool::new(64 * 1024, 10));

            // Simulate 100k chunks with optimizations
            let unique_files = 1000;
            let files: Vec<String> = (0..unique_files)
                .map(|i| format!("src/module_{}/file.rs", i))
                .collect();

            let embedding_dim = 1536;
            let sample_embedding: Vec<f32> = (0..embedding_dim)
                .map(|i| (i as f32 / embedding_dim as f32))
                .collect();

            for i in 0..100_000 {
                // String interning for file paths
                let file_path = &files[i % unique_files];
                let _interned = interner.intern(file_path);

                // Vector quantization for embeddings
                let quantized = quantize_embedding(&sample_embedding);

                // Buffer pooling for file I/O (simulated)
                let mut buffer = pool.acquire();
                // Convert i8 to u8 for buffer storage
                let bytes: Vec<u8> = quantized.iter().map(|&x| x as u8).collect();
                buffer.extend_from_slice(&bytes);

                // Memory tracking
                let allocation_size = quantized.len();
                metrics.record_allocation("indexer", allocation_size as u64);

                // Simulate processing
                black_box((&_interned, quantized.len(), buffer.len()));
            }

            // Get final statistics
            let interner_stats = interner.stats();
            let pool_stats = pool.stats();
            let mem_snapshot = metrics.snapshot();

            black_box((
                interner_stats.deduplication_ratio(),
                pool_stats.reuse_rate(),
                mem_snapshot.current_mb(),
            ))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_string_interning,
    bench_vector_quantization,
    bench_buffer_pool,
    bench_memory_metrics,
    bench_combined_optimizations,
);
criterion_main!(benches);
