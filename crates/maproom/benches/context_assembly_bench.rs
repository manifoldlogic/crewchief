//! Context assembly performance benchmarks.
//!
//! Measures context assembly latency with and without parallel processing optimizations.
//!
//! # Performance Targets
//!
//! - p50 assembly time: <50ms
//! - p95 assembly time: <120ms
//! - p99 assembly time: <200ms
//!
//! # Benchmarks
//!
//! 1. **Baseline Sequential**: Current implementation without parallelization
//! 2. **Parallel Loading**: With tokio::join! for concurrent operations
//! 3. **Varying Complexity**: Simple (primary only) vs Complex (with relationships)
//! 4. **Budget Impact**: Different token budgets (2k, 6k, 10k tokens)
//!
//! # Running
//!
//! ```bash
//! # Run all context assembly benchmarks
//! cargo bench --bench context_assembly_bench
//!
//! # Run specific benchmark group
//! cargo bench --bench context_assembly_bench -- simple
//! cargo bench --bench context_assembly_bench -- complex
//!
//! # Compare before/after optimizations
//! cargo bench --bench context_assembly_bench -- --save-baseline before
//! # ... make changes ...
//! cargo bench --bench context_assembly_bench -- --baseline before
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

/// Simulated chunk metadata for benchmarking (without database).
///
/// In real benchmarks with MAPROOM_DATABASE_URL, this would use actual database queries.
#[derive(Debug, Clone)]
struct SimulatedChunk {
    id: i64,
    relpath: String,
    symbol_name: Option<String>,
    kind: String,
    start_line: i32,
    end_line: i32,
    content_size: usize,
}

impl SimulatedChunk {
    fn simple() -> Self {
        Self {
            id: 1,
            relpath: "src/lib.rs".to_string(),
            symbol_name: Some("process_data".to_string()),
            kind: "func".to_string(),
            start_line: 10,
            end_line: 50,
            content_size: 1200, // ~400 tokens
        }
    }

    fn with_relationships() -> Vec<Self> {
        vec![
            Self::simple(),
            // Tests
            Self {
                id: 2,
                relpath: "tests/lib_test.rs".to_string(),
                symbol_name: Some("test_process_data".to_string()),
                kind: "test".to_string(),
                start_line: 5,
                end_line: 20,
                content_size: 800,
            },
            // Callers
            Self {
                id: 3,
                relpath: "src/main.rs".to_string(),
                symbol_name: Some("main".to_string()),
                kind: "func".to_string(),
                start_line: 1,
                end_line: 15,
                content_size: 600,
            },
            // Callees
            Self {
                id: 4,
                relpath: "src/utils.rs".to_string(),
                symbol_name: Some("validate".to_string()),
                kind: "func".to_string(),
                start_line: 20,
                end_line: 40,
                content_size: 900,
            },
            Self {
                id: 5,
                relpath: "src/utils.rs".to_string(),
                symbol_name: Some("format_output".to_string()),
                kind: "func".to_string(),
                start_line: 45,
                end_line: 60,
                content_size: 700,
            },
        ]
    }
}

/// Simulate sequential assembly (baseline).
async fn simulate_sequential_assembly(chunks: &[SimulatedChunk], budget: usize) -> usize {
    let mut total_tokens = 0;

    for chunk in chunks {
        if total_tokens >= budget {
            break;
        }

        // Simulate file I/O latency (2-5ms per file)
        tokio::time::sleep(Duration::from_micros(3000)).await;

        // Simulate token counting (fast, ~0.1ms)
        tokio::time::sleep(Duration::from_micros(100)).await;

        let tokens = chunk.content_size / 3; // Rough estimate
        if total_tokens + tokens <= budget {
            total_tokens += tokens;
        }
    }

    total_tokens
}

/// Simulate parallel assembly with tokio::join!.
async fn simulate_parallel_assembly(chunks: &[SimulatedChunk], budget: usize) -> usize {
    if chunks.is_empty() {
        return 0;
    }

    // Split chunks into categories for parallel loading
    let primary = &chunks[0];
    let related: Vec<_> = chunks.iter().skip(1).collect();

    // Load primary and all relationships in parallel
    let (primary_tokens, related_tokens) = tokio::join!(
        async {
            // Simulate file I/O for primary
            tokio::time::sleep(Duration::from_micros(3000)).await;
            tokio::time::sleep(Duration::from_micros(100)).await;
            primary.content_size / 3
        },
        async {
            // Load all related chunks in parallel
            let mut handles = vec![];
            for chunk in related.iter().take(4) {
                // Limit parallelism
                let chunk = (*chunk).clone();
                let handle = tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_micros(3000)).await;
                    tokio::time::sleep(Duration::from_micros(100)).await;
                    chunk.content_size / 3
                });
                handles.push(handle);
            }

            let mut total = 0;
            for handle in handles {
                if let Ok(tokens) = handle.await {
                    total += tokens;
                }
            }
            total
        }
    );

    let total_tokens = primary_tokens + related_tokens;
    if total_tokens > budget {
        budget
    } else {
        total_tokens
    }
}

/// Benchmark simple context assembly (primary chunk only).
fn bench_simple_assembly(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_assembly_simple");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let budgets = vec![2000, 6000, 10000];

    for budget in budgets {
        group.throughput(Throughput::Elements(1));

        // Sequential baseline
        group.bench_with_input(
            BenchmarkId::new("sequential", budget),
            &budget,
            |b, &budget| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                b.to_async(rt).iter(|| async {
                    let chunks = vec![SimulatedChunk::simple()];
                    black_box(simulate_sequential_assembly(&chunks, budget).await)
                });
            },
        );

        // Parallel (should be similar for single chunk)
        group.bench_with_input(
            BenchmarkId::new("parallel", budget),
            &budget,
            |b, &budget| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                b.to_async(rt).iter(|| async {
                    let chunks = vec![SimulatedChunk::simple()];
                    black_box(simulate_parallel_assembly(&chunks, budget).await)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark complex context assembly (with relationships).
fn bench_complex_assembly(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_assembly_complex");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let budgets = vec![2000, 6000, 10000];

    for budget in budgets {
        group.throughput(Throughput::Elements(5)); // 5 chunks total

        // Sequential baseline
        group.bench_with_input(
            BenchmarkId::new("sequential", budget),
            &budget,
            |b, &budget| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                b.to_async(rt).iter(|| async {
                    let chunks = SimulatedChunk::with_relationships();
                    black_box(simulate_sequential_assembly(&chunks, budget).await)
                });
            },
        );

        // Parallel optimization
        group.bench_with_input(
            BenchmarkId::new("parallel", budget),
            &budget,
            |b, &budget| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                b.to_async(rt).iter(|| async {
                    let chunks = SimulatedChunk::with_relationships();
                    black_box(simulate_parallel_assembly(&chunks, budget).await)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark latency percentiles for complex assembly.
fn bench_latency_distribution(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_assembly_latency");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(200); // More samples for better percentile accuracy

    let budget = 6000; // Realistic production budget

    // Sequential baseline
    group.bench_function("sequential_p95", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(rt).iter(|| async {
            let chunks = SimulatedChunk::with_relationships();
            black_box(simulate_sequential_assembly(&chunks, budget).await)
        });
    });

    // Parallel optimization
    group.bench_function("parallel_p95", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(rt).iter(|| async {
            let chunks = SimulatedChunk::with_relationships();
            black_box(simulate_parallel_assembly(&chunks, budget).await)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_simple_assembly,
    bench_complex_assembly,
    bench_latency_distribution
);
criterion_main!(benches);
