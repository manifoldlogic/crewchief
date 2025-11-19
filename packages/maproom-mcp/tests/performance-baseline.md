# Maproom Search Performance Baseline

## Overview

This document establishes the performance baseline for maproom search queries before implementing the file_type filter feature (FILETYPE-1001). These measurements enable objective evaluation of the "Performance impact <20%" success criterion.

## Test Environment

- **Date**: 2025-11-19T02:34:48.469Z
- **Database**: PostgreSQL + pgvector (maproom-postgres container)
- **Node.js**: v20.19.5
- **Platform**: Linux (devcontainer)
- **Repository**: crewchief
- **Index Size**:
  - Files: 2,106
  - Chunks: 74,384

## Test Methodology

### Query Details

- **Query**: "authentication"
- **Mode**: FTS (full-text search)
- **Ranking**: ts_rank_cd scoring
- **Result limit**: k=10
- **Iterations**: 10 runs

### SQL Query Executed

```sql
SELECT c.id, f.relpath, c.symbol_name, c.kind::text, c.start_line, c.end_line,
  ts_rank_cd(c.ts_doc, to_tsquery('simple', $2)) AS fts_score
FROM maproom.chunks c
JOIN maproom.files f ON f.id = c.file_id
WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
ORDER BY fts_score DESC
LIMIT 10
```

### Measurement Method

1. Direct PostgreSQL query execution via node-postgres client
2. JavaScript `performance.now()` for high-resolution timing
3. 10 consecutive executions with the same query
4. Outlier removal: values > 2 standard deviations from mean excluded
5. Baseline calculated from filtered mean

## Results

### Raw Measurements (ms)

| Run | Time (ms) |
|-----|-----------|
| 1   | 5.45      |
| 2   | 5.02      |
| 3   | 3.45      |
| 4   | 3.47      |
| 5   | 3.01      |
| 6   | 3.71      |
| 7   | 4.71      |
| 8   | 3.78      |
| 9   | 3.61      |
| 10  | 6.38      |

### Statistical Summary

| Metric                    | Value (ms) |
|---------------------------|------------|
| Mean (all runs)           | 4.26       |
| Mean (outliers removed)   | 4.02       |
| Median                    | 3.78       |
| Min                       | 3.01       |
| Max                       | 6.38       |
| Standard Deviation        | 1.02       |
| Outliers Removed          | 1          |

### Performance Baseline

- **Baseline**: **4.02 ms** (mean with outliers removed)
- **Acceptable Threshold** (+20%): **4.83 ms**

## Validation Criteria

For FILETYPE-1001 implementation to meet the "Performance impact <20%" criterion:

✅ **Pass**: Query execution time ≤ 4.83 ms (baseline × 1.2)

❌ **Fail**: Query execution time > 4.83 ms

## Reproducibility

To reproduce these measurements:

```bash
cd /workspace/packages/maproom-mcp

# Run the measurement script
node tests/measure-baseline-simple.mjs
```

### Prerequisites

1. Maproom PostgreSQL database running (`maproom-postgres` container)
2. Database schema initialized (migrations applied)
3. Repository indexed with at least 50k+ chunks
4. Node.js v20+ with `pg` package installed

## Notes

### Why These Numbers?

- **Sub-5ms baseline**: Reflects well-optimized FTS query with GIN index
- **Low variance**: Standard deviation of 1.02ms indicates stable performance
- **20% threshold**: Industry-standard acceptable performance degradation for new features

### Expected Impact of file_type Filter

The file_type filter implementation will add a WHERE clause:

```sql
AND f.relpath LIKE '%.ts'  -- Example for TypeScript files
```

**Predicted impact**:
- LIKE operator adds minimal overhead on indexed relpath column
- May actually improve performance by reducing result set size
- Expected to stay well within 4.83ms threshold

### Future Testing

After implementing FILETYPE-2001 (file_type filter):

1. Run same measurement script with identical query
2. Add file_type filter parameter to query
3. Compare execution time against 4.83ms threshold
4. Document results in FILETYPE-2001 verification notes

## Raw Data

Complete JSON output from measurement run:

```json
{
  "repository": {
    "name": "crewchief",
    "fileCount": 2106,
    "chunkCount": 74384
  },
  "testQuery": {
    "query": "authentication",
    "mode": "fts",
    "k": 10,
    "description": "Full-text search for \"authentication\" with ts_rank_cd scoring"
  },
  "measurements": {
    "timings": [5.45, 5.02, 3.45, 3.47, 3.01, 3.71, 4.71, 3.78, 3.61, 6.38],
    "mean": 4.26,
    "filteredMean": 4.02,
    "median": 3.78,
    "min": 3.01,
    "max": 6.38,
    "stdDev": 1.02,
    "outliersRemoved": 1
  },
  "baseline": {
    "value": 4.02,
    "threshold": 4.83,
    "method": "mean (outliers removed)"
  },
  "timestamp": "2025-11-19T02:34:48.469Z",
  "environment": {
    "nodeVersion": "v20.19.5",
    "platform": "linux",
    "database": "PostgreSQL + pgvector (maproom-postgres)"
  }
}
```

---

**Baseline established**: 4.02ms
**Threshold for FILETYPE-2001**: 4.83ms
**Measured by**: database-engineer (FILETYPE-1001)
**Status**: ✅ Ready for feature implementation
