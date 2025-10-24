# Ticket: HYBRID_SEARCH-4002: Index Tuning

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- vector-database-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Optimize database indices for hybrid search performance by benchmarking ivfflat parameters, creating partial indices for common filters, optimizing GIN index configuration, and analyzing query performance with pg_stat_statements. Target p95 latency under 50ms for search queries.

## Background
The hybrid search system uses multiple index types (ivfflat for vector similarity, GIN for full-text search, B-tree for filters) that require tuning for optimal performance. Current default ivfflat settings (lists=200, probes=10) may not be optimal for our data distribution. Partial indices can significantly improve performance for commonly filtered queries (recent chunks, high-churn files). GIN index settings affect both update and query performance. This ticket focuses on empirical benchmarking and optimization of these index configurations to meet our p95 latency target of <50ms.

This work is part of Phase 4 (Performance & Scale) of the hybrid search implementation roadmap, following query optimization work to ensure we have proper baseline measurements.

## Acceptance Criteria
- [ ] ivfflat parameters benchmarked across multiple configurations (lists: 100, 200, 400; probes: 5, 10, 20)
- [ ] Benchmark results documented with recall@10, latency p50/p95/p99, and index build time metrics
- [ ] Optimal ivfflat configuration identified and applied based on benchmark data
- [ ] Partial indices created: idx_chunks_recent (recency_score > 0.5) and idx_chunks_high_churn (churn_score > 10)
- [ ] Composite index created: idx_files_repo_worktree for common filter combinations
- [ ] GIN index optimized with fastupdate=off for better query performance
- [ ] pg_stat_statements analysis completed and documented (top queries, index usage, sequential scans)
- [ ] ANALYZE run on all affected tables after index changes
- [ ] p95 latency verified at <50ms for representative search queries
- [ ] Index size and maintenance overhead documented

## Technical Requirements
- PostgreSQL with pgvector extension for ivfflat index support
- pg_stat_statements extension enabled for query analysis
- Benchmark harness to test different ivfflat configurations systematically
- Representative dataset of embeddings (minimum 10,000 chunks) for realistic benchmarking
- Test queries covering common search patterns (vector-only, filtered by recency, filtered by churn)
- Migration script for index modifications that can be applied safely to production
- Rollback plan for index changes if performance degrades
- Documentation of before/after performance metrics

## Implementation Notes

### ivfflat Benchmarking Strategy
1. Test matrix: lists=[100, 200, 400] × probes=[5, 10, 20] = 9 configurations
2. Metrics to collect for each configuration:
   - Recall@10 (measure search quality)
   - Latency: p50, p95, p99 (measure performance distribution)
   - Index build time and size
   - Query planner cost estimates
3. Use consistent test dataset and query set across all runs
4. Consider current architecture default (lists=200, probes=10) as baseline
5. Select configuration optimizing recall/latency tradeoff

### Partial Index Design
Reference architecture (lines 383-402):
```sql
-- Partial indices for performance
CREATE INDEX idx_chunks_recent
  ON maproom.chunks (recency_score)
  WHERE recency_score > 0.5;

CREATE INDEX idx_chunks_high_churn
  ON maproom.chunks (churn_score)
  WHERE churn_score > 10;

-- Composite indices for common filters
CREATE INDEX idx_files_repo_worktree
  ON maproom.files (repo_id, worktree_id);
```

These indices target common query patterns:
- Recent chunks (recency_score > 0.5): Frequently used in result re-ranking
- High-churn files (churn_score > 10): Indicator of important/active code
- Repo+worktree filtering: Common isolation boundary for searches

### GIN Index Optimization
```sql
-- Optimize GIN index for full-text search
ALTER INDEX idx_chunks_ts_doc SET (fastupdate = off);
```
Rationale: Disabling fastupdate trades slower inserts for faster queries, appropriate for read-heavy workload.

### pg_stat_statements Analysis
Focus areas:
1. Identify most frequent queries (top 20 by call count)
2. Identify slowest queries (top 20 by mean/max execution time)
3. Verify index usage (seq_scan vs index_scan counts)
4. Look for opportunities for missing indices
5. Check for queries that would benefit from query plan hints

### Performance Targets
- p95 latency: <50ms for hybrid search queries
- Recall@10: >0.95 (maintain high search quality)
- Index build time: <5 minutes for 100k chunks
- Index size overhead: <30% of raw data size

## Dependencies
- HYBRID_SEARCH-4001 (query optimization) - Need baseline performance measurements before index tuning
- PostgreSQL database with pgvector extension installed
- pg_stat_statements extension enabled in database configuration

## Risk Assessment
- **Risk**: ivfflat parameter changes could degrade recall significantly
  - **Mitigation**: Benchmark recall@10 for all configurations; reject any configuration with recall <0.95; maintain conservative defaults if no clear winner emerges

- **Risk**: Partial indices may not be used by query planner despite WHERE conditions
  - **Mitigation**: Use EXPLAIN ANALYZE to verify index usage; adjust partial index predicates to match actual query patterns; collect pg_stat_user_indexes data

- **Risk**: GIN fastupdate=off may slow down indexing pipeline unacceptably
  - **Mitigation**: Benchmark insert performance before/after; consider batch insert optimization; can re-enable fastupdate if inserts become bottleneck

- **Risk**: Index bloat over time could degrade performance
  - **Mitigation**: Document maintenance procedures (REINDEX schedules); monitor index bloat metrics; consider pg_repack for zero-downtime reindexing

- **Risk**: Production database performance impact during index creation
  - **Mitigation**: Use CONCURRENTLY option for index creation; schedule during low-traffic periods; test migration on staging replica first

## Files/Packages Affected
- `crates/maproom/migrations/XXX_optimize_indices.sql` - Migration for index optimizations (CREATE/ALTER INDEX statements)
- `crates/maproom/benches/index_tuning_benchmark.rs` - Comprehensive benchmark suite for ivfflat parameters
- `crates/maproom/docs/index_tuning_results.md` - Benchmark results, analysis, and recommendations
- `crates/maproom/src/db/config.rs` - Update default ivfflat configuration based on benchmark results
- `crates/maproom/src/db/schema.rs` - Schema documentation updates for new indices
