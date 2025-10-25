-- refresh-views.sql
-- Manual script for refreshing materialized views
--
-- Usage:
--   psql $DATABASE_URL -f refresh-views.sql
--
-- This script provides three options:
-- 1. Refresh all views (recommended after bulk operations)
-- 2. Refresh specific views (targeted refresh)
-- 3. Check view staleness before refreshing

-- ==============================================================================
-- Option 1: Refresh All Views (using built-in function)
-- ==============================================================================

-- This is the recommended approach for most cases
-- Uses the maproom.refresh_all_views() function which handles dependencies
\echo '=== Refreshing All Materialized Views ==='
\timing on

SELECT view_name, refresh_time
FROM maproom.refresh_all_views();

\echo ''
\echo '=== All Views Refreshed Successfully ==='
\echo ''

-- ==============================================================================
-- Option 2: Refresh Specific Views
-- ==============================================================================

-- Uncomment the sections below to refresh specific views instead of all

-- Refresh chunk_importance only
-- (after edge creation or deletion)
/*
\echo '=== Refreshing chunk_importance ==='
REFRESH MATERIALIZED VIEW CONCURRENTLY maproom.chunk_importance;
ANALYZE maproom.chunk_importance;
\echo 'Done'
*/

-- Refresh chunk_edge_counts only
-- (after edge creation or deletion)
/*
\echo '=== Refreshing chunk_edge_counts ==='
REFRESH MATERIALIZED VIEW CONCURRENTLY maproom.chunk_edge_counts;
ANALYZE maproom.chunk_edge_counts;
\echo 'Done'
*/

-- Refresh chunk_search_view only
-- (after embedding updates or file changes)
/*
\echo '=== Refreshing chunk_search_view ==='
REFRESH MATERIALIZED VIEW CONCURRENTLY maproom.chunk_search_view;
ANALYZE maproom.chunk_search_view;
\echo 'Done'
*/

-- Refresh file_metadata_view only
-- (after file indexing or deletion)
/*
\echo '=== Refreshing file_metadata_view ==='
REFRESH MATERIALIZED VIEW CONCURRENTLY maproom.file_metadata_view;
ANALYZE maproom.file_metadata_view;
\echo 'Done'
*/

-- ==============================================================================
-- Option 3: Check View Staleness
-- ==============================================================================

\echo '=== Materialized View Staleness ==='
SELECT
  view_name,
  last_refresh,
  age,
  CASE
    WHEN is_stale THEN '⚠️  STALE'
    ELSE '✓  FRESH'
  END as status
FROM maproom.view_staleness()
ORDER BY is_stale DESC, age DESC;

-- ==============================================================================
-- View Statistics
-- ==============================================================================

\echo ''
\echo '=== Materialized View Statistics ==='
SELECT
  schemaname,
  matviewname AS view_name,
  pg_size_pretty(pg_relation_size(schemaname||'.'||matviewname)) AS size,
  n_tup_ins AS rows_inserted,
  n_tup_upd AS rows_updated,
  n_tup_del AS rows_deleted,
  last_autovacuum,
  last_autoanalyze
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
  AND relname IN ('chunk_importance', 'chunk_search_view',
                  'file_metadata_view', 'chunk_edge_counts')
ORDER BY pg_relation_size(schemaname||'.'||matviewname) DESC;

-- ==============================================================================
-- Index Usage on Views
-- ==============================================================================

\echo ''
\echo '=== Index Usage on Materialized Views ==='
SELECT
  schemaname,
  tablename,
  indexname,
  idx_scan AS scans,
  idx_tup_read AS tuples_read,
  idx_tup_fetch AS tuples_fetched,
  pg_size_pretty(pg_relation_size(indexrelid)) AS index_size
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
  AND tablename IN ('chunk_importance', 'chunk_search_view',
                    'file_metadata_view', 'chunk_edge_counts')
ORDER BY idx_scan DESC;

\timing off
\echo ''
\echo '=== Script Complete ==='
\echo ''
\echo 'Tips:'
\echo '  - Run this script after bulk indexing operations'
\echo '  - Schedule periodic refreshes (hourly/daily) based on data update frequency'
\echo '  - Monitor view staleness with: SELECT * FROM maproom.view_staleness();'
\echo '  - All refreshes use CONCURRENTLY to avoid blocking reads'
\echo ''
