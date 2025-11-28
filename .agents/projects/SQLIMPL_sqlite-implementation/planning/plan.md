# Plan: SQLite Implementation Completion

## Executive Summary

Complete the SQLite implementation for maproom by:
1. Migrating 35 test files from PostgreSQL to SQLite
2. **Wiring** search executors to existing SqliteStore methods (not reimplementing)
3. Implementing incremental update stubs (genuine new code)
4. Optionally completing context assembly (enhancement)
5. Enabling the watch command

**Key Insight:** Search functionality is already implemented in `src/db/sqlite/*.rs`. Phase 2 is about wiring executors to existing methods, not writing new SQL.

## Phase Overview

| Phase | Focus | Tickets | Complexity | Status |
|-------|-------|---------|------------|--------|
| 1 | Test Infrastructure | 5 | Medium | ✅ Complete |
| 2 | Search Wiring | 4 | **Low** | ✅ Complete |
| 3 | Incremental Updates | 4 | High | ✅ Complete |
| 4 | Context Assembly | 4 | Medium | **Required** |
| 5 | Watch Command | 2 | Low | ✅ Complete |

**Total: 19 tickets** (all required)

## Phase 1: Test Infrastructure

**Objective:** Enable test compilation so all subsequent phases can be validated.

**Pre-Phase Step:** Triage test files to determine: migrate, delete, or defer.

### Ticket 1001: Migrate Test Common Module + Triage
- **Triage all 35 test files** - classify as migrate/delete/defer
- Convert `tests/common/mod.rs` from PostgreSQL to SQLite
- Create in-memory database helpers using `SqliteStore::connect(":memory:")`
- Establish test fixture patterns
- **Files:** `tests/common/mod.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** Common module compiles with SQLite, triage list documented

### Ticket 1002: Migrate Test Files Batch 1 (Integration)
- Migrate 4 integration test files in `tests/integration/`
- Migrate core e2e tests: `e2e_workflow_simple.rs`, `e2e_multi_provider.rs`
- **Files:** 6 test files
- **Agent:** rust-indexer-engineer
- **Acceptance:** Files compile and run (tests may fail if stubs not implemented)

### Ticket 1003: Migrate Test Files Batch 2 (Search)
- Migrate search-related tests
- Files: `search_pipeline_integration_test.rs`, `search_executors_test.rs`, `fusion_*.rs`, `rrf_fusion_test.rs`
- **Files:** 6 test files
- **Agent:** rust-indexer-engineer
- **Acceptance:** Files compile

### Ticket 1004: Migrate Test Files Batch 3 (Incremental)
- Migrate incremental-related tests
- Files: `incremental_*.rs` (7 files)
- **Files:** 7 test files
- **Agent:** rust-indexer-engineer
- **Acceptance:** Files compile

### Ticket 1005: Migrate Test Files Batch 4 (Remaining)
- Migrate remaining test files
- Files: `vector_db_test.rs`, `graph_test.rs`, `watch_*.rs`, `store_compat.rs`, etc.
- Delete tests for features that won't be implemented
- **Files:** ~16 test files
- **Agent:** rust-indexer-engineer
- **Acceptance:** All test files compile

**Phase Gate:** `cargo test -p crewchief-maproom --no-run` compiles successfully

## Phase 2: Search Wiring (Not Reimplementation!)

**Objective:** Wire search executors to existing SqliteStore methods.

**IMPORTANT:** DO NOT write new SQL. The following SqliteStore methods already work:
- `search_chunks_fts()` - FTS5 search
- `search_chunks_vector()` - Vector similarity
- `find_callers()`, `find_callees()` - Graph traversal

### Ticket 2001: Wire FTS Executor to SqliteStore
- **Delegate** `fts.rs:159` to `SqliteStore::search_chunks_fts()`
- Convert `SearchHit` → `RankedResult`
- Remove warning log about "not fully implemented"
- **Files:** `src/search/fts.rs`
- **Agent:** rust-indexer-engineer
- **Pattern:** `let hits = store.search_chunks_fts(...).await?;`
- **Acceptance:** FTS executor returns non-empty results

### Ticket 2002: Wire Vector Executor to SqliteStore
- **Delegate** `vector.rs:112` to `SqliteStore::search_chunks_vector()`
- Convert `SearchHit` → `RankedResult`
- Use existing `distance_to_similarity()` for score normalization
- **Files:** `src/search/vector.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** Vector executor returns non-empty results (with embeddings)

### Ticket 2003: Wire Graph Executor to SqliteStore
- **Delegate** `graph.rs:76,105` to `SqliteStore::find_callers()` and `find_callees()`
- Convert `GraphResult` → `RankedResult`
- Score based on graph depth: `1.0 / (depth + 1.0)`
- **Files:** `src/search/graph.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** Graph executor returns related chunks

### Ticket 2004: Implement Signals Executor (NEW CODE)
- **This requires new implementation** - query `commits.committed_at` for recency
- Implement `signals.rs:86,116` - Recency and churn scoring
- Query chunk metadata via file → commit relationship
- Apply decay function: `score = 1.0 / (days_old + 1.0)`
- **Files:** `src/search/signals.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** Signals executor returns recency-weighted scores

**Phase Gate:** `cargo run -- search "function"` returns non-empty, ranked results

## Phase 3: Incremental Updates

**Objective:** Make file change detection and persistence work.

**Note:** These are genuine new implementations - no existing SqliteStore methods for this.

### Ticket 3001: Implement Change Detector
- **Verify schema:** Confirm `files` table has hash storage capability
- Implement `detector.rs:309,407,437,453` - 4 methods
- `get_hash_from_db()` - Query `files.content_hash` or `files.blob_sha`
- `store_hash_in_db()` - Update file hash in database
- `detect_move()` - Compare blob_sha to find renamed files
- **Files:** `src/incremental/detector.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** Hash queries work, move detection identifies renames

### Ticket 3002: Implement Processor
- Implement `processor.rs:258,339,384` - 3 methods
- `index_new_file()` - Parse file, create chunks, insert to DB
- `update_file()` - Delete old chunks, insert new chunks
- `remove_file()` - Delete chunks for removed file
- **Files:** `src/incremental/processor.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** File changes persist to database

### Ticket 3003: Implement Edge Updater
- Implement `edge_updater.rs:114,184,247,261` - 4 methods
- `compute_edges()` - Use tree-sitter to find call/import relationships
- `insert_edges()` - Store edges in `chunk_edges` table
- `update_edges()` - Recompute edges for changed files
- `find_test_targets()` - Identify test → implementation relationships
- **Files:** `src/incremental/edge_updater.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** Chunk edges are computed and stored

### Ticket 3004: Implement Tree SHA Update
- Implement `tree_sha_update.rs:129,188` - 2 methods
- `remove_worktree_from_chunks()` - Clean up `chunk_worktrees` entries
- `incremental_update()` - Coordinate incremental re-index
- **Files:** `src/incremental/tree_sha_update.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** Worktree cleanup and incremental update work

**Phase Gate:** Modify a file, run `upsert`, verify database updated correctly

## Phase 4: Context Assembly

**Objective:** Make context expansion and caching work.

**Status:** REQUIRED - completes the full feature set.

### Ticket 4001: Implement Context Cache
- **Verify schema:** Confirm `context_cache` table exists
- Implement `cache.rs` - 8 methods
- `get()`, `put()` - Basic cache operations using `store.run()`
- `invalidate()` - Delete by cache_key prefix
- `evict_expired()`, `evict_lru_if_needed()` - Cache management
- **Files:** `src/context/cache.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** Cache stores and retrieves contexts

### Ticket 4002: Implement Context Graph
- **Delegate** to existing `find_callers()`, `find_callees()`, `find_imports()`
- Implement `graph.rs:95,121,150` - 3 methods
- Build relationship maps for context expansion
- **Files:** `src/context/graph.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** Context graph returns related chunks

### Ticket 4003: Implement Language Detectors
- Review stubs - may need tree-sitter queries, not just SQL
- Implement JSX detector: `detectors/jsx.rs:79,98,117` - 3 methods
- Implement hooks detector: `detectors/hooks.rs:118,135,159` - 3 methods
- **Files:** `src/context/detectors/jsx.rs`, `src/context/detectors/hooks.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** Detectors identify JSX components and hooks

### Ticket 4004: Implement Language Strategies
- Implement `strategies/react.rs` - React-specific context
- Implement `strategies/python.rs` - Python-specific context
- Implement `strategies/rust.rs` - Rust-specific context
- **Files:** `src/context/strategies/*.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** Language-specific context expansion works

**Phase Gate:** Context assembly returns expanded results with related chunks

## Phase 5: Watch Command

**Objective:** Enable continuous file monitoring.

**Dependency:** Requires Phase 3 (Incremental) to be complete.

### Ticket 5001: Enable Watch Command in CLI
- Remove "temporarily unavailable" error in `src/main.rs`
- Wire watch to incremental module
- Ensure file system watcher (notify crate) is configured
- **Files:** `src/main.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** `cargo run -- watch` starts without error

### Ticket 5002: Validate Watch Integration
- Verify watch detects file changes
- Verify changes trigger re-indexing via incremental module
- Update/enable integration test in `tests/watch_integration.rs`
- **Files:** `tests/watch_integration.rs`
- **Agent:** rust-indexer-engineer
- **Acceptance:** Watch command monitors and updates continuously

**Phase Gate:** `cargo run -- watch` monitors and updates continuously

## Agent Assignments

| Agent | Responsibility |
|-------|---------------|
| **rust-indexer-engineer** | Primary implementer for all tickets |
| **unit-test-runner** | Execute tests after each ticket |
| **verify-ticket** | Validate acceptance criteria |
| **commit-ticket** | Create git commits |

## Dependencies

```
Phase 1 (Tests) ──┬──▶ Phase 2 (Search)
                  ├──▶ Phase 3 (Incremental)
                  └──▶ Phase 4 (Context)

Phase 3 (Incremental) ──▶ Phase 5 (Watch)
```

- Phase 1 must complete first (enables validation)
- Phases 2, 3, 4 can proceed in parallel after Phase 1
- Phase 5 depends on Phase 3

## Success Criteria

### Full Completion
- [x] All tests compile: `cargo test -p crewchief-maproom --no-run`
- [x] All tests pass: `cargo test -p crewchief-maproom`
- [x] Search returns results: Non-empty, ranked
- [x] Incremental updates persist: File changes detected and indexed
- [x] Watch command works: Monitors and updates
- [ ] Context assembly complete: Related chunks returned
- [ ] Language-specific detection: JSX, hooks, Python patterns

### Quality Metrics
- No `TODO` comments in stub locations
- All parameterized queries (security)
- Executors delegate to SqliteStore (no duplicate SQL)

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Test migration complex | Triage first, batch by category, delete obsolete tests |
| sqlite-vec API issues | Already working in SqliteStore - just wire up |
| Context assembly complex | May need tree-sitter integration |
| Watch depends on incremental | Complete Phase 3 fully before Phase 5 |

## Execution Recommendations

1. **Phase 1:** Execute sequentially (1001 → 1005), triage in 1001 ✅ Complete
2. **Phase 2:** Quick wins - mostly wiring existing code ✅ Complete
3. **Phase 3:** Core implementation work, take time to get right ✅ Complete
4. **Phase 4:** Context assembly - required for full feature set
5. **Phase 5:** Only after Phase 3 is verified working ✅ Complete

## Audit Checklist (Before Each Phase)

Before starting implementation in any phase:
- [ ] Verify what SqliteStore methods already exist
- [ ] Check if stubs can delegate to existing methods
- [ ] Confirm database schema matches expectations
- [ ] Review related tests to understand expected behavior
