# pg_stat_statements Analysis Guide

This document provides comprehensive guidance on using PostgreSQL's `pg_stat_statements` extension to analyze and optimize query performance in the Maproom system.

## Table of Contents

1. [Overview](#overview)
2. [Setup and Configuration](#setup-and-configuration)
3. [Analysis Queries](#analysis-queries)
4. [Interpretation Guidelines](#interpretation-guidelines)
5. [Optimization Recommendations](#optimization-recommendations)
6. [Maintenance and Best Practices](#maintenance-and-best-practices)

---

## Overview

`pg_stat_statements` is a PostgreSQL extension that tracks execution statistics for all SQL statements executed by the database. It provides critical insights for:

- Identifying slow queries
- Finding frequently executed queries
- Detecting missing indexes
- Optimizing query patterns
- Monitoring performance trends

**Key Metrics Tracked:**
- Total execution time (mean, min, max, stddev)
- Number of calls (execution frequency)
- Rows processed (fetched/returned)
- Buffer usage (blocks read/written/hit)
- Planning time vs execution time

---

## Setup and Configuration

### 1. Enable the Extension

First, enable `pg_stat_statements` in your PostgreSQL configuration:

#### postgresql.conf

```conf
# Add to shared_preload_libraries (requires restart)
shared_preload_libraries = 'pg_stat_statements'

# Configure tracking detail level
pg_stat_statements.track = all              # Track all statements (top-level + nested)
pg_stat_statements.max = 10000              # Store statistics for up to 10,000 statements
pg_stat_statements.track_utility = on       # Track utility commands (DDL, VACUUM, etc.)
pg_stat_statements.track_planning = on      # Track planning time separately
```

**Note:** Changes to `shared_preload_libraries` require a PostgreSQL restart.

#### Restart PostgreSQL

```bash
# Linux/macOS
sudo systemctl restart postgresql
# or
sudo pg_ctl restart -D /path/to/data/directory

# Docker
docker restart <postgres-container>
```

### 2. Create the Extension in Your Database

Connect to your database and create the extension:

```sql
-- Create extension (one-time setup)
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

-- Verify installation
SELECT * FROM pg_stat_statements LIMIT 1;
```

### 3. Verify Configuration

Check that the extension is properly configured:

```sql
-- Check current settings
SHOW shared_preload_libraries;  -- Should include 'pg_stat_statements'
SHOW pg_stat_statements.track;  -- Should be 'all' or 'top'
SHOW pg_stat_statements.max;    -- Should be 5000-10000

-- Check extension version
SELECT * FROM pg_available_extensions WHERE name = 'pg_stat_statements';
```

---

## Analysis Queries

Below are essential queries for analyzing database performance using `pg_stat_statements`.

### 1. Most Frequent Queries (Top 20 by Call Count)

Identify queries that execute most frequently. These are candidates for caching or optimization.

```sql
-- Most frequently executed queries
SELECT
  calls,
  ROUND(total_exec_time::numeric, 2) AS total_time_ms,
  ROUND(mean_exec_time::numeric, 2) AS mean_time_ms,
  ROUND((total_exec_time / SUM(total_exec_time) OVER ()) * 100, 2) AS percent_total_time,
  LEFT(query, 100) AS query_preview
FROM pg_stat_statements
WHERE query NOT LIKE '%pg_stat_statements%'  -- Exclude this monitoring query itself
ORDER BY calls DESC
LIMIT 20;
```

**Interpretation:**
- `calls`: Number of times query was executed
- `total_time_ms`: Total wall-clock time spent on this query
- `mean_time_ms`: Average execution time per call
- `percent_total_time`: Percentage of total database time consumed
- High call count + moderate mean time = good caching candidate
- High call count + high mean time = critical optimization target

**Example Output:**
```
 calls | total_time_ms | mean_time_ms | percent_total_time | query_preview
-------+---------------+--------------+--------------------+-----------------------------------
 45231 |       1234.56 |         0.03 |               5.23 | SELECT c.id, c.symbol_name FROM...
 12450 |       8765.43 |         0.70 |              37.12 | WITH lex_scores AS (SELECT...
```

---

### 2. Slowest Queries (Top 20 by Mean Execution Time)

Find queries with the highest average execution time. These are primary optimization targets.

```sql
-- Slowest queries by mean execution time
SELECT
  calls,
  ROUND(mean_exec_time::numeric, 2) AS mean_time_ms,
  ROUND(max_exec_time::numeric, 2) AS max_time_ms,
  ROUND(stddev_exec_time::numeric, 2) AS stddev_ms,
  ROUND(total_exec_time::numeric, 2) AS total_time_ms,
  LEFT(query, 120) AS query_preview
FROM pg_stat_statements
WHERE query NOT LIKE '%pg_stat_statements%'
  AND calls > 10  -- Filter out one-off queries
ORDER BY mean_exec_time DESC
LIMIT 20;
```

**Interpretation:**
- `mean_time_ms`: Average execution time (primary metric)
- `max_time_ms`: Worst-case execution time (p100)
- `stddev_ms`: Standard deviation (consistency indicator)
- High stddev = inconsistent performance (investigate query plan variations)
- Queries >50ms mean time: CRITICAL (violates p95 latency target)
- Queries 20-50ms: WARNING (monitor closely)
- Queries <20ms: GOOD (within target)

**Example Output:**
```
 calls | mean_time_ms | max_time_ms | stddev_ms | total_time_ms | query_preview
-------+--------------+-------------+-----------+---------------+-----------------------------------
   234 |       156.78 |      342.12 |     45.23 |      36685.52 | SELECT * FROM maproom.chunks WHERE...
   892 |        87.34 |      198.45 |     12.67 |      77906.28 | WITH RECURSIVE...
```

---

### 3. Slowest Queries (Top 20 by Total Time)

Identify queries consuming the most cumulative database time.

```sql
-- Queries consuming most total execution time
SELECT
  calls,
  ROUND(total_exec_time::numeric, 2) AS total_time_ms,
  ROUND(mean_exec_time::numeric, 2) AS mean_time_ms,
  ROUND((total_exec_time / SUM(total_exec_time) OVER ()) * 100, 2) AS percent_total_time,
  LEFT(query, 120) AS query_preview
FROM pg_stat_statements
WHERE query NOT LIKE '%pg_stat_statements%'
ORDER BY total_exec_time DESC
LIMIT 20;
```

**Interpretation:**
- `total_time_ms`: Cumulative time spent (calls × mean_time)
- `percent_total_time`: Share of total database workload
- High total time = biggest impact on overall system performance
- Optimize these first for maximum benefit
- Target: No single query should consume >10% of total time

**Example Output:**
```
 calls | total_time_ms | mean_time_ms | percent_total_time | query_preview
-------+---------------+--------------+--------------------+-----------------------------------
 12450 |      87654.32 |        70.40 |              42.15 | WITH lex_scores AS...
  3421 |      45678.90 |       133.50 |              21.98 | SELECT c.code_embedding <=>...
```

---

### 4. Index Usage Analysis

Verify that indexes are being used effectively and identify sequential scans on large tables.

```sql
-- Index usage statistics (from pg_stat_user_indexes)
SELECT
  schemaname,
  tablename,
  indexname,
  idx_scan AS index_scans,
  idx_tup_read AS tuples_read,
  idx_tup_fetch AS tuples_fetched,
  pg_size_pretty(pg_relation_size(indexrelid)) AS index_size
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
ORDER BY idx_scan DESC;
```

**Interpretation:**
- `index_scans`: Number of times index was used
- `tuples_read`: Rows read from index
- `tuples_fetched`: Rows actually returned (after visibility checks)
- High scans = useful index
- Zero scans = potentially unused index (consider dropping)
- Low fetch ratio (fetched/read < 0.1) = index selectivity issue

**Example Output:**
```
 schemaname | tablename | indexname              | index_scans | tuples_read | tuples_fetched | index_size
------------+-----------+------------------------+-------------+-------------+----------------+------------
 maproom    | chunks    | idx_chunks_code_vec    |       45231 |      452310 |         452310 | 512 MB
 maproom    | chunks    | idx_chunks_tsv         |       12450 |      124500 |         124500 | 128 MB
 maproom    | files     | idx_files_repo_worktree|        8923 |       89230 |          89230 | 16 MB
 maproom    | chunks    | idx_chunks_recent      |         234 |        2340 |           2340 | 4096 kB
```

---

### 5. Sequential Scan Detection (Missing Index Opportunities)

Find tables with high sequential scan counts, indicating potential missing indexes.

```sql
-- Tables with high sequential scan activity
SELECT
  schemaname,
  tablename,
  seq_scan AS sequential_scans,
  seq_tup_read AS seq_rows_read,
  idx_scan AS index_scans,
  idx_tup_fetch AS idx_rows_fetched,
  n_live_tup AS live_rows,
  ROUND(100.0 * seq_scan / NULLIF(seq_scan + idx_scan, 0), 2) AS seq_scan_pct,
  ROUND(seq_tup_read::numeric / NULLIF(seq_scan, 0), 2) AS avg_seq_read
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
  AND seq_scan > 0
ORDER BY seq_tup_read DESC;
```

**Interpretation:**
- `sequential_scans`: Number of full table scans
- `seq_rows_read`: Total rows read via sequential scans
- `seq_scan_pct`: Percentage of scans that are sequential (vs index)
- High seq_scan on large tables = CRITICAL (add index)
- Small tables (<1000 rows): Sequential scans are OK
- Large tables (>10k rows) with seq_scan_pct >10%: Investigate

**Red Flags:**
- `chunks` table with seq_scan_pct >5% (should be almost all index scans)
- `avg_seq_read` > 10,000 rows per scan (very expensive)

**Example Output:**
```
 schemaname | tablename | sequential_scans | seq_rows_read | index_scans | idx_rows_fetched | live_rows | seq_scan_pct | avg_seq_read
------------+-----------+------------------+---------------+-------------+------------------+-----------+--------------+--------------
 maproom    | chunks    |              123 |       1234567 |       45231 |           452310 |    100000 |         0.27 |     10037.13
 maproom    | files     |               45 |         45000 |        8923 |            89230 |     10000 |         0.50 |      1000.00
```

---

### 6. Buffer Usage Analysis (Cache Hit Ratio)

Analyze how effectively PostgreSQL's buffer cache is being utilized.

```sql
-- Buffer cache hit ratio per query
SELECT
  LEFT(query, 80) AS query_preview,
  calls,
  shared_blks_hit AS cache_hits,
  shared_blks_read AS disk_reads,
  ROUND(100.0 * shared_blks_hit / NULLIF(shared_blks_hit + shared_blks_read, 0), 2) AS cache_hit_ratio,
  ROUND(total_exec_time::numeric, 2) AS total_time_ms
FROM pg_stat_statements
WHERE shared_blks_hit + shared_blks_read > 0
  AND query NOT LIKE '%pg_stat_statements%'
ORDER BY shared_blks_read DESC
LIMIT 20;
```

**Interpretation:**
- `cache_hits`: Blocks read from memory (fast)
- `disk_reads`: Blocks read from disk (slow)
- `cache_hit_ratio`: Percentage of reads served from cache
- Target: >95% cache hit ratio for frequent queries
- Low ratio (<90%) = insufficient `shared_buffers` or working set too large

**Example Output:**
```
 query_preview                                           | calls | cache_hits | disk_reads | cache_hit_ratio | total_time_ms
---------------------------------------------------------+-------+------------+------------+-----------------+---------------
 WITH lex_scores AS (SELECT c.id, ts_rank_cd(c.ts_doc...)|  1245 |     456789 |       1234 |           99.73 |      87654.32
 SELECT c.id, c.symbol_name FROM maproom.chunks c...    |  4523 |      45230 |       5678 |           88.85 |      12345.67
```

---

### 7. Planning vs Execution Time Analysis

Identify queries where planning time is significant relative to execution time.

```sql
-- Planning time vs execution time breakdown
SELECT
  calls,
  ROUND(mean_plan_time::numeric, 2) AS mean_plan_ms,
  ROUND(mean_exec_time::numeric, 2) AS mean_exec_ms,
  ROUND(100.0 * mean_plan_time / NULLIF(mean_plan_time + mean_exec_time, 0), 2) AS plan_pct,
  ROUND(total_plan_time::numeric, 2) AS total_plan_ms,
  ROUND(total_exec_time::numeric, 2) AS total_exec_ms,
  LEFT(query, 100) AS query_preview
FROM pg_stat_statements
WHERE calls > 10
  AND mean_plan_time > 0.1  -- Only queries with measurable planning time
  AND query NOT LIKE '%pg_stat_statements%'
ORDER BY plan_pct DESC
LIMIT 20;
```

**Interpretation:**
- `mean_plan_ms`: Average time spent planning query
- `mean_exec_ms`: Average time spent executing query
- `plan_pct`: Planning time as percentage of total time
- High plan_pct (>20%) = consider using prepared statements
- Prepared statements cache the plan, eliminating planning overhead

**Optimization:**
- Use Rust's `client.prepare_cached()` for frequently executed queries
- Consider `PREPARE` statements for complex queries

**Example Output:**
```
 calls | mean_plan_ms | mean_exec_ms | plan_pct | total_plan_ms | total_exec_ms | query_preview
-------+--------------+--------------+----------+---------------+---------------+-----------------------------------
  1245 |        12.34 |        15.67 |    44.03 |       15363.3 |       19508.15| WITH RECURSIVE...
  4523 |         2.45 |        10.23 |    19.34 |       11081.35|       46270.29| SELECT c.id FROM maproom.chunks...
```

---

### 8. Query Normalization (Aggregate Similar Queries)

View aggregated statistics for parameterized query patterns.

```sql
-- Aggregate statistics for parameterized queries
-- (pg_stat_statements automatically normalizes literals to $1, $2, etc.)
SELECT
  calls,
  ROUND(total_exec_time::numeric, 2) AS total_time_ms,
  ROUND(mean_exec_time::numeric, 2) AS mean_time_ms,
  ROUND(stddev_exec_time::numeric, 2) AS stddev_ms,
  ROUND(min_exec_time::numeric, 2) AS min_ms,
  ROUND(max_exec_time::numeric, 2) AS max_ms,
  LEFT(query, 120) AS query_pattern
FROM pg_stat_statements
WHERE query NOT LIKE '%pg_stat_statements%'
  AND calls > 100  -- Focus on frequently executed patterns
ORDER BY total_exec_time DESC
LIMIT 20;
```

**Interpretation:**
- `pg_stat_statements` automatically normalizes literals to `$1`, `$2`, etc.
- This aggregates statistics for the same query with different parameter values
- Useful for identifying common query patterns across different inputs

---

### 9. Full Query Details (Complete SQL Text)

Get complete SQL text for a specific query (useful when preview is truncated).

```sql
-- Get full query text by queryid
SELECT
  queryid,
  calls,
  ROUND(mean_exec_time::numeric, 2) AS mean_time_ms,
  query
FROM pg_stat_statements
WHERE queryid = 1234567890  -- Replace with actual queryid from previous queries
\gx  -- Display in expanded format for readability
```

---

## Interpretation Guidelines

### Performance Targets (Maproom Hybrid Search)

Based on ticket HYBRID_SEARCH-4002 requirements:

| Metric | Target | Status |
|--------|--------|--------|
| p95 latency (hybrid search) | <50ms | ✅ ACHIEVED (28ms) |
| Recall@10 | >95% | ✅ ACHIEVED (96.34%) |
| Index scan vs seq scan | >95% index | Monitor |
| Cache hit ratio | >95% | Monitor |

### Query Classification

| Mean Execution Time | Classification | Action |
|---------------------|----------------|--------|
| <10ms | EXCELLENT | No action needed |
| 10-20ms | GOOD | Monitor for trends |
| 20-50ms | ACCEPTABLE | Consider optimization if frequent |
| 50-100ms | WARNING | Optimize if >1% of queries |
| >100ms | CRITICAL | Immediate optimization required |

### Index Usage Health

| Condition | Assessment | Action |
|-----------|------------|--------|
| idx_scan > 1000/day | HEALTHY | Continue monitoring |
| idx_scan < 100/day | UNDERUTILIZED | Consider dropping if persistently low |
| seq_scan > idx_scan (large table) | CRITICAL | Add appropriate index |
| cache_hit_ratio < 90% | WARNING | Increase shared_buffers or optimize queries |

---

## Optimization Recommendations

### 1. Frequent Queries with High Mean Time

**Problem:** Query executed many times with high average latency
```
calls=5234, mean_time_ms=78.45
```

**Actions:**
1. Run `EXPLAIN (ANALYZE, BUFFERS)` to identify bottleneck
2. Check for missing indexes
3. Consider query rewrite or denormalization
4. Implement caching layer if appropriate

---

### 2. High Sequential Scan Rate

**Problem:** Large table scanned sequentially
```
tablename='chunks', seq_scan=1234, seq_tup_read=123400000
```

**Actions:**
1. Identify frequently filtered columns
2. Create appropriate B-tree or partial index
3. Use `CREATE INDEX CONCURRENTLY` to avoid blocking writes
4. Verify index usage with `EXPLAIN`

**Example:**
```sql
-- Create partial index for common filter
CREATE INDEX CONCURRENTLY idx_chunks_active
  ON maproom.chunks (recency_score, churn_score)
  WHERE recency_score > 0.5 AND churn_score > 5;
```

---

### 3. Low Cache Hit Ratio

**Problem:** Too many disk reads
```
cache_hit_ratio=82.34% (target: >95%)
```

**Actions:**
1. Increase `shared_buffers` in postgresql.conf
2. Check working set size vs available memory
3. Identify specific queries causing disk I/O
4. Consider table partitioning for large tables

**Example postgresql.conf tuning:**
```conf
shared_buffers = 4GB              # Was: 2GB
effective_cache_size = 12GB       # 75% of system RAM
maintenance_work_mem = 512MB      # For index creation
```

---

### 4. High Planning Time

**Problem:** Planning time is significant relative to execution
```
mean_plan_ms=15.67, mean_exec_ms=10.23 (plan_pct=60.5%)
```

**Actions:**
1. Use prepared statements (Rust: `client.prepare_cached()`)
2. Reduce query complexity if possible
3. Update table statistics: `ANALYZE maproom.chunks;`

**Example Rust optimization:**
```rust
// Before: Planning overhead on every call
let rows = client.query(
    "SELECT * FROM maproom.chunks WHERE file_id = $1",
    &[&file_id]
).await?;

// After: Plan cached, only execute on subsequent calls
let stmt = client.prepare_cached(
    "SELECT * FROM maproom.chunks WHERE file_id = $1"
).await?;
let rows = client.query(&stmt, &[&file_id]).await?;
```

---

### 5. High Standard Deviation

**Problem:** Inconsistent query performance
```
mean_time_ms=45.67, stddev_ms=67.89 (very high variance)
```

**Actions:**
1. Check if query planner switches between plans
2. Verify statistics are up-to-date: `ANALYZE`
3. Consider using plan hints or `SET LOCAL` parameters
4. Investigate parameter sniffing issues

---

## Maintenance and Best Practices

### Regular Monitoring Schedule

```sql
-- Weekly: Review top 20 slowest queries
-- (Run queries from sections 2-3)

-- Daily: Check cache hit ratio
SELECT
  SUM(shared_blks_hit) AS cache_hits,
  SUM(shared_blks_read) AS disk_reads,
  ROUND(100.0 * SUM(shared_blks_hit) / NULLIF(SUM(shared_blks_hit + shared_blks_read), 0), 2) AS cache_hit_ratio
FROM pg_stat_statements;

-- Monthly: Reset statistics to start fresh baseline
-- (Only after archiving previous month's data)
-- SELECT pg_stat_statements_reset();
```

### Resetting Statistics

Reset `pg_stat_statements` data to clear counters (use cautiously):

```sql
-- Reset all pg_stat_statements data
SELECT pg_stat_statements_reset();

-- Note: This clears ALL historical data, irreversible
-- Best practice: Export data before resetting
```

### Exporting Statistics for Analysis

```bash
# Export to CSV for offline analysis
psql -d your_database -c "\copy (
  SELECT * FROM pg_stat_statements
  WHERE query NOT LIKE '%pg_stat_statements%'
  ORDER BY total_exec_time DESC
) TO '/tmp/pg_stats_$(date +%Y%m%d).csv' WITH CSV HEADER"
```

### Integration with Monitoring Tools

Consider integrating with monitoring platforms:

- **pgAdmin**: Built-in pg_stat_statements dashboard
- **pgBadger**: Log analyzer with pg_stat_statements support
- **Datadog/Prometheus**: PostgreSQL exporter includes pg_stat_statements metrics
- **Custom dashboards**: Grafana + PostgreSQL data source

### Automated Alerting

Set up alerts for critical conditions:

```sql
-- Example: Alert if any query has mean time >100ms
SELECT
  'ALERT: Slow query detected' AS message,
  LEFT(query, 100) AS query_preview,
  ROUND(mean_exec_time::numeric, 2) AS mean_time_ms,
  calls
FROM pg_stat_statements
WHERE mean_exec_time > 100
  AND calls > 10
  AND query NOT LIKE '%pg_stat_statements%'
ORDER BY mean_exec_time DESC;
```

---

## Troubleshooting

### Extension Not Loading

**Problem:** `pg_stat_statements` view doesn't exist

**Solution:**
```sql
-- Check if extension is installed
SELECT * FROM pg_available_extensions WHERE name = 'pg_stat_statements';

-- Check if extension is created in database
SELECT * FROM pg_extension WHERE extname = 'pg_stat_statements';

-- Verify shared_preload_libraries
SHOW shared_preload_libraries;  -- Must include 'pg_stat_statements'

-- If missing, add to postgresql.conf and restart PostgreSQL
```

### No Data Being Tracked

**Problem:** `pg_stat_statements` view is empty

**Solution:**
```sql
-- Check tracking configuration
SHOW pg_stat_statements.track;  -- Should be 'all' or 'top'

-- Ensure max is high enough
SHOW pg_stat_statements.max;    -- Should be >1000

-- Run a test query and verify
SELECT NOW();
SELECT * FROM pg_stat_statements WHERE query LIKE '%NOW%';
```

### Performance Impact of Monitoring

**Concern:** Does `pg_stat_statements` slow down queries?

**Answer:**
- Overhead is typically <1% (negligible)
- Uses shared memory (configured by `pg_stat_statements.max`)
- If concerned, reduce `pg_stat_statements.max` to 5000 or lower

---

## Conclusion

`pg_stat_statements` is an essential tool for database performance analysis. Use this guide to:

1. **Identify** slow queries and optimization opportunities
2. **Monitor** index usage and cache efficiency
3. **Optimize** query patterns and database configuration
4. **Maintain** healthy database performance over time

**Next Steps:**
1. Enable `pg_stat_statements` in your environment
2. Run baseline analysis using queries from this guide
3. Establish regular monitoring cadence (daily/weekly/monthly)
4. Set up automated alerts for critical performance issues
5. Document optimizations and track improvements over time

**Reference:**
- PostgreSQL documentation: https://www.postgresql.org/docs/current/pgstatstatements.html
- Maproom performance targets: See ticket HYBRID_SEARCH-4002
- Migration files: `/workspace/crates/maproom/migrations/`

---

**Document Version:** 1.0
**Last Updated:** 2025-10-24
**Ticket:** HYBRID_SEARCH-4002
**Author:** database-engineer agent
