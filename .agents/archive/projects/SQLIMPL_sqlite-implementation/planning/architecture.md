# Architecture: SQLite Implementation Completion

## Overview

This document describes the architecture for completing the SQLite implementation in the maproom crate. **Critical insight:** Most search database operations are already implemented in `src/db/sqlite/`. The stub implementations should **delegate** to these existing methods, not reimplement SQL.

## Key Architectural Principle: Delegation, Not Reimplementation

```
┌────────────────────────────────────────────────────────────────────────┐
│  EXISTING: src/db/sqlite/*.rs contains all SQL implementations         │
│  ├── fts.rs      → FTS5 queries, rank normalization                   │
│  ├── vector.rs   → sqlite-vec queries, distance→similarity            │
│  ├── graph.rs    → Recursive CTE traversal                            │
│  └── mod.rs      → SqliteStore with search_chunks_* methods           │
└────────────────────────────────────────────────────────────────────────┘
                                    ↑
                            DELEGATE TO
                                    ↑
┌────────────────────────────────────────────────────────────────────────┐
│  STUBS: src/search/*.rs, src/context/*.rs, src/incremental/*.rs       │
│  These should call SqliteStore methods, NOT write new SQL              │
└────────────────────────────────────────────────────────────────────────┘
```

## Existing Architecture

### Database Layer

```
┌─────────────────────────────────────────────────────────────┐
│                       SqliteStore                           │
│  (src/db/sqlite/mod.rs - primary database abstraction)      │
├─────────────────────────────────────────────────────────────┤
│  Tables:                                                    │
│  ├── chunks (id, repo, worktree, relpath, kind, content)   │
│  ├── fts_chunks (FTS5 virtual table for full-text search)  │
│  ├── code_embeddings (blob_sha, provider, embedding)        │
│  ├── vec_code / vec_code_768 (sqlite-vec virtual tables)   │
│  ├── chunk_edges (src_chunk_id, dst_chunk_id, type)        │
│  ├── files (id, repo_id, relpath, blob_sha)                │
│  ├── commits (id, repo_id, sha, committed_at)              │
│  └── chunk_worktrees (chunk_id, worktree_id)               │
└─────────────────────────────────────────────────────────────┘
```

### Existing SqliteStore Methods to Leverage

**DO NOT REIMPLEMENT THESE - they already work:**

| Method | File | What It Does |
|--------|------|--------------|
| `search_chunks_fts()` | mod.rs:611-748 | FTS5 search with proper joins and ranking |
| `search_chunks_vector()` | mod.rs:750-856 | Vector similarity via sqlite-vec |
| `search_chunks_hybrid()` | mod.rs:857+ | Combined FTS + vector with RRF fusion |
| `find_callers()` | mod.rs:1783-1799 | Recursive CTE for caller relationships |
| `find_callees()` | mod.rs:1800-1817 | Recursive CTE for callee relationships |
| `find_imports()` | mod.rs:1818-1836 | Import relationship traversal |
| `find_extensions()` | mod.rs:1837-1868 | Class inheritance traversal |

**Helper functions already implemented:**

| Function | File | Purpose |
|----------|------|---------|
| `normalize_fts_rank()` | fts.rs:28-30 | Convert FTS rank to 0-1 scale |
| `build_fts_query()` | fts.rs:41-71 | Sanitize user input for FTS5 |
| `distance_to_similarity()` | vector.rs:30-34 | Convert L2 distance to 0-1 score |
| `search_vector()` | vector.rs:46+ | Low-level vector search |

## Implementation Strategy

### Pattern 1: Search Executors (DELEGATION)

Search executors should delegate to SqliteStore, not write SQL:

```rust
// WRONG - Don't do this (reimplementing):
pub async fn execute(&self, query: &SearchQuery) -> Result<RankedResults> {
    let sql = "SELECT ... FROM fts_chunks ...";  // NO!
    let db = self.store.get_connection()?;
    // ... lots of SQL code ...
}

// CORRECT - Delegate to existing implementation:
pub async fn execute(&self, query: &SearchQuery) -> Result<RankedResults> {
    // Use existing SqliteStore method
    let hits = self.store.search_chunks_fts(
        &query.repo,
        query.worktree.as_deref(),
        &query.text,
        query.limit as i64,
        false,  // debug
    ).await?;

    // Convert SearchHit to RankedResult
    let results = hits.into_iter().map(|hit| {
        RankedResult {
            chunk_id: hit.chunk_id,
            score: hit.score,  // Already normalized by SqliteStore
            source: SearchSource::FTS,
            // ...
        }
    }).collect();

    Ok(RankedResults::new(results, SearchSource::FTS))
}
```

### Pattern 2: Graph Executor (DELEGATION)

```rust
// CORRECT - Use existing graph traversal:
pub async fn execute(&self, seed_chunks: &[i64]) -> Result<RankedResults> {
    let mut results = Vec::new();

    for chunk_id in seed_chunks {
        // Use existing find_callers/find_callees
        let callers = self.store.find_callers(*chunk_id, Some(2)).await?;
        let callees = self.store.find_callees(*chunk_id, Some(2)).await?;

        // Score based on graph distance
        for result in callers.into_iter().chain(callees) {
            let score = 1.0 / (result.depth as f64 + 1.0);
            results.push(RankedResult {
                chunk_id: result.chunk_id,
                score,
                source: SearchSource::Graph,
            });
        }
    }

    Ok(RankedResults::new(results, SearchSource::Graph))
}
```

### Pattern 3: Context Cache (NEW IMPLEMENTATION NEEDED)

The context cache stubs require new SQL because no SqliteStore methods exist for caching:

```rust
// This DOES need implementation - no existing method
pub async fn get(&self, key: &str) -> Result<Option<CachedContext>> {
    self.store.run(|conn| {
        let result = conn.query_row(
            "SELECT chunk_ids, created_at FROM context_cache WHERE cache_key = ?",
            [key],
            |row| {
                let chunk_ids_json: String = row.get(0)?;
                let created_at: String = row.get(1)?;
                Ok(CachedContext {
                    chunk_ids: serde_json::from_str(&chunk_ids_json)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)?,
                })
            }
        ).optional()?;
        Ok(result)
    }).await
}
```

### Pattern 4: Incremental Updates (NEW IMPLEMENTATION NEEDED)

The incremental module stubs require new SQL for hash storage:

```rust
// This DOES need implementation - no existing method
pub async fn get_hash_from_db(&self, file_id: i64) -> Result<Option<String>> {
    self.store.run(move |conn| {
        conn.query_row(
            "SELECT content_hash FROM files WHERE id = ?",
            [file_id],
            |row| row.get(0)
        ).optional()
        .map_err(Into::into)
    }).await
}
```

## Component Details

### Search Executors (Delegation)

| Executor | Delegates To | Conversion Needed |
|----------|-------------|-------------------|
| `FtsExecutor` | `SqliteStore::search_chunks_fts()` | SearchHit → RankedResult |
| `VectorExecutor` | `SqliteStore::search_chunks_vector()` | SearchHit → RankedResult |
| `GraphExecutor` | `SqliteStore::find_callers/callees()` | GraphResult → RankedResult |
| `SignalsExecutor` | Needs NEW impl (commits table) | Query committed_at for recency |

### Context Assembly (Mix of Delegation and New)

| Component | Implementation Approach |
|-----------|------------------------|
| `cache.rs` | NEW - query context_cache table |
| `graph.rs` | DELEGATE - use find_callers/callees/imports |
| `detectors/*.rs` | DELEGATE - use find_imports, find_extensions |
| `strategies/*.rs` | DELEGATE - combine existing graph methods |

### Incremental Module (Mostly New)

| Component | Implementation Approach |
|-----------|------------------------|
| `detector.rs` | NEW - query/update file hashes |
| `processor.rs` | NEW - insert/update/delete chunks |
| `edge_updater.rs` | NEW - compute and store chunk_edges |
| `tree_sha_update.rs` | NEW - track worktree state |

## What's Already Implemented vs What Needs Work

### Already Implemented (Just Wire Up)
- FTS search → `search_chunks_fts()`
- Vector search → `search_chunks_vector()`
- Hybrid search → `search_chunks_hybrid()`
- Graph traversal → `find_callers()`, `find_callees()`, `find_imports()`, `find_extensions()`
- Score normalization → `normalize_fts_rank()`, `distance_to_similarity()`

### Needs New Implementation
- Context cache operations (8 methods)
- Incremental hash storage/retrieval (4 methods)
- Incremental chunk processing (3 methods)
- Edge computation and storage (4 methods)
- Signals/recency scoring (query commits table)

## Technology Decisions

### Keep As-Is
- **rusqlite** - Mature, well-tested SQLite bindings
- **sqlite-vec** - Already vendored and integrated
- **FTS5** - Already configured in schema
- **tokio** - Async runtime for non-blocking I/O
- **SqliteStore pattern** - Use `store.run(|conn| { ... })` for all DB access

### Implementation Notes

1. **Use SqliteStore::run()** - All database access should go through this for connection pooling
2. **Reuse existing queries** - Don't copy-paste SQL from SqliteStore, call the methods
3. **Score normalization** - Use existing `normalize_fts_rank()` and `distance_to_similarity()`
4. **Error handling** - Convert `rusqlite::Error` to `anyhow::Error` via `.map_err(Into::into)`

## Validation Approach

After each phase, validate:

1. **Phase 1 (Tests)**: `cargo test -p crewchief-maproom` compiles
2. **Phase 2 (Search)**: Executors call SqliteStore methods, return non-empty results
3. **Phase 3 (Incremental)**: File modifications are detected and indexed
4. **Phase 4 (Context)**: Context assembly returns expanded results
5. **Phase 5 (Watch)**: `watch` command monitors and updates continuously

## Out of Scope

- Schema migrations (not needed)
- PostgreSQL restoration (explicitly excluded)
- New features (focus on completing existing)
- Performance optimization beyond baseline (future work)
- Reimplementing SQL that already exists in SqliteStore
