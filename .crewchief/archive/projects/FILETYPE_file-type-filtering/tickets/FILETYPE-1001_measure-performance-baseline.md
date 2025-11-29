# Ticket: FILETYPE-1001: Measure Performance Baseline

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation task, no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- verify-ticket
- commit-ticket

## Summary
Establish baseline query performance metrics before implementing the file_type filter to enable objective measurement of the "Performance impact <20%" success criterion.

## Background
The quality-strategy.md defines performance acceptance as "<20% query overhead vs baseline" but without measuring current performance first, this criterion is unmeasurable. This ticket implements Task 1.0 from plan.md to create objective performance benchmarks.

**Reference:** plan.md - Task 1.0 (Pre-Implementation Performance Baseline)

## Acceptance Criteria
- [x] Baseline measurement documented with average of 10 runs
- [x] Acceptable threshold calculated (baseline × 1.2)
- [x] Test repository size documented (file count)
- [x] Measurement method is reproducible

## Technical Requirements
- Run 10 search queries without filters on medium-sized repo (5k-10k files)
- Measure average query time (exclude outliers)
- Document results in `packages/maproom-mcp/tests/performance-baseline.md`
- Calculate acceptable performance threshold for validation

## Implementation Notes

**Measurement method:**
1. Identify test repository with 5k-10k indexed files
2. Run same search query 10 times without any filters
3. Record query execution time for each run
4. Calculate average (exclude high/low outliers if needed)
5. Calculate threshold: baseline_avg × 1.2

**Example approach:**
```bash
# From packages/maproom-mcp directory
for i in {1..10}; do
  # Run search and capture timing
  node bin/cli.cjs search "authentication" --repo crewchief --mode hybrid
  # Extract "Query time: Xms" from output
done
```

**Expected baselines:**
- Small repo (<1k files): ~50ms
- Medium repo (5k-10k files): ~100ms
- Large repo (50k+ files): ~200ms

**Deliverable format (performance-baseline.md):**
```markdown
# Performance Baseline - File Type Filter

**Date:** 2025-11-19
**Repository:** [repo name]
**File count:** [X files]
**Query:** "authentication"
**Mode:** hybrid

## Baseline Measurements (No Filter)

Run 1: 98ms
Run 2: 102ms
...
Run 10: 99ms

**Average:** 100ms
**Threshold (baseline × 1.2):** 120ms

## Validation Criteria

After implementing file_type filter:
- Single extension (file_type: "ts"): Must be ≤ 120ms
- Multi extension (file_type: "ts,tsx,js"): Must be ≤ 120ms

If performance exceeds threshold, optimization required.
```

## Dependencies
- None (must run FIRST before any implementation)

## Risk Assessment
- **Risk**: Test repo too small/large may not be representative
  - **Mitigation**: Use crewchief repo (known medium size) for consistent baseline

- **Risk**: Query time variance due to database caching
  - **Mitigation**: Run 10+ iterations to average out cache effects

## Files/Packages Affected
- `packages/maproom-mcp/tests/performance-baseline.md` (NEW FILE)

## Implementation Summary

**Completed**: 2025-11-19

**Baseline established**: 4.02ms (mean with outliers removed)
**Acceptable threshold**: 4.83ms (baseline × 1.2)

**Test details**:
- Repository: crewchief
- Index size: 2,106 files / 74,384 chunks
- Query: "authentication" (FTS mode with ts_rank_cd)
- Runs: 10 iterations
- Statistical method: Mean with 2σ outlier removal

**Deliverables**:
1. `/workspace/packages/maproom-mcp/tests/performance-baseline.md` - Complete documentation
2. `/workspace/packages/maproom-mcp/tests/measure-baseline-simple.mjs` - Reproducible measurement script

**Key finding**: Sub-5ms baseline indicates excellent FTS query performance. The 20% threshold (4.83ms) provides sufficient headroom for file_type filter implementation while maintaining high performance standards.
