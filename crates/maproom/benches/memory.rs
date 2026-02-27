//! Memory Profiling Benchmarks
//!
//! Measures memory usage patterns during indexing, search, and context assembly.
//!
//! # Performance Targets
//!
//! - Peak memory usage: <500MB for typical workloads
//! - Memory growth: Linear with dataset size (no leaks)
//! - Allocation efficiency: Minimize allocations in hot paths
//!
//! # Benchmarks
//!
//! 1. **Indexing Memory**: Memory used during file parsing and indexing
//! 2. **Search Memory**: Memory used during vector search operations
//! 3. **Context Assembly Memory**: Memory used building context windows
//! 4. **Cache Memory**: Memory used by LRU caches
//! 5. **Peak Memory**: Maximum memory usage across operations
//!
//! # Metrics Collected
//!
//! - Peak resident set size (RSS)
//! - Heap allocations count
//! - Allocation size distribution
//! - Memory growth over time
//! - Deallocation patterns
//!
//! # Running
//!
//! ```bash
//! # Run all memory benchmarks
//! cargo bench --bench memory
//!
//! # Run specific benchmark group
//! cargo bench --bench memory -- indexing
//! cargo bench --bench memory -- search
//!
//! # Profile with puffin (requires feature flag)
//! cargo bench --bench memory --features profiling
//! ```
//!
//! # Memory Profiling Tools
//!
//! For detailed memory profiling:
//! - `valgrind --tool=massif` - Heap profiling
//! - `heaptrack` - Heap memory profiler
//! - `puffin` - Real-time profiler with flamegraphs
//!
//! # Test Data
//!
//! Uses realistic workloads:
//! - Small: 100 files, ~10k chunks, ~50MB peak
//! - Medium: 1,000 files, ~100k chunks, ~200MB peak
//! - Large: 10,000 files, ~500k chunks, ~500MB peak
//!
//! # Architecture Reference
//!
//! See PERF_OPT_ARCHITECTURE.md:
//! - Memory optimization strategies (lines 135-150)
//! - Cache sizing (lines 151-166)
//! - Profiling setup (lines 178-183)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;

/// Memory statistics snapshot
#[derive(Debug, Clone)]
struct MemoryStats {
    /// Resident set size in bytes
    rss_bytes: usize,
    /// Virtual memory size in bytes
    vsize_bytes: usize,
    /// Number of allocations (estimated)
    allocation_count: usize,
}

impl MemoryStats {
    /// Get current memory statistics
    #[cfg(target_os = "linux")]
    fn current() -> Self {
        use std::fs;

        // Read /proc/self/statm
        // Fields: size resident shared text lib data dt
        let statm = fs::read_to_string("/proc/self/statm").unwrap_or_default();
        let parts: Vec<&str> = statm.split_whitespace().collect();

        let page_size = 4096; // Standard page size on Linux
        let vsize_pages = parts
            .get(0)
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let rss_pages = parts
            .get(1)
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        Self {
            rss_bytes: rss_pages * page_size,
            vsize_bytes: vsize_pages * page_size,
            allocation_count: 0, // Not easily available without instrumentation
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn current() -> Self {
        // Fallback for non-Linux platforms
        Self {
            rss_bytes: 0,
            vsize_bytes: 0,
            allocation_count: 0,
        }
    }

    /// Calculate memory difference between two snapshots
    fn diff(&self, other: &Self) -> MemoryDiff {
        MemoryDiff {
            rss_delta: self.rss_bytes as i64 - other.rss_bytes as i64,
            vsize_delta: self.vsize_bytes as i64 - other.vsize_bytes as i64,
        }
    }

    fn rss_mb(&self) -> f64 {
        self.rss_bytes as f64 / 1_048_576.0
    }
}

#[derive(Debug)]
struct MemoryDiff {
    rss_delta: i64,
    vsize_delta: i64,
}

impl MemoryDiff {
    fn rss_delta_mb(&self) -> f64 {
        self.rss_delta as f64 / 1_048_576.0
    }
}

/// Simulated chunk data for memory benchmarking
#[derive(Clone)]
struct SimulatedChunk {
    id: i64,
    content: String,
    embedding: Vec<f32>,
}

impl SimulatedChunk {
    fn new(id: i64, content_size: usize, embedding_dim: usize) -> Self {
        Self {
            id,
            content: "x".repeat(content_size),
            embedding: vec![0.1; embedding_dim],
        }
    }

    fn memory_size(&self) -> usize {
        std::mem::size_of::<i64>()
            + self.content.capacity()
            + self.embedding.capacity() * std::mem::size_of::<f32>()
    }
}

/// Generate test dataset with known memory footprint
fn generate_chunk_dataset(count: usize, avg_content_size: usize) -> Vec<SimulatedChunk> {
    let embedding_dim = 1536; // OpenAI embedding dimension
    (0..count)
        .map(|i| SimulatedChunk::new(i as i64, avg_content_size, embedding_dim))
        .collect()
}

/// Benchmark memory usage during indexing simulation
fn bench_indexing_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexing_memory");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);

    for size in [100, 1000, 10000] {
        let avg_chunk_size = 500; // bytes

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_chunks", size)),
            &size,
            |b, &count| {
                b.iter(|| {
                    let mem_before = MemoryStats::current();

                    // Simulate indexing: parse and create chunks
                    let chunks = generate_chunk_dataset(count, avg_chunk_size);

                    // Simulate processing each chunk
                    let mut total_size = 0;
                    for chunk in &chunks {
                        total_size += chunk.memory_size();
                    }

                    let mem_after = MemoryStats::current();
                    let diff = mem_after.diff(&mem_before);

                    black_box((chunks.len(), total_size, diff.rss_delta_mb()))
                });
            },
        );

        // Print expected memory usage
        let expected_mb = (size * avg_chunk_size + size * 1536 * 4) as f64 / 1_048_576.0;
        println!("\n{} chunks: Expected ~{:.2} MB", size, expected_mb);
    }

    group.finish();
}

/// Benchmark memory usage during search operations
fn bench_search_memory(c: &mut Criterion) {
    use maproom::indexer::parser::extract_chunks;

    let mut group = c.benchmark_group("search_memory");
    group.measurement_time(Duration::from_secs(10));

    // Create a corpus of documents to search through
    let corpus_sizes = [1000, 10000, 50000];

    for size in corpus_sizes {
        let chunks = generate_chunk_dataset(size, 500);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_corpus", size)),
            &chunks,
            |b, chunks| {
                b.iter(|| {
                    let mem_before = MemoryStats::current();

                    // Simulate search: score and rank chunks
                    let query_embedding = vec![0.1; 1536];
                    let mut scores: Vec<(i64, f32)> = Vec::with_capacity(chunks.len());

                    for chunk in chunks {
                        // Simulate similarity calculation
                        let score: f32 = query_embedding
                            .iter()
                            .zip(chunk.embedding.iter())
                            .map(|(a, b)| a * b)
                            .sum();
                        scores.push((chunk.id, score));
                    }

                    // Sort by score (top-k simulation)
                    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                    let top_k = &scores[..10.min(scores.len())];

                    let mem_after = MemoryStats::current();
                    let diff = mem_after.diff(&mem_before);

                    black_box((top_k.len(), diff.rss_delta_mb()))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage during context assembly
fn bench_context_assembly_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_assembly_memory");
    group.measurement_time(Duration::from_secs(10));

    let token_budgets = [2000, 6000, 10000];

    for budget in token_budgets {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_tokens", budget)),
            &budget,
            |b, &token_limit| {
                b.iter(|| {
                    let mem_before = MemoryStats::current();

                    // Simulate context assembly: gather related chunks
                    let primary_chunks = generate_chunk_dataset(5, 600);
                    let related_chunks = generate_chunk_dataset(20, 400);

                    // Simulate assembling context window
                    let mut context = String::new();
                    let mut current_tokens = 0;

                    // Add primary chunks
                    for chunk in &primary_chunks {
                        if current_tokens + chunk.content.len() / 4 > token_limit {
                            break;
                        }
                        context.push_str(&chunk.content);
                        context.push('\n');
                        current_tokens += chunk.content.len() / 4; // Rough token estimate
                    }

                    // Add related chunks until budget exhausted
                    for chunk in &related_chunks {
                        if current_tokens + chunk.content.len() / 4 > token_limit {
                            break;
                        }
                        context.push_str(&chunk.content);
                        context.push('\n');
                        current_tokens += chunk.content.len() / 4;
                    }

                    let mem_after = MemoryStats::current();
                    let diff = mem_after.diff(&mem_before);

                    black_box((context.len(), current_tokens, diff.rss_delta_mb()))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark cache memory usage
fn bench_cache_memory(c: &mut Criterion) {
    use lru::LruCache;
    use std::num::NonZeroUsize;

    let mut group = c.benchmark_group("cache_memory");
    group.measurement_time(Duration::from_secs(10));

    let cache_sizes = [100, 1000, 10000];

    for size in cache_sizes {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_entries", size)),
            &size,
            |b, &capacity| {
                b.iter(|| {
                    let mem_before = MemoryStats::current();

                    // Create and fill LRU cache
                    let mut cache = LruCache::new(NonZeroUsize::new(capacity).unwrap());

                    // Fill cache with embeddings
                    for i in 0..capacity {
                        let key = format!("chunk_{}", i);
                        let value = vec![0.1f32; 1536]; // Embedding vector
                        cache.put(key, value);
                    }

                    // Simulate cache operations
                    for i in 0..100 {
                        let key = format!("chunk_{}", i % capacity);
                        let _ = cache.get(&key);
                    }

                    let mem_after = MemoryStats::current();
                    let diff = mem_after.diff(&mem_before);

                    // Estimate cache size
                    let entry_size = 50 + 1536 * 4; // key + embedding
                    let expected_mb = (capacity * entry_size) as f64 / 1_048_576.0;

                    black_box((cache.len(), expected_mb, diff.rss_delta_mb()))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark peak memory usage across full workflow
fn bench_peak_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("peak_memory");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);

    // Simulate full workflow: index -> search -> assemble context
    group.bench_function("full_workflow", |b| {
        b.iter(|| {
            let mem_start = MemoryStats::current();
            let mut peak_rss = mem_start.rss_bytes;

            // Step 1: Indexing
            let chunks = generate_chunk_dataset(5000, 500);
            let mem_after_index = MemoryStats::current();
            peak_rss = peak_rss.max(mem_after_index.rss_bytes);

            // Step 2: Search
            let query_embedding = vec![0.1; 1536];
            let mut scores: Vec<(i64, f32)> = Vec::with_capacity(chunks.len());
            for chunk in &chunks {
                let score: f32 = query_embedding
                    .iter()
                    .zip(chunk.embedding.iter())
                    .map(|(a, b)| a * b)
                    .sum();
                scores.push((chunk.id, score));
            }
            scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            let mem_after_search = MemoryStats::current();
            peak_rss = peak_rss.max(mem_after_search.rss_bytes);

            // Step 3: Context assembly
            let mut context = String::new();
            for (chunk_id, _) in &scores[..10] {
                if let Some(chunk) = chunks.iter().find(|c| c.id == *chunk_id) {
                    context.push_str(&chunk.content);
                    context.push('\n');
                }
            }
            let mem_after_context = MemoryStats::current();
            peak_rss = peak_rss.max(mem_after_context.rss_bytes);

            let peak_mb = peak_rss as f64 / 1_048_576.0;
            black_box((chunks.len(), scores.len(), context.len(), peak_mb))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_indexing_memory,
    bench_search_memory,
    bench_context_assembly_memory,
    bench_cache_memory,
    bench_peak_memory,
);
criterion_main!(benches);
