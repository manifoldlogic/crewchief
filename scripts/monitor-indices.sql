-- monitor-indices.sql
-- Real-time index monitoring for Maproom
-- Part of PERF_OPT-2001: Index Optimization
--
-- Usage:
--   psql -d maproom -f scripts/monitor-indices.sql
--   or for continuous monitoring: watch -n 5 "psql -d maproom -f scripts/monitor-indices.sql"
--
-- This script provides:
-- 1. Real-time index usage statistics
-- 2. Cache hit ratios
-- 3. Query performance metrics
-- 4. Index health indicators
-- 5. Actionable recommendations

\echo '================================================================================'
\echo 'MAPROOM INDEX MONITORING DASHBOARD'
\echo '================================================================================'
\echo ''

-- ==============================================================================
-- SECTION 1: System Overview
-- ==============================================================================

\echo '1. SYSTEM OVERVIEW'
\echo '-----------------'
\echo ''

SELECT
  NOW() as report_timestamp,
  pg_size_pretty(pg_database_size(current_database())) as database_size,
  (SELECT COUNT(*) FROM pg_stat_activity WHERE state = 'active') as active_connections,
  (SELECT COUNT(*) FROM pg_stat_activity) as total_connections,
  version() as postgres_version;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 2: Index Cache Hit Ratio
-- ==============================================================================

\echo '2. CACHE HIT RATIOS'
\echo '------------------'
\echo 'Cache hit ratio should be >90% for good performance'
\echo ''

-- Overall cache hit ratio
SELECT
  'Overall Cache Hit Ratio' as metric,
  ROUND(
    (SUM(idx_blks_hit)::numeric / NULLIF(SUM(idx_blks_hit + idx_blks_read), 0) * 100), 2
  ) as percentage,
  CASE
    WHEN (SUM(idx_blks_hit)::numeric / NULLIF(SUM(idx_blks_hit + idx_blks_read), 0) * 100) >= 95 THEN '✓✓ Excellent'
    WHEN (SUM(idx_blks_hit)::numeric / NULLIF(SUM(idx_blks_hit + idx_blks_read), 0) * 100) >= 90 THEN '✓ Good'
    WHEN (SUM(idx_blks_hit)::numeric / NULLIF(SUM(idx_blks_hit + idx_blks_read), 0) * 100) >= 80 THEN '⚠️  Fair'
    ELSE '⚠️  Poor - consider increasing shared_buffers'
  END as status
FROM pg_statio_user_indexes
WHERE schemaname = 'maproom';

\echo ''

-- Per-index cache hit ratio
SELECT
  indexname,
  idx_blks_read as disk_reads,
  idx_blks_hit as cache_hits,
  CASE
    WHEN (idx_blks_hit + idx_blks_read) > 0 THEN
      ROUND((idx_blks_hit::numeric / (idx_blks_hit + idx_blks_read) * 100), 2)
    ELSE NULL
  END as cache_hit_pct,
  CASE
    WHEN (idx_blks_hit + idx_blks_read) = 0 THEN 'No activity'
    WHEN (idx_blks_hit::numeric / NULLIF(idx_blks_hit + idx_blks_read, 0) * 100) >= 95 THEN '✓✓ Excellent'
    WHEN (idx_blks_hit::numeric / NULLIF(idx_blks_hit + idx_blks_read, 0) * 100) >= 90 THEN '✓ Good'
    WHEN (idx_blks_hit::numeric / NULLIF(idx_blks_hit + idx_blks_read, 0) * 100) >= 80 THEN '⚠️  Fair'
    ELSE '⚠️  Poor'
  END as status
FROM pg_statio_user_indexes
WHERE schemaname = 'maproom'
ORDER BY (idx_blks_hit + idx_blks_read) DESC
LIMIT 15;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 3: Most Active Indices
-- ==============================================================================

\echo '3. MOST ACTIVE INDICES (Last 24h)'
\echo '---------------------------------'
\echo 'Indices ranked by scan frequency'
\echo ''

SELECT
  tablename,
  indexname,
  idx_scan as scans,
  idx_tup_read as tuples_read,
  idx_tup_fetch as tuples_fetched,
  CASE
    WHEN idx_scan > 0 THEN ROUND((idx_tup_read::numeric / idx_scan), 1)
    ELSE 0
  END as avg_tuples_per_scan,
  pg_size_pretty(pg_relation_size(indexrelid)) as size
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
ORDER BY idx_scan DESC
LIMIT 10;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 4: Index Scan Efficiency
-- ==============================================================================

\echo '4. INDEX SCAN EFFICIENCY'
\echo '-----------------------'
\echo 'Ratio of index usage vs sequential scans per table'
\echo ''

SELECT
  tablename,
  seq_scan as seq_scans,
  idx_scan as idx_scans,
  CASE
    WHEN (seq_scan + idx_scan) > 0 THEN
      ROUND((idx_scan::numeric / (seq_scan + idx_scan) * 100), 2)
    ELSE 0
  END as index_usage_pct,
  n_live_tup as live_rows,
  CASE
    WHEN (seq_scan + idx_scan) = 0 THEN 'No activity'
    WHEN (idx_scan::numeric / NULLIF(seq_scan + idx_scan, 0) * 100) >= 95 THEN '✓✓ Excellent'
    WHEN (idx_scan::numeric / NULLIF(seq_scan + idx_scan, 0) * 100) >= 80 THEN '✓ Good'
    WHEN (idx_scan::numeric / NULLIF(seq_scan + idx_scan, 0) * 100) >= 60 THEN '⚠️  Fair'
    ELSE '⚠️  Poor - consider adding indices'
  END as status
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
ORDER BY n_live_tup DESC;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 5: Recent Query Activity
-- ==============================================================================

\echo '5. RECENT QUERY ACTIVITY'
\echo '-----------------------'
\echo 'Currently running queries (if any)'
\echo ''

SELECT
  pid,
  NOW() - query_start as query_duration,
  state,
  LEFT(query, 80) as query_preview
FROM pg_stat_activity
WHERE datname = current_database()
  AND state != 'idle'
  AND query NOT LIKE '%pg_stat_activity%'
ORDER BY query_start;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 6: Index Growth Tracking
-- ==============================================================================

\echo '6. LARGEST INDICES'
\echo '-----------------'
\echo 'Indices sorted by size (watch for unexpected growth)'
\echo ''

SELECT
  tablename,
  indexname,
  pg_size_pretty(pg_relation_size(indexrelid)) as size,
  pg_size_pretty(pg_total_relation_size(indexrelid)) as total_size_with_toast,
  idx_scan as times_used,
  CASE
    WHEN idx_scan = 0 THEN '⚠️  UNUSED'
    WHEN pg_relation_size(indexrelid) > 104857600 AND idx_scan < 100 THEN '⚠️  Large but rarely used'
    ELSE '✓ OK'
  END as status
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
ORDER BY pg_relation_size(indexrelid) DESC
LIMIT 10;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 7: Dead Tuples and Bloat
-- ==============================================================================

\echo '7. TABLE HEALTH (Dead Tuples)'
\echo '-----------------------------'
\echo 'High dead tuple count indicates need for VACUUM'
\echo ''

SELECT
  tablename,
  n_live_tup as live_rows,
  n_dead_tup as dead_rows,
  ROUND((n_dead_tup::numeric / NULLIF(n_live_tup, 0) * 100), 2) as dead_row_pct,
  last_vacuum,
  last_autovacuum,
  CASE
    WHEN n_dead_tup = 0 THEN '✓ Clean'
    WHEN (n_dead_tup::numeric / NULLIF(n_live_tup, 0)) > 0.2 THEN '⚠️  High bloat - VACUUM needed'
    WHEN (n_dead_tup::numeric / NULLIF(n_live_tup, 0)) > 0.1 THEN '⚠️  Moderate bloat'
    ELSE '✓ Low bloat'
  END as status
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
ORDER BY n_dead_tup DESC;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 8: Index vs Table Size Ratio
-- ==============================================================================

\echo '8. INDEX TO TABLE SIZE RATIO'
\echo '---------------------------'
\echo 'Healthy ratio: 20-50% for transactional tables, 50-100% for search-heavy tables'
\echo ''

SELECT
  t.tablename,
  pg_size_pretty(pg_relation_size(t.schemaname||'.'||t.tablename)) as table_size,
  pg_size_pretty(SUM(pg_relation_size(i.indexrelid))) as index_size,
  ROUND(
    (SUM(pg_relation_size(i.indexrelid))::numeric /
     NULLIF(pg_relation_size(t.schemaname||'.'||t.tablename), 0) * 100), 2
  ) as ratio_pct,
  CASE
    WHEN (SUM(pg_relation_size(i.indexrelid))::numeric /
          NULLIF(pg_relation_size(t.schemaname||'.'||t.tablename), 0) * 100) > 150 THEN '⚠️  Very high - review indices'
    WHEN (SUM(pg_relation_size(i.indexrelid))::numeric /
          NULLIF(pg_relation_size(t.schemaname||'.'||t.tablename), 0) * 100) > 100 THEN '⚠️  High'
    WHEN (SUM(pg_relation_size(i.indexrelid))::numeric /
          NULLIF(pg_relation_size(t.schemaname||'.'||t.tablename), 0) * 100) > 50 THEN '✓ Good for search-heavy'
    WHEN (SUM(pg_relation_size(i.indexrelid))::numeric /
          NULLIF(pg_relation_size(t.schemaname||'.'||t.tablename), 0) * 100) > 20 THEN '✓ Good for transactional'
    ELSE '✓ Low'
  END as status
FROM pg_stat_user_tables t
JOIN pg_stat_user_indexes i ON i.schemaname = t.schemaname AND i.tablename = t.tablename
WHERE t.schemaname = 'maproom'
GROUP BY t.schemaname, t.tablename
ORDER BY pg_relation_size(t.schemaname||'.'||t.tablename) DESC;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 9: Index Operations Performance
-- ==============================================================================

\echo '9. INDEX OPERATION STATS'
\echo '-----------------------'
\echo ''

SELECT
  'Total Index Scans' as metric,
  SUM(idx_scan)::text as value
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
UNION ALL
SELECT
  'Total Tuples Read via Index' as metric,
  SUM(idx_tup_read)::text as value
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
UNION ALL
SELECT
  'Total Tuples Fetched via Index' as metric,
  SUM(idx_tup_fetch)::text as value
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
UNION ALL
SELECT
  'Total Sequential Scans' as metric,
  SUM(seq_scan)::text as value
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
UNION ALL
SELECT
  'Total Sequential Tuples Read' as metric,
  SUM(seq_tup_read)::text as value
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
UNION ALL
SELECT
  'Index Scan Efficiency' as metric,
  ROUND(
    (SUM(idx_scan)::numeric /
     NULLIF(SUM(idx_scan) + SUM(seq_scan), 0) * 100), 2
  )::text || '%' as value
FROM (
  SELECT SUM(idx_scan) as idx_scan FROM pg_stat_user_indexes WHERE schemaname = 'maproom'
) i,
(
  SELECT SUM(seq_scan) as seq_scan FROM pg_stat_user_tables WHERE schemaname = 'maproom'
) s;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 10: Real-Time Recommendations
-- ==============================================================================

\echo '10. REAL-TIME RECOMMENDATIONS'
\echo '----------------------------'
\echo ''

-- Check cache hit ratio
DO $$
DECLARE
  hit_ratio numeric;
BEGIN
  SELECT ROUND(
    (SUM(idx_blks_hit)::numeric / NULLIF(SUM(idx_blks_hit + idx_blks_read), 0) * 100), 2
  ) INTO hit_ratio
  FROM pg_statio_user_indexes
  WHERE schemaname = 'maproom';

  IF hit_ratio < 90 THEN
    RAISE NOTICE '⚠️  Cache hit ratio is %% (target: >90%%)', hit_ratio;
    RAISE NOTICE '   Action: Consider increasing shared_buffers in postgresql.conf';
  ELSIF hit_ratio < 95 THEN
    RAISE NOTICE '✓ Cache hit ratio is %% (good, target: >95%% for optimal)', hit_ratio;
  ELSE
    RAISE NOTICE '✓✓ Cache hit ratio is %% (excellent)', hit_ratio;
  END IF;
END $$;

-- Check for indices needing VACUUM
DO $$
DECLARE
  bloat_count int;
BEGIN
  SELECT COUNT(*) INTO bloat_count
  FROM pg_stat_user_tables
  WHERE schemaname = 'maproom'
    AND n_dead_tup > 1000
    AND n_dead_tup::numeric / NULLIF(n_live_tup, 0) > 0.15;

  IF bloat_count > 0 THEN
    RAISE NOTICE '⚠️  % table(s) have significant bloat (>15%% dead tuples)', bloat_count;
    RAISE NOTICE '   Action: Run VACUUM ANALYZE maproom.<tablename>; or wait for autovacuum';
  ELSE
    RAISE NOTICE '✓ All tables have acceptable bloat levels';
  END IF;
END $$;

-- Check for stale statistics
DO $$
DECLARE
  stale_count int;
BEGIN
  SELECT COUNT(*) INTO stale_count
  FROM pg_stat_user_tables
  WHERE schemaname = 'maproom'
    AND n_live_tup > 1000
    AND (last_analyze IS NULL OR last_analyze < NOW() - INTERVAL '7 days')
    AND (last_autoanalyze IS NULL OR last_autoanalyze < NOW() - INTERVAL '7 days');

  IF stale_count > 0 THEN
    RAISE NOTICE '⚠️  % table(s) have stale statistics (>7 days)', stale_count;
    RAISE NOTICE '   Action: Run ANALYZE maproom.<tablename>;';
  ELSE
    RAISE NOTICE '✓ Statistics are fresh';
  END IF;
END $$;

-- Check for unused indices
DO $$
DECLARE
  unused_count int;
BEGIN
  SELECT COUNT(*) INTO unused_count
  FROM pg_stat_user_indexes
  WHERE schemaname = 'maproom'
    AND idx_scan = 0
    AND indexrelname NOT LIKE '%_pkey'
    AND pg_relation_size(indexrelid) > 1048576;  -- > 1MB

  IF unused_count > 0 THEN
    RAISE NOTICE '⚠️  % unused index(es) found (size >1MB)', unused_count;
    RAISE NOTICE '   Action: Review with scripts/analyze-indices.sql and consider dropping';
  ELSE
    RAISE NOTICE '✓ All significant indices are being used';
  END IF;
END $$;

\echo ''
\echo '================================================================================'
\echo 'MONITORING COMPLETE'
\echo '================================================================================'
\echo ''
\echo 'For continuous monitoring, run:'
\echo '  watch -n 5 "psql -d maproom -f scripts/monitor-indices.sql"'
\echo ''
\echo 'For detailed analysis, run:'
\echo '  psql -d maproom -f scripts/analyze-indices.sql'
\echo ''
\echo 'To profile specific queries, use:'
\echo '  EXPLAIN (ANALYZE, BUFFERS, VERBOSE) <your_query>;'
\echo ''
