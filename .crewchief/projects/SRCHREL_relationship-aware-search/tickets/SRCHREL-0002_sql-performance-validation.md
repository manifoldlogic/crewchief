# Ticket: SRCHREL-0002 - SQL Performance Validation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- performance-engineer
- verify-ticket
- commit-ticket

## Summary

Validate that the quality-weighted SQL query meets performance budget (<30ms p95 latency) on realistic database sizes. This prerequisite ensures the enhanced graph scoring won't degrade search performance.

## Background

The proposed quality-weighted graph scoring uses a more complex SQL query with JOIN operations and CASE statements for quality computation. Before implementing in production code, we must validate that:
1. Query latency is within budget (<30ms p95)
2. Query uses database indexes efficiently (no full table scans)
3. Performance scales sub-linearly with database size

The actual CrewChief database has 458 calls edges (validated in EDGEEXT). This ticket will test with both real data and synthetic larger databases to ensure scalability.

## Acceptance Criteria

- [ ] Implement prototype quality-weighted SQL query in test harness
- [ ] Benchmark on real CrewChief database (458 edges, actual chunk count)
- [ ] Create synthetic database with 100,000 chunks and 500,000 edges
- [ ] Measure p50, p95, p99 latencies (cold and warm cache)
- [ ] Run `EXPLAIN QUERY PLAN` to verify index usage
- [ ] Confirm no full table scans in query plan
- [ ] Test with different database sizes (10K, 100K, 1M chunks)
- [ ] Verify p95 latency <30ms on 100K chunk database
- [ ] Document performance results in architecture.md
- [ ] If latency >30ms, document optimization options (indexes, query simplification, pre-computation)

## Technical Requirements

**Prototype SQL Query (Quality-Weighted):**

```sql
-- Quality-weighted graph importance calculation
WITH quality_edges AS (
  SELECT
    ce.dst_chunk_id as chunk_id,
    -- Edge type weight (Phase 1: calls only)
    CASE ce.type
      WHEN 'calls' THEN 1.0
      ELSE 1.0
    END *
    -- Source code type weight (test detection)
    CASE
      WHEN src_file.relpath LIKE '%/test/%'
        OR src_file.relpath LIKE '%/tests/%'
        OR src_file.relpath LIKE '%/__tests__/%'
        OR src_file.relpath LIKE '%.test.%'
        OR src_file.relpath LIKE '%.spec.%'
        OR src_file.relpath LIKE '%_test.%'
        OR src_chunk.kind LIKE '%test%'
      THEN 0.5  -- Test code penalty
      ELSE 1.0  -- Production code baseline
    END as edge_quality
  FROM chunk_edges ce
  JOIN chunks src_chunk ON src_chunk.id = ce.src_chunk_id
  JOIN files src_file ON src_file.id = src_chunk.file_id
  WHERE ce.dst_chunk_id IN (
    SELECT c.id FROM chunks c
    JOIN files f ON f.id = c.file_id
    WHERE f.repo_id = ?repo_id
      AND (?worktree_id IS NULL OR f.worktree_id = ?worktree_id)
  )
),
importance_scores AS (
  SELECT
    chunk_id,
    SUM(edge_quality) as quality_weighted_sum
  FROM quality_edges
  GROUP BY chunk_id
)
SELECT
  chunk_id,
  LOG(2.0 + COALESCE(quality_weighted_sum, 0.0)) as graph_score
FROM importance_scores
ORDER BY graph_score DESC
LIMIT ?limit;
```

**Benchmark Harness (Rust):**

```rust
// Create in crates/maproom/benches/graph_quality_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use maproom::db::sqlite::SqliteStore;

fn benchmark_quality_weighted_query(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let store = runtime.block_on(setup_test_db(100_000)); // 100K chunks

    c.bench_function("quality_weighted_graph", |b| {
        b.iter(|| {
            runtime.block_on(async {
                // Execute quality-weighted query
                store.calculate_graph_importance_quality(
                    black_box(1), // repo_id
                    black_box(None), // worktree_id
                    black_box(10), // limit
                ).await
            })
        });
    });
}

criterion_group!(benches, benchmark_quality_weighted_query);
criterion_main!(benches);
```

**Synthetic Database Creation:**

Create helper function to populate test database with realistic data:
- Chunk distribution: 70% production, 30% test code
- Edge types: 100% calls (Phase 1 scope)
- File path patterns: Mix of /src/, /test/, /lib/ directories
- Chunk kinds: Realistic tree-sitter node types

**Performance Targets:**

| Metric | Target | Alert | Critical |
|--------|--------|-------|----------|
| p50 latency | <15ms | >20ms | >25ms |
| p95 latency | <30ms | >35ms | >40ms |
| p99 latency | <50ms | >60ms | >80ms |
| Rows scanned | <10K | >50K | >100K |

**EXPLAIN Analysis:**

Run `EXPLAIN QUERY PLAN` and verify:
- Index usage on `chunk_edges(dst_chunk_id)`
- Index usage on `chunks(file_id)`
- Index usage on `files(id)`
- No SCAN TABLE operations (only SEARCH operations)

## Implementation Notes

**Benchmark Methodology:**
1. **Cold cache:** Clear OS page cache before each run (`sync; echo 3 > /proc/sys/vm/drop_caches`)
2. **Warm cache:** Run query 5 times, measure from 3rd run onward
3. **Statistical significance:** Run 100 iterations, report p50/p95/p99

**Database Size Scaling:**
- 10K chunks: Small project (like a utility library)
- 100K chunks: Medium project (like CrewChief)
- 1M chunks: Large monorepo (stress test)

**Expected Results:**
- Real database (458 edges): <10ms p95
- Synthetic 100K chunks: 20-30ms p95
- Scaling: Should be O(log n) or better due to indexes

**Optimization Options (if needed):**

If latency exceeds 30ms p95:
1. **Add composite index:** `CREATE INDEX idx_chunk_edges_quality ON chunk_edges(dst_chunk_id, type, src_chunk_id)`
2. **Pre-compute test chunks:** Add `is_test` boolean column to `chunks` table
3. **Simplify CASE statements:** Reduce number of LIKE conditions
4. **Materialize edge quality:** Pre-compute quality scores during indexing

**Go/No-Go Decision:**

- **Go (continue to Phase 1):** p95 <30ms on 100K database
- **Optimize and retry:** p95 30-40ms, add indexes or simplify query
- **Defer feature:** p95 >50ms, fundamental performance issue

## Dependencies

**Prerequisites:**
- SRCHREL-0001 (schema validation confirms query structure)
- criterion benchmark harness in Cargo.toml

**Blocks:**
- All Phase 1 tickets (cannot implement if performance fails)

## Risk Assessment

**Risk:** Query latency exceeds 30ms p95
**Probability:** Low (similar complexity to existing graph executor ~20ms)
**Mitigation:** Have optimization options ready (indexes, pre-computation). Can defer to Phase 2 if fundamental issue.

**Risk:** Full table scans detected in EXPLAIN
**Probability:** Medium (complex JOIN might miss indexes)
**Mitigation:** Add composite indexes, simplify subquery, use INDEXED BY hint.

**Risk:** Performance degrades non-linearly with database size
**Probability:** Low (indexes should provide O(log n) scaling)
**Mitigation:** Test with multiple database sizes, identify inflection point, add indexes or limits.

## Files/Packages Affected

**New Files:**
- `crates/maproom/benches/graph_quality_bench.rs` (benchmark harness)
- `crates/maproom/tests/helpers/synthetic_db.rs` (synthetic database generator)
- `.crewchief/projects/SRCHREL_relationship-aware-search/planning/performance-results.md` (benchmark results)

**Modified Files:**
- `crates/maproom/Cargo.toml` (add criterion dev-dependency if missing)
- `.crewchief/projects/SRCHREL_relationship-aware-search/planning/architecture.md` (document performance results)

**Database:**
- Temporary test databases for benchmarking (in-memory or temporary files)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Prerequisite 2, lines 57-86)
- Architecture: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/architecture.md` (Performance considerations, lines 543-570)
- Quality Strategy: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/quality-strategy.md` (Performance tests, lines 82-136)
