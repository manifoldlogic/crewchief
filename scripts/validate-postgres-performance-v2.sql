-- PostgreSQL Performance Validation Script for LOCAL-4008 (Schema-Compliant)
-- This script creates minimal test data and runs comprehensive performance tests

\timing on
\set ECHO all

-- ============================================================================
-- PART 1: Create Test Data
-- ============================================================================

\echo ''
\echo '=== Creating Test Repository, Worktree, Commit, and Files ==='
\echo ''

-- Insert test repository
INSERT INTO maproom.repos (id, name, root_path)
VALUES (1, 'test-repo', '/test/path')
ON CONFLICT (id) DO NOTHING;

-- Insert test worktree
INSERT INTO maproom.worktrees (id, repo_id, name, root_path, branch_name, is_active)
VALUES (1, 1, 'main', '/test/path', 'main', true)
ON CONFLICT (id) DO NOTHING;

-- Insert test commit
INSERT INTO maproom.commits (id, repo_id, commit_hash, committer_email, commit_date, message)
VALUES (1, 1, 'abc123def456', 'test@example.com', now(), 'Test commit')
ON CONFLICT (id) DO NOTHING;

-- Insert test files (100 files)
INSERT INTO maproom.files (id, repo_id, worktree_id, commit_id, relpath, language, content_hash, size_bytes)
SELECT
    i,
    1,
    1,
    1,
    'src/file_' || i || '.ts',
    'typescript',
    md5(i::text),
    1000 + i * 10
FROM generate_series(1, 100) as i
ON CONFLICT (id) DO NOTHING;

\echo ''
\echo '=== Creating Test Chunks with 1536-dim Vectors and FTS ==='
\echo ''

-- Generate test chunks with realistic data
-- Using 1536 dimensions to match schema
INSERT INTO maproom.chunks (
    id, file_id, symbol_name, kind, start_line, end_line, preview,
    code_embedding, text_embedding, ts_doc,
    recency_score, churn_score
)
SELECT
    i,
    ((i - 1) % 100) + 1, -- Distribute across 100 files
    'function_' || i,
    'function',
    i * 10,
    i * 10 + 10,
    'export function function_' || i || '() { /* implementation */ return ' || i || '; }',
    -- Generate random 1536-dimensional vector for code embedding
    (SELECT array_agg(random())::vector FROM generate_series(1, 1536))::vector(1536),
    -- Generate random 1536-dimensional vector for text embedding
    (SELECT array_agg(random())::vector FROM generate_series(1, 1536))::vector(1536),
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

SELECT 'Repos' as entity, COUNT(*) as count FROM maproom.repos
UNION ALL SELECT 'Worktrees', COUNT(*) FROM maproom.worktrees
UNION ALL SELECT 'Commits', COUNT(*) FROM maproom.commits
UNION ALL SELECT 'Files', COUNT(*) FROM maproom.files
UNION ALL SELECT 'Chunks', COUNT(*) FROM maproom.chunks;

-- Verify vector dimensions
SELECT
    'code' as embedding_type,
    vector_dims(code_embedding) as dimensions
FROM maproom.chunks
LIMIT 1
UNION ALL
SELECT
    'text',
    vector_dims(text_embedding)
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
    CASE
        WHEN unit = '8kB' THEN pg_size_pretty((setting::bigint * 8192)::bigint)
        WHEN unit = 'kB' THEN pg_size_pretty((setting::bigint * 1024)::bigint)
        ELSE setting || ' ' || COALESCE(unit, '')
    END as readable_value
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
    'default_statistics_target'
)
ORDER BY name;

-- ============================================================================
-- PART 3: Performance Test 1 - Full-Text Search
-- ============================================================================

\echo ''
\echo '=== TEST 1: Full-Text Search (FTS) Performance ==='
\echo ''

\echo 'Query 1.1: Search for "function" (common term)'
EXPLAIN (ANALYZE, BUFFERS)
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
\echo 'Query 1.2: Search for "function & class" (AND query)'
EXPLAIN (ANALYZE, BUFFERS)
SELECT
    c.id,
    c.symbol_name,
    ts_rank_cd(c.ts_doc, to_tsquery('simple', 'function & class')) as rank
FROM maproom.chunks c
WHERE c.ts_doc @@ to_tsquery('simple', 'function & class')
ORDER BY rank DESC
LIMIT 10;

\echo ''
\echo 'Query 1.3: Search for "database | query" (OR query)'
EXPLAIN (ANALYZE, BUFFERS)
SELECT
    c.id,
    c.symbol_name,
    ts_rank_cd(c.ts_doc, to_tsquery('simple', 'database | query')) as rank
FROM maproom.chunks c
WHERE c.ts_doc @@ to_tsquery('simple', 'database | query')
ORDER BY rank DESC
LIMIT 10;

-- ============================================================================
-- PART 4: Performance Test 2 - Vector Similarity Search
-- ============================================================================

\echo ''
\echo '=== TEST 2: Vector Similarity Search Performance ==='
\echo ''

\echo 'Query 2.1: Vector similarity (code_embedding, k=10)'
EXPLAIN (ANALYZE, BUFFERS)
WITH query_vec AS (
    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 1536))::vector(1536) as vec
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
\echo 'Query 2.2: Vector similarity (code_embedding, k=50)'
EXPLAIN (ANALYZE, BUFFERS)
WITH query_vec AS (
    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 1536))::vector(1536) as vec
)
SELECT
    c.id,
    c.symbol_name,
    1.0 - (c.code_embedding <=> query_vec.vec) as similarity
FROM maproom.chunks c, query_vec
ORDER BY c.code_embedding <=> query_vec.vec
LIMIT 50;

\echo ''
\echo 'Query 2.3: Vector similarity (text_embedding, k=20)'
EXPLAIN (ANALYZE, BUFFERS)
WITH query_vec AS (
    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 1536))::vector(1536) as vec
)
SELECT
    c.id,
    c.symbol_name,
    1.0 - (c.text_embedding <=> query_vec.vec) as similarity
FROM maproom.chunks c, query_vec
ORDER BY c.text_embedding <=> query_vec.vec
LIMIT 20;

-- ============================================================================
-- PART 5: Performance Test 3 - Hybrid Search
-- ============================================================================

\echo ''
\echo '=== TEST 3: Hybrid Search (FTS + Vector) Performance ==='
\echo ''

\echo 'Query 3.1: Hybrid search combining FTS and vector similarity'
EXPLAIN (ANALYZE, BUFFERS)
WITH
query_vec AS (
    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 1536))::vector(1536) as vec
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
-- PART 6: Latency Measurement Tests
-- ============================================================================

\echo ''
\echo '=== TEST 4: Latency Measurements (10 iterations each) ==='
\echo ''

-- FTS Latency
\echo 'FTS Query Latency (10 iterations):'
SELECT
    MIN(elapsed_ms) as min_ms,
    AVG(elapsed_ms) as avg_ms,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY elapsed_ms) as p50_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY elapsed_ms) as p95_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY elapsed_ms) as p99_ms,
    MAX(elapsed_ms) as max_ms
FROM (
    SELECT
        i,
        extract(milliseconds from (clock_timestamp() - t_start)) as elapsed_ms
    FROM (
        SELECT
            i,
            clock_timestamp() as t_start,
            (SELECT COUNT(*)
             FROM (
                SELECT c.id
                FROM maproom.chunks c
                WHERE c.ts_doc @@ to_tsquery('simple', 'function')
                LIMIT 10
             ) x
            ) as result_count
        FROM generate_series(1, 10) i
    ) timings
) measurements;

-- Vector Similarity Latency
\echo ''
\echo 'Vector Similarity Query Latency (10 iterations):'
SELECT
    MIN(elapsed_ms) as min_ms,
    AVG(elapsed_ms) as avg_ms,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY elapsed_ms) as p50_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY elapsed_ms) as p95_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY elapsed_ms) as p99_ms,
    MAX(elapsed_ms) as max_ms
FROM (
    SELECT
        i,
        extract(milliseconds from (clock_timestamp() - t_start)) as elapsed_ms
    FROM (
        SELECT
            i,
            clock_timestamp() as t_start,
            (
                WITH query_vec AS (
                    SELECT (SELECT array_agg(random())::vector FROM generate_series(1, 1536))::vector(1536) as vec
                )
                SELECT COUNT(*)
                FROM (
                    SELECT c.id
                    FROM maproom.chunks c, query_vec
                    ORDER BY c.code_embedding <=> query_vec.vec
                    LIMIT 10
                ) x
            ) as result_count
        FROM generate_series(1, 10) i
    ) timings
) measurements;

-- ============================================================================
-- PART 7: Database Statistics
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
\echo '=== Index Usage Statistics ==='
\echo ''

SELECT
    schemaname,
    relname as table_name,
    indexrelname as index_name,
    idx_scan as index_scans,
    idx_tup_read as tuples_read,
    idx_tup_fetch as tuples_fetched,
    pg_size_pretty(pg_relation_size(indexrelid)) as index_size
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
ORDER BY idx_scan DESC NULLS LAST;

\echo ''
\echo '=== Sequential Scans vs Index Scans ==='
\echo ''

SELECT
    schemaname,
    relname as table_name,
    seq_scan as sequential_scans,
    seq_tup_read as seq_tuples_read,
    idx_scan as index_scans,
    idx_tup_fetch as idx_tuples_fetched,
    CASE
        WHEN seq_scan + idx_scan > 0 THEN
            round(100.0 * idx_scan / NULLIF(seq_scan + idx_scan, 0), 2)
        ELSE NULL
    END as index_scan_ratio_percent
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
ORDER BY relname;

\echo ''
\echo '=== Table and Index Sizes ==='
\echo ''

SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as total_size,
    pg_size_pretty(pg_relation_size(schemaname||'.'||tablename)) as table_size,
    pg_size_pretty(
        pg_total_relation_size(schemaname||'.'||tablename) -
        pg_relation_size(schemaname||'.'||tablename)
    ) as indexes_size
FROM pg_tables
WHERE schemaname = 'maproom'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

\echo ''
\echo '=== Performance Validation Complete ==='
\echo ''
