# Ticket: MPEMBED-6003: Performance benchmarks for multi-provider

## Status
- [x] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- performance-engineer
- rust-test-runner
- verify-ticket
- commit-ticket

## Summary
Create performance benchmarks measuring search latency with COALESCE queries, embedding throughput for all providers, index sizes, and compare against baseline from MPEMBED-0002. Report any regressions > 5%.

## Background
This ticket implements performance validation for Phase 6 (Testing and Validation) to ensure multi-provider support doesn't degrade system performance. Benchmarks measure impact of COALESCE queries, dimension selection logic, and provider overhead.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-6-testing-validation.md

## Acceptance Criteria
- [x] Baseline comparison with MPEMBED-0002 results
- [x] Search latency benchmarks (hybrid, vector-only, FTS-only)
- [x] COALESCE query performance measured
- [x] Embedding throughput measured for all providers
- [x] Index size comparisons (768 vs 1536 dimensions)
- [x] Performance regression < 5% vs baseline
- [x] Benchmark results documented in markdown
- [x] CI integration for regression detection

## Technical Requirements
- Use criterion for Rust benchmarks
- Measure p50, p95, p99 latencies
- Test with realistic dataset sizes (1K, 10K, 100K chunks)
- Compare mixed embeddings vs single provider
- Measure database query explain plans
- Document hardware specs for reproducibility
- Create benchmark fixtures for consistent testing

## Implementation Notes
```rust
// crates/maproom/benches/multi_provider_performance.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use maproom::embedding::factory::create_provider;
use maproom::search::hybrid::*;
use sqlx::PgPool;

async fn setup_benchmark_fixture(pool: &PgPool, size: usize, provider: &str) -> Vec<Uuid> {
    // Create chunks with embeddings
    let provider = create_provider(provider).unwrap();
    let pipeline = EmbeddingPipeline::new(provider, pool.clone());

    // Generate test chunks
    let chunk_ids = generate_test_chunks(pool, size).await;
    pipeline.process_chunks(chunk_ids.clone()).await.unwrap();

    chunk_ids
}

fn benchmark_search_latency(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let pool = runtime.block_on(create_test_pool());

    let mut group = c.benchmark_group("search_latency");

    for size in [1_000, 10_000, 100_000] {
        // Benchmark: Search with single provider (768-dim)
        group.bench_with_input(
            BenchmarkId::new("ollama_only", size),
            &size,
            |b, &size| {
                let chunks = runtime.block_on(setup_benchmark_fixture(&pool, size, "ollama"));
                let provider = create_provider("ollama").unwrap();
                let query_emb = runtime.block_on(
                    provider.embed(vec!["test query".to_string()])
                ).unwrap();

                b.to_async(&runtime).iter(|| async {
                    hybrid_search(
                        &pool,
                        "test query",
                        black_box(query_emb[0].clone()),
                        "bench",
                        "main",
                        10,
                        0.5,
                        0.5,
                    ).await.unwrap()
                });
            },
        );

        // Benchmark: Search with mixed embeddings (768 + 1536)
        group.bench_with_input(
            BenchmarkId::new("mixed_embeddings", size),
            &size,
            |b, &size| {
                // Setup: 50% Ollama, 50% OpenAI
                let half = size / 2;
                runtime.block_on(async {
                    setup_benchmark_fixture(&pool, half, "ollama").await;
                    setup_benchmark_fixture(&pool, half, "openai").await;
                });

                let provider = create_provider("ollama").unwrap();
                let query_emb = runtime.block_on(
                    provider.embed(vec!["test query".to_string()])
                ).unwrap();

                b.to_async(&runtime).iter(|| async {
                    hybrid_search(
                        &pool,
                        "test query",
                        black_box(query_emb[0].clone()),
                        "bench",
                        "main",
                        10,
                        0.5,
                        0.5,
                    ).await.unwrap()
                });
            },
        );
    }

    group.finish();
}

fn benchmark_embedding_throughput(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("embedding_throughput");

    let texts: Vec<String> = (0..100)
        .map(|i| format!("function test{i}() {{ return {i}; }}"))
        .collect();

    // Benchmark Ollama throughput
    if std::env::var("TEST_OLLAMA").is_ok() {
        group.bench_function("ollama_100_texts", |b| {
            let provider = create_provider("ollama").unwrap();
            b.to_async(&runtime).iter(|| async {
                provider.embed(black_box(texts.clone())).await.unwrap()
            });
        });
    }

    // Benchmark OpenAI throughput
    if std::env::var("OPENAI_API_KEY").is_ok() {
        group.bench_function("openai_100_texts", |b| {
            let provider = create_provider("openai").unwrap();
            b.to_async(&runtime).iter(|| async {
                provider.embed(black_box(texts.clone())).await.unwrap()
            });
        });
    }

    // Benchmark Google throughput
    if std::env::var("GOOGLE_PROJECT_ID").is_ok() {
        group.bench_function("google_100_texts", |b| {
            let provider = create_provider("google").unwrap();
            b.to_async(&runtime).iter(|| async {
                provider.embed(black_box(texts.clone())).await.unwrap()
            });
        });
    }

    group.finish();
}

fn benchmark_coalesce_overhead(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let pool = runtime.block_on(create_test_pool());

    let mut group = c.benchmark_group("coalesce_overhead");

    // Setup: Create 10K chunks with both embeddings
    runtime.block_on(async {
        setup_benchmark_fixture(&pool, 10_000, "ollama").await;
        setup_benchmark_fixture(&pool, 10_000, "openai").await;
    });

    let provider = create_provider("ollama").unwrap();
    let query_emb = runtime.block_on(
        provider.embed(vec!["test".to_string()])
    ).unwrap();

    // Benchmark: COALESCE query (mixed embeddings)
    group.bench_function("with_coalesce", |b| {
        b.to_async(&runtime).iter(|| async {
            sqlx::query!(
                r#"
                SELECT id,
                    1 - (COALESCE(code_embedding_ollama, code_embedding) <=> $1::vector(768)) AS score
                FROM chunks
                WHERE repo = 'bench'
                ORDER BY score DESC
                LIMIT 10
                "#,
                &query_emb[0]
            )
            .fetch_all(&pool)
            .await
            .unwrap()
        });
    });

    // Benchmark: Direct query (single embedding column)
    group.bench_function("without_coalesce", |b| {
        b.to_async(&runtime).iter(|| async {
            sqlx::query!(
                r#"
                SELECT id,
                    1 - (code_embedding_ollama <=> $1::vector(768)) AS score
                FROM chunks
                WHERE repo = 'bench'
                ORDER BY score DESC
                LIMIT 10
                "#,
                &query_emb[0]
            )
            .fetch_all(&pool)
            .await
            .unwrap()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_search_latency,
    benchmark_embedding_throughput,
    benchmark_coalesce_overhead
);
criterion_main!(benches);
```

**Benchmark Report Template:**
```markdown
# Multi-Provider Performance Benchmarks

**Date:** 2025-01-XX
**Hardware:** AMD Ryzen 9 5950X, 64GB RAM, NVIDIA RTX 3090
**Database:** PostgreSQL 16 with pgvector 0.5.1

## Baseline Comparison (vs MPEMBED-0002)

| Metric | Baseline (OpenAI only) | Multi-Provider | Change |
|--------|------------------------|----------------|--------|
| Search latency (10K chunks) | 23ms | 24ms | +4.3% ✓ |
| Search latency (100K chunks) | 89ms | 92ms | +3.4% ✓ |
| Index size (1K chunks) | 12MB | 12MB | 0% ✓ |
| Embedding throughput (OpenAI) | 45 chunks/s | 45 chunks/s | 0% ✓ |

**Status:** All metrics within 5% regression threshold ✓

## Search Latency (p50/p95/p99)

### Ollama Only (768-dim)
- 1K chunks: 8ms / 12ms / 18ms
- 10K chunks: 22ms / 34ms / 47ms
- 100K chunks: 87ms / 120ms / 156ms

### Mixed Embeddings (50% Ollama, 50% OpenAI)
- 1K chunks: 9ms / 13ms / 19ms (+1ms overhead)
- 10K chunks: 24ms / 36ms / 51ms (+2ms overhead)
- 100K chunks: 92ms / 125ms / 162ms (+5ms overhead)

**Analysis:** COALESCE adds ~1-5ms overhead, acceptable for mixed embeddings use case.

## Embedding Throughput

| Provider | Chunks/sec | Tokens/sec | Latency (100 chunks) |
|----------|-----------|------------|---------------------|
| Ollama (GPU) | 87 | 26,100 | 1.15s |
| OpenAI | 45 | 13,500 | 2.22s |
| Google | 38 | 11,400 | 2.63s |

**Analysis:** Ollama nearly 2x faster than cloud providers (local GPU advantage).

## COALESCE Overhead

| Query Type | Latency | Overhead |
|------------|---------|----------|
| Direct column (no COALESCE) | 22ms | - |
| COALESCE (mixed embeddings) | 24ms | +2ms (9%) |

**Analysis:** COALESCE overhead is minimal, well within acceptable range.

## Index Sizes

| Embedding Type | Size per 1K chunks | Size per 100K chunks |
|---------------|-------------------|---------------------|
| 768-dim only | 6MB | 600MB |
| 1536-dim only | 12MB | 1.2GB |
| Both (mixed) | 18MB | 1.8GB |

**Analysis:** Mixed embeddings increase storage by 50%, acceptable trade-off for flexibility.

## Regression Analysis

✓ **No significant regressions detected**

All metrics within 5% of baseline:
- Search latency: +3-4%
- Embedding throughput: 0%
- Index size: 0%

## Recommendations

1. **Use Ollama for bulk indexing** - 2x faster than cloud providers
2. **Mixed embeddings acceptable** - Only 2ms search overhead
3. **Consider 768-dim for new projects** - 50% storage savings vs 1536-dim
```

## Dependencies
- MPEMBED-0002 (Baseline benchmarks must exist)
- MPEMBED-6002 (E2E tests to validate correctness before benchmarking)

## Risk Assessment
- **Risk**: Benchmarks may be environment-specific
  - **Mitigation**: Document hardware specs, run on multiple machines
- **Risk**: Network variability for cloud providers
  - **Mitigation**: Use average of 10 runs, exclude outliers

## Files/Packages Affected
- crates/maproom/benches/multi_provider_performance.rs (create)
- benchmarks/multi_provider_performance.md (create - results)
- Cargo.toml (add criterion dev-dependency)
