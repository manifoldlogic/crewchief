# Vector Search Configuration and Performance Guide

This document covers the shipped vector index (HNSW on the `code_embeddings` pool)
and the supporting PostgreSQL configuration for the Maproom hybrid search system.

## Table of Contents
1. [Overview](#overview)
2. [Database Configuration](#database-configuration)
3. [Index Configuration](#index-configuration)
4. [Performance Tuning](#performance-tuning)
5. [Query Patterns](#query-patterns)
6. [Monitoring](#monitoring)
7. [Troubleshooting](#troubleshooting)

## Overview

The Maproom hybrid search system uses PostgreSQL with the pgvector extension to provide
vector similarity search alongside full-text search and graph signals.

> **Backend note.** The Postgres vector backend requires a `--features postgres` build;
> the default SQLite build uses sqlite-vec with a separate per-dimension vector table
> per-repo (`vec_code`, `vec_code_768`, `vec_code_1024`) and is not covered here. The
> configuration knobs, SQL patterns, and monitoring queries in this document are
> Postgres-only.

### Shipped index design (migration `0004_vector_ann.sql`)

**Storage**: a content-addressed `code_embeddings` pool, with one typed vector column
per supported dimension:
- `embedding_768 vector(768)` — Ollama nomic-embed-text
- `embedding_1024 vector(1024)` — Ollama mxbai-embed-large (default)
- `embedding_1536 vector(1536)` — OpenAI text-embedding-3-small

Each row populates exactly the column matching its `embedding_dim`; the other two are
NULL (NULLs cost no storage in Postgres).

**Index**: one partial HNSW index per dimension, using `vector_cosine_ops`:
```sql
CREATE INDEX idx_code_embeddings_hnsw_768
    ON code_embeddings USING hnsw (embedding_768 vector_cosine_ops)
    WHERE embedding_768 IS NOT NULL;
CREATE INDEX idx_code_embeddings_hnsw_1024
    ON code_embeddings USING hnsw (embedding_1024 vector_cosine_ops)
    WHERE embedding_1024 IS NOT NULL;
CREATE INDEX idx_code_embeddings_hnsw_1536
    ON code_embeddings USING hnsw (embedding_1536 vector_cosine_ops)
    WHERE embedding_1536 IS NOT NULL;
```

**Query operator**: cosine distance `<=>`. Similarity = `1 - cosine_distance`
(`cosine_distance_to_similarity`, PG-local — SQLite keeps its L2 `1/(1+d)`). Score
parity is on membership + ordering, not raw scores.

**First-connect build**: migration `0004` runs inside a single advisory-locked
transaction, which lifts `statement_timeout`. The HNSW indexes are built with a
**locking** (non-`CONCURRENT`) build — Postgres forbids `CREATE INDEX CONCURRENTLY`
inside a transaction block. Fast on a fresh/small pool; a large pre-existing pool will
block writers during the first-connect migration.

**Approximate, not reproducible**: HNSW is an approximate index. The KNN uses
`ORDER BY distance ASC` with NO secondary tiebreak on purpose — adding `, c.id` causes
Postgres to fall back to a full seq-scan + sort (~150× slower, EXPLAIN-verified). The
pool is content-addressed, so tied rows are identical content; their relative order
at the `k` boundary is not pinned run-to-run. This is an accepted tradeoff.

**EXPLAIN benchmark** (5 000-row dim-768 corpus): planner uses
`Index Scan using idx_code_embeddings_hnsw_768` (0.38 ms) vs. forced brute-force
seq-scan + top-N sort (58 ms) — ~150× faster.

**pgvector requirement**: `pgvector >= 0.5.0` (0.8.0 in CI).

### Architecture Components

```
┌─────────────────────────────────────────────────┐
│            Hybrid Search Query                   │
├─────────────────────────────────────────────────┤
│  FTS (tsvector)  │  Vector (pgvector)  │ Signals│
│   GIN index      │   HNSW index        │ B-tree │
└─────────────────────────────────────────────────┘
                        ▼
            ┌───────────────────────┐
            │   Score Fusion        │
            │   (Weighted + RRF)    │
            └───────────────────────┘
```

## Database Configuration

### Extension Setup

```sql
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS unaccent;
```

**Minimum versions:**
- `pgvector >= 0.5.0` (HNSW index support; 0.8.0 in CI)
- `pg_trgm >= 1.4`
- `unaccent >= 1.1`

### Vector Storage Schema

The `code_embeddings` table stores one row per unique `blob_sha`:

```sql
-- Simplified view; see migration 0004 for exact DDL
CREATE TABLE code_embeddings (
    blob_sha       TEXT PRIMARY KEY,
    embedding_dim  INTEGER NOT NULL,       -- 768, 1024, or 1536
    embedding_768  vector(768),            -- populated when embedding_dim = 768
    embedding_1024 vector(1024),           -- populated when embedding_dim = 1024
    embedding_1536 vector(1536),           -- populated when embedding_dim = 1536
    model_version  TEXT NOT NULL,
    created_at     TIMESTAMPTZ DEFAULT NOW()
);
```

The `chunks` table joins to `code_embeddings` via `blob_sha` for vector search.

## Index Configuration

### HNSW Indices (Shipped)

Three partial HNSW indices provide approximate nearest neighbor search (one per
supported dimension):

```sql
CREATE INDEX idx_code_embeddings_hnsw_768
    ON code_embeddings USING hnsw (embedding_768 vector_cosine_ops)
    WHERE embedding_768 IS NOT NULL;

CREATE INDEX idx_code_embeddings_hnsw_1024
    ON code_embeddings USING hnsw (embedding_1024 vector_cosine_ops)
    WHERE embedding_1024 IS NOT NULL;

CREATE INDEX idx_code_embeddings_hnsw_1536
    ON code_embeddings USING hnsw (embedding_1536 vector_cosine_ops)
    WHERE embedding_1536 IS NOT NULL;
```

**Distance metric**: `vector_cosine_ops` (cosine similarity via `<=>` operator)

### Runtime Parameter: `hnsw.ef_search`

The `hnsw.ef_search` parameter controls the accuracy/speed tradeoff at query time.
Set per KNN transaction from config:

**Environment variable**: `MAPROOM_SEARCH_INDEX_HNSW_EF_SEARCH`
- **Default**: 40
- **Clamped**: automatically raised to match the query's `k` (so ef_search is never
  less than k)
- **No index rebuild needed** to change this setting

```sql
-- Session-level override (for current connection)
SET hnsw.ef_search = 40;

-- Transaction-level (for current transaction only)
SET LOCAL hnsw.ef_search = 40;
```

**Performance characteristics:**

| ef_search | Latency (p95) | Recall | Use Case |
|-----------|---------------|--------|----------|
| 10 | <10ms | ~80% | Speed-critical |
| **40** | **<25ms** | **~90%** | **Default** |
| 100 | <50ms | ~95% | High accuracy requirements |
| 200 | <80ms | ~98% | Maximum accuracy |

**Recommendation**: The default (40) is appropriate for most workloads. Raise only if
recall is demonstrably insufficient. ef_search has no impact on index storage or build.

### Partial Indices

Partial indices optimize common filter patterns:

```sql
-- Repo + worktree filtering (core hybrid query pattern)
CREATE INDEX idx_files_repo_worktree
  ON files (repo_id, worktree_id);

-- Symbol name lookups (exclude nulls)
CREATE INDEX idx_chunks_symbol_name
  ON chunks (symbol_name)
  WHERE symbol_name IS NOT NULL;
```

**Benefits:**
- Smaller index size (only subset of rows)
- Faster index scans for matching queries
- Lower maintenance overhead

### Full-Text Search Index

GIN index for tsvector-based full-text search:

```sql
CREATE INDEX idx_chunks_tsv
  ON chunks USING GIN (ts_doc);
```

**Usage:**
```sql
WHERE c.ts_doc @@ to_tsquery('simple', 'auth & login')
```

## Performance Tuning

### PostgreSQL Configuration

Edit `postgresql.conf` or use `ALTER SYSTEM`:

```ini
# Memory Settings (adjust based on available RAM)
shared_buffers = 2GB              # 25% of system RAM (minimum)
effective_cache_size = 6GB        # 75% of system RAM
work_mem = 50MB                   # Per-operation memory
maintenance_work_mem = 512MB      # Index creation, VACUUM

# SSD Optimization
random_page_cost = 1.1            # Default 4.0 assumes HDD
effective_io_concurrency = 200    # For SSD storage

# Query Planner
default_statistics_target = 100   # More statistics for better plans

# Connection Management
max_connections = 100             # Adjust for workload
```

**Reload configuration:**
```bash
pg_ctl reload
# or
SELECT pg_reload_conf();
```

### Statistics Maintenance

Run `ANALYZE` after bulk operations or schema changes:

```sql
-- Update all maproom tables
ANALYZE chunks;
ANALYZE files;
ANALYZE code_embeddings;
ANALYZE chunk_edges;
ANALYZE repos;
ANALYZE worktrees;
```

**Autovacuum configuration:**
```ini
autovacuum = on
autovacuum_max_workers = 3
autovacuum_naptime = 10s          # More frequent for active writes
autovacuum_vacuum_scale_factor = 0.1
autovacuum_analyze_scale_factor = 0.05
```

## Query Patterns

### Pattern 1: Vector Similarity Search (Postgres)

```sql
-- Find top-k similar chunks via HNSW on the code_embeddings pool.
-- Replace <dim> with 768, 1024, or 1536 to match your query embedding.
SET LOCAL hnsw.ef_search = 40;  -- set by Rust before the KNN

SELECT c.id, c.symbol_name, c.preview,
       1 - (ce.embedding_<dim> <=> $1::vector(<dim>)) as similarity
FROM chunks c
JOIN files f ON f.id = c.file_id
JOIN code_embeddings ce ON ce.blob_sha = c.blob_sha
WHERE f.repo_id = $2
  AND ($3::bigint IS NULL OR f.worktree_id = $3)
  AND ce.embedding_<dim> IS NOT NULL
ORDER BY ce.embedding_<dim> <=> $1::vector(<dim>)
LIMIT $4;
```

**Expected EXPLAIN plan:**
```
Limit
  -> Nested Loop
    -> Index Scan using idx_code_embeddings_hnsw_<dim> on code_embeddings ce
         Order By: (embedding_<dim> <=> $1::vector(<dim>))
         Filter: embedding_<dim> IS NOT NULL     -- partial-index predicate
    -> Index Scan using chunks_blob_sha_idx on chunks c
    -> Index Scan using files_pkey on files f
         Filter: (repo_id = $2)
```

**Performance target**: <25ms for k=10 (HNSW ef_search=40, warm cache)

### Pattern 2: Hybrid Search (FTS + Vector + Signals)

```sql
WITH lex_scores AS (
  -- Full-text search
  SELECT c.id, ts_rank_cd(c.ts_doc, query) as lex_rank
  FROM chunks c
  JOIN files f ON f.id = c.file_id,
       to_tsquery('simple', $1) as query
  WHERE f.repo_id = $2
    AND ($3::bigint IS NULL OR f.worktree_id = $3)
    AND c.ts_doc @@ query
),
sem_scores AS (
  -- Vector similarity via HNSW (dim chosen from query embedding)
  SELECT c.id,
    1.0 - (ce.embedding_1024 <=> $4::vector(1024)) as sem_score
  FROM chunks c
  JOIN files f ON f.id = c.file_id
  JOIN code_embeddings ce ON ce.blob_sha = c.blob_sha
  WHERE f.repo_id = $2
    AND ($3::bigint IS NULL OR f.worktree_id = $3)
    AND ce.embedding_1024 IS NOT NULL
  ORDER BY ce.embedding_1024 <=> $4::vector(1024)
  LIMIT 100
)
SELECT c.id, f.relpath, c.symbol_name, c.kind::text,
       c.start_line, c.end_line, c.preview,
       (
         0.55 * COALESCE(l.lex_rank, 0) +
         0.40 * COALESCE(s.sem_score, 0) +
         0.03 * c.recency_score +
         0.02 * (1.0 / (1.0 + c.churn_score))
       ) AS score
FROM chunks c
JOIN files f ON f.id = c.file_id
LEFT JOIN lex_scores l ON l.id = c.id
LEFT JOIN sem_scores s ON s.id = c.id
WHERE c.id IN (
  SELECT id FROM lex_scores UNION SELECT id FROM sem_scores
)
ORDER BY score DESC
LIMIT $5;
```

**Weight configuration:**
- FTS: 55% (lexical matching)
- Vector: 40% (semantic similarity)
- Recency: 3% (prefer recent code)
- Churn: 2% (penalize unstable code)

**Performance target**: <50ms for k=10

### Pattern 3: Filtered Vector Search (Recent Code)

```sql
SET LOCAL hnsw.ef_search = 40;

SELECT c.id, c.symbol_name,
       1 - (ce.embedding_1024 <=> $1::vector(1024)) as similarity,
       c.recency_score
FROM chunks c
JOIN files f ON f.id = c.file_id
JOIN code_embeddings ce ON ce.blob_sha = c.blob_sha
WHERE f.repo_id = $2
  AND c.recency_score > 0.5
  AND ce.embedding_1024 IS NOT NULL
ORDER BY ce.embedding_1024 <=> $1::vector(1024)
LIMIT $3;
```

**Performance target**: <25ms for k=10

## Monitoring

### Index Usage Statistics

```sql
-- Check HNSW index usage and sizes
SELECT
  schemaname,
  tablename,
  indexname,
  pg_size_pretty(pg_relation_size(indexrelid)) as index_size,
  idx_scan as times_used,
  idx_tup_read as tuples_read,
  idx_tup_fetch as tuples_fetched,
  CASE
    WHEN idx_scan > 0 THEN round(idx_tup_read::numeric / idx_scan, 2)
    ELSE 0
  END as avg_tuples_per_scan
FROM pg_stat_user_indexes
WHERE indexname LIKE '%hnsw%'
ORDER BY pg_relation_size(indexrelid) DESC;
```

**What to look for:**
- `times_used = 0`: HNSW index not being picked up — check the IS NOT NULL predicate
- Large `index_size` with low `times_used`: Expensive unused index

### Table Statistics

```sql
-- Check table health and statistics freshness
SELECT
  schemaname,
  tablename,
  n_live_tup as live_rows,
  n_dead_tup as dead_rows,
  round(100.0 * n_dead_tup / NULLIF(n_live_tup + n_dead_tup, 0), 2) as dead_pct,
  last_vacuum,
  last_autovacuum,
  last_analyze,
  last_autoanalyze
FROM pg_stat_user_tables
WHERE tablename IN ('code_embeddings', 'chunks', 'files')
ORDER BY n_live_tup DESC;
```

**Warning signs:**
- `dead_pct > 20%`: Need VACUUM
- `last_analyze` > 7 days old: Statistics may be stale
- High `dead_rows`: VACUUM not running frequently enough

### Sequential Scan Detection

```sql
-- Find tables with excessive sequential scans (vector search should use HNSW)
SELECT
  schemaname,
  tablename,
  seq_scan,
  seq_tup_read,
  idx_scan,
  n_live_tup,
  round(100.0 * seq_scan / NULLIF(seq_scan + idx_scan, 0), 2) as seq_scan_pct
FROM pg_stat_user_tables
WHERE tablename = 'code_embeddings'
  AND n_live_tup > 1000
ORDER BY seq_tup_read DESC;
```

**Warning signs:**
- `seq_scan_pct > 50%` on `code_embeddings` with >1 000 rows: HNSW not being used —
  check the IS NOT NULL predicate in your query and in `pg_stat_user_indexes`.

### Query Performance (pg_stat_statements)

Enable `pg_stat_statements`:
```sql
-- In postgresql.conf:
shared_preload_libraries = 'pg_stat_statements'

-- Create extension:
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
```

Monitor slow queries:
```sql
SELECT
  round(mean_exec_time::numeric, 2) as avg_ms,
  round(total_exec_time::numeric, 2) as total_ms,
  calls,
  round((100 * total_exec_time / sum(total_exec_time) OVER ())::numeric, 2) as pct_total,
  left(query, 80) as query_preview
FROM pg_stat_statements
WHERE query LIKE '%code_embeddings%'
ORDER BY mean_exec_time DESC
LIMIT 20;
```

## Troubleshooting

### Issue: Slow Vector Queries

**Symptoms:**
- Vector similarity queries taking >100ms
- p95 latency exceeding targets

**Diagnosis:**
```sql
-- Check ef_search for the current session
SHOW hnsw.ef_search;

-- Check HNSW index usage
EXPLAIN (ANALYZE, BUFFERS)
SELECT ce.blob_sha FROM code_embeddings ce
WHERE ce.embedding_1024 IS NOT NULL
ORDER BY ce.embedding_1024 <=> '[...]'::vector(1024)
LIMIT 10;
```

**Solutions:**
1. **Decrease ef_search**: Lower `MAPROOM_SEARCH_INDEX_HNSW_EF_SEARCH` (e.g., 20)
   to sacrifice recall for speed — no rebuild required.
2. **Verify IS NOT NULL clause**: The partial-index predicate requires
   `WHERE embedding_<dim> IS NOT NULL` to match.
3. **Rebuild statistics**: `ANALYZE code_embeddings;`
4. **Confirm index exists**: `\di` or query `pg_stat_user_indexes` filtered on `hnsw`.

### Issue: Low Recall (<80%)

**Symptoms:**
- Expected results not appearing in top-k
- User feedback on missing relevant results

**Diagnosis:**
```sql
-- Check ef_search
SHOW hnsw.ef_search;

-- Check recall with a known good pair
SELECT
  (ce.embedding_1024 <=> $1::vector(1024)) as distance,
  1 - (ce.embedding_1024 <=> $1::vector(1024)) as similarity
FROM code_embeddings ce
WHERE ce.blob_sha = $2;  -- blob_sha of a known relevant chunk
```

**Solutions:**
1. **Increase ef_search**: Set `MAPROOM_SEARCH_INDEX_HNSW_EF_SEARCH=100` (no rebuild).
2. **Check embedding quality**: Verify embeddings are generated with the expected model.
3. **Adjust fusion weights**: Increase vector weight vs FTS in the scoring CTE.
4. **Increase candidate pool**: Raise the `LIMIT 100` in the `sem_scores` CTE.

### Issue: Index Not Being Used

**Symptoms:**
- EXPLAIN shows Sequential Scan on `code_embeddings` instead of Index Scan
- Queries slower than expected

**Diagnosis:**
```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT ce.blob_sha FROM code_embeddings ce
WHERE ce.embedding_1024 IS NOT NULL
ORDER BY ce.embedding_1024 <=> '[...]'::vector(1024)
LIMIT 10;
```

**Common causes and solutions:**

1. **Missing IS NOT NULL clause** (partial-index predicate not met):
   ```sql
   -- Bad (planner cannot use the partial HNSW index):
   ORDER BY ce.embedding_1024 <=> $1::vector(1024)

   -- Good (partial-index predicate satisfied):
   WHERE ce.embedding_1024 IS NOT NULL
   ORDER BY ce.embedding_1024 <=> $1::vector(1024)
   ```

2. **Statistics outdated:**
   ```sql
   ANALYZE code_embeddings;
   ```

3. **Index missing:**
   ```sql
   -- List all HNSW indexes
   SELECT indexname, indexdef
   FROM pg_indexes
   WHERE indexname LIKE '%hnsw%';
   ```

4. **Table too small** (planner prefers seq-scan for small tables):
   - Normal for < 1 000 rows
   - Force for testing: `SET enable_seqscan = off;` (testing only!)

### Issue: Out of Memory During Migration (`0004`) HNSW Build

**Symptoms:**
- First-connect migration hangs or fails with memory error
- Postgres OOM during `CREATE INDEX`

**Context**: Migration `0004` builds the HNSW indexes inside a transaction with
statement_timeout lifted. On a large pre-existing pool this blocks writers. On very
large pools it may exhaust `maintenance_work_mem`.

**Solutions:**
1. **Increase maintenance_work_mem** before the first connect:
   ```sql
   ALTER SYSTEM SET maintenance_work_mem = '1GB';
   SELECT pg_reload_conf();
   ```
   Then restart the Maproom process to trigger migration.

2. **Fresh pool**: If the pool was never indexed in Postgres, migration runs on an
   empty table — build is a no-op and completes in milliseconds.

### Issue: High Churn on Dead Rows

**Symptoms:**
- Many dead rows in pg_stat_user_tables
- VACUUM runs frequently but dead_pct stays high

**Diagnosis:**
```sql
SELECT n_live_tup, n_dead_tup,
       last_vacuum, last_autovacuum
FROM pg_stat_user_tables
WHERE tablename = 'code_embeddings';
```

**Solutions:**
1. **Manual VACUUM:**
   ```sql
   VACUUM ANALYZE code_embeddings;
   ```

2. **Tune autovacuum:**
   ```sql
   ALTER TABLE code_embeddings SET (
     autovacuum_vacuum_scale_factor = 0.05,
     autovacuum_analyze_scale_factor = 0.02
   );
   ```

3. **Check for long-running transactions:**
   ```sql
   SELECT pid, query_start, state, query
   FROM pg_stat_activity
   WHERE state != 'idle'
     AND query_start < now() - interval '1 hour';
   ```

## Performance Baselines

### Single Vector Query (Isolated, HNSW ef_search=40)

| Metric | Target | Measured |
|--------|--------|----------|
| p50 latency | <15ms | TBD |
| p95 latency | <25ms | TBD |
| p99 latency | <40ms | TBD |
| Recall@10 | >80% | TBD |

**Benchmark command (psql):**
```sql
\timing on
SET hnsw.ef_search = 40;
SELECT ce.blob_sha FROM code_embeddings ce
WHERE ce.embedding_1024 IS NOT NULL
ORDER BY ce.embedding_1024 <=> '[...]'::vector(1024)
LIMIT 10;
```

### Hybrid Search Query

| Metric | Target | Measured |
|--------|--------|----------|
| p50 latency | <30ms | TBD |
| p95 latency | <50ms | TBD |
| p99 latency | <100ms | TBD |
| Recall@10 | >80% | TBD |

### Concurrent Load

| Metric | Target | Measured |
|--------|--------|----------|
| Throughput | 10+ QPS | TBD |
| p95 latency | <50ms | TBD |
| Max connections | <50 | TBD |

**Load testing:**
Use `pgbench` or a custom load generator to simulate concurrent searches. The daemon
amortizes Postgres connect cost (warm requests measured at ~0.6 ms).

## References

- [pgvector Documentation](https://github.com/pgvector/pgvector)
- [PostgreSQL Index Documentation](https://www.postgresql.org/docs/current/indexes.html)
- [PostgreSQL Performance Tuning](https://wiki.postgresql.org/wiki/Performance_Optimization)
- Migration: `crates/maproom/migrations_pg/0004_vector_ann.sql`
- Crate CLAUDE.md conventions (HNSW details): `crates/maproom/CLAUDE.md`
- Database architecture: `docs/architecture/DATABASE_ARCHITECTURE.md`

## Changelog

| Date | Version | Changes |
|------|---------|---------|
| 2025-10-24 | 1.0.0 | Initial documentation for HYBRID_SEARCH-1002 |
| 2026-07-09 | 2.0.0 | Full rewrite to reflect shipped HNSW/`code_embeddings` design (migration 0004); removed pre-ship ivfflat content |

---

**Maintenance Note**: This document should be updated when:
- HNSW parameters or ef_search defaults change
- Performance targets are revised
- New query patterns are introduced
- New embedding dimensions are added
