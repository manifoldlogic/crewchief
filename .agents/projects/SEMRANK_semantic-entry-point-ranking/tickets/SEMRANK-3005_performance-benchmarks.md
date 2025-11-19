# Ticket: SEMRANK-3005: Performance Benchmarks

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (benchmarks, not unit tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- verify-ticket
- commit-ticket

## Summary
Run performance benchmarks comparing semantic ranking against baseline FTS to ensure latency remains acceptable (<10% p95 increase) using the golden query set and benchmark script from SEMRANK-1005.

## Background
The semantic ranking implementation adds SQL CASE statements for kind multipliers and exact match detection. While these operations are computationally simple, they add overhead to every search query. This ticket validates that the performance impact is acceptable.

This ticket implements performance validation from Phase 3 of the SEMRANK execution plan (plan.md, lines 209-215). The success criteria require:
- p95 latency increase <10% vs baseline
- Target: <200ms for medium corpus (100K chunks)

Baseline metrics were collected in SEMRANK-1005, including:
- Golden query set: 20 representative queries across languages
- Benchmark script: `/packages/maproom-mcp/scripts/benchmark-search.ts`
- Baseline results: `/packages/maproom-mcp/benchmarks/baseline-fts.csv`
- Metrics: p50, p95, p99 latencies over 100 runs per query

This ticket re-runs the same benchmark with semantic ranking enabled and compares results.

## Acceptance Criteria
- [ ] Run benchmark script with semantic ranking enabled (100 runs per query)
- [ ] Measure p50, p95, p99 latencies for all 20 golden queries
- [ ] Export results to `/packages/maproom-mcp/benchmarks/semantic-ranking-fts.csv`
- [ ] Compare against baseline: `/packages/maproom-mcp/benchmarks/baseline-fts.csv`
- [ ] p95 latency increase <10% vs baseline (target: <200ms for medium corpus)
- [ ] Document any queries with >10% increase and investigate root cause
- [ ] Create comparison report: `/packages/maproom-mcp/benchmarks/performance-comparison.md`
- [ ] All performance targets met or issues documented with mitigation plan

## Technical Requirements
- Use existing benchmark script: `/packages/maproom-mcp/scripts/benchmark-search.ts`
- Run against same test corpus used for baseline (from SEMRANK-1004)
- Ensure database is in consistent state (no concurrent writes during benchmark)
- Collect same metrics as baseline:
  - p50, p95, p99 latencies (in milliseconds)
  - Min/max latencies
  - Standard deviation
- Run 100 iterations per query (same as baseline)
- Use same hardware/environment as baseline for fair comparison
- Export results in same CSV format as baseline for easy comparison

## Implementation Notes

### Benchmark Execution Steps

1. **Prepare Environment:**
   ```bash
   # Ensure database is idle
   # Clear query cache if applicable
   # Verify test corpus is indexed
   ```

2. **Run Benchmark:**
   ```bash
   cd /workspace/packages/maproom-mcp
   pnpm run benchmark:search
   ```

3. **Export Results:**
   ```bash
   # Script should write to benchmarks/semantic-ranking-fts.csv
   # Format: query, p50, p95, p99, min, max, stddev
   ```

4. **Compare Results:**
   ```bash
   # Create comparison report
   # Calculate percentage differences
   # Identify outliers
   ```

### CSV Output Format
```csv
query,p50_ms,p95_ms,p99_ms,min_ms,max_ms,stddev_ms
authenticate,45.2,78.3,95.1,38.7,102.4,12.5
validate_provider,52.1,89.4,110.2,44.3,125.6,15.8
...
```

### Comparison Report Template
Create `/packages/maproom-mcp/benchmarks/performance-comparison.md`:

```markdown
# Performance Comparison: Baseline FTS vs Semantic Ranking

## Summary
- Queries tested: 20
- Queries within target (<10% increase): X/20
- Queries exceeding target: Y/20
- Overall p95 median change: +Z%

## Metrics
| Query | Baseline p95 (ms) | Semantic p95 (ms) | Change (%) | Status |
|-------|------------------|------------------|------------|--------|
| authenticate | 75.3 | 78.3 | +4.0% | ✓ Pass |
| validate_provider | 82.1 | 89.4 | +8.9% | ✓ Pass |
| complex_query | 105.2 | 118.7 | +12.8% | ✗ Fail |

## Analysis

### Queries Exceeding 10% Threshold
[For each query with >10% increase:]
- **Query:** complex_query
- **Baseline p95:** 105.2ms
- **Semantic p95:** 118.7ms
- **Increase:** +12.8% (+13.5ms)
- **Investigation:** [Explain why - e.g., multiple CASE evaluations, complex normalization]
- **Mitigation:** [Proposed fix or acceptance rationale]

### Overall Performance Impact
- **Median p50 change:** +X%
- **Median p95 change:** +Y%
- **Median p99 change:** +Z%

### Query Plan Analysis
[Include EXPLAIN ANALYZE output for slowest queries]

### Recommendations
- [ ] All queries meet target: Deploy as-is
- [ ] Some queries exceed target: Optimize identified queries
- [ ] Widespread slowdown: Revisit SQL implementation

## Conclusion
[Pass/Fail verdict with justification]
```

### Investigation Process for Slow Queries

If any query exceeds 10% threshold:

1. **Run EXPLAIN ANALYZE:**
   ```sql
   EXPLAIN ANALYZE
   SELECT ..., (ts_rank_cd(...) * kind_mult * exact_mult) AS final_score
   FROM maproom.chunks
   WHERE ts_doc @@ to_tsquery($1)
   ORDER BY final_score DESC
   LIMIT 20;
   ```

2. **Check for:**
   - Sequential scans (should use ts_doc index)
   - Expensive CASE evaluations
   - Large intermediate result sets
   - Inefficient sort operations

3. **Potential Optimizations:**
   - Add index on kind column if CASE is slow
   - Simplify CASE logic
   - Use materialized view for kind multipliers (future)
   - Adjust LIMIT to reduce result set processing

4. **Document Findings:**
   - Root cause of slowdown
   - Whether optimization is needed
   - Trade-off analysis (correctness vs speed)

### Performance Targets

Per plan.md success criteria:
- **Primary target:** p95 latency increase <10%
- **Absolute target:** p95 <200ms for medium corpus (100K chunks)
- **Acceptable range:** 5-10% increase is normal for added SQL logic
- **Action threshold:** >15% increase requires investigation and mitigation

### Corpus Size Validation

Verify test corpus size matches baseline:
```sql
SELECT COUNT(*) FROM maproom.chunks;
-- Should match corpus size from SEMRANK-1004
```

If corpus size differs, note in report as potential confounding factor.

## Dependencies
- SEMRANK-1004 (test corpus indexed - same corpus as baseline)
- SEMRANK-1005 (baseline metrics collected, golden query set defined, benchmark script created)
- SEMRANK-2005 (combined multipliers in SQL - the implementation being benchmarked)
- SEMRANK-2006 (debug mode - may be used to analyze slow queries)

## Risk Assessment
- **Risk**: Performance may exceed 10% threshold
  - **Mitigation**: Investigate root cause, optimize SQL if needed, or adjust target if justified
- **Risk**: Benchmark environment differs from baseline (hardware, database state)
  - **Mitigation**: Verify environment consistency, document any differences
- **Risk**: Some queries are inherently slower due to normalization complexity
  - **Mitigation**: Acceptable if <15% of queries exceed threshold; document trade-offs
- **Risk**: Benchmark may not represent production workload
  - **Mitigation**: Golden query set designed to be representative; note limitations

## Files/Packages Affected
- `/packages/maproom-mcp/scripts/benchmark-search.ts` (existing script, run with semantic ranking)
- `/packages/maproom-mcp/benchmarks/semantic-ranking-fts.csv` (new file - benchmark results)
- `/packages/maproom-mcp/benchmarks/performance-comparison.md` (new file - analysis report)
- `/packages/maproom-mcp/src/tools/search.ts` (may need optimization if benchmarks fail)
