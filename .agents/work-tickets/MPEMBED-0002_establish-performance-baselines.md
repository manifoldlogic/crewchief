# Ticket: MPEMBED-0002: Establish performance baselines for search and embedding generation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- performance-engineer
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Measure current system performance (search latency, index sizes, embedding throughput) to detect regressions after multi-provider changes.

## Background
The system currently uses OpenAI text-embedding-3-small (1536-dim) for all embeddings. The upcoming multi-provider migration will introduce a COALESCE pattern to fall back across providers, add new 768-dim vector columns, and change query patterns.

Establishing performance baselines is critical for validating acceptance criteria in later phases (e.g., "<5% search latency regression"). Without baselines, we cannot objectively assess whether the migration maintains system performance.

**Reference**: `crewchief_context/maproom/MPEMBED-multi-provider-embeddings/` - Phase 0, Day 0

## Acceptance Criteria
- [x] Search latency measured: p50, p95, p99 for 10-result queries (target: ~50ms p95)
- [x] Index sizes documented: 1536-dim IVFFlat indexes (expected: ~150MB for 23K chunks)
- [x] OpenAI embedding throughput measured: chunks/second (expected: ~50-200 chunks/s)
- [x] Baseline metrics saved to `benchmarks/mpembed_baseline.md`
- [x] Benchmarking script is repeatable (can re-run after changes)

## Technical Requirements
- Measure search across 100 diverse queries (from real usage logs if available, otherwise synthetic)
- Use `EXPLAIN ANALYZE` for query profiling
- Measure index sizes: `SELECT pg_size_pretty(pg_relation_size('idx_chunks_code_vec'));`
- Time embedding generation for 1,000-chunk batch
- Run benchmarks on production-like hardware (same PostgreSQL version, RAM)
- Document hardware specs (CPU, RAM, PostgreSQL version, pgvector version)
- Take median of 10 runs for stability

## Implementation Notes

**Search Latency Benchmarking**:
```rust
// Benchmark search latency
let queries = vec!["authentication", "error handling", "database query", "message handling"];
let mut latencies = Vec::new();

for query in queries {
    for _ in 0..10 {  // 10 runs per query
        let start = Instant::now();
        let results = hybrid_search(&pool, query, 10).await?;
        let elapsed = start.elapsed();
        latencies.push(elapsed);
    }
}

// Calculate percentiles
latencies.sort();
let p50 = latencies[latencies.len() / 2];
let p95 = latencies[latencies.len() * 95 / 100];
let p99 = latencies[latencies.len() * 99 / 100];
```

**Index Size Measurement**:
```sql
-- Measure all vector indexes
SELECT
    indexrelid::regclass AS index_name,
    pg_size_pretty(pg_relation_size(indexrelid)) AS index_size,
    idx_scan AS times_used
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
  AND indexrelid::regclass::text LIKE '%_vec%';
```

**Embedding Throughput**:
```rust
// Measure OpenAI embedding generation speed
let start = Instant::now();
let batch = select_random_chunks(&pool, 1000).await?;
for chunk in batch {
    let embedding = openai_client.embed(&chunk.text).await?;
}
let elapsed = start.elapsed();
let throughput = 1000.0 / elapsed.as_secs_f64();
println!("OpenAI throughput: {:.1} chunks/sec", throughput);
```

**Output Format** (`benchmarks/mpembed_baseline.md`):
- Hardware specs (CPU, RAM, PostgreSQL version)
- Search latency: p50/p95/p99 with query examples
- Index sizes: all vector indexes with usage stats
- Embedding throughput: OpenAI chunks/sec
- Query plans: EXPLAIN ANALYZE output for sample queries

## Dependencies
- MPEMBED-0001 (use fixture for repeatable measurements)

## Risk Assessment
- **Risk**: Production load varies, baselines might not be stable
  - **Mitigation**: Run benchmarks during off-peak hours, take median of 10 runs per query

- **Risk**: Synthetic queries might not represent real usage patterns
  - **Mitigation**: Use real queries from MCP logs if available; document query selection strategy

- **Risk**: Hardware differences between dev and production
  - **Mitigation**: Document hardware specs; run baselines on same environment where post-migration benchmarks will run

## Files/Packages Affected
- benchmarks/mpembed_baseline.md (create)
- crates/maproom/benches/search_latency.rs (create)
- crates/maproom/scripts/measure_baselines.sh (create)
