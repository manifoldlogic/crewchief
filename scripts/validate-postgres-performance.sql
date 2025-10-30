-- PostgreSQL Performance Validation Script for LOCAL-4008
-- This script creates test data and runs comprehensive performance tests

\timing on
\set ECHO all

-- ============================================================================
-- PART 1: Create Test Data
-- ============================================================================

\echo ''
\echo '=== Creating Test Repository and Files ==='
\echo ''

-- Insert test repository
INSERT INTO maproom.repos (id, name, remote_url)
VALUES (1, 'test-repo', 'https://github.com/test/repo.git')
ON CONFLICT (id) DO NOTHING;

-- Insert test worktree
INSERT INTO maproom.worktrees (id, repo_id, name, path, is_active, created_at)
VALUES (1, 1, 'main', '/test/path', true, now())
ON CONFLICT (id) DO NOTHING;

-- Insert test files
INSERT INTO maproom.files (id, repo_id, worktree_id, relpath, file_type, created_at)
SELECT
    i,
    1,
    1,
    'src/file_' || i || '.ts',
    'typescript',
    now()
FROM generate_series(1, 100) as i
ON CONFLICT (id) DO NOTHING;

\echo ''
\echo '=== Creating Test Chunks with Vectors and FTS ==='
\echo ''

-- Generate test chunks with realistic data
-- This creates chunks with:
-- - Random text embeddings (384 dimensions)
-- - Random code embeddings (384 dimensions)
-- - Full-text search vectors
-- - Realistic symbol names and content
INSERT INTO maproom.chunks (
    id, file_id, symbol_name, kind, start_line, end_line, preview,
    code_embedding, text_embedding, ts_doc,
    recency_score, churn_score
)
SELECT
    gen_random_uuid(),
    ((i - 1) % 100) + 1, -- Distribute across 100 files
    'function_' || i,
    'function',
    i * 10,
    i * 10 + 10,
    'export function function_' || i || '() { /* implementation */ return ' || i || '; }',
    -- Generate random 384-dimensional vector for code embedding
    (SELECT array_agg(random())::vector FROM generate_series(1, 384))::vector(384),
    -- Generate random 384-dimensional vector for text embedding
    (SELECT array_agg(random())::vector FROM generate_series(1, 384))::vector(384),
    -- Generate FTS vector from common programming terms
    to_tsvector('simple',
        'function export import class interface type const variable ' ||
        'async await promise callback handler service controller model ' ||
        'database query select insert update delete join where ' ||
        'test describe expect assert mock stub fixture ' ||
        'error exception throw catch try finally ' ||
        'component render state props effect hook ' ||
        'function_' || i
    ),
    random(), -- recency_score
    random()  -- churn_score
FROM generate_series(1, 1000) as i
ON CONFLICT (id) DO NOTHING;

\echo ''
\echo '=== Test Data Summary ==='
\echo ''

SELECT
    'Repos' as entity,
    COUNT(*) as count
FROM maproom.repos
UNION ALL
SELECT 'Files', COUNT(*) FROM maproom.files
UNION ALL
SELECT 'Chunks', COUNT(*) FROM maproom.chunks;

-- Verify vector dimensions
SELECT
    vector_dims(code_embedding) as code_dims,
    vector_dims(text_embedding) as text_dims
FROM maproom.chunks
LIMIT 1;

-- ============================================================================
-- PART 2: Verify Configuration Applied
-- ============================================================================

\echo ''
\echo '=== PostgreSQL Configuration Verification ==='
\echo ''

SELECT
    name,
    setting,
    unit,
    context
FROM pg_settings
WHERE name IN (
    'shared_buffers',
    'work_mem',
    'maintenance_work_mem',
    'effective_cache_size',
    'random_page_cost',
    'effective_io_concurrency',
    'max_connections',
    'wal_buffers',
    'checkpoint_completion_target',
    'default_statistics_target',
    'max_worker_processes',
    'max_parallel_workers_per_gather',
    'max_parallel_workers',
    'cpu_tuple_cost',
    'cpu_index_tuple_cost'
)
ORDER BY name;

-- ============================================================================
-- PART 3: Index Statistics BEFORE Tests
-- ============================================================================

\echo ''
\echo '=== Index Statistics BEFORE Performance Tests ==='
\echo ''

SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan as index_scans,
    idx_tup_read as tuples_read,
    idx_tup_fetch as tuples_fetched
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
ORDER BY tablename, indexname;

-- ============================================================================
-- PART 4: Performance Test 1 - Full-Text Search
-- ============================================================================

\echo ''
\echo '=== TEST 1: Full-Text Search (FTS) Performance ==='
\echo ''

-- Test common programming terms
\echo 'Query 1.1: Search for "function"'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT
    c.id,
    c.symbol_name,
    c.kind,
    ts_rank_cd(c.ts_doc, to_tsquery('simple', 'function')) as rank
FROM maproom.chunks c
WHERE c.ts_doc @@ to_tsquery('simple', 'function')
ORDER BY rank DESC
LIMIT 10;

\echo ''
\echo 'Query 1.2: Search for "function & class"'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT
    c.id,
    c.symbol_name,
    c.kind,
    ts_rank_cd(c.ts_doc, to_tsquery('simple', 'function & class')) as rank
FROM maproom.chunks c
WHERE c.ts_doc @@ to_tsquery('simple', 'function & class')
ORDER BY rank DESC
LIMIT 10;

\echo ''
\echo 'Query 1.3: Search for "database | query"'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT
    c.id,
    c.symbol_name,
    c.kind,
    ts_rank_cd(c.ts_doc, to_tsquery('simple', 'database | query')) as rank
FROM maproom.chunks c
WHERE c.ts_doc @@ to_tsquery('simple', 'database | query')
ORDER BY rank DESC
LIMIT 10;

-- ============================================================================
-- PART 5: Performance Test 2 - Vector Similarity Search
-- ============================================================================

\echo ''
\echo '=== TEST 2: Vector Similarity Search Performance ==='
\echo ''

-- Create a test query vector (random 384-dim vector)
\echo 'Query 2.1: Vector similarity search (code_embedding, k=10)'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
WITH query_vec AS (
    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 384))::vector(384) as vec
)
SELECT
    c.id,
    c.symbol_name,
    c.kind,
    1.0 - (c.code_embedding <=> query_vec.vec) as similarity
FROM maproom.chunks c, query_vec
ORDER BY c.code_embedding <=> query_vec.vec
LIMIT 10;

\echo ''
\echo 'Query 2.2: Vector similarity search (code_embedding, k=50)'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
WITH query_vec AS (
    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 384))::vector(384) as vec
)
SELECT
    c.id,
    c.symbol_name,
    c.kind,
    1.0 - (c.code_embedding <=> query_vec.vec) as similarity
FROM maproom.chunks c, query_vec
ORDER BY c.code_embedding <=> query_vec.vec
LIMIT 50;

\echo ''
\echo 'Query 2.3: Vector similarity search (text_embedding, k=20)'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
WITH query_vec AS (
    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 384))::vector(384) as vec
)
SELECT
    c.id,
    c.symbol_name,
    c.kind,
    1.0 - (c.text_embedding <=> query_vec.vec) as similarity
FROM maproom.chunks c, query_vec
ORDER BY c.text_embedding <=> query_vec.vec
LIMIT 20;

-- ============================================================================
-- PART 6: Performance Test 3 - Hybrid Search
-- ============================================================================

\echo ''
\echo '=== TEST 3: Hybrid Search (FTS + Vector) Performance ==='
\echo ''

\echo 'Query 3.1: Hybrid search combining FTS and vector similarity'
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
WITH
query_vec AS (
    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 384))::vector(384) as vec
),
lex_scores AS (
    SELECT
        c.id,
        ts_rank_cd(c.ts_doc, to_tsquery('simple', 'function')) as lex_rank
    FROM maproom.chunks c
    WHERE c.ts_doc @@ to_tsquery('simple', 'function')
),
sem_scores AS (
    SELECT
        c.id,
        1.0 - (c.code_embedding <=> query_vec.vec) as sem_code,
        1.0 - (c.text_embedding <=> query_vec.vec) as sem_text
    FROM maproom.chunks c, query_vec
    ORDER BY c.code_embedding <=> query_vec.vec
    LIMIT 100
)
SELECT
    c.id,
    c.symbol_name,
    c.kind,
    c.preview,
    (
        0.55 * COALESCE(l.lex_rank, 0) +
        0.30 * COALESCE(s.sem_code, 0) +
        0.10 * COALESCE(s.sem_text, 0) +
        0.03 * c.recency_score +
        0.02 * (1.0 / (1.0 + c.churn_score))
    ) as score
FROM maproom.chunks c
LEFT JOIN lex_scores l ON l.id = c.id
LEFT JOIN sem_scores s ON s.id = c.id
WHERE c.id IN (
    SELECT id FROM lex_scores
    UNION
    SELECT id FROM sem_scores
)
ORDER BY score DESC
LIMIT 10;

-- ============================================================================
-- PART 7: Performance Test 4 - Concurrent Query Simulation
-- ============================================================================

\echo ''
\echo '=== TEST 4: Sequential Execution of Multiple Queries ==='
\echo '(Simulating concurrent workload pattern)'
\echo ''

-- Run 10 different queries in sequence to simulate load
\echo 'Running 10 FTS queries...'
SELECT COUNT(*) FROM (
    SELECT c.id FROM maproom.chunks c
    WHERE c.ts_doc @@ to_tsquery('simple', 'function')
    LIMIT 10
) t;

SELECT COUNT(*) FROM (
    SELECT c.id FROM maproom.chunks c
    WHERE c.ts_doc @@ to_tsquery('simple', 'class')
    LIMIT 10
) t;

SELECT COUNT(*) FROM (
    SELECT c.id FROM maproom.chunks c
    WHERE c.ts_doc @@ to_tsquery('simple', 'interface')
    LIMIT 10
) t;

SELECT COUNT(*) FROM (
    SELECT c.id FROM maproom.chunks c
    WHERE c.ts_doc @@ to_tsquery('simple', 'database')
    LIMIT 10
) t;

SELECT COUNT(*) FROM (
    SELECT c.id FROM maproom.chunks c
    WHERE c.ts_doc @@ to_tsquery('simple', 'query')
    LIMIT 10
) t;

\echo 'Running 5 vector similarity queries...'
WITH query_vec AS (
    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 384))::vector(384) as vec
)
SELECT COUNT(*) FROM (
    SELECT c.id FROM maproom.chunks c, query_vec
    ORDER BY c.code_embedding <=> query_vec.vec
    LIMIT 10
) t;

WITH query_vec AS (
    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 384))::vector(384) as vec
)
SELECT COUNT(*) FROM (
    SELECT c.id FROM maproom.chunks c, query_vec
    ORDER BY c.code_embedding <=> query_vec.vec
    LIMIT 10
) t;

WITH query_vec AS (
    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 384))::vector(384) as vec
)
SELECT COUNT(*) FROM (
    SELECT c.id FROM maproom.chunks c, query_vec
    ORDER BY c.code_embedding <=> query_vec.vec
    LIMIT 10
) t;

WITH query_vec AS (
    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 384))::vector(384) as vec
)
SELECT COUNT(*) FROM (
    SELECT c.id FROM maproom.chunks c, query_vec
    ORDER BY c.code_embedding <=> query_vec.vec
    LIMIT 10
) t;

WITH query_vec AS (
    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 384))::vector(384) as vec
)
SELECT COUNT(*) FROM (
    SELECT c.id FROM maproom.chunks c, query_vec
    ORDER BY c.code_embedding <=> query_vec.vec
    LIMIT 10
) t;

-- ============================================================================
-- PART 8: Database Statistics AFTER Tests
-- ============================================================================

\echo ''
\echo '=== Cache Hit Ratios ==='
\echo ''

SELECT
    datname,
    blks_read as blocks_read_from_disk,
    blks_hit as blocks_hit_in_cache,
    round(
        100.0 * blks_hit / NULLIF(blks_hit + blks_read, 0),
        2
    ) as cache_hit_ratio_percent
FROM pg_stat_database
WHERE datname = 'maproom';

\echo ''
\echo '=== Index Usage Statistics AFTER Tests ==='
\echo ''

SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan as index_scans,
    idx_tup_read as tuples_read,
    idx_tup_fetch as tuples_fetched,
    pg_size_pretty(pg_relation_size(indexrelid)) as index_size
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
ORDER BY idx_scan DESC;

\echo ''
\echo '=== Sequential Scans vs Index Scans ==='
\echo ''

SELECT
    schemaname,
    tablename,
    seq_scan as sequential_scans,
    seq_tup_read as seq_tuples_read,
    idx_scan as index_scans,
    idx_tup_fetch as idx_tuples_fetched,
    CASE
        WHEN seq_scan > 0 THEN
            round(100.0 * idx_scan / NULLIF(seq_scan + idx_scan, 0), 2)
        ELSE 100.0
    END as index_scan_ratio_percent
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
ORDER BY tablename;

\echo ''
\echo '=== Table and Index Sizes ==='
\echo ''

SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as total_size,
    pg_size_pretty(pg_relation_size(schemaname||'.'||tablename)) as table_size,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename) - pg_relation_size(schemaname||'.'||tablename)) as indexes_size
FROM pg_tables
WHERE schemaname = 'maproom'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

\echo ''
\echo '=== Buffer Cache Statistics ==='
\echo ''

SELECT
    'checkpoints_timed' as metric,
    checkpoints_timed as value
FROM pg_stat_bgwriter
UNION ALL
SELECT 'checkpoints_req', checkpoints_req FROM pg_stat_bgwriter
UNION ALL
SELECT 'buffers_checkpoint', buffers_checkpoint FROM pg_stat_bgwriter
UNION ALL
SELECT 'buffers_clean', buffers_clean FROM pg_stat_bgwriter
UNION ALL
SELECT 'buffers_backend', buffers_backend FROM pg_stat_bgwriter
UNION ALL
SELECT 'buffers_alloc', buffers_alloc FROM pg_stat_bgwriter;

\echo ''
\echo '=== Performance Validation Complete ==='
\echo ''
