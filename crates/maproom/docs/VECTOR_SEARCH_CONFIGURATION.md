# Vector Search Configuration and Performance Guide

This document provides comprehensive guidance on configuring and optimizing PostgreSQL with pgvector for the Maproom hybrid search system.

## Table of Contents
1. [Overview](#overview)
2. [Database Configuration](#database-configuration)
3. [Index Configuration](#index-configuration)
4. [Performance Tuning](#performance-tuning)
5. [Query Patterns](#query-patterns)
6. [Monitoring](#monitoring)
7. [Troubleshooting](#troubleshooting)

## Overview

The Maproom hybrid search system uses PostgreSQL with the pgvector extension to provide vector similarity search alongside full-text search and graph signals. This guide covers the database layer optimization for achieving:

- **p95 latency**: <50ms for hybrid queries
- **Recall**: >80% on test queries
- **Throughput**: 10+ queries per second

### Architecture Components

```
┌─────────────────────────────────────────────────┐
│            Hybrid Search Query                   │
├─────────────────────────────────────────────────┤
│  FTS (tsvector)  │  Vector (pgvector)  │ Signals│
│   GIN index      │   ivfflat index     │ B-tree │
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
- `pgvector >= 0.5.0` (for HNSW index support in future)
- `pg_trgm >= 1.4`
- `unaccent >= 1.1`

### Vector Columns

The `maproom.chunks` table contains two vector columns:

```sql
code_embedding VECTOR(1536)  -- Code representation embedding
text_embedding VECTOR(1536)  -- Natural language representation embedding
```

**Embedding model**: OpenAI `text-embedding-3-small` (1536 dimensions)

**Null handling**: Both columns can be NULL. Queries must filter for non-NULL values:
```sql
WHERE c.code_embedding IS NOT NULL
```

## Index Configuration

### ivfflat Indices

Two ivfflat indices provide approximate nearest neighbor search:

```sql
CREATE INDEX idx_chunks_code_vec
  ON maproom.chunks USING ivfflat (code_embedding vector_cosine_ops)
  WITH (lists = 200);

CREATE INDEX idx_chunks_text_vec
  ON maproom.chunks USING ivfflat (text_embedding vector_cosine_ops)
  WITH (lists = 200);
```

**Parameters:**

| Parameter | Value | Purpose | Tuning |
|-----------|-------|---------|--------|
| `lists` | 200 | Number of clusters for ANN | Set to `sqrt(row_count)` |
| `probes` | 10 | Runtime search accuracy | Higher = more accurate but slower |

**Distance metric**: `vector_cosine_ops` (cosine similarity via `<=>` operator)

**Scaling guidelines:**

| Dataset Size | Recommended lists | Reasoning |
|--------------|-------------------|-----------|
| 10k chunks | 100 | sqrt(10000) ≈ 100 |
| 40k chunks | 200 | sqrt(40000) ≈ 200 ✓ current |
| 100k chunks | 316 | sqrt(100000) ≈ 316 |
| 500k chunks | 707 | sqrt(500000) ≈ 707 |
| 1M chunks | 1000 | sqrt(1000000) = 1000 |

**Reindexing procedure** (when increasing lists):

```sql
-- Use CONCURRENTLY to avoid blocking writes
DROP INDEX CONCURRENTLY IF EXISTS maproom.idx_chunks_code_vec;

CREATE INDEX CONCURRENTLY idx_chunks_code_vec
  ON maproom.chunks
  USING ivfflat (code_embedding vector_cosine_ops)
  WITH (lists = 707);

-- Update statistics after reindexing
ANALYZE maproom.chunks;
```

### Runtime Parameter: ivfflat.probes

The `ivfflat.probes` parameter controls the accuracy/speed tradeoff at query time.

**Setting levels:**

```sql
-- Database-level (default for all connections)
ALTER DATABASE postgres SET ivfflat.probes = 10;

-- Session-level (for current connection)
SET ivfflat.probes = 10;

-- Transaction-level (for current transaction only)
SET LOCAL ivfflat.probes = 10;
```

**Performance characteristics:**

| probes | Latency (p95) | Recall | Use Case |
|--------|---------------|--------|----------|
| 1 | <10ms | 50-60% | Speed-critical, low accuracy ok |
| 5 | <15ms | 70-75% | Balanced for small datasets |
| **10** | **<25ms** | **80-85%** | **Recommended default** |
| 20 | <40ms | 90-95% | High accuracy requirements |
| 50 | <80ms | 95-98% | Maximum accuracy, latency acceptable |

**Recommendation**: Start with `probes=10` for 80%+ recall with acceptable latency. Increase only if recall is insufficient.

### Partial Indices

Partial indices optimize common filter patterns:

```sql
-- High recency score (recently modified code)
CREATE INDEX idx_chunks_recent
  ON maproom.chunks (recency_score)
  WHERE recency_score > 0.5;

-- High churn score (frequently modified code)
CREATE INDEX idx_chunks_high_churn
  ON maproom.chunks (churn_score)
  WHERE churn_score > 10;

-- Repo + worktree filtering (core hybrid query pattern)
CREATE INDEX idx_files_repo_worktree
  ON maproom.files (repo_id, worktree_id);

-- Symbol name lookups (exclude nulls)
CREATE INDEX idx_chunks_symbol_name
  ON maproom.chunks (symbol_name)
  WHERE symbol_name IS NOT NULL;
```

**Benefits:**
- Smaller index size (only subset of rows)
- Faster index scans for matching queries
- Lower maintenance overhead

**Usage requirements:**
- Query `WHERE` clause must match or be more restrictive than index predicate
- Example: `WHERE recency_score > 0.7` can use `idx_chunks_recent` (predicate: `> 0.5`)

### Full-Text Search Index

GIN index for tsvector-based full-text search:

```sql
CREATE INDEX idx_chunks_tsv
  ON maproom.chunks USING GIN (ts_doc);
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
ANALYZE maproom.chunks;
ANALYZE maproom.files;
ANALYZE maproom.chunk_edges;
ANALYZE maproom.repos;
ANALYZE maproom.worktrees;
ANALYZE maproom.commits;

-- Or analyze entire schema
ANALYZE maproom.*;
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

### Pattern 1: Vector Similarity Search (Code Mode)

```sql
-- Find top-k similar code chunks
SELECT c.id, c.symbol_name, c.preview,
       1 - (c.code_embedding <=> $1::vector(1536)) as similarity
FROM maproom.chunks c
JOIN maproom.files f ON f.id = c.file_id
WHERE f.repo_id = $2
  AND ($3::bigint IS NULL OR f.worktree_id = $3)
  AND c.code_embedding IS NOT NULL
ORDER BY c.code_embedding <=> $1::vector(1536)
LIMIT $4;
```

**Parameters:**
- `$1`: Query embedding (vector)
- `$2`: Repository ID
- `$3`: Worktree ID (optional)
- `$4`: Limit (k)

**Expected EXPLAIN plan:**
```
Limit
  -> Nested Loop
    -> Index Scan using idx_chunks_code_vec on chunks c
         Order By: (code_embedding <=> $1::vector)
    -> Index Scan using files_pkey on files f
         Index Cond: (id = c.file_id)
         Filter: (repo_id = $2)
```

**Performance target**: <20ms for k=10

### Pattern 2: Hybrid Search (FTS + Vector + Signals)

```sql
WITH lex_scores AS (
  -- Full-text search
  SELECT c.id, ts_rank_cd(c.ts_doc, query) as lex_rank
  FROM maproom.chunks c
  JOIN maproom.files f ON f.id = c.file_id,
       to_tsquery('simple', $1) as query
  WHERE f.repo_id = $2
    AND ($3::bigint IS NULL OR f.worktree_id = $3)
    AND c.ts_doc @@ query
),
sem_scores AS (
  -- Vector similarity
  SELECT c.id,
    1.0 - (c.code_embedding <=> $4::vector) as sem_code,
    1.0 - (c.text_embedding <=> $4::vector) as sem_text
  FROM maproom.chunks c
  JOIN maproom.files f ON f.id = c.file_id
  WHERE f.repo_id = $2
    AND ($3::bigint IS NULL OR f.worktree_id = $3)
    AND c.code_embedding IS NOT NULL
  ORDER BY
    CASE $5
      WHEN 'code' THEN c.code_embedding <=> $4::vector
      WHEN 'text' THEN c.text_embedding <=> $4::vector
      ELSE LEAST(c.code_embedding <=> $4::vector,
                 c.text_embedding <=> $4::vector)
    END
  LIMIT 100
)
SELECT c.id, f.relpath, c.symbol_name, c.kind::text,
       c.start_line, c.end_line, c.preview,
       (
         0.55 * COALESCE(l.lex_rank, 0) +
         0.30 * CASE WHEN $5 = 'code' THEN COALESCE(s.sem_code, 0)
                     ELSE COALESCE(s.sem_text, 0) END +
         0.10 * CASE WHEN $5 = 'code' THEN COALESCE(s.sem_text, 0)
                     ELSE COALESCE(s.sem_code, 0) END +
         0.03 * c.recency_score +
         0.02 * (1.0 / (1.0 + c.churn_score))
       ) AS score
FROM maproom.chunks c
JOIN maproom.files f ON f.id = c.file_id
LEFT JOIN lex_scores l ON l.id = c.id
LEFT JOIN sem_scores s ON s.id = c.id
WHERE c.id IN (
  SELECT id FROM lex_scores UNION SELECT id FROM sem_scores
)
ORDER BY score DESC
LIMIT $6;
```

**Parameters:**
- `$1`: FTS query string (e.g., "auth & login")
- `$2`: Repository ID
- `$3`: Worktree ID (optional)
- `$4`: Query embedding (vector)
- `$5`: Search mode ('code', 'text', or 'auto')
- `$6`: Final result limit (k)

**Weight configuration:**
- FTS: 55% (lexical matching)
- Primary vector: 30% (semantic similarity)
- Secondary vector: 10% (alternative perspective)
- Recency: 3% (prefer recent code)
- Churn: 2% (penalize unstable code)

**Performance target**: <50ms for k=10

### Pattern 3: Filtered Vector Search (Recent Code)

```sql
-- Find similar code in recently modified files
SELECT c.id, c.symbol_name,
       1 - (c.code_embedding <=> $1::vector) as similarity,
       c.recency_score
FROM maproom.chunks c
JOIN maproom.files f ON f.id = c.file_id
WHERE f.repo_id = $2
  AND c.recency_score > 0.5
  AND c.code_embedding IS NOT NULL
ORDER BY c.code_embedding <=> $1::vector
LIMIT $3;
```

**Expected plan:**
```
Limit
  -> Nested Loop
    -> Index Scan using idx_chunks_code_vec
    -> Index Scan using idx_chunks_recent (partial index)
```

**Performance target**: <15ms for k=10

## Monitoring

### Index Usage Statistics

```sql
-- Check index usage and sizes
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
WHERE schemaname = 'maproom'
ORDER BY pg_relation_size(indexrelid) DESC;
```

**What to look for:**
- `times_used = 0`: Unused index (consider dropping)
- Large `index_size` with low `times_used`: Expensive unused index
- High `avg_tuples_per_scan`: May need more selective index

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
WHERE schemaname = 'maproom'
ORDER BY n_live_tup DESC;
```

**Warning signs:**
- `dead_pct > 20%`: Need VACUUM
- `last_analyze` > 7 days old: Statistics may be stale
- High `dead_rows`: VACUUM not running frequently enough

### Sequential Scan Detection

```sql
-- Find tables with excessive sequential scans
SELECT
  schemaname,
  tablename,
  seq_scan,
  seq_tup_read,
  idx_scan,
  n_live_tup,
  round(100.0 * seq_scan / NULLIF(seq_scan + idx_scan, 0), 2) as seq_scan_pct
FROM pg_stat_user_tables
WHERE schemaname = 'maproom'
  AND n_live_tup > 1000
ORDER BY seq_tup_read DESC;
```

**Warning signs:**
- `seq_scan_pct > 50%` on large tables: May need additional indices
- High `seq_tup_read`: Queries scanning entire table

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
WHERE query LIKE '%maproom%'
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
-- Check current probes setting
SHOW ivfflat.probes;

-- Check index usage
EXPLAIN (ANALYZE, BUFFERS)
SELECT c.id FROM maproom.chunks c
WHERE c.code_embedding IS NOT NULL
ORDER BY c.code_embedding <=> '[...]'::vector
LIMIT 10;
```

**Solutions:**
1. **Decrease probes**: `SET ivfflat.probes = 5;` (sacrifice recall for speed)
2. **Check index exists**: Ensure `idx_chunks_code_vec` is present
3. **Verify NULL filtering**: Add `WHERE code_embedding IS NOT NULL`
4. **Rebuild statistics**: `ANALYZE maproom.chunks;`
5. **Reindex with more lists**: If dataset grew significantly

### Issue: Low Recall (<80%)

**Symptoms:**
- Expected results not appearing in top-k
- User feedback on missing relevant results

**Diagnosis:**
```sql
-- Check recall with known good pairs
SELECT
  (c.code_embedding <=> $1::vector) as distance,
  1 - (c.code_embedding <=> $1::vector) as similarity
FROM maproom.chunks c
WHERE c.id = $2;  -- Known relevant chunk
```

**Solutions:**
1. **Increase probes**: `SET ivfflat.probes = 20;` (accept latency cost)
2. **Check embedding quality**: Verify embeddings are being generated correctly
3. **Adjust fusion weights**: Increase vector weight vs FTS
4. **Increase candidate pool**: Fetch more candidates before fusion (e.g., LIMIT 200)

### Issue: Index Not Being Used

**Symptoms:**
- EXPLAIN shows Sequential Scan instead of Index Scan
- Queries slower than expected

**Diagnosis:**
```sql
EXPLAIN (ANALYZE, BUFFERS)
<your query here>;
```

**Common causes and solutions:**

1. **Missing WHERE clause for NULL:**
   ```sql
   -- Bad (won't use index):
   ORDER BY c.code_embedding <=> $1::vector

   -- Good (uses index):
   WHERE c.code_embedding IS NOT NULL
   ORDER BY c.code_embedding <=> $1::vector
   ```

2. **Statistics outdated:**
   ```sql
   ANALYZE maproom.chunks;
   ```

3. **Index missing:**
   ```sql
   \di maproom.*  -- List all indices
   ```

4. **Query planner prefers seq scan (small table):**
   - This is OK for small datasets (<1000 rows)
   - Force index: `SET enable_seqscan = off;` (testing only!)

### Issue: Out of Memory During Index Creation

**Symptoms:**
- `CREATE INDEX` fails with memory error
- Server becomes unresponsive during reindex

**Solutions:**
1. **Increase maintenance_work_mem:**
   ```sql
   SET maintenance_work_mem = '1GB';
   CREATE INDEX ...;
   ```

2. **Use CONCURRENTLY (slower but safer):**
   ```sql
   CREATE INDEX CONCURRENTLY idx_name ON ...;
   ```

3. **Reduce lists parameter temporarily:**
   ```sql
   -- Create with fewer lists, rebuild later
   WITH (lists = 100)
   ```

### Issue: High Churn on Dead Rows

**Symptoms:**
- Many dead rows in pg_stat_user_tables
- VACUUM runs frequently but dead_pct stays high

**Diagnosis:**
```sql
SELECT n_live_tup, n_dead_tup,
       last_vacuum, last_autovacuum
FROM pg_stat_user_tables
WHERE tablename = 'chunks';
```

**Solutions:**
1. **Manual VACUUM:**
   ```sql
   VACUUM ANALYZE maproom.chunks;
   ```

2. **Tune autovacuum:**
   ```sql
   ALTER TABLE maproom.chunks SET (
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

### Single Vector Query (Isolated)

| Metric | Target | Measured |
|--------|--------|----------|
| p50 latency | <15ms | TBD |
| p95 latency | <25ms | TBD |
| p99 latency | <40ms | TBD |
| Recall@10 | >80% | TBD |

**Benchmark command:**
```sql
\timing on
SELECT c.id FROM maproom.chunks c
WHERE c.code_embedding IS NOT NULL
ORDER BY c.code_embedding <=> '[...]'::vector
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
Use `pgbench` or custom load generator to simulate concurrent searches.

## References

- [pgvector Documentation](https://github.com/pgvector/pgvector)
- [PostgreSQL Index Documentation](https://www.postgresql.org/docs/current/indexes.html)
- [PostgreSQL Performance Tuning](https://wiki.postgresql.org/wiki/Performance_Optimization)
- HYBRID_SEARCH Architecture: `/workspace/.agents/archive/projects/HYBRID_SEARCH_hybrid-retrieval-system/planning/HYBRID_SEARCH_ARCHITECTURE.md`
- Migration: `/workspace/crates/maproom/migrations/0004_optimize_vector_indices.sql`

## Changelog

| Date | Version | Changes |
|------|---------|---------|
| 2025-10-24 | 1.0.0 | Initial documentation for HYBRID_SEARCH-1002 |

---

**Maintenance Note**: This document should be updated when:
- Index configurations change
- Performance targets are revised
- New query patterns are introduced
- Scaling thresholds are reached
