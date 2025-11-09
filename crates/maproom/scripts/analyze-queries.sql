-- analyze-queries.sql
-- Query performance analysis and optimization verification
--
-- Usage:
--   psql $MAPROOM_DATABASE_URL -f analyze-queries.sql
--
-- This script helps verify that query optimizations are working correctly
-- by analyzing actual query plans and performance characteristics.

\echo '================================================================================'
\echo '  Maproom Query Performance Analysis'
\echo '================================================================================'
\echo ''

-- Enable timing for all queries
\timing on

-- ==============================================================================
-- SECTION 1: Table and View Statistics
-- ==============================================================================

\echo '=== Table Statistics ==='
SELECT
  schemaname,
  tablename,
  n_live_tup AS live_rows,
  n_dead_tup AS dead_rows,
  ROUND(100.0 * n_dead_tup / NULLIF(n_live_tup, 0), 2) AS dead_ratio_pct,
  last_vacuum,
  last_autovacuum,
  last_analyze,
  last_autoanalyze
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
ORDER BY n_live_tup DESC;

\echo ''
\echo '=== Materialized View Statistics ==='
SELECT
  c.relname AS view_name,
  pg_size_pretty(pg_relation_size(c.oid)) AS size,
  pg_stat_get_tuples_inserted(c.oid) AS rows,
  pg_stat_get_last_analyze_time(c.oid) AS last_analyzed
FROM pg_class c
JOIN pg_namespace n ON n.oid = c.relnamespace
WHERE c.relkind = 'm'
  AND n.nspname = 'maproom'
ORDER BY pg_relation_size(c.oid) DESC;

-- ==============================================================================
-- SECTION 2: Index Usage Analysis
-- ==============================================================================

\echo ''
\echo '=== Index Usage (Top 20) ==='
SELECT
  schemaname,
  tablename,
  indexname,
  idx_scan AS scans,
  idx_tup_read AS tuples_read,
  idx_tup_fetch AS tuples_fetched,
  pg_size_pretty(pg_relation_size(indexrelid)) AS size
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
ORDER BY idx_scan DESC
LIMIT 20;

\echo ''
\echo '=== Unused Indices (Scans = 0) ==='
SELECT
  schemaname,
  tablename,
  indexname,
  pg_size_pretty(pg_relation_size(indexrelid)) AS wasted_size
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
  AND idx_scan = 0
  AND indexrelname NOT LIKE '%_pkey'  -- Exclude primary keys
ORDER BY pg_relation_size(indexrelid) DESC;

-- ==============================================================================
-- SECTION 3: Query Plan Analysis - Search Queries
-- ==============================================================================

\echo ''
\echo '=== EXPLAIN ANALYZE: Full-Text Search with Materialized View ==='
\echo 'This should use idx_search_view_fts (GIN index) and idx_search_view_repo'
\echo ''

EXPLAIN (ANALYZE, BUFFERS, VERBOSE, COSTS)
SELECT id, symbol_name, relpath, importance_score
FROM maproom.chunk_search_view
WHERE repo_id = (SELECT id FROM maproom.repos LIMIT 1)
  AND ts_doc @@ to_tsquery('simple', 'function & search')
ORDER BY importance_score DESC
LIMIT 20;

\echo ''
\echo '=== EXPLAIN ANALYZE: Vector Search with Materialized View ==='
\echo 'This should use idx_search_view_code_embedding (IVFFlat index)'
\echo ''

-- Generate a random embedding for testing (1536 dimensions)
WITH random_embedding AS (
  SELECT array_agg(random()::real) AS embedding
  FROM generate_series(1, 1536)
)
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, COSTS)
SELECT id, symbol_name, relpath
FROM maproom.chunk_search_view csv
CROSS JOIN random_embedding re
WHERE csv.repo_id = (SELECT id FROM maproom.repos LIMIT 1)
  AND csv.code_embedding IS NOT NULL
ORDER BY csv.code_embedding <=> re.embedding::vector
LIMIT 20;

-- ==============================================================================
-- SECTION 4: Query Plan Analysis - Context Assembly
-- ==============================================================================

\echo ''
\echo '=== EXPLAIN ANALYZE: Chunk Metadata Lookup ==='
\echo 'This should use chunks_pkey and avoid sequential scans'
\echo ''

EXPLAIN (ANALYZE, BUFFERS, VERBOSE, COSTS)
SELECT
  c.id,
  f.relpath,
  w.abs_path AS worktree_path,
  c.symbol_name,
  c.kind::text,
  c.start_line,
  c.end_line,
  c.signature,
  c.docstring
FROM maproom.chunks c
JOIN maproom.files f ON f.id = c.file_id
LEFT JOIN maproom.worktrees w ON w.id = f.worktree_id
WHERE c.id = (SELECT id FROM maproom.chunks ORDER BY random() LIMIT 1);

\echo ''
\echo '=== EXPLAIN ANALYZE: Graph Traversal with CTEs ==='
\echo 'This should use idx_chunk_edges_src_covering and idx_chunk_edges_dst_covering'
\echo ''

EXPLAIN (ANALYZE, BUFFERS, VERBOSE, COSTS)
WITH RECURSIVE related AS (
  -- Base case
  SELECT id, 0 as depth, 1.0 as relevance
  FROM maproom.chunks
  WHERE id = (SELECT id FROM maproom.chunks ORDER BY random() LIMIT 1)

  UNION ALL

  -- Forward traversal
  SELECT DISTINCT
    e.dst_chunk_id as id,
    r.depth + 1 as depth,
    r.relevance * 0.7 as relevance
  FROM related r
  JOIN maproom.chunk_edges e ON e.src_chunk_id = r.id
  WHERE r.depth < 3

  UNION ALL

  -- Backward traversal
  SELECT DISTINCT
    e.src_chunk_id as id,
    r.depth + 1 as depth,
    r.relevance * 0.7 as relevance
  FROM related r
  JOIN maproom.chunk_edges e ON e.dst_chunk_id = r.id
  WHERE r.depth < 3
)
SELECT DISTINCT
  c.id,
  f.relpath,
  c.symbol_name,
  c.kind::text,
  r.depth,
  r.relevance
FROM related r
JOIN maproom.chunks c ON c.id = r.id
JOIN maproom.files f ON f.id = c.file_id
ORDER BY r.relevance DESC, r.depth ASC
LIMIT 20;

-- ==============================================================================
-- SECTION 5: Query Plan Analysis - File Operations
-- ==============================================================================

\echo ''
\echo '=== EXPLAIN ANALYZE: File Metadata Lookup with Materialized View ==='
\echo 'This should use idx_file_metadata_repo_path'
\echo ''

EXPLAIN (ANALYZE, BUFFERS, VERBOSE, COSTS)
SELECT
  relpath,
  language,
  size_bytes,
  chunk_count,
  embedded_chunk_count,
  max_recency_score
FROM maproom.file_metadata_view
WHERE repo_id = (SELECT id FROM maproom.repos LIMIT 1)
  AND relpath LIKE 'src/%'
ORDER BY max_recency_score DESC
LIMIT 50;

-- ==============================================================================
-- SECTION 6: Query Plan Analysis - Graph Importance
-- ==============================================================================

\echo ''
\echo '=== EXPLAIN ANALYZE: Edge Count Aggregations with Materialized View ==='
\echo 'This should use idx_chunk_edge_counts_id (direct lookup)'
\echo ''

EXPLAIN (ANALYZE, BUFFERS, VERBOSE, COSTS)
SELECT
  c.id,
  c.symbol_name,
  ec.calls_in,
  ec.calls_out,
  ec.total_in,
  ec.total_out
FROM maproom.chunks c
JOIN maproom.chunk_edge_counts ec ON ec.chunk_id = c.id
WHERE ec.total_in > 5
ORDER BY ec.total_in DESC
LIMIT 20;

-- ==============================================================================
-- SECTION 7: Sequential Scan Detection
-- ==============================================================================

\echo ''
\echo '=== Sequential Scans on Large Tables (Potential Issues) ==='
SELECT
  schemaname,
  tablename,
  seq_scan AS sequential_scans,
  seq_tup_read AS tuples_read_seq,
  idx_scan AS index_scans,
  idx_tup_fetch AS tuples_read_idx,
  ROUND(100.0 * seq_scan / NULLIF(seq_scan + idx_scan, 0), 2) AS seq_scan_pct,
  n_live_tup AS rows
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
  AND n_live_tup > 1000  -- Only check tables with > 1000 rows
ORDER BY seq_scan DESC;

\echo ''
\echo 'Note: High sequential scan percentage on large tables may indicate missing indices'
\echo ''

-- ==============================================================================
-- SECTION 8: Buffer Cache Analysis
-- ==============================================================================

\echo ''
\echo '=== Buffer Cache Hit Ratio (should be > 99%) ==='
SELECT
  schemaname,
  tablename,
  heap_blks_read AS disk_reads,
  heap_blks_hit AS cache_hits,
  ROUND(
    100.0 * heap_blks_hit / NULLIF(heap_blks_hit + heap_blks_read, 0),
    2
  ) AS cache_hit_ratio_pct
FROM pg_statio_user_tables
WHERE schemaname = 'maproom'
  AND (heap_blks_hit + heap_blks_read) > 0
ORDER BY cache_hit_ratio_pct ASC;

-- ==============================================================================
-- SECTION 9: Query Performance Tracking (requires pg_stat_statements)
-- ==============================================================================

\echo ''
\echo '=== Top 10 Slowest Queries (requires pg_stat_statements extension) ==='
\echo 'Run: CREATE EXTENSION IF NOT EXISTS pg_stat_statements;'
\echo ''

DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements'
  ) THEN
    RAISE NOTICE 'pg_stat_statements is available, showing slow queries';
    PERFORM 1;
  ELSE
    RAISE NOTICE 'pg_stat_statements extension not installed';
    RAISE NOTICE 'Install with: CREATE EXTENSION pg_stat_statements;';
    RAISE NOTICE 'Then restart PostgreSQL';
  END IF;
END;
$$;

-- Uncomment if pg_stat_statements is installed:
/*
SELECT
  LEFT(query, 80) AS query_preview,
  calls,
  ROUND(total_exec_time::numeric / 1000, 2) AS total_time_sec,
  ROUND(mean_exec_time::numeric, 2) AS mean_time_ms,
  ROUND(max_exec_time::numeric, 2) AS max_time_ms,
  ROUND(stddev_exec_time::numeric, 2) AS stddev_time_ms
FROM pg_stat_statements
WHERE query LIKE '%maproom%'
  AND query NOT LIKE '%pg_stat_statements%'
ORDER BY mean_exec_time DESC
LIMIT 10;
*/

-- ==============================================================================
-- Summary and Recommendations
-- ==============================================================================

\echo ''
\echo '================================================================================'
\echo '  Analysis Complete'
\echo '================================================================================'
\echo ''
\echo 'Key Metrics to Monitor:'
\echo '  1. Index usage: All critical queries should use index scans'
\echo '  2. Sequential scans: Should be < 10% on large tables (chunks, files, edges)'
\echo '  3. Cache hit ratio: Should be > 99% for hot tables'
\echo '  4. Materialized view staleness: Refresh if stale (> 1 hour old)'
\echo ''
\echo 'Optimization Checklist:'
\echo '  ✓ Verify all queries use indices (no seq scans on large tables)'
\echo '  ✓ Check materialized views are fresh (SELECT * FROM maproom.view_staleness())'
\echo '  ✓ Monitor buffer cache hit ratio (> 99% is ideal)'
\echo '  ✓ Review unused indices (consider dropping if idx_scan = 0)'
\echo '  ✓ Update statistics regularly (ANALYZE after bulk operations)'
\echo ''
\echo 'Next Steps:'
\echo '  - If seq scans detected: Add missing indices or adjust queries'
\echo '  - If views stale: Run ./scripts/refresh-views.sql'
\echo '  - If cache hit ratio low: Increase shared_buffers or investigate disk I/O'
\echo '  - If slow queries: Enable pg_stat_statements and analyze query plans'
\echo ''

\timing off
