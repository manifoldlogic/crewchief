# VECSTORE Implementation Plan

## Project Summary

Complete the `VectorStore` trait abstraction by adding all missing database operations. This is the **foundation project** that must complete before MAPROOMCLI, MCPDB, VSCODEDB, SQLITEINFRA, or **EMBPERF** can begin.

**Priority**: Ollama (768-dim) support is CRITICAL - it enables the zero-config experience (SQLite + Ollama).

## Phase Overview

| Phase | Focus | Tickets | Dependencies |
|-------|-------|---------|--------------|
| 0 | **SQLite 768-dim Support** | VECSTORE-1000 | None (CRITICAL) |
| 1 | Search Methods | VECSTORE-1001, VECSTORE-1002 | Phase 0 |
| 2 | Context Methods | VECSTORE-1003 | Phase 1 |
| 3 | Repository Query Methods | VECSTORE-1004 | None |
| 4 | Index State Methods | VECSTORE-1005 | None |
| 5 | Cleanup Methods | VECSTORE-1006 | Phase 3 |
| 6 | Integration Testing | VECSTORE-1007 | All above |

## Phase 0: SQLite Multi-Dimension Support (CRITICAL)

### VECSTORE-1000: SQLite 768-dim Embedding Support

**Agent**: rust-indexer-engineer

**Objective**: Enable SQLite to store and search 768-dimensional embeddings (Ollama/nomic-embed-text).

**Priority**: CRITICAL - Blocks EMBPERF project and zero-config experience

**Context**:
- SQLite is currently hardcoded to 1536-dim only (OpenAI)
- Ollama produces 768-dim embeddings
- Zero-config = SQLite (no PostgreSQL) + Ollama (no API keys)
- EMBPERF project optimizes Ollama throughput but needs SQLite storage

**Scope**:
- Add 768-dim sqlite-vec virtual tables to schema
- Update migration to create new tables
- Update embeddings.rs to route by dimension
- Update vector.rs to query correct table based on embedding size
- Remove hardcoded 1536 validation
- Add dimension parity tests

**Sub-tasks**:
1. Add SQLite migration for 768-dim tables (`vec_code_embeddings_768`, `vec_text_embeddings_768`)
2. Create `get_vec_table_name(dimension)` routing function
3. Update `upsert_embedding()` to accept 768-dim embeddings
4. Update `batch_upsert_embeddings()` to handle mixed dimensions
5. Update `search_vector()` to query correct table
6. Update `search_hybrid()` to work with 768-dim
7. Add tests for 768-dim storage and retrieval
8. Add parity tests: same query returns same results from either dimension

**Acceptance Criteria**:
- [ ] SQLite migration creates 768-dim tables
- [ ] `upsert_embeddings(..., dimension=768)` succeeds
- [ ] `upsert_embeddings(..., dimension=1536)` continues to work
- [ ] `search_chunks_vector()` works with 768-dim query embedding
- [ ] `search_chunks_hybrid()` works with 768-dim embedding
- [ ] Existing 1536-dim data unaffected
- [ ] Tests pass with `--features sqlite`

## Phase 1: Search Methods

### VECSTORE-1001: Vector Search Methods

**Agent**: rust-indexer-engineer

**Objective**: Add vector similarity search to VectorStore trait.

**Scope**:
- Add `search_chunks_vector()` to VectorStore trait
- **Write `search_chunks_vector()` in queries.rs** (PostgreSQL function does NOT exist)
- Implement in PostgresStore (wrapping new queries.rs function)
- Implement in SqliteStore (wrapping sqlite/vector.rs)
- Add contract tests

**Sub-tasks**:
1. Write PostgreSQL vector search query using pgvector `<=>` operator
2. Add `search_chunks_vector()` function to queries.rs
3. Add trait method to VectorStore
4. Implement in PostgresStore
5. Implement in SqliteStore
6. Add contract tests

**Acceptance Criteria**:
- [ ] `search_chunks_vector` method added to trait
- [ ] PostgreSQL query function written in queries.rs
- [ ] PostgresStore implementation passes tests
- [ ] SqliteStore implementation passes tests (requires sqlite-vec)
- [ ] Graceful error when sqlite-vec not available

### VECSTORE-1002: Hybrid Search Methods

**Agent**: rust-indexer-engineer

**Objective**: Add hybrid (FTS + vector) search to VectorStore trait.

**Scope**:
- Add `search_chunks_hybrid()` to VectorStore trait
- **Write `search_chunks_hybrid()` in queries.rs** (PostgreSQL function does NOT exist)
- Implement in PostgresStore (RRF fusion of FTS + pgvector)
- Implement in SqliteStore (wrapping sqlite/hybrid.rs)
- Add parity tests

**Sub-tasks**:
1. Write PostgreSQL hybrid query combining ts_rank_cd + pgvector with RRF fusion
2. Add `search_chunks_hybrid()` function to queries.rs
3. Add trait method to VectorStore
4. Implement in PostgresStore
5. Implement in SqliteStore
6. Add parity tests (rank-order comparison, not absolute scores)

**Acceptance Criteria**:
- [ ] `search_chunks_hybrid` method added to trait
- [ ] PostgreSQL query function written in queries.rs
- [ ] PostgresStore implementation produces ranked results
- [ ] SqliteStore implementation produces equivalent ranking
- [ ] Parity test verifies similar top-k results (rank order, not scores)

## Phase 2: Context Methods

### VECSTORE-1003: Context Assembly Methods

**Agent**: rust-indexer-engineer

**Objective**: Add chunk context retrieval to VectorStore trait.

**Scope**:
- Define `ChunkFull`, `ChunkSummary`, `ChunkContext` types in `db/mod.rs`
- Add `get_chunk_by_id()`, `get_file_chunks()`, `get_chunk_context()` to trait
- **Write PostgreSQL query functions** (do NOT exist in queries.rs)
- Implement in PostgresStore
- Implement in SqliteStore

**Sub-tasks**:
1. Define `ChunkFull` type (read-only view with full content, related to `ChunkRecord`)
2. Define `ChunkSummary` type (lightweight chunk reference)
3. Define `ChunkContext` type (chunk + surrounding + related)
4. Write `get_chunk_by_id()` PostgreSQL query in queries.rs
5. Write `get_file_chunks()` PostgreSQL query in queries.rs
6. Write `get_chunk_context()` PostgreSQL query in queries.rs (simplified version)
7. Add trait methods to VectorStore
8. Implement in PostgresStore
9. Implement in SqliteStore
10. Add contract tests

**Note on Context Complexity**: The existing `context/assembler.rs` module has sophisticated context assembly logic (400+ lines). For VECSTORE, implement a simplified `get_chunk_context()` that returns surrounding chunks by line number. The full context assembly strategy pattern remains as higher-level code that uses VectorStore.

**Acceptance Criteria**:
- [ ] Domain types defined in `db/mod.rs`
- [ ] All three methods added to trait
- [ ] PostgreSQL query functions written in queries.rs
- [ ] PostgresStore implementations work
- [ ] SqliteStore implementations work
- [ ] `get_chunk_context` returns surrounding chunks (by line number)

## Phase 3: Repository Query Methods

### VECSTORE-1004: Repository and Worktree Query Methods

**Agent**: rust-indexer-engineer

**Objective**: Add repo/worktree lookup methods to VectorStore trait.

**Scope**:
- Define `RepoInfo`, `WorktreeInfo` types in `db/mod.rs`
- Add `get_repo_by_name()`, `get_worktree_by_name()`, `list_repos()`, `list_worktrees()` to trait
- **Write PostgreSQL query functions** (do NOT exist in queries.rs)
- Implement in both stores

**Sub-tasks**:
1. Define `RepoInfo` and `WorktreeInfo` types
2. Write `get_repo_by_name()` PostgreSQL query in queries.rs
3. Write `list_repos()` PostgreSQL query in queries.rs
4. Write `get_worktree_by_name()` PostgreSQL query in queries.rs
5. Write `list_worktrees()` PostgreSQL query in queries.rs
6. Add trait methods to VectorStore
7. Implement in PostgresStore
8. Implement in SqliteStore
9. Add contract tests

**Acceptance Criteria**:
- [ ] Domain types defined
- [ ] All four methods added to trait
- [ ] PostgreSQL query functions written in queries.rs
- [ ] PostgresStore implementations work
- [ ] SqliteStore implementations work
- [ ] `list_*` methods return empty vec for empty database

## Phase 4: Index State Methods

### VECSTORE-1005: Index State Management Methods

**Agent**: rust-indexer-engineer

**Objective**: Add index state tracking to VectorStore trait.

**Scope**:
- Add `get_last_indexed_tree()`, `update_index_state()` to trait
- Migrate from `db/index_state.rs` functions to trait methods
- Implement in both stores

**Acceptance Criteria**:
- [ ] Both methods added to trait
- [ ] PostgresStore wraps existing `index_state.rs` queries
- [ ] SqliteStore has equivalent implementation
- [ ] State persists across connections

## Phase 5: Cleanup Methods

### VECSTORE-1006: Cleanup and Maintenance Methods

**Agent**: rust-indexer-engineer

**Objective**: Add stale data detection and cleanup to VectorStore trait.

**Scope**:
- Define `StaleWorktree`, `CleanupReport` types (partially exist in cleanup.rs)
- Add `detect_stale_worktrees()`, `delete_worktree_data()` to trait
- Add `delete_chunks_by_file()`, `get_chunks_by_blob_sha()` for incremental ops
- **Write PostgreSQL query functions** (some exist in cleanup.rs, but need trait-friendly versions)
- Implement in both stores

**Sub-tasks**:
1. Define `StaleWorktree` type (already exists in cleanup.rs, may need to move to db/mod.rs)
2. Define `CleanupReport` type
3. Write `detect_stale_worktrees()` PostgreSQL query (refactor from StaleWorktreeDetector)
4. Write `delete_worktree_data()` PostgreSQL query
5. Write `delete_chunks_by_file()` PostgreSQL query
6. Write `get_chunks_by_blob_sha()` PostgreSQL query
7. Add trait methods to VectorStore
8. Implement in PostgresStore
9. Implement in SqliteStore
10. Add contract tests

**Acceptance Criteria**:
- [ ] All methods added to trait
- [ ] PostgreSQL query functions written (refactored from cleanup.rs)
- [ ] PostgresStore wraps cleanup functionality
- [ ] SqliteStore has equivalent implementation
- [ ] Cleanup returns accurate counts

## Phase 6: Integration Testing

### VECSTORE-1007: Contract and Parity Test Suite

**Agent**: integration-tester

**Objective**: Comprehensive test suite verifying trait completeness.

**Scope**:
- Create `tests/vectorstore_contract.rs` with backend-agnostic tests
- Create `tests/backend_parity.rs` comparing both backends
- Update CI to run both test suites

**Acceptance Criteria**:
- [ ] Contract tests pass for PostgresStore
- [ ] Contract tests pass for SqliteStore
- [ ] Parity tests verify equivalent results
- [ ] CI runs both backends in test matrix

## Agent Assignments

| Ticket | Primary Agent | Backup Agent |
|--------|--------------|--------------|
| VECSTORE-1000 | rust-indexer-engineer | database-engineer |
| VECSTORE-1001 | rust-indexer-engineer | database-engineer |
| VECSTORE-1002 | rust-indexer-engineer | database-engineer |
| VECSTORE-1003 | rust-indexer-engineer | database-engineer |
| VECSTORE-1004 | rust-indexer-engineer | database-engineer |
| VECSTORE-1005 | rust-indexer-engineer | database-engineer |
| VECSTORE-1006 | rust-indexer-engineer | database-engineer |
| VECSTORE-1007 | integration-tester | unit-test-runner |

## Testing Milestones

1. **After Phase 0**: SQLite accepts 768-dim embeddings, `cargo test --features sqlite` passes dimension tests
2. **After Phase 1**: `cargo test --features sqlite --lib db::sqlite::vector` passes (both 768 and 1536 dim)
3. **After Phase 2**: `cargo test --features sqlite --lib db::sqlite::hybrid` passes
4. **After Phase 4**: Index state tests pass on both backends
5. **After Phase 6**: Full contract and parity test suites pass

## Security Checkpoints

1. **Each ticket**: Verify parameterized queries (no SQL injection)
2. **Phase 6**: Security-focused edge case tests included

## Success Criteria (Project Complete)

1. ✅ All 8 tickets completed and committed (VECSTORE-1000 through VECSTORE-1007)
2. ✅ **SQLite supports 768-dim embeddings** (Ollama zero-config works)
3. ✅ `cargo test --features sqlite` passes all trait tests (both dimensions)
4. ✅ `cargo test` (PostgreSQL) passes all trait tests
5. ✅ No raw SQL queries outside `db/postgres/` or `db/sqlite/`
6. ✅ `get_store()` returns working store for both backends
7. ✅ All new trait methods implemented in both `PostgresStore` and `SqliteStore`
8. ✅ Contract tests verify both backends implement trait correctly

**Note:** CLI/daemon/indexer migration to use `Arc<dyn VectorStore>` is handled by MAPROOMCLI project after VECSTORE completes.

## Dependencies

**Blocks**: MAPROOMCLI, MCPDB, VSCODEDB, SQLITEINFRA, **EMBPERF**

**Blocked By**: None (foundation project)

**EMBPERF Relationship**: The EMBPERF project optimizes Ollama embedding throughput (10-20x improvement). EMBPERF produces 768-dim embeddings that must be stored in SQLite. VECSTORE-1000 (768-dim support) must complete before EMBPERF can be fully utilized with SQLite.

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Interface changes during development | Each phase is additive; existing methods unchanged |
| PostgreSQL unavailable for testing | SQLite tests can run independently |
| Complex query differences between backends | Parity tests catch divergence early |
| Scope creep | Out of scope items documented; CLI/daemon migration is MAPROOMCLI |
