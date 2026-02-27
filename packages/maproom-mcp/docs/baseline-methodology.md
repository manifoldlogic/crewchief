# Baseline Search Quality Metrics Methodology

**Ticket**: SEMRANK-1005
**Date**: 2025-11-19
**Purpose**: Document FTS (Full-Text Search) baseline performance and ranking behavior before implementing semantic ranking improvements (Phase 2/3)

## Overview

This document describes the methodology for measuring baseline search quality metrics on the SEMRANK test corpus. These baselines will be used to validate that future semantic ranking improvements:

1. **Improve search quality** - Implementations rank above tests/docs for exact function searches
2. **Maintain acceptable latency** - Latency remains within ±10% of baseline values

## Test Environment

### Database Configuration
- **Connection**: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
- **Container**: `maproom-postgres` (Docker)
- **PostgreSQL Version**: 15+
- **Extensions**: pgvector, pg_trgm

### Test Corpus
- **Repository**: `test-corpus`
- **Worktree**: `main`
- **Total Files**: 13
- **Total Chunks**: 104
- **Languages**: Rust (3 files), TypeScript (3 files), Python (3 files), Markdown (4 files)
- **Indexed**: SEMRANK-1004 (2025-11-19)

### Search Binary
- **Binary**: `/workspace/packages/cli/bin/linux-arm64/maproom`
- **Version**: maproom 0.1.0
- **Search Mode**: FTS only (full-text search via PostgreSQL `ts_rank_cd`)

### Hardware/Environment
- **Platform**: Linux ARM64 (devcontainer)
- **Node Version**: v20.19.5
- **Measurement Tool**: TypeScript benchmark script (`scripts/benchmark-search.ts`)

## Golden Query Set

20 representative queries across multiple dimensions:

### Exact Function Names (8 queries)
- `authenticate` - Function name across all 3 languages
- `validate_token` - Python/Rust snake_case function
- `validateToken` - TypeScript camelCase function
- `create_session` - Python/Rust function
- `connect_database` - Rust-specific function
- `execute_query` - Method name across classes
- `useAuth` - React hook name
- `login` - Function within React hook

### Class/Type Names (2 queries)
- `DatabaseConnection` - Python class name
- `AuthenticationError` - Python exception class

### Concept Searches (4 queries)
- `user authentication` - Multi-word concept search
- `database connection` - DB functionality concept
- `session management` - Session handling concept
- `token validation` - Validation logic concept

### Documentation Searches (2 queries)
- `API reference` - Documentation heading
- `Python Authentication` - Language-specific docs

### Edge Cases (4 queries)
- `test_authenticate` - Test function name (should rank lower than implementation)
- `close` - Short common word (method name)
- `__init__` - Python dunder method
- `SEMRANK` - Project acronym in README

## Measurement Protocol

### Warmup Phase
- **Iterations**: 10 warmup queries per query term
- **Purpose**: Warm database caches (shared buffers, OS page cache)
- **Results**: Discarded (not included in metrics)

### Measurement Phase
- **Iterations**: 100 executions per query term
- **Timing**: Wall-clock time (includes binary spawn, search execution, JSON parsing)
- **Metrics Collected**:
  - **p50 (median)**: 50th percentile latency
  - **p95**: 95th percentile latency
  - **p99**: 99th percentile latency

### Ranking Analysis
For each query, analyze top 20 results to determine:

1. **Top 3 kinds**: Chunk types of top 3 results (e.g., `func,func,heading_2`)
2. **Implementation rank**: Position of first implementation chunk
   - Implementation = `func`, `class`, `method`, `hook`, `component` from `src/` directories
   - Excludes test files (`/test`, `_test.`) and docs (`/doc`, `.md`)
3. **Test rank**: Position of first test chunk
   - Test = chunks from files matching `/test` or `_test.`
4. **Doc rank**: Position of first documentation chunk
   - Doc = `heading_*`, `markdown_section`, or files from `/doc` or `.md`

## Results Format

### CSV Output (`benchmarks/baseline-fts.csv`)

```csv
query,description,latency_p50_ms,latency_p95_ms,latency_p99_ms,top_3_kinds,implementation_rank,test_rank,doc_rank
authenticate,Exact function name across all languages,30,38,41,"heading_2,heading_1,heading_2",8,6,1
```

**Columns**:
- `query`: Search query text
- `description`: Human-readable description of query purpose
- `latency_p50_ms`: Median latency in milliseconds
- `latency_p95_ms`: 95th percentile latency
- `latency_p99_ms`: 99th percentile latency
- `top_3_kinds`: Comma-separated chunk kinds of top 3 results
- `implementation_rank`: 1-based position of first implementation (empty if none)
- `test_rank`: 1-based position of first test (empty if none)
- `doc_rank`: 1-based position of first doc (empty if none)

### Query Plans (`benchmarks/baseline-query-plans.txt`)

EXPLAIN ANALYZE output for 5 representative queries:
1. `authenticate` - Exact function name
2. `user authentication` - Concept search
3. `test_authenticate` - Test function search
4. `DatabaseConnection` - Class name
5. `API reference` - Documentation search

**Purpose**: Document database query execution plans to identify:
- Index usage (should use GIN index on `ts_doc`)
- Sequential scans (should be minimal)
- Join strategies (nested loop vs merge join)
- Buffer usage (shared hit/read ratios)

## Baseline Results Summary

### Latency Performance
- **Average p50**: 39.4ms
- **Average p95**: 47.5ms
- **Average p99**: 55.8ms

**Interpretation**: Current FTS search is fast (sub-50ms p95), meeting the <50ms target for k=10 results. Phase 2/3 semantic ranking must maintain latency within 10% (p95 < 52ms).

### Ranking Behavior Issues

**Problem**: Documentation often ranks higher than implementations for exact function searches.

**Evidence**:
- `authenticate`: Docs rank #1, implementation ranks #8, test ranks #6
- `validate_token`: Docs rank #1, implementation ranks #9, test ranks #4
- `validateToken`: Docs rank #1, implementation ranks #4
- `DatabaseConnection`: Docs rank #1, implementation ranks #2

**Queries where implementations rank first** (good):
- `connect_database`: Implementation #1 (no docs/tests for this function)
- `execute_query`: Implementation #1 (method name)
- `user authentication`: Implementation #1 (concept search)
- `database connection`: Implementation #1 (concept search)
- `close`: Implementation #1 (short method name)
- `__init__`: Implementation #1 (dunder method)

**Queries where tests rank above implementations** (bad):
- `create_session`: Test #1, implementation #2
- `validate_token`: Test #4, implementation #9
- `authenticate`: Test #6, implementation #8

**Summary**: 3/20 queries show implementations ranking before tests, 3/20 show tests ranking before implementations. This validates the need for semantic ranking improvements (Phase 2).

## Query Plan Analysis

All 5 analyzed queries show expected behavior:

### Index Usage
- **GIN Index**: `ts_doc` GIN index is used correctly for all FTS queries
- **Filter Operation**: `c.ts_doc @@ to_tsquery(...)` uses index scan
- **No Sequential Scans**: All queries avoid full table scans on `chunks` table

### Query Structure
```sql
WITH ranked_chunks AS (
  SELECT c.id, f.relpath, c.symbol_name, c.kind::text,
         c.start_line, c.end_line,
         ts_rank_cd(c.ts_doc, to_tsquery('simple', $1)) AS rank
  FROM maproom.chunks c
  JOIN maproom.files f ON f.id = c.file_id
  JOIN maproom.repos r ON r.id = f.repo_id
  WHERE r.name = $2
    AND c.ts_doc @@ to_tsquery('simple', $1)
  ORDER BY rank DESC
  LIMIT 20
)
SELECT * FROM ranked_chunks;
```

### Performance Characteristics
- **Execution Time**: 0.4-1.7ms (database-side only)
- **Shared Buffers**: 95-250 hits (warm cache)
- **Read Operations**: 0-90 disk reads (cold start)
- **Sort Method**: Quicksort in memory (25-28kB)

**Note**: Wall-clock latencies (30-70ms) include binary spawn overhead (~20-30ms) which is acceptable for this use case.

## Variance and Reproducibility

### Acceptable Variance
- **Target**: ±5ms between runs
- **Observed**: Latency variance within acceptable range for p50/p95
- **p99 Variance**: Some queries show higher p99 variance due to outliers (cold cache, OS scheduling)

### Reproducibility Steps

To reproduce these baseline measurements:

```bash
# 1. Ensure test corpus is indexed
cd /tmp/semrank-test-corpus
/workspace/packages/cli/bin/linux-arm64/maproom scan \
  --repo test-corpus \
  --worktree main \
  --path /tmp/semrank-test-corpus \
  --commit HEAD \
  --force

# 2. Run benchmark script
cd /workspace/packages/maproom-mcp
npx tsx scripts/benchmark-search.ts

# 3. Results written to:
# - benchmarks/baseline-fts.csv
# - benchmarks/baseline-query-plans.txt
```

## Acceptance Criteria Verification

- ✅ **Golden query set defined**: 20 queries across Rust, TypeScript, Python
- ✅ **Latency baselines measured**: p50, p95, p99 over 100 runs per query
- ✅ **Baseline CSV format documented**: Includes latency + ranking metrics
- ✅ **Benchmark script created**: Automated, reproducible (`scripts/benchmark-search.ts`)
- ✅ **Current ranking behavior documented**: Examples where docs rank above implementations
- ✅ **Database query plans logged**: EXPLAIN ANALYZE for 5 representative queries

## Next Steps (Phase 2)

Use these baselines to validate Phase 2 semantic ranking improvements:

1. **Quality Improvement Targets**:
   - Implementations should rank #1-3 for exact function name queries
   - Tests should rank below implementations for non-test queries
   - Docs should rank below implementations for exact matches

2. **Latency Constraints**:
   - p95 latency must remain < 52ms (within 10% of 47.5ms baseline)
   - p99 latency should remain < 61ms (within 10% of 55.8ms baseline)

3. **Regression Testing**:
   - Re-run benchmark script after Phase 2 implementation
   - Compare results CSV to baseline
   - Verify improvements in implementation_rank without latency regression

## Files Generated

1. `/workspace/packages/maproom-mcp/scripts/benchmark-search.ts` - Benchmark script
2. `/workspace/packages/maproom-mcp/benchmarks/baseline-fts.csv` - Baseline metrics (CSV)
3. `/workspace/packages/maproom-mcp/benchmarks/baseline-query-plans.txt` - Query plans
4. `/workspace/packages/maproom-mcp/docs/baseline-methodology.md` - This document
