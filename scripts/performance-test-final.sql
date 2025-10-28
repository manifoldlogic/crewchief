-- PostgreSQL Performance Validation for LOCAL-4008
-- Comprehensive performance testing with actual data

\timing on

-- ============================================================================
-- PART 1: Create Realistic Test Data
-- ============================================================================

\echo ''
\echo '=== Creating Test Data (1000 chunks across 100 files) ==='
\echo ''

-- Clean up any existing test data
DELETE FROM maproom.chunks WHERE file_id IN (SELECT id FROM maproom.files WHERE repo_id = 1);
DELETE FROM maproom.files WHERE repo_id = 1;
DELETE FROM maproom.commits WHERE repo_id = 1;
DELETE FROM maproom.worktrees WHERE repo_id = 1;
DELETE FROM maproom.repos WHERE id = 1;

-- Insert repo
INSERT INTO maproom.repos (id, name, root_path)
VALUES (1, 'test-perf-repo', '/test/perf');

-- Insert worktree
INSERT INTO maproom.worktrees (id, repo_id, name, abs_path)
VALUES (1, 1, 'main', '/test/perf');

-- Insert commit
INSERT INTO maproom.commits (id, repo_id, sha, committed_at)
VALUES (1, 1, 'abc123def456', now());

-- Insert 100 files
INSERT INTO maproom.files (id, repo_id, worktree_id, commit_id, relpath, language, content_hash, size_bytes)
SELECT
    i,
    1,
    1,
    1,
    'src/module_' || (i / 10) || '/file_' || i || '.ts',
    'TypeScript',
    md5(i::text),
    1000 + i * 50
FROM generate_series(1, 100) as i;

-- Insert 1000 chunks (10 per file)
-- Using proper symbol_kind enum values and 1536-dim vectors
INSERT INTO maproom.chunks (
    id, file_id, symbol_name, kind, start_line, end_line, preview,
    code_embedding, text_embedding, ts_doc,
    recency_score, churn_score
)
SELECT
    i,
    ((i - 1) / 10) + 1, -- 10 chunks per file
    CASE (i % 5)
        WHEN 0 THEN 'function_' || i
        WHEN 1 THEN 'class_' || i
        WHEN 2 THEN 'component_' || i
        WHEN 3 THEN 'hook_' || i
        ELSE 'module_' || i
    END,
    CASE (i % 5)
        WHEN 0 THEN 'func'::maproom.symbol_kind
        WHEN 1 THEN 'class'::maproom.symbol_kind
        WHEN 2 THEN 'component'::maproom.symbol_kind
        WHEN 3 THEN 'hook'::maproom.symbol_kind
        ELSE 'module'::maproom.symbol_kind
    END,
    (i * 10) + 1,
    (i * 10) + 15,
    CASE (i % 5)
        WHEN 0 THEN 'export function func_' || i || '() { return processData(); }'
        WHEN 1 THEN 'export class Class_' || i || ' { constructor() {} method() {} }'
        WHEN 2 THEN 'export function Component_' || i || '() { return <div>content</div>; }'
        WHEN 3 THEN 'export function useHook_' || i || '() { const [state, setState] = useState(); }'
        ELSE 'export const module_' || i || ' = { init() {}, cleanup() {} };'
    END,
    -- Random 1536-dim code embedding
    (SELECT ('[' || string_agg(random()::text, ',') || ']')::vector
     FROM generate_series(1, 1536))::vector(1536),
    -- Random 1536-dim text embedding
    (SELECT ('[' || string_agg(random()::text, ',') || ']')::vector
     FROM generate_series(1, 1536))::vector(1536),
    -- FTS document with varied content
    to_tsvector('simple',
        'function export import class interface type const variable ' ||
        'async await promise callback handler service controller model ' ||
        'database query select insert update delete join where ' ||
        'test describe expect assert mock stub fixture ' ||
        'error exception throw catch try finally ' ||
        'component render state props effect hook useState useEffect ' ||
        'processData handleClick fetchUser saveRecord ' ||
        CASE (i % 5)
            WHEN 0 THEN 'function func_' || i
            WHEN 1 THEN 'class Class_' || i
            WHEN 2 THEN 'component Component_' || i
            WHEN 3 THEN 'hook useHook_' || i
            ELSE 'module module_' || i
        END
    ),
    0.5 + (random() * 0.5), -- recency_score 0.5-1.0
    random() * 2.0  -- churn_score 0-2.0
FROM generate_series(1, 1000) as i;

\echo ''
SELECT
    'Test data created:' as status,
    (SELECT COUNT(*) FROM maproom.repos WHERE id = 1) as repos,
    (SELECT COUNT(*) FROM maproom.files WHERE repo_id = 1) as files,
    (SELECT COUNT(*) FROM maproom.chunks WHERE file_id IN (SELECT id FROM maproom.files WHERE repo_id = 1)) as chunks;

-- ============================================================================
-- PART 2: Verify Configuration
-- ============================================================================

\echo ''
\echo '=== PostgreSQL Configuration (Post-Tuning) ==='
\echo ''

SELECT
    name,
    setting,
    unit,
    CASE
        WHEN unit = '8kB' THEN pg_size_pretty((setting::bigint * 8192)::bigint)
        WHEN unit = 'kB' THEN pg_size_pretty((setting::bigint * 1024)::bigint)
        ELSE setting || COALESCE(' ' || unit, '')
    END as readable_value
FROM pg_settings
WHERE name IN (
    'shared_buffers',
    'work_mem',
    'maintenance_work_mem',
    'effective_cache_size',
    'random_page_cost',
    'effective_io_concurrency',
    'max_connections'
)
ORDER BY name;

-- ============================================================================
-- PART 3: Full-Text Search Performance
-- ============================================================================

\echo ''
\echo '=== TEST 1: Full-Text Search (FTS) with EXPLAIN ANALYZE ==='
\echo ''

\echo 'FTS Query 1: Common term "function"'
EXPLAIN (ANALYZE, BUFFERS)
SELECT
    c.id,
    c.symbol_name,
    c.kind::text,
    ts_rank_cd(c.ts_doc, to_tsquery('simple', 'function')) as rank
FROM maproom.chunks c
WHERE c.ts_doc @@ to_tsquery('simple', 'function')
ORDER BY rank DESC
LIMIT 10;

\echo ''
\echo 'FTS Query 2: AND query "function & class"'
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
\echo 'FTS Query 3: OR query "component | hook"'
EXPLAIN (ANALYZE, BUFFERS)
SELECT
    c.id,
    c.symbol_name,
    ts_rank_cd(c.ts_doc, to_tsquery('simple', 'component | hook')) as rank
FROM maproom.chunks c
WHERE c.ts_doc @@ to_tsquery('simple', 'component | hook')
ORDER BY rank DESC
LIMIT 10;

-- ============================================================================
-- PART 4: Vector Similarity Performance
-- ============================================================================

\echo ''
\echo '=== TEST 2: Vector Similarity Search with EXPLAIN ANALYZE ==='
\echo ''

\echo 'Vector Query 1: code_embedding similarity (k=10)'
EXPLAIN (ANALYZE, BUFFERS)
WITH query_vec AS (
    SELECT (SELECT ('[' || string_agg(random()::text, ',') || ']')::vector
            FROM generate_series(1, 1536))::vector(1536) as vec
)
SELECT
    c.id,
    c.symbol_name,
    c.kind::text,
    1.0 - (c.code_embedding <=> query_vec.vec) as similarity
FROM maproom.chunks c, query_vec
ORDER BY c.code_embedding <=> query_vec.vec
LIMIT 10;

\echo ''
\echo 'Vector Query 2: code_embedding similarity (k=50)'
EXPLAIN (ANALYZE, BUFFERS)
WITH query_vec AS (
    SELECT (SELECT ('[' || string_agg(random()::text, ',') || ']')::vector
            FROM generate_series(1, 1536))::vector(1536) as vec
)
SELECT
    c.id,
    c.symbol_name,
    1.0 - (c.code_embedding <=> query_vec.vec) as similarity
FROM maproom.chunks c, query_vec
ORDER BY c.code_embedding <=> query_vec.vec
LIMIT 50;

\echo ''
\echo 'Vector Query 3: text_embedding similarity (k=20)'
EXPLAIN (ANALYZE, BUFFERS)
WITH query_vec AS (
    SELECT (SELECT ('[' || string_agg(random()::text, ',') || ']')::vector
            FROM generate_series(1, 1536))::vector(1536) as vec
)
SELECT
    c.id,
    c.symbol_name,
    1.0 - (c.text_embedding <=> query_vec.vec) as similarity
FROM maproom.chunks c, query_vec
ORDER BY c.text_embedding <=> query_vec.vec
LIMIT 20;

-- ============================================================================
-- PART 5: Hybrid Search Performance
-- ============================================================================

\echo ''
\echo '=== TEST 3: Hybrid Search (FTS + Vector) with EXPLAIN ANALYZE ==='
\echo ''

EXPLAIN (ANALYZE, BUFFERS)
WITH
query_vec AS (
    SELECT (SELECT ('[' || string_agg(random()::text, ',') || ']')::vector
            FROM generate_series(1, 1536))::vector(1536) as vec
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
    c.kind::text,
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
-- PART 6: Latency Measurements (Actual Timing)
-- ============================================================================

\echo ''
\echo '=== TEST 4: Actual Query Latency Measurements (20 iterations) ==='
\echo ''

\echo 'FTS Latency (20 iterations):'
WITH timings AS (
    SELECT
        i,
        (SELECT extract(epoch from clock_timestamp()) * 1000) as t_start,
        (
            SELECT COUNT(*)
            FROM (
                SELECT c.id
                FROM maproom.chunks c
                WHERE c.ts_doc @@ to_tsquery('simple', 'function')
                ORDER BY ts_rank_cd(c.ts_doc, to_tsquery('simple', 'function')) DESC
                LIMIT 10
            ) x
        ) as cnt,
        (SELECT extract(epoch from clock_timestamp()) * 1000) as t_end
    FROM generate_series(1, 20) i
),
measurements AS (
    SELECT i, (t_end - t_start) as elapsed_ms
    FROM timings
)
SELECT
    ROUND(MIN(elapsed_ms)::numeric, 3) as min_ms,
    ROUND(AVG(elapsed_ms)::numeric, 3) as avg_ms,
    ROUND(PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY elapsed_ms)::numeric, 3) as p50_ms,
    ROUND(PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY elapsed_ms)::numeric, 3) as p95_ms,
    ROUND(PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY elapsed_ms)::numeric, 3) as p99_ms,
    ROUND(MAX(elapsed_ms)::numeric, 3) as max_ms
FROM measurements;

\echo ''
\echo 'Vector Similarity Latency (20 iterations):'
WITH timings AS (
    SELECT
        i,
        (SELECT extract(epoch from clock_timestamp()) * 1000) as t_start,
        (
            WITH query_vec AS (
                SELECT (SELECT ('[' || string_agg(random()::text, ',') || ']')::vector
                        FROM generate_series(1, 1536))::vector(1536) as vec
            )
            SELECT COUNT(*)
            FROM (
                SELECT c.id
                FROM maproom.chunks c, query_vec
                ORDER BY c.code_embedding <=> query_vec.vec
                LIMIT 10
            ) x
        ) as cnt,
        (SELECT extract(epoch from clock_timestamp()) * 1000) as t_end
    FROM generate_series(1, 20) i
),
measurements AS (
    SELECT i, (t_end - t_start) as elapsed_ms
    FROM timings
)
SELECT
    ROUND(MIN(elapsed_ms)::numeric, 3) as min_ms,
    ROUND(AVG(elapsed_ms)::numeric, 3) as avg_ms,
    ROUND(PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY elapsed_ms)::numeric, 3) as p50_ms,
    ROUND(PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY elapsed_ms)::numeric, 3) as p95_ms,
    ROUND(PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY elapsed_ms)::numeric, 3) as p99_ms,
    ROUND(MAX(elapsed_ms)::numeric, 3) as max_ms
FROM measurements;

-- ============================================================================
-- PART 7: Database Statistics
-- ============================================================================

\echo ''
\echo '=== Database Statistics Post-Testing ==='
\echo ''

\echo 'Cache Hit Ratio:'
SELECT
    datname,
    blks_read as disk_reads,
    blks_hit as cache_hits,
    ROUND(100.0 * blks_hit / NULLIF(blks_hit + blks_read, 0), 2) as cache_hit_pct
FROM pg_stat_database
WHERE datname = 'maproom';

\echo ''
\echo 'Index Usage:'
SELECT
    relname as table_name,
    indexrelname as index_name,
    idx_scan as scans,
    idx_tup_read as tuples_read,
    pg_size_pretty(pg_relation_size(indexrelid)) as size
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom' AND relname = 'chunks'
ORDER BY idx_scan DESC NULLS LAST;

\echo ''
\echo 'Table Access Patterns:'
SELECT
    relname as table_name,
    seq_scan,
    seq_tup_read,
    idx_scan,
    idx_tup_fetch,
    CASE
        WHEN (seq_scan + idx_scan) > 0 THEN
            ROUND(100.0 * idx_scan / (seq_scan + idx_scan), 2)
        ELSE NULL
    END as index_scan_pct
FROM pg_stat_user_tables
WHERE schemaname = 'maproom' AND relname = 'chunks';

\echo ''
\echo '=== Performance Validation Complete ==='
\echo ''
