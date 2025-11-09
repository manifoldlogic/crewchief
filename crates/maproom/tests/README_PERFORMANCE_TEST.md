# Query Performance Test for Migration 0017

## Overview

This test validates that migration 0017's two-index strategy maintains acceptable query performance for both small and large preview chunks.

## Running the Test

```bash
cd /workspace/crates/maproom/tests
./test_query_performance.sh
```

## What It Tests

### Test 1: Small Preview Search
- **Query**: Chunks with `preview <= 2000 bytes`
- **Expected Index**: `idx_chunks_search_small_preview`
- **Expected Scan Type**: Index Only Scan (no heap fetch)
- **Performance Target**: < 20ms execution time

### Test 2: Large Preview Search
- **Query**: Chunks with `preview > 2704 bytes`
- **Expected Index**: `idx_chunks_search_basic`
- **Expected Scan Type**: Index Scan with heap fetch
- **Performance Target**: < 50ms execution time

### Test 3: Mixed Query
- **Query**: Returns both small and large previews
- **Expected**: Appropriate index selection
- **Performance Target**: < 50ms execution time

## Success Criteria

✅ Small preview queries use `idx_chunks_search_small_preview` with Index Only Scan
✅ Large preview queries use `idx_chunks_search_basic` with Index Scan
✅ No Sequential Scans (indicates indexes are working)
✅ Query execution times meet performance thresholds

## Test Data

- **Total Rows**: 200 chunks
- **Distribution**: 95% small previews (≤ 2000 bytes), 5% large previews (> 2704 bytes)
- **Database**: PostgreSQL 15 with pgvector extension
- **Container**: Isolated test container (auto-cleanup)

## Output

The test provides:
- EXPLAIN (ANALYZE, BUFFERS) output for each query
- Index selection verification
- Scan type verification
- Execution time measurements
- Pass/fail status for each criterion
- Summary of all test results

## Dependencies

- Docker (for PostgreSQL container)
- Migration files: `0001_init.sql` and `0017_fix_index_size_limits.sql`

## Notes

- The test automatically spins up and tears down a PostgreSQL container
- Uses random port to avoid conflicts
- All cleanup is automatic via trap handler
- Test is idempotent and safe to run multiple times
