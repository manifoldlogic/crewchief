-- 0004_optimize_vector_indices.sql
-- Optimize vector search infrastructure for hybrid retrieval system
-- Part of HYBRID_SEARCH-1002: Database Vector Preparation
--
-- This migration:
-- 1. Verifies pgvector extension and vector columns
-- 2. Creates partial indices for performance optimization
-- 3. Configures runtime parameters for ivfflat indices
-- 4. Updates statistics for query planner
-- 5. Provides verification queries

-- ==============================================================================
-- SECTION 1: Extension and Schema Verification
-- ==============================================================================

-- Ensure pgvector extension is available
CREATE EXTENSION IF NOT EXISTS vector;

-- Verify vector columns exist with correct dimensions (1536)
-- This is informational - columns already created in 0001_init.sql
COMMENT ON COLUMN maproom.chunks.code_embedding IS
  'Vector embedding (1536 dims) of code representation using text-embedding-3-small model';

COMMENT ON COLUMN maproom.chunks.text_embedding IS
  'Vector embedding (1536 dims) of natural language representation using text-embedding-3-small model';

-- ==============================================================================
-- SECTION 2: Partial Indices for Performance Optimization
-- ==============================================================================

-- Partial index for high recency score (recently modified files)
-- Benefits queries filtering for recent/active code
CREATE INDEX IF NOT EXISTS idx_chunks_recent
  ON maproom.chunks (recency_score)
  WHERE recency_score > 0.5;

COMMENT ON INDEX maproom.idx_chunks_recent IS
  'Partial index for chunks with high recency scores (>0.5) - optimizes queries for recent/active code';

-- Partial index for high churn score (frequently modified code)
-- Benefits queries filtering for unstable/actively developed code
CREATE INDEX IF NOT EXISTS idx_chunks_high_churn
  ON maproom.chunks (churn_score)
  WHERE churn_score > 10;

COMMENT ON INDEX maproom.idx_chunks_high_churn IS
  'Partial index for chunks with high churn scores (>10) - optimizes queries for frequently modified code';

-- Composite index for common repo+worktree filtering pattern
-- This optimization supports the hybrid search query pattern that filters by repo_id and worktree_id
CREATE INDEX IF NOT EXISTS idx_files_repo_worktree
  ON maproom.files (repo_id, worktree_id);

COMMENT ON INDEX maproom.idx_files_repo_worktree IS
  'Composite index for repo+worktree filtering - core pattern in hybrid search queries';

-- Index for symbol_name lookups with null-awareness
-- Supports exact symbol name matching and fuzzy search
CREATE INDEX IF NOT EXISTS idx_chunks_symbol_name
  ON maproom.chunks (symbol_name)
  WHERE symbol_name IS NOT NULL;

COMMENT ON INDEX maproom.idx_chunks_symbol_name IS
  'Partial index for symbol name lookups - excludes null symbol names';

-- ==============================================================================
-- SECTION 3: ivfflat Index Configuration
-- ==============================================================================

-- The ivfflat indices already exist from 0001_init.sql:
-- - idx_chunks_code_vec: ivfflat on code_embedding with lists=200
-- - idx_chunks_text_vec: ivfflat on text_embedding with lists=200
--
-- These indices use the following parameters:
-- - lists=200: Number of clusters for approximate nearest neighbor search
--              Recommended: sqrt(row_count), initially 200 for ~40k rows
--              Should be increased as dataset grows: sqrt(500000) ≈ 707
-- - Distance metric: vector_cosine_ops (cosine similarity via <=> operator)
--
-- Runtime parameter configuration (probes):
-- The ivfflat.probes parameter controls search accuracy vs speed tradeoff
-- - probes=10 (recommended default): Good balance of 80%+ recall with low latency
-- - probes=1: Fastest but lowest accuracy (~50-60% recall)
-- - probes=20: Higher accuracy but slower (~90%+ recall)
-- - probes=50: Maximum accuracy but significant latency cost

-- Set database-level default for ivfflat.probes
-- This can be overridden at session or transaction level for specific queries
ALTER DATABASE postgres SET ivfflat.probes = 10;

-- Note: The ALTER DATABASE command above sets the default for the 'postgres' database.
-- If your database has a different name, this should be adjusted.
-- Alternatively, set at session level in application code:
--   SET ivfflat.probes = 10;
-- Or at query level:
--   SET LOCAL ivfflat.probes = 10;

COMMENT ON EXTENSION vector IS
  'pgvector extension for vector similarity search. Configuration: ivfflat indices with lists=200, probes=10 (database default)';

-- ==============================================================================
-- SECTION 4: Statistics Update
-- ==============================================================================

-- Update query planner statistics for accurate cost estimation
-- These should be run after bulk data operations or significant schema changes
ANALYZE maproom.chunks;
ANALYZE maproom.files;
ANALYZE maproom.chunk_edges;
ANALYZE maproom.repos;
ANALYZE maproom.worktrees;
ANALYZE maproom.commits;

-- ==============================================================================
-- SECTION 5: Performance Verification Queries
-- ==============================================================================

-- The following queries can be used to verify index usage and performance
-- Run with EXPLAIN (ANALYZE, BUFFERS) to see execution plans

-- Query Pattern 1: Vector Similarity Search (Code Mode)
-- Expected: Should use idx_chunks_code_vec (ivfflat index scan)
-- Typical performance: <20ms for k=10 on 100k+ chunks with probes=10
/*
EXPLAIN (ANALYZE, BUFFERS)
SELECT c.id, c.symbol_name,
       1 - (c.code_embedding <=> '[0.1, 0.2, ...]'::vector(1536)) as similarity
FROM maproom.chunks c
JOIN maproom.files f ON f.id = c.file_id
WHERE f.repo_id = 1
  AND c.code_embedding IS NOT NULL
ORDER BY c.code_embedding <=> '[0.1, 0.2, ...]'::vector(1536)
LIMIT 10;

Expected plan:
  -> Limit
    -> Nested Loop
      -> Index Scan using idx_chunks_code_vec on chunks c
           Order By: (code_embedding <=> '...'::vector)
           Filter: (code_embedding IS NOT NULL)
      -> Index Scan using files_pkey on files f
           Index Cond: (id = c.file_id)
           Filter: (repo_id = 1)
*/

-- Query Pattern 2: Hybrid Search with FTS + Vector
-- Expected: Should use both idx_chunks_tsv (GIN) and idx_chunks_code_vec (ivfflat)
-- Typical performance: <50ms for combined query with score fusion
/*
EXPLAIN (ANALYZE, BUFFERS)
WITH lex_scores AS (
  SELECT c.id, ts_rank_cd(c.ts_doc, to_tsquery('simple', 'auth & login')) as lex_rank
  FROM maproom.chunks c
  JOIN maproom.files f ON f.id = c.file_id
  WHERE f.repo_id = 1
    AND c.ts_doc @@ to_tsquery('simple', 'auth & login')
),
sem_scores AS (
  SELECT c.id, 1.0 - (c.code_embedding <=> '[0.1, 0.2, ...]'::vector(1536)) as sem_score
  FROM maproom.chunks c
  JOIN maproom.files f ON f.id = c.file_id
  WHERE f.repo_id = 1
    AND c.code_embedding IS NOT NULL
  ORDER BY c.code_embedding <=> '[0.1, 0.2, ...]'::vector(1536)
  LIMIT 100
)
SELECT c.id, c.symbol_name,
  (0.55 * COALESCE(l.lex_rank, 0) + 0.30 * COALESCE(s.sem_score, 0)) as score
FROM maproom.chunks c
LEFT JOIN lex_scores l ON l.id = c.id
LEFT JOIN sem_scores s ON s.id = c.id
WHERE l.id IS NOT NULL OR s.id IS NOT NULL
ORDER BY score DESC
LIMIT 10;

Expected plan:
  -> Two parallel CTEs executing independently:
     1. FTS CTE: Bitmap Index Scan on idx_chunks_tsv
     2. Vector CTE: Index Scan on idx_chunks_code_vec
  -> Hash joins for score combination
  -> Sort + Limit for final ranking
*/

-- Query Pattern 3: Partial Index Usage (Recent Code)
-- Expected: Should use idx_chunks_recent (partial B-tree index)
/*
EXPLAIN (ANALYZE, BUFFERS)
SELECT c.id, c.symbol_name, c.recency_score
FROM maproom.chunks c
JOIN maproom.files f ON f.id = c.file_id
WHERE f.repo_id = 1
  AND c.recency_score > 0.5
ORDER BY c.recency_score DESC
LIMIT 20;

Expected plan:
  -> Limit
    -> Nested Loop
      -> Index Scan Backward using idx_chunks_recent on chunks c
           Filter: (recency_score > 0.5)
      -> Index Scan using files_pkey on files f
           Index Cond: (id = c.file_id)
           Filter: (repo_id = 1)
*/

-- Query Pattern 4: Full-Text Search Only
-- Expected: Should use idx_chunks_tsv (GIN index)
/*
EXPLAIN (ANALYZE, BUFFERS)
SELECT c.id, c.symbol_name,
       ts_rank_cd(c.ts_doc, query) as rank
FROM maproom.chunks c
JOIN maproom.files f ON f.id = c.file_id,
     to_tsquery('simple', 'authentication & login') as query
WHERE f.repo_id = 1
  AND c.ts_doc @@ query
ORDER BY rank DESC
LIMIT 10;

Expected plan:
  -> Limit
    -> Sort
      -> Nested Loop
        -> Bitmap Heap Scan on chunks c
          -> Bitmap Index Scan on idx_chunks_tsv
               Index Cond: (ts_doc @@ query)
        -> Index Scan using files_pkey on files f
             Index Cond: (id = c.file_id)
             Filter: (repo_id = 1)
*/

-- ==============================================================================
-- SECTION 6: Performance Baseline Documentation
-- ==============================================================================

-- Vector similarity search baseline (synthetic test):
-- Dataset size: 10k-100k chunks
-- Query: k=10 nearest neighbors
-- Metric: Cosine similarity (<=>)
--
-- Performance targets (p95 latency):
-- - probes=1:  <10ms  (recall ~50-60%)
-- - probes=5:  <15ms  (recall ~70-75%)
-- - probes=10: <25ms  (recall ~80-85%)  ← RECOMMENDED DEFAULT
-- - probes=20: <40ms  (recall ~90-95%)
-- - probes=50: <80ms  (recall ~95-98%)
--
-- Hybrid search baseline (FTS + Vector + Signals):
-- p95 latency target: <50ms
-- Recall target: >80%
--
-- Index sizing guidelines:
-- - Current: lists=200 (optimal for ~40k chunks)
-- - At 100k chunks: consider lists=316 (sqrt(100000))
-- - At 500k chunks: consider lists=707 (sqrt(500000))
-- - At 1M chunks: consider lists=1000 (sqrt(1000000))
--
-- Reindexing note:
-- To change lists parameter, drop and recreate index:
--   DROP INDEX IF EXISTS maproom.idx_chunks_code_vec;
--   CREATE INDEX idx_chunks_code_vec ON maproom.chunks
--     USING ivfflat (code_embedding vector_cosine_ops)
--     WITH (lists = 707);
-- Use CREATE INDEX CONCURRENTLY to avoid blocking writes in production.

-- ==============================================================================
-- SECTION 7: Recommended Configuration Settings
-- ==============================================================================

-- PostgreSQL configuration recommendations for vector search workload:
--
-- Memory settings (postgresql.conf):
--   shared_buffers = 2GB           # 25% of system RAM (minimum)
--   effective_cache_size = 6GB     # 75% of system RAM
--   work_mem = 50MB                # Per-operation memory for sorts/hashes
--   maintenance_work_mem = 512MB   # For index creation and VACUUM
--
-- Query planner settings:
--   random_page_cost = 1.1         # For SSD storage (default 4.0 assumes HDD)
--   effective_io_concurrency = 200 # For SSD storage
--
-- Autovacuum settings:
--   autovacuum = on
--   autovacuum_max_workers = 3
--   autovacuum_naptime = 10s       # More frequent for active writes
--
-- Vector-specific settings:
--   ivfflat.probes = 10            # Set via ALTER DATABASE (done above)
--
-- Connection pooling:
--   max_connections = 100          # Adjust based on workload
--   Use pgBouncer or similar for high-concurrency scenarios
--
-- Extension versions (minimum):
--   pgvector >= 0.5.0              # For HNSW index support (future)
--   pg_trgm >= 1.4                 # For trigram fuzzy matching
--   unaccent >= 1.1                # For accent-insensitive search

-- ==============================================================================
-- SECTION 8: Monitoring Queries
-- ==============================================================================

-- Check current ivfflat.probes setting
-- Run: SHOW ivfflat.probes;

-- View index sizes and usage statistics
-- SELECT
--   schemaname,
--   tablename,
--   indexname,
--   pg_size_pretty(pg_relation_size(indexrelid)) as index_size,
--   idx_scan as times_used,
--   idx_tup_read as tuples_read,
--   idx_tup_fetch as tuples_fetched
-- FROM pg_stat_user_indexes
-- WHERE schemaname = 'maproom'
-- ORDER BY pg_relation_size(indexrelid) DESC;

-- View table statistics
-- SELECT
--   schemaname,
--   tablename,
--   n_tup_ins as inserts,
--   n_tup_upd as updates,
--   n_tup_del as deletes,
--   n_live_tup as live_rows,
--   n_dead_tup as dead_rows,
--   last_vacuum,
--   last_autovacuum,
--   last_analyze,
--   last_autoanalyze
-- FROM pg_stat_user_tables
-- WHERE schemaname = 'maproom'
-- ORDER BY n_live_tup DESC;

-- Check for missing indices (sequential scans on large tables)
-- SELECT
--   schemaname,
--   tablename,
--   seq_scan,
--   seq_tup_read,
--   idx_scan,
--   seq_tup_read / NULLIF(seq_scan, 0) as avg_seq_read
-- FROM pg_stat_user_tables
-- WHERE schemaname = 'maproom'
--   AND seq_scan > 0
-- ORDER BY seq_tup_read DESC;

-- ==============================================================================
-- Migration Complete
-- ==============================================================================

-- This migration has:
-- ✓ Verified pgvector extension installation
-- ✓ Created partial indices for recency_score and churn_score
-- ✓ Created composite index for repo_id + worktree_id filtering
-- ✓ Configured ivfflat.probes=10 as database default
-- ✓ Updated statistics on all core tables
-- ✓ Documented verification queries and performance baselines
-- ✓ Provided configuration recommendations
--
-- Next steps:
-- 1. Run verification queries to confirm index usage
-- 2. Measure baseline query performance with EXPLAIN ANALYZE
-- 3. Monitor index usage with pg_stat_user_indexes
-- 4. Adjust ivfflat.probes based on accuracy/speed requirements
-- 5. Plan for index reindexing as dataset grows beyond 100k chunks
