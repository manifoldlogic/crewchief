-- Maproom Database Query Performance Analysis
--
-- This script analyzes query performance, index usage, and identifies bottlenecks.
--
-- Prerequisites:
--   - PostgreSQL with pg_stat_statements extension enabled
--   - Run: CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
--
-- Usage:
--   psql $DATABASE_URL -f scripts/analyze-queries.sql
--
-- Sections:
--   1. Slow Queries - Find queries exceeding latency targets
--   2. Index Usage - Verify indexes are being used effectively
--   3. Sequential Scans - Identify missing indexes
--   4. Table Statistics - Check if statistics are up-to-date
--   5. Query Plans - Analyze critical query execution plans

\echo '=== Maproom Database Performance Analysis ==='
\echo ''

-- 1. SLOW QUERIES
\echo '1. SLOW QUERIES (mean execution time > 50ms)'
\echo '   Target: p95 < 50ms for search queries'
\echo ''

SELECT
    substring(query, 1, 80) as query_snippet,
    calls,
    round(mean_exec_time::numeric, 2) as mean_ms,
    round(max_exec_time::numeric, 2) as max_ms,
    round(stddev_exec_time::numeric, 2) as stddev_ms,
    round((mean_exec_time * calls)::numeric, 2) as total_ms
FROM pg_stat_statements
WHERE query LIKE '%maproom.%'
  AND mean_exec_time > 50
ORDER BY mean_exec_time DESC
LIMIT 20;

\echo ''
\echo '2. MOST FREQUENTLY CALLED QUERIES'
\echo '   Optimization impact: High call frequency × execution time'
\echo ''

SELECT
    substring(query, 1, 80) as query_snippet,
    calls,
    round(mean_exec_time::numeric, 2) as mean_ms,
    round((mean_exec_time * calls)::numeric, 2) as total_time_ms,
    round((100.0 * mean_exec_time * calls / sum(mean_exec_time * calls) OVER ())::numeric, 2) as pct_total
FROM pg_stat_statements
WHERE query LIKE '%maproom.%'
  AND calls > 10
ORDER BY (mean_exec_time * calls) DESC
LIMIT 20;

\echo ''
\echo '3. INDEX USAGE - maproom schema'
\echo '   Goal: All indexes should have idx_scan > 0'
\echo ''

SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan as scans,
    idx_tup_read as tuples_read,
    idx_tup_fetch as tuples_fetched,
    CASE
        WHEN idx_scan = 0 THEN '⚠️  UNUSED'
        WHEN idx_scan < 100 THEN '⚡ LOW'
        ELSE '✅ OK'
    END as status
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
ORDER BY idx_scan DESC;

\echo ''
\echo '4. SEQUENTIAL SCANS (should be minimal for large tables)'
\echo '   Warning: High seq_scan on tables with >10k rows indicates missing indexes'
\echo ''

SELECT
    schemaname,
    tablename,
    n_live_tup as rows,
    seq_scan as sequential_scans,
    idx_scan as index_scans,
    CASE
        WHEN seq_scan > idx_scan AND n_live_tup > 10000 THEN '⚠️  PROBLEM'
        WHEN seq_scan > 100 AND n_live_tup > 1000 THEN '⚡ WARNING'
        ELSE '✅ OK'
    END as status,
    round((100.0 * seq_scan / NULLIF(seq_scan + idx_scan, 0))::numeric, 2) as seq_scan_pct
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
ORDER BY seq_scan DESC;

\echo ''
\echo '5. TABLE STATISTICS'
\echo '   Check: Last analyze/vacuum times (should be recent)'
\echo ''

SELECT
    schemaname,
    tablename,
    n_live_tup as live_rows,
    n_dead_tup as dead_rows,
    last_vacuum,
    last_autovacuum,
    last_analyze,
    last_autoanalyze,
    CASE
        WHEN last_analyze IS NULL AND last_autoanalyze IS NULL THEN '⚠️  NEVER'
        WHEN COALESCE(last_analyze, last_autoanalyze) < NOW() - INTERVAL '7 days' THEN '⚡ STALE'
        ELSE '✅ OK'
    END as stats_status
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
ORDER BY n_live_tup DESC;

\echo ''
\echo '6. CACHE HIT RATIO'
\echo '   Target: >99% cache hit ratio for optimal performance'
\echo ''

SELECT
    'Index' as cache_type,
    sum(idx_blks_hit) as hits,
    sum(idx_blks_read) as reads,
    round((100.0 * sum(idx_blks_hit) / NULLIF(sum(idx_blks_hit + idx_blks_read), 0))::numeric, 2) as hit_ratio_pct
FROM pg_statio_user_indexes
WHERE schemaname = 'maproom'
UNION ALL
SELECT
    'Table' as cache_type,
    sum(heap_blks_hit) as hits,
    sum(heap_blks_read) as reads,
    round((100.0 * sum(heap_blks_hit) / NULLIF(sum(heap_blks_hit + heap_blks_read), 0))::numeric, 2) as hit_ratio_pct
FROM pg_statio_user_tables
WHERE schemaname = 'maproom';

\echo ''
\echo '=== DETAILED QUERY PLAN ANALYSIS ==='
\echo ''
\echo 'Run EXPLAIN ANALYZE on critical queries to identify bottlenecks:'
\echo ''

-- 7. CHUNK SEARCH QUERY (Vector similarity)
\echo '7. Vector Similarity Search Query Plan'
\echo '   Target: <30ms execution time'
\echo ''

EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT c.id, c.symbol_name, c.kind::text, c.start_line, c.end_line,
       c.embedding <=> $1::vector as distance
FROM maproom.chunks c
JOIN maproom.files f ON f.id = c.file_id
WHERE f.repo_id = 1
  AND c.embedding IS NOT NULL
ORDER BY c.embedding <=> $1::vector
LIMIT 10;

\echo ''
\echo '8. Full-Text Search Query Plan'
\echo '   Target: <20ms execution time'
\echo ''

EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT c.id, c.symbol_name, c.kind::text,
       ts_rank_cd(c.content_tsv, query) as rank
FROM maproom.chunks c
JOIN maproom.files f ON f.id = c.file_id,
     to_tsquery('english', 'authentication & user') as query
WHERE f.repo_id = 1
  AND c.content_tsv @@ query
ORDER BY rank DESC
LIMIT 10;

\echo ''
\echo '9. Graph Traversal Query Plan'
\echo '   Target: <15ms execution time'
\echo ''

EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT c.id, c.symbol_name, COUNT(ce.id) as edge_count
FROM maproom.chunks c
JOIN maproom.files f ON f.id = c.file_id
LEFT JOIN maproom.chunk_edges ce ON ce.src_chunk_id = c.id
WHERE f.repo_id = 1
GROUP BY c.id, c.symbol_name
ORDER BY edge_count DESC
LIMIT 10;

\echo ''
\echo '10. Context Assembly Metadata Query Plan'
\echo '    Target: <5ms execution time'
\echo ''

EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT c.id, f.relpath, w.abs_path as worktree_path,
       c.symbol_name, c.kind::text, c.start_line, c.end_line,
       c.signature, c.docstring
FROM maproom.chunks c
JOIN maproom.files f ON f.id = c.file_id
LEFT JOIN maproom.worktrees w ON w.id = f.worktree_id
WHERE c.id = 12345;

\echo ''
\echo '=== OPTIMIZATION RECOMMENDATIONS ==='
\echo ''

SELECT
    CASE
        WHEN EXISTS (
            SELECT 1 FROM pg_stat_user_indexes
            WHERE schemaname = 'maproom' AND idx_scan = 0
        ) THEN '⚠️  Remove unused indexes to reduce write overhead'
        ELSE '✅ All indexes are being used'
    END as unused_indexes,

    CASE
        WHEN EXISTS (
            SELECT 1 FROM pg_stat_user_tables
            WHERE schemaname = 'maproom'
              AND seq_scan > idx_scan
              AND n_live_tup > 10000
        ) THEN '⚠️  Add indexes to eliminate sequential scans on large tables'
        ELSE '✅ Sequential scans are reasonable'
    END as sequential_scans,

    CASE
        WHEN (
            SELECT round((100.0 * sum(heap_blks_hit) / NULLIF(sum(heap_blks_hit + heap_blks_read), 0))::numeric, 2)
            FROM pg_statio_user_tables
            WHERE schemaname = 'maproom'
        ) < 99 THEN '⚠️  Increase shared_buffers to improve cache hit ratio'
        ELSE '✅ Cache hit ratio is excellent'
    END as cache_tuning,

    CASE
        WHEN EXISTS (
            SELECT 1 FROM pg_stat_user_tables
            WHERE schemaname = 'maproom'
              AND (last_analyze IS NULL OR last_analyze < NOW() - INTERVAL '7 days')
        ) THEN '⚠️  Run ANALYZE on stale tables to update query planner statistics'
        ELSE '✅ Table statistics are up-to-date'
    END as statistics_freshness;

\echo ''
\echo 'Analysis complete. Review the output above for optimization opportunities.'
\echo ''
\echo 'Key Performance Targets:'
\echo '  - Search p95 latency: <50ms'
\echo '  - Context assembly p95: <120ms'
\echo '  - Cache hit ratio: >99%'
\echo '  - Index usage: All indexes should have scans > 0'
\echo ''
