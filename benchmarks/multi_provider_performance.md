# Multi-Provider Performance Benchmarks

**Date:** [To be filled after running benchmarks]
**Hardware:** [CPU model], [RAM], [GPU if applicable]
**Database:** PostgreSQL [version] with pgvector [version]
**Ticket:** MPEMBED-6003

---

## Executive Summary

| Metric | Baseline (MPEMBED-0002) | Multi-Provider | Change | Status |
|--------|-------------------------|----------------|--------|--------|
| Search p50 (10K chunks) | 30.97ms | [TBD]ms | [TBD]% | [✓/⚠️] |
| Search p95 (10K chunks) | 33.62ms | [TBD]ms | [TBD]% | [✓/⚠️] |
| Search p99 (10K chunks) | 34.91ms | [TBD]ms | [TBD]% | [✓/⚠️] |
| Index size (23K chunks) | 6.4MB | [TBD]MB | [TBD]% | [✓/⚠️] |
| Embedding throughput (Ollama) | N/A | [TBD] chunks/s | N/A | [✓/⚠️] |

**Overall Status:** [✓ All metrics within 5% regression threshold / ⚠️ Regressions detected / ❌ Significant regressions]

---

## Hardware & Software Configuration

### Hardware
- **CPU:** [CPU model and cores]
- **RAM:** [Total RAM]
- **GPU:** [GPU model if applicable, especially for Ollama]
- **Disk:** [Storage type and speed]

### Software
- **Operating System:** [OS and version]
- **PostgreSQL:** [Version]
- **pgvector:** [Version]
- **Rust:** [Version]
- **Cargo:** [Version]

### Database Configuration
- **Total Chunks:** [Number of chunks in test database]
- **Chunks with 768-dim embeddings:** [Count]
- **Chunks with 1536-dim embeddings:** [Count]
- **Mixed embedding chunks:** [Count]
- **Table Size:** [Size in MB]
- **Index Configuration:** [IVFFlat parameters]

---

## Baseline Comparison (vs MPEMBED-0002)

### Search Latency

Reference baseline from `/workspace/benchmarks/mpembed_baseline.md`:
- **p50:** 30.97ms
- **p95:** 33.62ms (target: <50ms)
- **p99:** 34.91ms

| Metric | Baseline | Single Provider (Ollama) | Mixed Providers (50/50) | Regression |
|--------|----------|-------------------------|-------------------------|------------|
| p50 | 30.97ms | [TBD]ms | [TBD]ms | [TBD]% |
| p95 | 33.62ms | [TBD]ms | [TBD]ms | [TBD]% |
| p99 | 34.91ms | [TBD]ms | [TBD]ms | [TBD]% |
| Mean | 30.60ms | [TBD]ms | [TBD]ms | [TBD]% |

**Analysis:**
- [Add interpretation of results]
- [Note any significant deviations]
- [Explain if regression threshold is met or exceeded]

### Index Sizes

| Configuration | Index Size (23K chunks) | Size per Chunk | Notes |
|---------------|------------------------|----------------|-------|
| 1536-dim only (baseline) | 6.4MB | ~280 bytes | Two indexes (code_vec + text_vec) |
| 768-dim only | [TBD]MB | [TBD] bytes | Two indexes (code_embedding_ollama + text_embedding_ollama) |
| Mixed (both dimensions) | [TBD]MB | [TBD] bytes | Four indexes total |

**Analysis:**
- [Interpret storage impact]
- [Compare with expected values]
- [Note any optimization opportunities]

---

## Search Latency Benchmarks

### 1. Single Provider (100% Ollama, 768-dim)

**Test Configuration:**
- Dataset: [Size] chunks, all with Ollama embeddings
- Query corpus: 10 diverse queries × 10 runs each
- Search modes: FTS-only, Vector-only, Hybrid

| Search Mode | p50 (ms) | p95 (ms) | p99 (ms) | Mean (ms) |
|-------------|----------|----------|----------|-----------|
| FTS-only | [TBD] | [TBD] | [TBD] | [TBD] |
| Vector-only | [TBD] | [TBD] | [TBD] | [TBD] |
| Hybrid | [TBD] | [TBD] | [TBD] | [TBD] |

**Query Breakdown (top 5 by latency):**

| Query | Mode | p50 (ms) | p95 (ms) | Notes |
|-------|------|----------|----------|-------|
| [Query 1] | Hybrid | [TBD] | [TBD] | [Notes] |
| [Query 2] | Hybrid | [TBD] | [TBD] | [Notes] |
| [Query 3] | Vector | [TBD] | [TBD] | [Notes] |
| [Query 4] | FTS | [TBD] | [TBD] | [Notes] |
| [Query 5] | Hybrid | [TBD] | [TBD] | [Notes] |

### 2. Mixed Providers (50% Ollama 768-dim, 50% OpenAI 1536-dim)

**Test Configuration:**
- Dataset: [Size] chunks, mixed embeddings
- COALESCE queries: `COALESCE(code_embedding_ollama, code_embedding)`
- Same query corpus as single provider test

| Search Mode | p50 (ms) | p95 (ms) | p99 (ms) | Overhead vs Single |
|-------------|----------|----------|----------|--------------------|
| FTS-only | [TBD] | [TBD] | [TBD] | [TBD]ms ([TBD]%) |
| Vector-only | [TBD] | [TBD] | [TBD] | [TBD]ms ([TBD]%) |
| Hybrid | [TBD] | [TBD] | [TBD] | [TBD]ms ([TBD]%) |

**Analysis:**
- COALESCE overhead: [TBD]ms average ([TBD]%)
- [Interpret whether overhead is acceptable]
- [Note any query patterns with higher overhead]

### 3. Dataset Size Scalability

| Dataset Size | Hybrid p50 (ms) | Hybrid p95 (ms) | Notes |
|--------------|----------------|-----------------|-------|
| 1K chunks | [TBD] | [TBD] | Small repository |
| 10K chunks | [TBD] | [TBD] | Medium repository |
| 100K chunks | [TBD] | [TBD] | Large repository |

**Scalability Analysis:**
- [Describe scaling behavior: linear, logarithmic, etc.]
- [Note if performance degrades unexpectedly at larger scales]

---

## COALESCE Query Performance

### Direct Comparison

**Test Setup:**
- 10,000 chunks: 50% with ollama embeddings, 50% with original embeddings
- Query: "authentication flow"
- Runs: 100 iterations for statistical significance

| Query Type | Mean (ms) | p50 (ms) | p95 (ms) | Overhead |
|------------|-----------|----------|----------|----------|
| Direct column (no COALESCE) | [TBD] | [TBD] | [TBD] | - |
| COALESCE (mixed embeddings) | [TBD] | [TBD] | [TBD] | +[TBD]ms ([TBD]%) |

**Sample EXPLAIN ANALYZE:**

```sql
-- Without COALESCE (direct column)
SELECT id, 1 - (code_embedding_ollama <=> $1::vector(768)) AS score
FROM maproom.chunks
WHERE repo_id = $2
ORDER BY score DESC
LIMIT 10;

[Insert EXPLAIN ANALYZE output]

-- With COALESCE (mixed embeddings)
SELECT id, 1 - (COALESCE(code_embedding_ollama, code_embedding) <=> $1::vector(768)) AS score
FROM maproom.chunks
WHERE repo_id = $2
ORDER BY score DESC
LIMIT 10;

[Insert EXPLAIN ANALYZE output]
```

**Analysis:**
- [Interpret query plans]
- [Note index usage patterns]
- [Explain COALESCE performance impact]

---

## Embedding Throughput

### Provider Comparison

**Test Configuration:**
- Batch size: 100 chunks (~200 tokens each)
- Runs: 10 iterations per provider
- Cache: Cleared before each run

| Provider | Model | Dimension | Mean (s) | Chunks/sec | Tokens/sec | Relative Speed |
|----------|-------|-----------|----------|------------|------------|----------------|
| Ollama (GPU) | nomic-embed-text | 768 | [TBD] | [TBD] | [TBD] | [TBD]x |
| OpenAI | text-embedding-3-small | 1536 | [TBD] | [TBD] | [TBD] | 1.0x (baseline) |
| Google Vertex AI | text-embedding-004 | 768 | [TBD] | [TBD] | [TBD] | [TBD]x |

**Expected Performance (from documentation):**
- Ollama (GPU): ~87 chunks/sec
- OpenAI: ~45 chunks/sec
- Google: ~38 chunks/sec

**Analysis:**
- [Compare actual vs expected throughput]
- [Note factors affecting performance: network latency, GPU availability, etc.]
- [Recommend optimal provider for different use cases]

### Batch Size Impact

| Batch Size | Ollama (s) | OpenAI (s) | Google (s) |
|------------|-----------|-----------|------------|
| 10 chunks | [TBD] | [TBD] | [TBD] |
| 50 chunks | [TBD] | [TBD] | [TBD] |
| 100 chunks | [TBD] | [TBD] | [TBD] |
| 500 chunks | [TBD] | [TBD] | [TBD] |

**Analysis:**
- [Interpret optimal batch size for each provider]
- [Note any rate limiting or performance degradation]

---

## Dimension Comparison (768-dim vs 1536-dim)

### Search Performance

| Dimension | Mean (ms) | p50 (ms) | p95 (ms) | Index Size (10K chunks) |
|-----------|-----------|----------|----------|-------------------------|
| 768-dim | [TBD] | [TBD] | [TBD] | [TBD]MB |
| 1536-dim | [TBD] | [TBD] | [TBD] | [TBD]MB |
| Difference | [TBD]ms | [TBD]ms | [TBD]ms | [TBD]MB ([TBD]%) |

**Analysis:**
- Expected: 768-dim should be ~10-15% faster due to lower dimensionality
- Actual: [TBD]% faster/slower
- Storage savings: [TBD]% with 768-dim
- [Recommend dimension choice for different use cases]

### Embedding Quality vs Performance Trade-off

| Dimension | Search Accuracy (NDCG@10) | Speed | Storage | Recommendation |
|-----------|--------------------------|-------|---------|----------------|
| 768-dim | [TBD] | Faster | 50% less | [Use case] |
| 1536-dim | [TBD] | Slower | Baseline | [Use case] |

---

## Regression Analysis

### Acceptance Criteria Status

| Criterion | Target | Actual | Status |
|-----------|--------|--------|--------|
| Baseline comparison | Complete | [✓/❌] | [✓/❌] |
| Search latency benchmarks | p50, p95, p99 | [✓/❌] | [✓/❌] |
| COALESCE performance measured | < 5ms overhead | [TBD]ms | [✓/⚠️/❌] |
| Embedding throughput measured | All providers | [✓/❌] | [✓/❌] |
| Index size comparisons | 768 vs 1536 | [✓/❌] | [✓/❌] |
| Performance regression | < 5% vs baseline | [TBD]% | [✓/⚠️/❌] |
| Results documented | This file | ✓ | ✓ |

### Detailed Regression Breakdown

| Metric | Baseline | Current | Change | Threshold | Status |
|--------|----------|---------|--------|-----------|--------|
| Search p50 | 30.97ms | [TBD]ms | +[TBD]% | < +5% | [✓/⚠️/❌] |
| Search p95 | 33.62ms | [TBD]ms | +[TBD]% | < +5% | [✓/⚠️/❌] |
| Search p99 | 34.91ms | [TBD]ms | +[TBD]% | < +5% | [✓/⚠️/❌] |
| Index size | 6.4MB | [TBD]MB | +[TBD]% | N/A | [Info] |

**Overall Verdict:**
- [✓ All regressions within acceptable threshold]
- [⚠️ Some metrics near threshold, monitor closely]
- [❌ Significant regressions detected, optimization needed]

---

## Performance Optimization Recommendations

### Immediate Actions
1. [Recommendation 1 based on results]
2. [Recommendation 2 based on results]
3. [Recommendation 3 based on results]

### Future Optimizations
1. [Long-term optimization 1]
2. [Long-term optimization 2]
3. [Long-term optimization 3]

### Configuration Tuning
```sql
-- Suggested pgvector index parameters
CREATE INDEX idx_chunks_code_vec ON maproom.chunks
USING ivfflat (code_embedding vector_cosine_ops)
WITH (lists = [TBD based on dataset size]);

-- IVFFlat probes setting
SET ivfflat.probes = [TBD based on accuracy/speed trade-off];
```

---

## CI Integration for Regression Detection

### Benchmark Execution

```bash
# Run benchmarks and save baseline
cargo bench --bench multi_provider_performance -- --save-baseline main

# After changes, compare against baseline
cargo bench --bench multi_provider_performance -- --baseline main

# Generate detailed report
cargo bench --bench multi_provider_performance -- --output-format json > benchmarks/results.json
```

### Automated Regression Checks

**Proposed CI workflow:**
1. Run benchmarks on PR
2. Compare against `main` branch baseline
3. Fail CI if p95 latency regression > 5%
4. Post benchmark summary as PR comment

**Example CI configuration:**

```yaml
# .github/workflows/performance-regression.yml
name: Performance Regression Tests

on:
  pull_request:
    paths:
      - 'crates/maproom/src/search/**'
      - 'crates/maproom/src/embedding/**'
      - 'crates/maproom/src/db/**'

jobs:
  benchmark:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: pgvector/pgvector:pg16
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3
      - name: Run benchmarks
        run: cargo bench --bench multi_provider_performance -- --save-baseline pr
      - name: Compare with main
        run: |
          git checkout main
          cargo bench --bench multi_provider_performance -- --save-baseline main
          git checkout -
          cargo bench --bench multi_provider_performance -- --baseline main
      - name: Check regression threshold
        run: |
          # Parse Criterion output and fail if regression > 5%
          python scripts/check_regression.py benchmarks/results.json
```

---

## Reproducibility

### Running These Benchmarks

```bash
# 1. Set up test database
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/maproom_bench"
psql $DATABASE_URL < crates/maproom/scripts/create_test_fixture.sql

# 2. Optional: Configure providers for embedding benchmarks
export TEST_OLLAMA=1  # If Ollama is running
export OPENAI_API_KEY="sk-..."  # If testing OpenAI
export GOOGLE_PROJECT_ID="your-project"  # If testing Google

# 3. Run benchmarks
cargo bench --bench multi_provider_performance

# 4. View results
open target/criterion/report/index.html
```

### Benchmark Consistency Factors

- **Database state:** Use consistent fixture with known data distribution
- **Cache state:** Benchmarks clear caches between runs
- **System load:** Run during off-peak hours, close unnecessary processes
- **Sample size:** 100 iterations for search latency, 10 for throughput
- **Hardware:** Document hardware specs for cross-run comparisons

---

## Appendix: Sample EXPLAIN ANALYZE Outputs

### Hybrid Search (Single Provider)

```sql
[Insert sample EXPLAIN ANALYZE for hybrid search with single provider]
```

### Hybrid Search (Mixed Providers with COALESCE)

```sql
[Insert sample EXPLAIN ANALYZE for hybrid search with COALESCE]
```

### Vector-Only Search (768-dim)

```sql
[Insert sample EXPLAIN ANALYZE for 768-dim vector search]
```

### Vector-Only Search (1536-dim)

```sql
[Insert sample EXPLAIN ANALYZE for 1536-dim vector search]
```

---

## Next Steps

1. **Review Results:** Validate that multi-provider implementation meets performance targets
2. **Regression Investigation:** If regressions > 5%, investigate and optimize
3. **Documentation Updates:** Update MPEMBED project docs with performance characteristics
4. **CI Integration:** Implement automated regression detection in GitHub Actions
5. **Production Monitoring:** Set up alerts for latency regressions in production

**Benchmark Completed:** [Date]
**Ticket:** MPEMBED-6003
**Status:** [PASS/FAIL/NEEDS_OPTIMIZATION]
