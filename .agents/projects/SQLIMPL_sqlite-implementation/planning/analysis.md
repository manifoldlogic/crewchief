# Analysis: SQLite Implementation Completion

## Problem Definition

The maproom indexer (`crates/maproom/`) underwent a PostgreSQL removal effort that deleted the PostgreSQL backend code but left many functions as stubs returning placeholder values. The crate compiles and basic commands work, but:

1. **52 TODO stubs** across 23 files return empty results or no-ops
2. **35 test files** still reference PostgreSQL types and won't run
3. **Core features non-functional**: Search returns empty, context assembly disabled, incremental updates don't persist

This is not a migration - the PostgreSQL code is gone. This is completing the SQLite implementation that was started but never finished.

## Current State Assessment

### What Works

```bash
cargo run --bin crewchief-maproom -- scan --path /repo      # Compiles, creates chunks
cargo run --bin crewchief-maproom -- upsert --paths src/    # Compiles, processes files
cargo run --bin crewchief-maproom -- generate-embeddings    # Works with Ollama
cargo run --bin crewchief-maproom -- search "function"      # Returns results (basic)
```

### What Doesn't Work

| Feature | Issue | Impact |
|---------|-------|--------|
| Semantic search ranking | FTS, vector, graph executors return empty | Search results not ranked properly |
| Context assembly | 19 methods stubbed, cache disabled | Cannot expand context around chunks |
| Incremental indexing | 13 methods stubbed, changes not persisted | File changes detected but not processed |
| Watch command | Disabled at CLI level | Returns error message |
| Test suite | 35 files reference PostgreSQL | Cannot validate functionality |

### Stub Analysis by Module

**Search Executors (7 stubs)** - `src/search/`
- `fts.rs:159` - Full-text search returns `RankedResults::empty()`
- `vector.rs:112` - Vector similarity returns empty
- `graph.rs:76,105` - Graph importance (2 methods) stubbed
- `signals.rs:86,116` - Recency/churn scoring stubbed

**Context Assembly (19 stubs)** - `src/context/`
- `cache.rs` - 8 methods: get/put/invalidate/evict all no-ops
- `graph.rs` - 3 methods: relationship queries unimplemented
- `detectors/jsx.rs` - 3 methods: JSX component detection
- `detectors/hooks.rs` - 3 methods: React hooks detection
- `strategies/*.rs` - 2 methods: Language-specific expansion

**Incremental Updates (13 stubs)** - `src/incremental/`
- `detector.rs` - 4 methods: hash queries, move detection
- `processor.rs` - 3 methods: file index/update/delete
- `edge_updater.rs` - 4 methods: relationship computation
- `tree_sha_update.rs` - 2 methods: worktree operations

**Database (2 stubs)** - `src/db/sqlite/`
- `mod.rs:818,841` - FTS kind multipliers not applied

## Root Cause Analysis

The previous project (IDXABS) conflated two separate concerns:

1. **Removing PostgreSQL** - Deleting files, traits, feature flags (completed)
2. **Implementing SQLite** - Writing actual query logic (never done)

The assumption was that stubbing methods would allow incremental implementation, but:
- Status tracking became unreliable (stubs marked "complete")
- No tests validated actual functionality
- Scope creep from "remove" to "implement" was untracked

## Research: SQLite-vec and FTS5

### sqlite-vec (Vector Search)
The crate already includes `sqlite-vec` as a vendored dependency:
- Located at `vendor/sqlite-vec/`
- Provides `vec0` virtual table for vector storage
- Supports cosine similarity via `vec_distance_cosine()`

Current schema already has vector support:
```sql
CREATE VIRTUAL TABLE IF NOT EXISTS chunk_embeddings_vec USING vec0(
    embedding FLOAT[1024] distance_metric=cosine
);
```

### FTS5 (Full-Text Search)
SQLite FTS5 is configured and working:
```sql
CREATE VIRTUAL TABLE chunks_fts USING fts5(
    content,
    content='chunks',
    content_rowid='id'
);
```

The infrastructure exists - the stub implementations just need to query these tables.

## Existing SqliteStore Methods (Already Implemented)

**CRITICAL DISCOVERY:** Many database operations are already fully implemented in `src/db/sqlite/`. Search executors should **delegate** to these methods, not reimplement SQL.

### Search Methods (src/db/sqlite/mod.rs)
| Method | Lines | Status | Description |
|--------|-------|--------|-------------|
| `search_chunks_fts()` | 611-748 | **Working** | FTS5 search with BM25 ranking |
| `search_chunks_vector()` | 750-856 | **Working** | Vector similarity with sqlite-vec |
| `search_chunks_hybrid()` | 857+ | **Working** | Combined FTS + vector search |
| `search_fts()` | 1600-1628 | **Working** | Simplified FTS wrapper |
| `search_vector()` | 1576-1599 | **Working** | Simplified vector wrapper |
| `search_hybrid()` | 1629-1658 | **Working** | Simplified hybrid wrapper |
| `search_hybrid_ranked()` | 1718-1782 | **Working** | Hybrid with semantic ranking |

### Graph Methods (src/db/sqlite/mod.rs + graph.rs)
| Method | Lines | Status | Description |
|--------|-------|--------|-------------|
| `find_callers()` | 1783-1799 | **Working** | Recursive CTE caller traversal |
| `find_callees()` | 1800-1817 | **Working** | Recursive CTE callee traversal |
| `find_imports()` | 1818-1836 | **Working** | Import relationship traversal |
| `find_extensions()` | 1837-1868 | **Working** | Extension/inheritance traversal |

### Helper Modules (src/db/sqlite/*.rs)
| Module | Status | Key Functions |
|--------|--------|---------------|
| `fts.rs` | **Working** | `build_fts_query()`, `normalize_fts_rank()`, `search_fts()` |
| `vector.rs` | **Working** | `search_vector()`, `distance_to_similarity()` |
| `graph.rs` | **Working** | `find_callers()`, `find_callees()`, `find_imports()`, `find_extensions()` |
| `hybrid.rs` | **Working** | `search_hybrid()`, RRF fusion logic |

### What This Means for Implementation

The **stub functions in `src/search/*.rs` should NOT reimplement SQL**. Instead:

```rust
// WRONG approach (reimplementing):
pub async fn execute(...) -> Result<RankedResults> {
    let sql = "SELECT ... FROM chunks_fts ...";  // Don't do this!
    // ...
}

// CORRECT approach (delegating):
pub async fn execute(...) -> Result<RankedResults> {
    let hits = self.store.search_chunks_fts(repo, worktree, query, limit, debug).await?;
    Ok(RankedResults::from_search_hits(hits, SearchSource::FTS))
}
```

The remaining work is primarily:
1. **Wiring** executors to existing methods
2. **Converting** return types (SearchHit → RankedResult)
3. **Applying** score normalization (functions already exist)

## Dependencies and Constraints

### Internal Dependencies
- `SqliteStore` - Primary database abstraction, **many methods already implemented**
- `ChunkStore` trait - Interface for chunk operations
- `EmbeddingStore` trait - Interface for vector operations

### External Dependencies
- `rusqlite` - SQLite bindings (already used)
- `sqlite-vec` - Vector operations (vendored)
- `tree-sitter` - Code parsing (working)
- `tokio` - Async runtime (working)

### Constraints
1. **No PostgreSQL restoration** - The code is deleted and should stay deleted
2. **Backward compatibility** - Existing database files must continue to work
3. **Performance** - Search must remain sub-second for typical codebases
4. **Memory** - Embedding operations should not exceed 4GB RAM

## Success Criteria

### Minimum Viable Completion

1. **All tests pass**: `cargo test -p crewchief-maproom` exits 0
2. **Search returns real results**: Not empty, properly ranked
3. **Incremental updates work**: File changes are persisted to database
4. **Watch command functional**: CLI `watch` subcommand works

### Extended Goals (if time permits)

1. **Context assembly complete**: Full graph traversal and language-aware expansion
2. **Performance benchmarks**: Document baseline performance
3. **Integration tests**: E2E tests for common workflows

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Test migration reveals more issues | High | Medium | Fix as discovered, don't block |
| sqlite-vec API different than expected | Medium | High | Read docs thoroughly before implementing |
| Performance regression | Low | Medium | Benchmark before/after |
| Database schema changes needed | Low | High | Avoid if possible, add migration if needed |

## Recommended Approach

### Phase 1: Test Infrastructure (Foundation)
Migrate the 35 test files from PostgreSQL to SQLite. This enables validation for all subsequent phases.

### Phase 2: Search Implementation (Core Value)
Implement the 7 search executor stubs. This delivers the primary feature users care about.

### Phase 3: Incremental Updates (Developer Experience)
Implement the 13 incremental module stubs. This enables efficient re-indexing.

### Phase 4: Context Assembly (Enhancement)
Implement the 19 context assembly stubs. This improves result quality.

### Phase 5: Watch Command (Convenience)
Enable the watch command at CLI level once incremental updates work.

## Conclusion

This project is well-scoped with clear deliverables:
- 52 stub implementations to complete
- 35 test files to migrate
- 1 CLI command to enable

The infrastructure (SQLite, sqlite-vec, FTS5) is already in place. The work is primarily implementing query logic to use existing tables and returning real data instead of placeholders.
