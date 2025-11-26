# VECSTORE Ticket Index

## Project Overview
Complete the `VectorStore` trait abstraction so both PostgreSQL and SQLite backends can be used interchangeably.

**Ollama Priority**: VECSTORE-1000 (SQLite 768-dim support) is CRITICAL for enabling the zero-config experience (SQLite + Ollama).

---

## Tickets by Phase

### Phase 0: SQLite Multi-Dimension Support (CRITICAL)

| Ticket | Title | Agent | Priority | Status |
|--------|-------|-------|----------|--------|
| [VECSTORE-1000](VECSTORE-1000_sqlite-768-dim-support.md) | SQLite 768-dim Embedding Support | rust-indexer-engineer | **CRITICAL** | Not Started |

**Objective**: Enable SQLite to store and search 768-dimensional embeddings (Ollama/nomic-embed-text).

---

### Phase 1: Search Methods

| Ticket | Title | Agent | Priority | Status |
|--------|-------|-------|----------|--------|
| [VECSTORE-1001](VECSTORE-1001_vector-search-methods.md) | Vector Search Methods | rust-indexer-engineer | High | Not Started |
| [VECSTORE-1002](VECSTORE-1002_hybrid-search-methods.md) | Hybrid Search Methods | rust-indexer-engineer | High | Not Started |

**Objective**: Add vector similarity and hybrid (FTS + vector) search to VectorStore trait.

---

### Phase 2: Context Methods

| Ticket | Title | Agent | Priority | Status |
|--------|-------|-------|----------|--------|
| [VECSTORE-1003](VECSTORE-1003_context-assembly-methods.md) | Context Assembly Methods | rust-indexer-engineer | Medium | Not Started |

**Objective**: Add chunk context retrieval methods for displaying search results with surrounding code.

---

### Phase 3: Repository Query Methods

| Ticket | Title | Agent | Priority | Status |
|--------|-------|-------|----------|--------|
| [VECSTORE-1004](VECSTORE-1004_repository-query-methods.md) | Repository and Worktree Query Methods | rust-indexer-engineer | Medium | Not Started |

**Objective**: Add repo/worktree lookup methods (get by name, list all).

---

### Phase 4: Index State Methods

| Ticket | Title | Agent | Priority | Status |
|--------|-------|-------|----------|--------|
| [VECSTORE-1005](VECSTORE-1005_index-state-methods.md) | Index State Management Methods | rust-indexer-engineer | Medium | Not Started |

**Objective**: Add index state tracking for incremental indexing support.

---

### Phase 5: Cleanup Methods

| Ticket | Title | Agent | Priority | Status |
|--------|-------|-------|----------|--------|
| [VECSTORE-1006](VECSTORE-1006_cleanup-methods.md) | Cleanup and Maintenance Methods | rust-indexer-engineer | Medium | Not Started |

**Objective**: Add stale data detection and cleanup operations.

---

### Phase 6: Integration Testing

| Ticket | Title | Agent | Priority | Status |
|--------|-------|-------|----------|--------|
| [VECSTORE-1007](VECSTORE-1007_contract-parity-tests.md) | Contract and Parity Test Suite | integration-tester | High | Not Started |

**Objective**: Comprehensive test suite verifying trait correctness and backend parity.

---

## Dependency Graph

```
VECSTORE-1000 (768-dim Support) - CRITICAL, no dependencies
       │
       ▼
VECSTORE-1001 (Vector Search) ─────────┐
       │                                │
       ▼                                ▼
VECSTORE-1002 (Hybrid Search)    VECSTORE-1003 (Context)
                                        │
                                        ▼
                                 VECSTORE-1004 (Repo Queries)
                                        │
                                        ▼
                                 VECSTORE-1005 (Index State)
                                        │
                                        ▼
                                 VECSTORE-1006 (Cleanup)
                                        │
                                        ▼
                                 VECSTORE-1007 (Tests) - requires all above
```

---

## Implementation Order

Recommended execution sequence:

1. **VECSTORE-1000** - SQLite 768-dim Support (CRITICAL - blocks Ollama zero-config)
2. **VECSTORE-1001** - Vector Search Methods
3. **VECSTORE-1002** - Hybrid Search Methods
4. **VECSTORE-1003** - Context Assembly Methods
5. **VECSTORE-1004** - Repository Query Methods
6. **VECSTORE-1005** - Index State Methods
7. **VECSTORE-1006** - Cleanup Methods
8. **VECSTORE-1007** - Contract and Parity Tests

**Note**: Phases 3-5 (VECSTORE-1003 through VECSTORE-1005) can run in parallel after Phase 1 completes.

---

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

---

## Success Criteria (Project Complete)

1. ✅ All 8 tickets completed and committed (VECSTORE-1000 through VECSTORE-1007)
2. ✅ **SQLite supports 768-dim embeddings** (Ollama zero-config works)
3. ✅ `cargo test --features sqlite` passes all trait tests (both dimensions)
4. ✅ `cargo test` (PostgreSQL) passes all trait tests
5. ✅ No raw SQL queries outside `db/postgres/` or `db/sqlite/`
6. ✅ `get_store()` returns working store for both backends
7. ✅ All new trait methods implemented in both `PostgresStore` and `SqliteStore`
8. ✅ Contract tests verify both backends implement trait correctly

---

## Project Dependencies

**Blocks**: MAPCLI, MCPDB, VSCODEDB, SQLINFRA, **EMBPERF**

**Blocked By**: None (foundation project)

**EMBPERF Relationship**: The EMBPERF project optimizes Ollama embedding throughput (10-20x improvement). EMBPERF produces 768-dim embeddings that must be stored in SQLite. VECSTORE-1000 (768-dim support) must complete before EMBPERF can be fully utilized with SQLite.
