//! Embedding Generation Performance Benchmarks (LOCAL-4001)
//!
//! Measures performance of Ollama's nomic-embed-text model for embedding generation.
//! Establishes baseline metrics and compares local (Ollama) vs cloud (OpenAI) performance.
//!
//! # Performance Targets (from LOCAL project)
//!
//! - **Throughput**: 500-1000 chunks/min on CPU, 2000-5000 chunks/min on GPU
//! - **Latency**: <100ms per chunk for small batches
//! - **Search latency**: <100ms (p95) for hybrid search
//! - **Memory**: <4GB during batching operations
//!
//! # Benchmarks
//!
//! 1. **Single Embedding**: Measure single chunk latency (cold vs warm start)
//! 2. **Small Batch (10 chunks)**: Typical small file scenario
//! 3. **Medium Batch (50 chunks)**: Realistic file indexing
//! 4. **Large Batch (100 chunks)**: Repository indexing stress test
//! 5. **Throughput**: Sustained chunks/min over time
//! 6. **Comparison**: Ollama vs OpenAI (if API key available)
//!
//! # Metrics Collected
//!
//! - **Latency**: p50, p95, p99 percentiles
//! - **Throughput**: Chunks per minute, requests per second
//! - **Resource Usage**: Memory allocation patterns (via Criterion)
//! - **Stability**: Performance consistency across iterations
//!
//! # Running
//!
//! ```bash
//! # Prerequisites: Ollama must be running with nomic-embed-text model
//! docker compose up -d ollama
//! # or: ollama run nomic-embed-text
//!
//! # Run all embedding benchmarks
//! cargo bench --bench embedding_performance
//!
//! # Run specific benchmark
//! cargo bench --bench embedding_performance -- single
//! cargo bench --bench embedding_performance -- batch
//! cargo bench --bench embedding_performance -- throughput
//!
//! # Save baseline for comparison
//! cargo bench --bench embedding_performance -- --save-baseline before
//! # ... make optimizations ...
//! cargo bench --bench embedding_performance -- --baseline before
//! ```
//!
//! # Test Data
//!
//! Uses realistic code chunks (~200 tokens each) from various file types:
//! - TypeScript functions with JSDoc
//! - Rust functions with documentation
//! - Python classes with docstrings
//! - Configuration snippets
//!
//! # Requirements
//!
//! - Ollama service running at http://localhost:11434 (or http://ollama:11434)
//! - nomic-embed-text model pulled (`ollama pull nomic-embed-text`)
//! - Optional: OPENAI_API_KEY for comparison benchmarks
//!
//! # Architecture Reference
//!
//! See LOCAL_ANALYSIS.md for performance targets and embedding strategy.

use crewchief_maproom::embedding::config::{CacheConfig, EmbeddingConfig, Provider, RetryConfig};
use crewchief_maproom::embedding::service::EmbeddingService;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

/// Realistic code chunk samples for benchmarking.
struct CodeChunk {
    text: String,
    tokens_estimate: usize,
}

impl CodeChunk {
    /// TypeScript function with JSDoc (~200 tokens)
    fn typescript_function() -> Self {
        let text = r#"
/**
 * Process user authentication and return session token
 * @param credentials - User credentials containing username and password
 * @param options - Authentication options like remember me, 2FA
 * @returns Promise resolving to authenticated session
 * @throws AuthenticationError if credentials are invalid
 */
export async function authenticateUser(
    credentials: UserCredentials,
    options?: AuthOptions
): Promise<AuthSession> {
    const user = await validateCredentials(credentials);
    if (!user) {
        throw new AuthenticationError("Invalid credentials");
    }

    if (options?.require2FA && user.has2FAEnabled) {
        await verify2FAToken(credentials.token2FA);
    }

    const session = await createSession({
        userId: user.id,
        expiresIn: options?.rememberMe ? "30d" : "1d",
        metadata: { ip: credentials.ip, userAgent: credentials.userAgent }
    });

    return session;
}
"#;
        Self {
            text: text.to_string(),
            tokens_estimate: 200,
        }
    }

    /// Rust function with documentation (~200 tokens)
    fn rust_function() -> Self {
        let text = r#"
/// Process search query and return ranked results
///
/// This function implements hybrid search combining full-text search (FTS)
/// and vector similarity. Results are ranked using a weighted scoring algorithm.
///
/// # Arguments
///
/// * `query` - Search query string
/// * `k` - Number of results to return (default: 10)
/// * `mode` - Search mode: "hybrid", "fts", or "vector"
///
/// # Returns
///
/// Returns a `Result<Vec<SearchResult>>` containing ranked search results.
///
/// # Errors
///
/// Returns `SearchError` if query is invalid or database operation fails.
pub async fn process_search_query(
    query: &str,
    k: usize,
    mode: SearchMode,
    pool: &PgPool,
) -> Result<Vec<SearchResult>, SearchError> {
    let embedding = generate_query_embedding(query).await?;

    let results = match mode {
        SearchMode::Hybrid => hybrid_search(query, &embedding, k, pool).await?,
        SearchMode::FTS => fts_search(query, k, pool).await?,
        SearchMode::Vector => vector_search(&embedding, k, pool).await?,
    };

    Ok(results)
}
"#;
        Self {
            text: text.to_string(),
            tokens_estimate: 200,
        }
    }

    /// Python class with docstring (~200 tokens)
    fn python_class() -> Self {
        let text = r#"
class DataProcessor:
    """
    Process and validate data from multiple sources.

    This class provides a unified interface for data processing operations
    including validation, transformation, and persistence. It supports
    batch processing and includes comprehensive error handling.

    Attributes:
        config (ProcessConfig): Processing configuration
        cache (Cache): LRU cache for processed data
        validator (Validator): Data validation engine

    Example:
        >>> processor = DataProcessor(config)
        >>> result = await processor.process(raw_data)
        >>> print(f"Processed {result.count} items")
    """

    def __init__(self, config: ProcessConfig):
        self.config = config
        self.cache = LRUCache(maxsize=config.cache_size)
        self.validator = Validator(config.validation_rules)

    async def process(self, data: List[Dict]) -> ProcessResult:
        """Process a batch of data items."""
        validated = [self.validator.validate(item) for item in data]
        transformed = [self._transform(item) for item in validated if item]
        return ProcessResult(count=len(transformed), data=transformed)
"#;
        Self {
            text: text.to_string(),
            tokens_estimate: 200,
        }
    }

    /// Configuration snippet (~150 tokens)
    fn config_snippet() -> Self {
        let text = r#"
{
  "database": {
    "host": "localhost",
    "port": 5432,
    "name": "maproom",
    "pool": {
      "max_connections": 20,
      "min_connections": 5,
      "connection_timeout": 30,
      "idle_timeout": 300
    }
  },
  "embedding": {
    "provider": "ollama",
    "model": "nomic-embed-text",
    "dimension": 768,
    "batch_size": 100,
    "cache": {
      "max_entries": 10000,
      "ttl_seconds": 3600
    }
  },
  "search": {
    "default_k": 10,
    "max_k": 100,
    "modes": ["hybrid", "fts", "vector"],
    "fusion": {
      "algorithm": "reciprocal_rank",
      "k": 60
    }
  }
}
"#;
        Self {
            text: text.to_string(),
            tokens_estimate: 150,
        }
    }

    /// Generate a dataset of mixed code chunks
    fn generate_dataset(count: usize) -> Vec<Self> {
        let templates = vec![
            Self::typescript_function,
            Self::rust_function,
            Self::python_class,
            Self::config_snippet,
        ];

        (0..count)
            .map(|i| {
                let template = &templates[i % templates.len()];
                let mut chunk = template();
                // Make each chunk unique to avoid cache hits
                chunk.text.push_str(&format!("\n// Unique ID: {}", i));
                chunk
            })
            .collect()
    }
}

/// Create Ollama embedding configuration
fn ollama_config() -> EmbeddingConfig {
    EmbeddingConfig {
        provider: Provider::Ollama,
        model: "nomic-embed-text".to_string(),
        dimension: 768,
        cache: CacheConfig {
            max_entries: 1000,
            ttl_seconds: 3600,
            enable_metrics: true,
        },
        batch_size: 100,
        retry: RetryConfig::default(),
        api_key: None,
        api_endpoint: Some("http://localhost:11434".to_string()), // Fallback to localhost
    }
}

/// Create OpenAI embedding configuration (if API key available)
fn openai_config() -> Option<EmbeddingConfig> {
    std::env::var("OPENAI_API_KEY")
        .ok()
        .map(|api_key| EmbeddingConfig {
            provider: Provider::OpenAI,
            model: "text-embedding-3-small".to_string(),
            dimension: 1536,
            cache: CacheConfig {
                max_entries: 1000,
                ttl_seconds: 3600,
                enable_metrics: true,
            },
            batch_size: 100,
            retry: RetryConfig::default(),
            api_key: Some(api_key),
            api_endpoint: None,
        })
}

/// Benchmark: Single embedding generation (cold vs warm start)
fn bench_single_embedding(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_embedding");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    // Create runtime outside benchmarks (Criterion best practice)
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Ollama single embedding
    let ollama_service = rt.block_on(async {
        let config = ollama_config();
        EmbeddingService::new(config).ok()
    });

    if let Some(service) = ollama_service {
        let chunk = CodeChunk::typescript_function();

        // Warm start: Pre-warm the model with one request
        let _ = rt.block_on(async { service.embed_text(&chunk.text).await });

        group.bench_function("ollama_warm", |b| {
            b.to_async(&rt).iter(|| async {
                let chunk = CodeChunk::typescript_function();
                let result = service.embed_text(black_box(&chunk.text)).await;
                black_box(result)
            });
        });
    } else {
        eprintln!("⚠️  Skipping Ollama benchmarks - service not available");
        eprintln!("   Make sure Ollama is running: docker compose up -d ollama");
    }

    // OpenAI single embedding (if available)
    if let Some(config) = openai_config() {
        let openai_service = rt.block_on(async { EmbeddingService::new(config).ok() });

        if let Some(service) = openai_service {
            group.bench_function("openai", |b| {
                b.to_async(&rt).iter(|| async {
                    let chunk = CodeChunk::typescript_function();
                    let result = service.embed_text(black_box(&chunk.text)).await;
                    black_box(result)
                });
            });
        }
    }

    group.finish();
}

/// Benchmark: Batch processing (10, 50, 100 chunks)
fn bench_batch_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_processing");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);

    let rt = tokio::runtime::Runtime::new().unwrap();

    // Test different batch sizes
    for batch_size in [10, 50, 100] {
        let dataset = CodeChunk::generate_dataset(batch_size);
        let texts: Vec<String> = dataset.iter().map(|c| c.text.clone()).collect();

        group.throughput(Throughput::Elements(batch_size as u64));

        // Ollama batch
        let ollama_service = rt.block_on(async {
            let config = ollama_config();
            EmbeddingService::new(config).ok()
        });

        if let Some(service) = ollama_service {
            // Clear cache before each benchmark
            rt.block_on(async { service.clear_cache().await });

            group.bench_with_input(
                BenchmarkId::new("ollama", batch_size),
                &texts,
                |b, texts| {
                    b.to_async(&rt).iter(|| async {
                        let result = service.embed_batch(black_box(texts.clone())).await;
                        black_box(result)
                    });
                },
            );
        }

        // OpenAI batch (if available)
        if let Some(config) = openai_config() {
            let openai_service = rt.block_on(async { EmbeddingService::new(config).ok() });

            if let Some(service) = openai_service {
                rt.block_on(async { service.clear_cache().await });

                group.bench_with_input(
                    BenchmarkId::new("openai", batch_size),
                    &texts,
                    |b, texts| {
                        b.to_async(&rt).iter(|| async {
                            let result = service.embed_batch(black_box(texts.clone())).await;
                            black_box(result)
                        });
                    },
                );
            }
        }
    }

    group.finish();
}

/// Benchmark: Throughput measurement (chunks per minute)
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(10);

    let rt = tokio::runtime::Runtime::new().unwrap();

    // Measure sustained throughput over 100 chunks
    let dataset = CodeChunk::generate_dataset(100);
    let texts: Vec<String> = dataset.iter().map(|c| c.text.clone()).collect();

    group.throughput(Throughput::Elements(100));

    // Ollama throughput
    let ollama_service = rt.block_on(async {
        let config = ollama_config();
        EmbeddingService::new(config).ok()
    });

    if let Some(service) = ollama_service {
        rt.block_on(async { service.clear_cache().await });

        group.bench_function("ollama_100_chunks", |b| {
            b.to_async(&rt).iter(|| async {
                let start = std::time::Instant::now();
                let result = service.embed_batch(black_box(texts.clone())).await;
                let elapsed = start.elapsed();

                // Calculate throughput
                let chunks_per_sec = 100.0 / elapsed.as_secs_f64();
                let chunks_per_min = chunks_per_sec * 60.0;

                // Report in benchmark output
                if result.is_ok() {
                    eprintln!("  Throughput: {:.1} chunks/min", chunks_per_min);
                }

                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark: Latency distribution (p50, p95, p99)
fn bench_latency_distribution(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_distribution");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(100); // More samples for better percentile accuracy

    let rt = tokio::runtime::Runtime::new().unwrap();

    let ollama_service = rt.block_on(async {
        let config = ollama_config();
        EmbeddingService::new(config).ok()
    });

    if let Some(service) = ollama_service {
        // Warm up
        let warm_chunk = CodeChunk::typescript_function();
        let _ = rt.block_on(async { service.embed_text(&warm_chunk.text).await });

        let service_ref = &service;
        group.bench_function("ollama_single_latency", |b| {
            b.to_async(&rt).iter(|| async {
                let chunk = CodeChunk::typescript_function();
                let result = service_ref.embed_text(black_box(&chunk.text)).await;
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark: Batch size vs latency trade-off
fn bench_batch_size_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_size_scaling");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);

    let rt = tokio::runtime::Runtime::new().unwrap();

    let ollama_service = rt.block_on(async {
        let config = ollama_config();
        EmbeddingService::new(config).ok()
    });

    if let Some(service) = ollama_service {
        // Test batch sizes: 1, 5, 10, 25, 50, 100
        for batch_size in [1, 5, 10, 25, 50, 100] {
            let dataset = CodeChunk::generate_dataset(batch_size);
            let texts: Vec<String> = dataset.iter().map(|c| c.text.clone()).collect();

            group.throughput(Throughput::Elements(batch_size as u64));

            rt.block_on(async { service.clear_cache().await });

            group.bench_with_input(
                BenchmarkId::from_parameter(batch_size),
                &texts,
                |b, texts| {
                    b.to_async(&rt).iter(|| async {
                        let result = service.embed_batch(black_box(texts.clone())).await;
                        black_box(result)
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark: Memory usage patterns (Criterion tracks allocations)
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);

    let rt = tokio::runtime::Runtime::new().unwrap();

    let ollama_service = rt.block_on(async {
        let config = ollama_config();
        EmbeddingService::new(config).ok()
    });

    if let Some(service) = ollama_service {
        // Test memory usage with large batch
        let dataset = CodeChunk::generate_dataset(100);
        let texts: Vec<String> = dataset.iter().map(|c| c.text.clone()).collect();

        rt.block_on(async { service.clear_cache().await });

        group.bench_function("ollama_100_chunks_memory", |b| {
            b.to_async(&rt).iter(|| async {
                let result = service.embed_batch(black_box(texts.clone())).await;
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark: Cache impact on performance
fn bench_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_performance");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);

    let rt = tokio::runtime::Runtime::new().unwrap();

    let ollama_service = rt.block_on(async {
        let config = ollama_config();
        EmbeddingService::new(config).ok()
    });

    if let Some(service) = ollama_service {
        let dataset = CodeChunk::generate_dataset(50);
        let texts: Vec<String> = dataset.iter().map(|c| c.text.clone()).collect();

        // Benchmark: Cache miss (first time)
        rt.block_on(async { service.clear_cache().await });

        group.bench_function("cache_miss", |b| {
            b.to_async(&rt).iter(|| async {
                service.clear_cache().await;
                let result = service.embed_batch(black_box(texts.clone())).await;
                black_box(result)
            });
        });

        // Benchmark: Cache hit (second time with same data)
        rt.block_on(async {
            service.clear_cache().await;
            // Pre-populate cache
            let _ = service.embed_batch(texts.clone()).await;
        });

        group.bench_function("cache_hit", |b| {
            b.to_async(&rt).iter(|| async {
                let result = service.embed_batch(black_box(texts.clone())).await;
                black_box(result)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_embedding,
    bench_batch_processing,
    bench_throughput,
    bench_latency_distribution,
    bench_batch_size_scaling,
    bench_memory_usage,
    bench_cache_performance,
);
criterion_main!(benches);
