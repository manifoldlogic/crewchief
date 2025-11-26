# Analysis: Full SQLite Implementation

## Problem Definition

The Maproom semantic code search system currently requires PostgreSQL with pgvector for full functionality. This creates adoption friction:

1. **Docker dependency** - Users must run PostgreSQL in Docker
2. **Cross-environment complexity** - Managing database across IDEs, devcontainers, and machines
3. **Setup overhead** - Non-trivial configuration before first use

The goal is a **zero-config SQLite implementation** that provides equivalent functionality without external dependencies.

## Current State

### What Exists (from SQLFIX project)

The SQLFIX project fixed compilation issues and basic functionality:

| Component | Status | Notes |
|-----------|--------|-------|
| Connection/pooling | Working | r2d2_sqlite, WAL mode, busy_timeout |
| Schema creation | Working | Tables: repos, worktrees, commits, files, chunks |
| CRUD operations | Working | All VectorStore trait methods compile |
| FTS5 search | Working | Basic full-text search functional |
| Unit tests | Working | 10 tests pass |

### What's Missing (Critical Gaps)

| Gap | Impact | Complexity |
|-----|--------|------------|
| **Vector search** | No semantic similarity | High |
| **Embedding deduplication** | 70-90% storage waste | Medium |
| **Hybrid search (RRF)** | No fused ranking | Medium |
| **worktree_ids dedup** | Data corruption | Low |
| **768-dim embeddings** | No Ollama support | Low (deferred) |
| **Semantic ranking** | Missing kind/exact multipliers | Medium |
| **Graph traversal** | Caller/callee broken | Medium |

### PostgreSQL Reference Architecture

The PostgreSQL implementation provides:

```
Search Pipeline:
  Query → FTSExecutor → FTS Results (ts_rank)
       → VectorExecutor → Vector Results (cosine similarity)
       → RRF Fusion → Combined ranked results
       → Semantic Ranking → Final scores with multipliers

Storage:
  chunks table → Individual code chunks
  code_embeddings table → Deduplicated embeddings by blob_sha
  chunk_edges table → Graph relationships (caller/callee)
```

## Design Constraint: SQLite-Only

**Key decision**: Build for SQLite directly, not behind a trait abstraction.

Rationale:
- Simpler implementation
- No need to maintain PostgreSQL parity
- Can optimize for SQLite-specific features
- Avoids abstraction overhead during PMF phase

This means:
- New code goes in `crates/maproom/src/db/sqlite/`
- May duplicate some logic from PostgreSQL (acceptable)
- PostgreSQL backend remains but is separate path
- VSCode extension will need SQLite-specific integration (future project)

## Reusable Utilities

The following existing utilities should be imported and reused, not reimplemented:

| Utility | Location | Purpose |
|---------|----------|---------|
| `normalize_for_exact_match()` | `src/search/fts.rs` | Normalize query/symbol for exact match detection (handles camelCase, kebab-case, etc.) |
| `spawn_blocking` pattern | `src/db/sqlite/mod.rs` | Async wrapper for synchronous rusqlite operations |
| RRF constants (k=60) | `src/search/executor_types.rs` | Standard RRF fusion constant |

## sqlite-vec Capabilities

The `sqlite-vec` extension provides vector operations:

```sql
-- Create vector table
CREATE VIRTUAL TABLE vec_items USING vec0(
  embedding float[1536]
);

-- Insert vectors
INSERT INTO vec_items(rowid, embedding) VALUES (1, vec_f32('[0.1, 0.2, ...]'));

-- Similarity search (KNN)
SELECT rowid, distance
FROM vec_items
WHERE embedding MATCH vec_f32('[0.1, 0.2, ...]')
ORDER BY distance
LIMIT 10;
```

Key characteristics:
- **MATCH operator** for KNN search (not <=> like pgvector)
- **distance** column returns L2 distance (lower = more similar)
- **float[N]** syntax for fixed dimensions
- Must be loaded at runtime via `sqlite3_load_extension()`

## FTS5 Capabilities

SQLite FTS5 provides full-text search:

```sql
-- Create FTS table
CREATE VIRTUAL TABLE fts_chunks USING fts5(
  content, docstring, symbol_name,
  content='chunks',
  content_rowid='id'
);

-- Search with ranking
SELECT rowid, rank
FROM fts_chunks
WHERE fts_chunks MATCH 'authentication'
ORDER BY rank;  -- More negative = better match
```

Key characteristics:
- **External content table** syncs with chunks
- **rank** is BM25-based (negative values, lower = better)
- Query syntax: `term*` (prefix), `"exact phrase"`, `term1 AND term2`

## Hybrid Search Strategy

Combine FTS5 and sqlite-vec results using Reciprocal Rank Fusion:

```
RRF Score = 1/(k + rank_fts) + 1/(k + rank_vector)

Where k = 60 (standard constant)
```

Implementation approach:
1. Run FTS5 query → Get ranked results with rowids
2. Run vector query → Get ranked results with rowids
3. Merge by chunk_id, compute RRF scores
4. Sort by combined score
5. Apply semantic multipliers (kind, exact match)

## Embedding Deduplication Strategy

Store one embedding per unique content (blob_sha):

```sql
CREATE TABLE code_embeddings (
  blob_sha TEXT PRIMARY KEY,
  code_embedding BLOB,      -- float[1536] as bytes
  text_embedding BLOB,      -- float[1536] as bytes
  model_version TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE VIRTUAL TABLE vec_embeddings USING vec0(
  embedding float[1536]
);
```

Lookup flow:
1. Hash chunk content → blob_sha
2. Check if blob_sha exists in code_embeddings
3. If exists: reuse embedding
4. If new: generate embedding, store once

Join for search:
```sql
SELECT c.*, vec.distance
FROM chunks c
JOIN code_embeddings e ON c.blob_sha = e.blob_sha
JOIN vec_embeddings vec ON vec.rowid = e.rowid
WHERE vec.embedding MATCH ?query_vector
```

## worktree_ids Strategy

**Current problem**: JSON array allows duplicates

**Options considered**:

1. **Junction table** (normalized):
   ```sql
   CREATE TABLE chunk_worktrees (
     chunk_id INTEGER REFERENCES chunks(id),
     worktree_id INTEGER REFERENCES worktrees(id),
     PRIMARY KEY (chunk_id, worktree_id)
   );
   ```
   - Pro: No duplicates possible
   - Con: More JOINs, schema divergence from PostgreSQL

2. **JSON with dedup check** (current approach, fixed):
   ```sql
   worktree_ids = CASE
     WHEN json_contains(worktree_ids, json(?)) THEN worktree_ids
     ELSE json_insert(worktree_ids, '$[#]', ?)
   END
   ```
   - Pro: Same schema as PostgreSQL
   - Con: O(n) contains check

**Decision**: Use junction table for SQLite.

Rationale:
- We're not maintaining PostgreSQL parity
- Junction table is SQLite-idiomatic
- Eliminates entire class of bugs
- Better query performance with indexes

> **Note**: No data migration needed. There are no existing SQLite databases with data. Fresh indexing populates all tables including the junction table.

## Graph Traversal Strategy

The `chunk_edges` table stores relationships:

```sql
CREATE TABLE chunk_edges (
  source_chunk_id INTEGER REFERENCES chunks(id),
  target_chunk_id INTEGER REFERENCES chunks(id),
  edge_type TEXT NOT NULL,  -- 'calls', 'imports', 'extends'
  PRIMARY KEY (source_chunk_id, target_chunk_id, edge_type)
);
```

Recursive CTE for multi-hop traversal:

```sql
WITH RECURSIVE callers AS (
  -- Base: direct callers
  SELECT source_chunk_id, 1 as depth
  FROM chunk_edges
  WHERE target_chunk_id = ?target AND edge_type = 'calls'

  UNION ALL

  -- Recursive: callers of callers
  SELECT e.source_chunk_id, c.depth + 1
  FROM chunk_edges e
  JOIN callers c ON e.target_chunk_id = c.source_chunk_id
  WHERE c.depth < ?max_depth AND e.edge_type = 'calls'
)
SELECT DISTINCT source_chunk_id FROM callers;
```

## Success Criteria

A complete SQLite implementation must support:

1. **Full CRUD cycle**: repos → worktrees → commits → files → chunks → edges
2. **FTS5 search**: Text search with BM25 ranking
3. **Vector search**: Semantic similarity via sqlite-vec
4. **Hybrid search**: RRF fusion of FTS + vector results
5. **Embedding deduplication**: One embedding per blob_sha
6. **Multi-worktree tracking**: Junction table approach
7. **Graph traversal**: Recursive CTEs for caller/callee
8. **Semantic ranking**: Kind multipliers, exact match boosts

## Future Enhancements (Post-MVP)

The following items are explicitly deferred to post-MVP:

1. **768-dim embedding support** - Ollama integration, separate vec_code_768 table
2. **Custom FTS column weights** - BM25 weight tuning per column
3. **Database encryption** - SQLite Encryption Extension (SEE)
4. **Multi-worktree deduplication** - Optimization for shared chunks

## Out of Scope

- VSCode extension integration (separate project)
- PostgreSQL parity or shared abstractions
- Migration from PostgreSQL to SQLite
- Multi-user/networked SQLite access
