# Execution Plan: SQLite-Only Migration

## Overview

Remove PostgreSQL support entirely, making SQLite the only database backend. This simplifies the codebase by deleting PostgreSQL-specific files (~5 in db/), removing the VectorStore trait abstraction, eliminating feature flags, and refactoring ~26 files with PostgreSQL references (~140 references total).

**Estimated Duration**: 60-90 hours (Phases 1-5: 20-29 hours + Phase 6: 40-60 hours)
**Total Tickets**: 18 (14 original + 4 completion)
**Primary Agent**: rust-indexer-engineer

> **Note**: The original Phase 1-5 work is complete but resulted in stubbed implementations. Phase 6 was added to complete the full migration.

## Phase 1: Delete PostgreSQL Code

**Goal**: Remove all PostgreSQL-specific files and dependencies.

### Ticket 1001: Delete PostgreSQL Database Files

**Summary**: Remove all PostgreSQL-specific files from the db/ module.

**Scope**:
- Delete `db/postgres/` directory entirely
- Delete `db/pool.rs` (PostgreSQL connection pooling)
- Delete `db/queries.rs` (28 PostgreSQL-specific query functions)
- Delete `db/factory.rs` (backend switching)
- Delete `db/materialized_views.rs` (PostgreSQL materialized views)

**Acceptance Criteria**:
- [ ] All listed files deleted
- [ ] `cargo check` shows missing import errors (expected)
- [ ] No PostgreSQL files remain in db/

**Agent**: rust-indexer-engineer

### Ticket 1002: Simplify db/mod.rs and connection.rs

**Summary**: Remove VectorStore trait, add simple connect() function, simplify connection logic.

**Scope**:
- Remove `VectorStore` trait definition
- Remove `BackendType` enum
- Remove PostgreSQL imports and feature flags
- Add `pub use sqlite::SqliteStore;`
- Add `pub async fn connect() -> Result<SqliteStore>`
- Export sqlite module directly
- **Simplify `db/connection.rs`:**
  - Remove PostgreSQL fallback logic
  - Remove `#[cfg(feature = "sqlite")]` conditionals
  - Keep SQLite-only URL resolution

**Acceptance Criteria**:
- [ ] `db::connect()` returns `SqliteStore`
- [ ] No VectorStore trait in codebase
- [ ] `connection.rs` has no PostgreSQL references
- [ ] `cargo check` shows fewer errors (trait usage errors)

**Agent**: rust-indexer-engineer

### Ticket 1003: Update Cargo.toml

**Summary**: Remove PostgreSQL dependencies and feature flags.

**Scope**:
- Remove `tokio-postgres` dependency
- Remove `pgvector` dependency
- Remove `deadpool-postgres` dependency
- Remove `deadpool` dependency
- **Remove feature flags:**
  - Remove `default = ["postgres"]`
  - Remove `postgres = []` feature
  - Remove `sqlite = ["rusqlite", "r2d2", "r2d2_sqlite"]` (no longer optional)
- **Make SQLite deps required (not optional):**
  - `rusqlite` - remove `optional = true`
  - `r2d2` - remove `optional = true`
  - `r2d2_sqlite` - remove `optional = true`
- Document remaining compilation errors for Phase 2 reference

**Acceptance Criteria**:
- [ ] `cargo build` succeeds with just SQLite deps
- [ ] No PostgreSQL crates in Cargo.lock
- [ ] No `default = ["postgres"]` in features
- [ ] No `sqlite` feature flag (SQLite is default)
- [ ] `rusqlite`, `r2d2`, `r2d2_sqlite` are required dependencies
- [ ] Document list of compilation errors remaining for Phase 2

**Agent**: rust-indexer-engineer

## Phase 2: Refactor Core Modules

**Goal**: Update indexer, embedding, and other modules to use SqliteStore directly.

### Ticket 2001: Refactor Indexer Module

**Summary**: Update indexer/mod.rs to use SqliteStore instead of &Client.

**Scope**:
- Change all function signatures from `&Client` to `&SqliteStore`
- Replace `db::*` calls with `store.*` method calls
- Remove `scan_worktree_parallel` (merge into scan_worktree)
- Update helper functions (`process_file_with_treesitter`, etc.)
- Delete or merge `indexer/parallel.rs`

**Acceptance Criteria**:
- [ ] `scan_worktree(&store, ...)` compiles
- [ ] `upsert_files(&store, ...)` compiles
- [ ] `watch_worktree(&store, ...)` compiles
- [ ] No `&Client` references in indexer/

**Agent**: rust-indexer-engineer

### Ticket 2002: Refactor Embedding Pipeline

**Summary**: Update embedding/pipeline.rs to use SqliteStore, implementing missing methods.

**Scope**:
- Change all method signatures from `&Client` to `&SqliteStore`
- Replace raw SQL queries with SqliteStore method calls
- **Implement missing SqliteStore methods:**
  - `get_chunks_needing_embeddings_count()` - Returns count of chunks with NULL embeddings
  - `copy_existing_embeddings_from_cache()` - Bulk copy from code_embeddings to chunks
  - `fetch_chunks_needing_embeddings(incremental: bool, sample_size: Option<usize>)` - Get chunk data for embedding generation
- Update `EmbeddingPipeline::run()`, `copy_existing_embeddings()`, etc.

**Acceptance Criteria**:
- [ ] SqliteStore has all methods required by EmbeddingPipeline
- [ ] EmbeddingPipeline works with SqliteStore
- [ ] No raw SQL queries in pipeline (use store methods)
- [ ] No `tokio_postgres` imports in embedding/

**Agent**: rust-indexer-engineer

### Ticket 2003: Refactor Search Module

**Summary**: Update ALL search modules to use SqliteStore.

**Scope**:
- Update all files in `search/` directory with PostgreSQL references:
  - `search/pipeline.rs` (2 refs)
  - `search/fts.rs` (2 refs)
  - `search/vector.rs` (6 refs)
  - `search/graph.rs` (3 refs)
  - `search/signals.rs` (4 refs)
  - `search/executors.rs` (2 refs)
  - `search/mod.rs` (2 refs)
- Check and update if needed: `query_processor.rs`, `results.rs`, `dedup.rs`, `cache.rs`, `warming.rs`, `expander.rs`, `tokenizer.rs`, `types.rs`, `fusion/` subdirectory
- Replace `&Client` with `&SqliteStore` throughout

**Acceptance Criteria**:
- [ ] All search tests pass
- [ ] No `&Client` references in search/
- [ ] No `tokio_postgres` imports in search/
- [ ] Verification: `grep -r "tokio_postgres\|&Client" crates/maproom/src/search/` returns nothing
- [ ] Search functionality unchanged

**Agent**: rust-indexer-engineer

### Ticket 2004: Refactor Context Module

**Summary**: Update ALL context modules to use SqliteStore.

**Scope**:
- Update core files with PostgreSQL references:
  - `context/relationships.rs` (8 refs)
  - `context/graph.rs` (6 refs)
  - `context/assembler.rs` (verify)
  - `context/cache.rs` (verify)
- **Update detector files with PostgreSQL references:**
  - `context/detectors/hooks.rs` (4 refs)
  - `context/detectors/jsx.rs` (4 refs)
  - `context/detectors/component.rs` (verify)
  - `context/detectors/mod.rs` (verify)
- Check and update if needed: `strategies/` subdirectory, other files
- Replace `&Client` with `&SqliteStore` throughout

**Acceptance Criteria**:
- [ ] All context tests pass
- [ ] No `&Client` references in context/
- [ ] No `tokio_postgres` imports in context/
- [ ] Verification: `grep -r "tokio_postgres\|&Client" crates/maproom/src/context/` returns nothing
- [ ] Context assembly functionality unchanged

**Agent**: rust-indexer-engineer

### Ticket 2005: Refactor db Support Files and Migrate Module

**Summary**: Update db/cleanup.rs, db/index_state.rs, and migrate/markdown.rs.

**Scope**:
- Update `db/cleanup.rs` to use SqliteStore
- Update `db/index_state.rs` to use SqliteStore
- **Update `migrate/markdown.rs`** (2 PostgreSQL refs):
  - Replace `&Client` with `&SqliteStore`
  - Update any PostgreSQL-specific queries
- Remove any PostgreSQL-specific logic

**Acceptance Criteria**:
- [ ] Cleanup commands work
- [ ] Index state tracking works
- [ ] No `&Client` references in db/cleanup.rs, db/index_state.rs
- [ ] No `&Client` references in migrate/markdown.rs
- [ ] Verification: `grep -r "tokio_postgres\|&Client" crates/maproom/src/migrate/` returns nothing

**Agent**: rust-indexer-engineer

### Ticket 2006: Refactor Incremental Module

**Summary**: Update ALL incremental modules to use SqliteStore.

**Scope**:
- Update files in `incremental/` directory with PostgreSQL references:
  - `incremental/edge_updater.rs` (4 refs)
  - `incremental/processor.rs` (1 ref)
  - `incremental/tree_sha_update.rs` (3 refs)
- Replace `&Client` with `&SqliteStore` throughout
- Update any raw SQL queries to use SqliteStore methods

**Acceptance Criteria**:
- [ ] All incremental tests pass
- [ ] No `&Client` references in incremental/
- [ ] No `tokio_postgres` imports in incremental/
- [ ] Verification: `grep -r "tokio_postgres\|&Client" crates/maproom/src/incremental/` returns nothing
- [ ] Watch command works correctly (uses incremental module)

**Agent**: rust-indexer-engineer

### Ticket 2007: Refactor Upsert Module

**Summary**: Update upsert.rs to use SqliteStore.

**Scope**:
- Update `upsert.rs` (7 PostgreSQL refs):
  - Replace `tokio_postgres::Client` with `SqliteStore`
  - Update cache-aware chunk upserting logic
  - Update any raw SQL queries to use SqliteStore methods
- Ensure embedding deduplication by blob_sha works with SqliteStore

**Acceptance Criteria**:
- [ ] Upsert functionality works
- [ ] No `&Client` references in upsert.rs
- [ ] No `tokio_postgres` imports in upsert.rs
- [ ] Verification: `grep -r "tokio_postgres\|&Client" crates/maproom/src/upsert.rs` returns nothing
- [ ] Cache-aware upserting unchanged in behavior

**Agent**: rust-indexer-engineer

## Phase 3: Main.rs Cleanup

**Goal**: Remove backend switching and SQLite blockers from CLI.

### Ticket 3001: Clean Up main.rs

**Summary**: Remove all backend switching logic and SQLite blockers.

**Scope**:
- Remove `BackendType` enum usage
- Remove `get_store_with_type()` function
- Remove all `if backend_type == BackendType::SQLite` blocks
- Update all command handlers to use `db::connect()`
- Remove `--parallel` flag or make it a no-op
- Update `auto_generate_embeddings()` to use SqliteStore

**Acceptance Criteria**:
- [ ] `scan` command works
- [ ] `upsert` command works
- [ ] `watch` command works
- [ ] `generate-embeddings` command works
- [ ] `search` command works
- [ ] No backend type checks in main.rs

**Agent**: rust-indexer-engineer

## Phase 4: Testing & Validation

**Goal**: Ensure all functionality works and tests pass.

### Ticket 4001: Fix and Update Tests

**Summary**: Update tests to use SqliteStore, delete PostgreSQL tests.

**Scope**:
- Remove `#[cfg(feature = "sqlite")]` guards
- Delete PostgreSQL-specific tests
- Update test helpers to create SqliteStore
- Fix any broken tests

**Acceptance Criteria**:
- [ ] `cargo test` passes
- [ ] No feature flags in tests
- [ ] All test helpers use SqliteStore

**Agent**: rust-indexer-engineer

### Ticket 4002: E2E Validation Script

**Summary**: Create script to validate full workflow.

**Scope**:
- Script exercises: scan → generate-embeddings → search → upsert
- Uses default SQLite database location
- Documents expected output

**Acceptance Criteria**:
- [ ] Script runs successfully
- [ ] Full workflow works end-to-end
- [ ] Added to CI

**Agent**: rust-indexer-engineer

## Phase 5: Documentation

**Goal**: Update documentation to reflect SQLite-only architecture.

### Ticket 5001: Update CLAUDE.md

**Summary**: Update crate documentation.

**Scope**:
- Remove PostgreSQL references from `crates/maproom/CLAUDE.md`
- Update environment variable documentation
- Update example commands

**Acceptance Criteria**:
- [ ] Documentation accurate for SQLite-only
- [ ] Examples work as documented

**Agent**: rust-indexer-engineer

## Agent Assignments

| Agent | Tickets | Role |
|-------|---------|------|
| rust-indexer-engineer | All | Primary implementer |
| verify-ticket | All | Acceptance verification |
| commit-ticket | All | Git commits |

## Dependencies

```
Phase 1: 1001 → 1002 → 1003
          ↓
Phase 2: 2001 → 2002 → 2003 → 2004 → 2005 → 2006 → 2007
          ↓
Phase 3: 3001
          ↓
Phase 4: 4001 → 4002
          ↓
Phase 5: 5001
```

Note: Phase 2 tickets 2001-2007 can be partially parallelized as they touch different modules, but sequential execution is safer due to potential shared dependencies.

## Success Metrics

### Must Have
- All commands work without `--features sqlite`
- `cargo test` passes
- Database at `~/.maproom/maproom.db` by default

### Should Have
- Clean, simple codebase
- Fast compilation

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Breaking existing functionality | Test incrementally |
| Missing SqliteStore methods | Add as needed during refactor |
| Test failures | Fix as encountered |

## Phase 6: Completion (Post-Review Addition)

**Goal**: Complete stubbed implementations and ensure all tests pass.

> **Background**: The original migration (Phases 1-5) left many functions as stubs with TODO comments. Phase 6 was added after project review to complete the work.

### Ticket 6001: Migrate Test Files to SQLite

**Summary**: Migrate 29 test files from PostgreSQL to SQLite.

**Scope**:
- Migrate all test files that reference `tokio_postgres`, `PgPool`, or `postgres::`
- Update test helpers to use `SqliteStore::connect(":memory:")`
- Delete PostgreSQL-specific tests that don't apply

**Acceptance Criteria**:
- [ ] All tests compile (`cargo test -p crewchief-maproom --no-run`)
- [ ] No PostgreSQL references in tests
- [ ] Core tests pass

**Agent**: rust-indexer-engineer
**Estimated**: 8-12 hours

### Ticket 6002: Implement Incremental Module

**Summary**: Implement 13 stubbed functions in the incremental module.

**Scope**:
- `processor.rs`: index_new_file(), update_file(), remove_file()
- `detector.rs`: get_hash_from_db(), store_hash_in_db(), detect_move()
- `edge_updater.rs`: update_edges(), compute_edges(), find_test_targets(), insert_edges()
- `tree_sha_update.rs`: remove_worktree_from_chunks(), incremental_update()

**Acceptance Criteria**:
- [ ] All incremental functions implemented
- [ ] Incremental tests pass
- [ ] Upsert command triggers incremental updates

**Agent**: rust-indexer-engineer
**Estimated**: 10-15 hours

### Ticket 6003: Implement Watch Command

**Summary**: Replace watch command stub with working implementation.

**Scope**:
- Integrate with implemented incremental module
- Use notify crate for file watching
- Support NDJSON output for VSCode extension

**Acceptance Criteria**:
- [ ] `crewchief-maproom watch` starts watching
- [ ] File changes are detected and indexed
- [ ] Ctrl+C terminates gracefully

**Agent**: rust-indexer-engineer
**Estimated**: 6-10 hours

### Ticket 6004: Complete Remaining Stubs

**Summary**: Implement 52 TODO stubs across 21 files (minus overlap with 6002).

**Scope** (by priority):
1. **High**: Search module (7 TODOs) - recency, churn, graph, fts, vector
2. **High**: Context assembler (3 TODOs) - context assembly
3. **Medium**: Context cache (8 TODOs) - caching operations
4. **Medium**: Context graph (3 TODOs) - relationship queries
5. **Lower**: Language strategies (6 TODOs) - React/Python/Rust context
6. **Lower**: Detectors (6 TODOs) - React hooks/JSX
7. **Lower**: Other (5 TODOs) - migrate, embedding, db scoring

**Acceptance Criteria**:
- [ ] All TODOs either implemented or documented as intentionally deferred
- [ ] Search functionality complete
- [ ] Context assembly complete

**Agent**: rust-indexer-engineer
**Estimated**: 16-23 hours

> **Note**: Consider splitting 6004 into sub-tickets if scope becomes unwieldy during execution.

## Timeline

| Phase | Tickets | Estimated Duration | Status |
|-------|---------|-------------------|--------|
| Phase 1 | 1001-1003 | 4-5 hours | ✅ Complete |
| Phase 2 | 2001-2007 | 10-15 hours | ⚠️ Stubbed |
| Phase 3 | 3001 | 2-3 hours | ✅ Complete |
| Phase 4 | 4001-4002 | 3-4 hours | ⚠️ Partial |
| Phase 5 | 5001 | 1-2 hours | ✅ Complete |
| **Phase 6** | **6001-6004** | **40-60 hours** | 🔴 Not Started |
| **Total** | | **60-90 hours** | |
