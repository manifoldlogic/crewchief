-- analyze-indices.sql
-- Comprehensive index effectiveness analysis for Maproom
-- Part of PERF_OPT-2001: Index Optimization
--
-- Usage:
--   psql -d maproom -f scripts/analyze-indices.sql
--   or from psql: \i scripts/analyze-indices.sql
--
-- This script analyzes:
-- 1. Index usage statistics
-- 2. Index size and bloat
-- 3. Query performance with/without indices
-- 4. Missing indices and unused indices
-- 5. Index health metrics

\echo '================================================================================'
\echo 'MAPROOM INDEX ANALYSIS REPORT'
\echo '================================================================================'
\echo ''

-- ==============================================================================
-- SECTION 1: Index Usage Statistics
-- ==============================================================================

\echo '1. INDEX USAGE STATISTICS'
\echo '------------------------'
\echo 'Shows how frequently each index is used and its effectiveness'
\echo ''

SELECT
  schemaname,
  tablename,
  indexname,
  idx_scan as times_used,
  idx_tup_read as tuples_read,
  idx_tup_fetch as tuples_fetched,
  pg_size_pretty(pg_relation_size(indexrelid)) as index_size,
  CASE
    WHEN idx_scan = 0 THEN '⚠️  UNUSED'
    WHEN idx_scan < 100 THEN '⚠️  Low usage'
    WHEN idx_scan < 1000 THEN '✓ Moderate usage'
    ELSE '✓✓ High usage'
  END as usage_status
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
ORDER BY idx_scan DESC, pg_relation_size(indexrelid) DESC;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 2: Index Size and Efficiency
-- ==============================================================================

\echo '2. INDEX SIZE AND EFFICIENCY'
\echo '---------------------------'
\echo 'Shows index sizes and their relationship to table sizes'
\echo ''

SELECT
  t.schemaname,
  t.tablename,
  COUNT(i.indexname) as index_count,
  pg_size_pretty(pg_relation_size(t.schemaname||'.'||t.tablename)) as table_size,
  pg_size_pretty(SUM(pg_relation_size(i.indexrelid))) as total_index_size,
  ROUND(
    (SUM(pg_relation_size(i.indexrelid))::numeric /
     NULLIF(pg_relation_size(t.schemaname||'.'||t.tablename), 0) * 100), 2
  ) as index_to_table_ratio_pct
FROM pg_stat_user_tables t
JOIN pg_stat_user_indexes i ON i.schemaname = t.schemaname AND i.tablename = t.tablename
WHERE t.schemaname = 'maproom'
GROUP BY t.schemaname, t.tablename
ORDER BY pg_relation_size(t.schemaname||'.'||t.tablename) DESC;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 3: Individual Index Details
-- ==============================================================================

\echo '3. DETAILED INDEX INFORMATION'
\echo '----------------------------'
\echo 'Comprehensive details for each index'
\echo ''

SELECT
  i.schemaname,
  i.tablename,
  i.indexname,
  pg_size_pretty(pg_relation_size(i.indexrelid)) as size,
  i.idx_scan as scans,
  i.idx_tup_read as tuples_read,
  i.idx_tup_fetch as tuples_fetched,
  CASE
    WHEN i.idx_scan > 0 THEN ROUND((i.idx_tup_read::numeric / i.idx_scan), 2)
    ELSE 0
  END as avg_tuples_per_scan,
  pg_index.indisunique as is_unique,
  pg_index.indisprimary as is_primary,
  pg_index.indisvalid as is_valid,
  pg_get_indexdef(i.indexrelid) as definition
FROM pg_stat_user_indexes i
JOIN pg_index ON pg_index.indexrelid = i.indexrelid
WHERE i.schemaname = 'maproom'
ORDER BY pg_relation_size(i.indexrelid) DESC;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 4: Table Statistics (Related to Indexing)
-- ==============================================================================

\echo '4. TABLE STATISTICS'
\echo '------------------'
\echo 'Table-level statistics that affect index performance'
\echo ''

SELECT
  schemaname,
  tablename,
  n_live_tup as live_rows,
  n_dead_tup as dead_rows,
  ROUND((n_dead_tup::numeric / NULLIF(n_live_tup, 0) * 100), 2) as dead_row_pct,
  seq_scan as sequential_scans,
  seq_tup_read as seq_tuples_read,
  idx_scan as index_scans,
  idx_tup_fetch as idx_tuples_fetched,
  CASE
    WHEN idx_scan > 0 AND seq_scan > 0 THEN
      ROUND((idx_scan::numeric / (idx_scan + seq_scan) * 100), 2)
    WHEN idx_scan > 0 THEN 100.0
    ELSE 0.0
  END as index_usage_pct,
  last_vacuum,
  last_autovacuum,
  last_analyze,
  last_autoanalyze
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
ORDER BY n_live_tup DESC;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 5: Unused Indices
-- ==============================================================================

\echo '5. UNUSED INDICES'
\echo '----------------'
\echo 'Indices that have never been used (candidates for removal)'
\echo ''

SELECT
  schemaname,
  tablename,
  indexname,
  pg_size_pretty(pg_relation_size(indexrelid)) as wasted_size,
  pg_get_indexdef(indexrelid) as definition
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
  AND idx_scan = 0
  AND indexrelname NOT LIKE '%_pkey'  -- Exclude primary keys
ORDER BY pg_relation_size(indexrelid) DESC;

\echo ''
\echo 'Note: Primary keys and unique constraints are excluded from this list'
\echo 'even if unused, as they enforce data integrity.'
\echo ''
\echo ''

-- ==============================================================================
-- SECTION 6: Missing Indices (High Sequential Scan Activity)
-- ==============================================================================

\echo '6. POTENTIAL MISSING INDICES'
\echo '---------------------------'
\echo 'Tables with high sequential scan activity may benefit from additional indices'
\echo ''

SELECT
  schemaname,
  tablename,
  seq_scan as sequential_scans,
  seq_tup_read as seq_tuples_read,
  idx_scan as index_scans,
  n_live_tup as live_rows,
  CASE
    WHEN seq_scan > 0 THEN ROUND((seq_tup_read::numeric / seq_scan), 0)
    ELSE 0
  END as avg_seq_read,
  CASE
    WHEN seq_scan > 100 AND n_live_tup > 1000 AND seq_tup_read > idx_tup_fetch THEN '⚠️  Consider adding index'
    WHEN seq_scan > 1000 THEN '⚠️  High seq scan activity'
    ELSE '✓ OK'
  END as recommendation
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
  AND seq_scan > 0
ORDER BY seq_tup_read DESC;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 7: Index Bloat Estimation
-- ==============================================================================

\echo '7. INDEX BLOAT ESTIMATION'
\echo '------------------------'
\echo 'Estimates index bloat (excess space from dead tuples)'
\echo ''

SELECT
  schemaname,
  tablename,
  indexname,
  pg_size_pretty(pg_relation_size(indexrelid)) as index_size,
  CASE
    WHEN pg_relation_size(indexrelid) > 10485760 THEN  -- > 10MB
      CASE
        WHEN (SELECT n_dead_tup::numeric / NULLIF(n_live_tup, 0)
              FROM pg_stat_user_tables
              WHERE schemaname = pg_stat_user_indexes.schemaname
              AND tablename = pg_stat_user_indexes.tablename) > 0.2
        THEN '⚠️  High bloat (>20% dead tuples) - consider REINDEX'
        WHEN (SELECT n_dead_tup::numeric / NULLIF(n_live_tup, 0)
              FROM pg_stat_user_tables
              WHERE schemaname = pg_stat_user_indexes.schemaname
              AND tablename = pg_stat_user_indexes.tablename) > 0.1
        THEN '⚠️  Moderate bloat (>10% dead tuples)'
        ELSE '✓ Low bloat'
      END
    ELSE '✓ Small index (bloat not significant)'
  END as bloat_status,
  (SELECT ROUND((n_dead_tup::numeric / NULLIF(n_live_tup, 0) * 100), 2)
   FROM pg_stat_user_tables
   WHERE schemaname = pg_stat_user_indexes.schemaname
   AND tablename = pg_stat_user_indexes.tablename) as table_dead_row_pct
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
ORDER BY pg_relation_size(indexrelid) DESC;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 8: Index Type Distribution
-- ==============================================================================

\echo '8. INDEX TYPE DISTRIBUTION'
\echo '-------------------------'
\echo 'Shows the types of indices used in the schema'
\echo ''

SELECT
  am.amname as index_type,
  COUNT(*) as count,
  pg_size_pretty(SUM(pg_relation_size(i.indexrelid))) as total_size,
  ROUND(AVG(pg_relation_size(i.indexrelid))::numeric / 1024 / 1024, 2) as avg_size_mb
FROM pg_stat_user_indexes i
JOIN pg_class c ON c.oid = i.indexrelid
JOIN pg_am am ON am.oid = c.relam
WHERE i.schemaname = 'maproom'
GROUP BY am.amname
ORDER BY COUNT(*) DESC;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 9: Index Performance Metrics
-- ==============================================================================

\echo '9. INDEX PERFORMANCE METRICS'
\echo '---------------------------'
\echo 'Key performance indicators for index effectiveness'
\echo ''

SELECT
  'Total Tables' as metric,
  COUNT(DISTINCT tablename)::text as value
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
UNION ALL
SELECT
  'Total Indices' as metric,
  COUNT(*)::text as value
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
UNION ALL
SELECT
  'Unused Indices' as metric,
  COUNT(*)::text as value
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom' AND idx_scan = 0 AND indexrelname NOT LIKE '%_pkey'
UNION ALL
SELECT
  'Total Index Size' as metric,
  pg_size_pretty(SUM(pg_relation_size(indexrelid))) as value
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
UNION ALL
SELECT
  'Total Table Size' as metric,
  pg_size_pretty(SUM(pg_relation_size(schemaname||'.'||tablename))) as value
FROM (SELECT DISTINCT schemaname, tablename FROM pg_stat_user_indexes WHERE schemaname = 'maproom') t
UNION ALL
SELECT
  'Index to Table Ratio' as metric,
  ROUND(
    (SELECT SUM(pg_relation_size(indexrelid))::numeric
     FROM pg_stat_user_indexes WHERE schemaname = 'maproom') /
    NULLIF((SELECT SUM(pg_relation_size(schemaname||'.'||tablename))::numeric
            FROM (SELECT DISTINCT schemaname, tablename FROM pg_stat_user_indexes WHERE schemaname = 'maproom') t), 0) * 100, 2
  )::text || '%' as value;

\echo ''
\echo ''

-- ==============================================================================
-- SECTION 10: Recommendations
-- ==============================================================================

\echo '10. RECOMMENDATIONS'
\echo '------------------'
\echo ''

-- Check for high dead tuple ratio
DO $$
DECLARE
  high_bloat_count int;
BEGIN
  SELECT COUNT(*) INTO high_bloat_count
  FROM pg_stat_user_tables
  WHERE schemaname = 'maproom'
    AND n_dead_tup > 1000
    AND n_dead_tup::numeric / NULLIF(n_live_tup, 0) > 0.2;

  IF high_bloat_count > 0 THEN
    RAISE NOTICE '⚠️  Found % table(s) with high bloat (>20%% dead tuples)', high_bloat_count;
    RAISE NOTICE '   Recommendation: Run VACUUM ANALYZE or consider REINDEX CONCURRENTLY';
  ELSE
    RAISE NOTICE '✓ Table bloat is within acceptable limits';
  END IF;
END $$;

-- Check for unused indices
DO $$
DECLARE
  unused_count int;
  unused_size bigint;
BEGIN
  SELECT COUNT(*), COALESCE(SUM(pg_relation_size(indexrelid)), 0)
  INTO unused_count, unused_size
  FROM pg_stat_user_indexes
  WHERE schemaname = 'maproom'
    AND idx_scan = 0
    AND indexrelname NOT LIKE '%_pkey';

  IF unused_count > 0 THEN
    RAISE NOTICE '⚠️  Found % unused index(es) wasting %', unused_count, pg_size_pretty(unused_size);
    RAISE NOTICE '   Recommendation: Review and consider dropping unused indices';
  ELSE
    RAISE NOTICE '✓ All indices are being used';
  END IF;
END $$;

-- Check for high sequential scan activity
DO $$
DECLARE
  high_seq_count int;
BEGIN
  SELECT COUNT(*) INTO high_seq_count
  FROM pg_stat_user_tables
  WHERE schemaname = 'maproom'
    AND seq_scan > 100
    AND n_live_tup > 1000
    AND seq_tup_read > idx_tup_fetch;

  IF high_seq_count > 0 THEN
    RAISE NOTICE '⚠️  Found % table(s) with high sequential scan activity', high_seq_count;
    RAISE NOTICE '   Recommendation: Review queries and consider additional indices';
  ELSE
    RAISE NOTICE '✓ Sequential scan activity is within normal limits';
  END IF;
END $$;

-- Check statistics freshness
DO $$
DECLARE
  stale_stats_count int;
BEGIN
  SELECT COUNT(*) INTO stale_stats_count
  FROM pg_stat_user_tables
  WHERE schemaname = 'maproom'
    AND (last_analyze IS NULL OR last_analyze < NOW() - INTERVAL '7 days')
    AND (last_autoanalyze IS NULL OR last_autoanalyze < NOW() - INTERVAL '7 days')
    AND n_live_tup > 1000;

  IF stale_stats_count > 0 THEN
    RAISE NOTICE '⚠️  Found % table(s) with stale statistics (>7 days old)', stale_stats_count;
    RAISE NOTICE '   Recommendation: Run ANALYZE to update query planner statistics';
  ELSE
    RAISE NOTICE '✓ Query planner statistics are fresh';
  END IF;
END $$;

\echo ''
\echo '================================================================================'
\echo 'ANALYSIS COMPLETE'
\echo '================================================================================'
\echo ''
\echo 'For detailed query performance analysis, run:'
\echo '  EXPLAIN (ANALYZE, BUFFERS) <your_query>;'
\echo ''
\echo 'To monitor index usage over time, use:'
\echo '  psql -d maproom -f scripts/monitor-indices.sql'
\echo ''
