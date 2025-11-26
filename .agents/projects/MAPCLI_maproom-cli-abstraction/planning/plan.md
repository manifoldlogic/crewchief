# Plan: MAPCLI - Maproom CLI Abstraction

## Project Overview

**Objective**: Update the `crewchief-maproom` CLI binary and daemon to use the `VectorStore` trait abstraction, enabling SQLite backend support for search, status, and daemon operations.

**Dependencies**: VECSTORE (completed)

**MVP Scope** (Phase 1):
1. Search commands work with both PostgreSQL and SQLite
2. Status command works with both backends
3. Daemon serves JSON-RPC requests using VectorStore trait
4. Backend auto-detected from URL or environment
5. All existing PostgreSQL functionality preserved

**Deferred to Phase 2**:
- scan/upsert/watch commands with SQLite (requires indexer abstraction)
- generate-embeddings with SQLite
- Parallel scan mode for SQLite

## Phase 1: Prerequisite (MAPCLI-1000)

### Ticket: MAPCLI-1000 - Add BackendType Enum and Trait Method

**Summary**: Add `BackendType` enum to VectorStore trait for runtime backend detection

**Agent**: rust-indexer-engineer

**Scope**:
- Add `BackendType` enum to `db/mod.rs`
- Add `backend_type(&self) -> BackendType` to VectorStore trait
- Implement in `PostgresStore` returning `BackendType::PostgreSQL`
- Implement in `SqliteStore` returning `BackendType::SQLite`

**Key Changes**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    PostgreSQL,
    SQLite,
}

impl VectorStore for PostgresStore {
    fn backend_type(&self) -> BackendType { BackendType::PostgreSQL }
}

impl VectorStore for SqliteStore {
    fn backend_type(&self) -> BackendType { BackendType::SQLite }
}
```

**Acceptance Criteria**:
- [ ] `BackendType` enum exists in db/mod.rs
- [ ] `backend_type()` method added to VectorStore trait
- [ ] Both stores implement `backend_type()` correctly
- [ ] Compilation succeeds without `--features sqlite`
- [ ] Compilation succeeds with `--features sqlite`
- [ ] `cargo test` passes

**Testing Checkpoint**: `cargo test` and `cargo build --features sqlite`

---

## Phase 1: Foundation (MAPCLI-1001)

### Ticket: MAPCLI-1001 - Update main.rs to use get_store() Factory

**Summary**: Replace direct `db::connect()` calls with `get_store()` factory pattern for MVP commands

**Agent**: rust-indexer-engineer

**Depends On**: MAPCLI-1000 (BackendType enum)

**Scope**:
- Update main.rs to use `get_store()` for search/status/cleanup commands
- Add helper function `get_store_with_type()` for backend detection
- Add graceful errors for SQLite when using PostgreSQL-only commands (scan/upsert/watch)
- Handle `db migrate` - skip for SQLite with message
- Disable `--parallel` for SQLite with warning

**Key Changes**:
```rust
// MVP commands use trait abstraction
Commands::Search { .. } => {
    let store = db::factory::get_store().await?;
    // Use trait methods...
}

// PostgreSQL-only commands (deferred to Phase 2)
Commands::Scan { .. } => {
    let store = db::factory::get_store().await?;
    if store.backend_type() == BackendType::SQLite {
        anyhow::bail!("scan command requires PostgreSQL backend (SQLite support coming in Phase 2)");
    }
    let client = db::connect().await?;
    // Use existing indexer...
}
```

**Acceptance Criteria**:
- [ ] `get_store()` returns appropriate backend based on URL
- [ ] MVP commands (search, status, cleanup) use trait abstraction
- [ ] PostgreSQL-only commands show helpful error for SQLite
- [ ] `db migrate` skips for SQLite with message
- [ ] Compilation succeeds without `--features sqlite`
- [ ] Compilation succeeds with `--features sqlite`
- [ ] Existing tests pass

**Testing Checkpoint**: `cargo test` and `cargo test --features sqlite`

---

## Phase 1: Daemon Refactoring (MAPCLI-1002)

### Ticket: MAPCLI-1002 - Refactor Daemon to use VectorStore Trait

**Summary**: Replace `PgPool` in DaemonState with `Arc<dyn VectorStore>` and remove raw SQL queries

**Agent**: rust-indexer-engineer

**Depends On**: MAPCLI-1000 (BackendType enum)

**Scope**:
- Update `DaemonState` struct to use `Arc<dyn VectorStore>`
- Update `run()` to use `get_store()`
- **Replace raw SQL queries in `execute_search()` with trait methods**:
  - Remove direct `client.query_one()` for repo lookup
  - Remove direct `client.query_opt()` for chunk details
  - Use `store.search_chunks_fts()`, `store.search_chunks_vector()`, `store.search_chunks_hybrid()`
- Use `SearchHit` struct from trait methods (already contains all needed fields)
- Handle all search modes (fts, vector, hybrid) through trait

**Key Changes**:
```rust
// Before (daemon/mod.rs lines 108-150)
let repo_row = client.query_one(
    "SELECT id FROM maproom.repos WHERE name = $1",
    &[&params.repo],
)?;
let chunk_row = client.query_opt(...)

// After
let hits = state.store.search_chunks_fts(
    &params.repo,
    params.worktree.as_deref(),
    &params.query,
    params.limit.unwrap_or(10) as i64,
    false
).await?;
// SearchHit already contains chunk_id, file_path, start_line, end_line, etc.
```

**Acceptance Criteria**:
- [ ] Daemon starts with SQLite backend
- [ ] `ping` RPC returns "pong"
- [ ] `search` RPC (fts mode) returns results from SQLite
- [ ] `search` RPC (vector mode) returns results from SQLite
- [ ] `search` RPC (hybrid mode) returns results from SQLite
- [ ] **No raw SQL queries remain in daemon** (all use trait methods)
- [ ] Daemon still works with PostgreSQL backend
- [ ] Error handling for missing backends

**Testing Checkpoint**: Manual daemon testing with both backends, all search modes

---

## Phase 1: Backend Detection & Configuration (MAPCLI-1003)

### Ticket: MAPCLI-1003 - Add SQLite Backend Detection and Configuration

**Summary**: Implement auto-detection of SQLite database and configuration options

**Agent**: rust-indexer-engineer

**Depends On**: MAPCLI-1000 (BackendType enum)

**Scope**:
- Update `get_database_url()` for SQLite-first detection
- Check for `~/.maproom/maproom.db` existence
- Create parent directories if needed on SQLite connection
- Add helpful error messages for missing configuration
- Update factory to default to SQLite when no config exists

**Detection Order**:
1. `MAPROOM_DATABASE_URL` environment variable
2. `~/.maproom/maproom.db` if exists
3. Default to SQLite at `~/.maproom/maproom.db` (auto-create)

**Acceptance Criteria**:
- [ ] `MAPROOM_DATABASE_URL=sqlite://...` selects SQLite
- [ ] `MAPROOM_DATABASE_URL=postgresql://...` selects PostgreSQL
- [ ] Auto-detection finds existing SQLite database at `~/.maproom/maproom.db`
- [ ] SQLite database created automatically if none exists
- [ ] Parent directory `~/.maproom/` created if needed
- [ ] Helpful error when configuration is invalid

**Testing Checkpoint**: Test all detection scenarios

---

## Phase 1: CLI Command Updates (MAPCLI-1004)

### Ticket: MAPCLI-1004 - Update CLI Commands and Refactor status.rs

**Summary**: Update CLI commands to use VectorStore trait methods and **refactor status.rs to remove direct PostgreSQL connection**

**Agent**: rust-indexer-engineer

**Depends On**: MAPCLI-1001 (main.rs factory pattern)

**Scope**:
- Update `search` command to use `store.search_chunks_fts()`
- Update `vector-search` command to use `store.search_chunks_vector()`
- **CRITICAL: Refactor `status.rs` to accept `Arc<dyn VectorStore>`**:
  - Remove direct `tokio_postgres::connect()` call
  - Replace PostgreSQL-specific JSONB queries with trait methods
  - Use `store.list_repos()` and `store.list_worktrees()` for status data
- Update `db cleanup-stale` to use `store.detect_stale_worktrees()`

**status.rs Refactoring** (Critical):
```rust
// Current (BREAKS with SQLite)
let (client, connection) = tokio_postgres::connect(&database_url, tokio_postgres::NoTls).await?;
// Uses: c.worktree_ids @> jsonb_build_array(w.id)

// After
pub async fn get_status(
    store: Arc<dyn VectorStore>,
    repo_filter: Option<&str>,
    worktree_filter: Option<&str>,
) -> Result<StatusResponse> {
    let repos = store.list_repos().await?;
    // ... iterate and build status
}
```

**Commands to Update**:
| Command | Change | SQLite Support |
|---------|--------|----------------|
| `search` | Use trait search methods | ✅ MVP |
| `vector-search` | Use trait vector search | ✅ MVP |
| `status` | **Complete refactor** to use trait | ✅ MVP |
| `db cleanup-stale` | Use trait cleanup methods | ✅ MVP |
| `db migrate` | Skip for SQLite | ✅ MVP |
| `scan` | Graceful error for SQLite | ⏸️ Phase 2 |
| `upsert` | Graceful error for SQLite | ⏸️ Phase 2 |
| `watch` | Graceful error for SQLite | ⏸️ Phase 2 |

**Acceptance Criteria**:
- [ ] `search` works with SQLite backend
- [ ] `vector-search` works with SQLite (or graceful fallback if sqlite-vec unavailable)
- [ ] **`status` works with SQLite backend** (critical refactor complete)
- [ ] `status.rs` no longer creates its own PostgreSQL connection
- [ ] `db cleanup-stale` detects stale worktrees in SQLite
- [ ] `db migrate` skips for SQLite with informative message
- [ ] scan/upsert/watch show helpful "Phase 2" error for SQLite

**Testing Checkpoint**: Test each command with SQLite backend

---

## Phase 1: Integration Testing (MAPCLI-1005)

### Ticket: MAPCLI-1005 - E2E Integration Tests with SQLite Backend

**Summary**: Create integration tests verifying MVP CLI flow with pre-indexed SQLite database

**Agent**: integration-tester

**Depends On**: MAPCLI-1004 (CLI commands updated)

**Scope**:
- Create test fixture with **pre-indexed** SQLite database
- Write E2E test script for SQLite flow
- Test: search → status → cleanup cycle (no scan - deferred)
- Verify daemon JSON-RPC flow with all search modes
- Document test procedures

**Testing Approach**:
Since scan is deferred to Phase 2, tests use a pre-populated SQLite database:
1. Create fixture SQLite database with test data
2. Test search/status/cleanup against this database
3. Test daemon search RPC against this database

**Test Cases**:
1. ~~Fresh scan of repository with SQLite~~ (deferred to Phase 2)
2. Search returns indexed results from pre-populated database
3. Vector search returns results (or graceful fallback)
4. Status shows correct statistics
5. Daemon search RPC (fts mode) returns results
6. Daemon search RPC (vector mode) returns results
7. Daemon search RPC (hybrid mode) returns results
8. Cleanup detects stale worktrees

**Acceptance Criteria**:
- [ ] Pre-indexed SQLite test fixture exists
- [ ] E2E test script exists and passes
- [ ] Test covers all MVP commands (search, status, cleanup)
- [ ] Daemon tests cover all search modes
- [ ] Test runs in CI (SQLite job)
- [ ] Documentation for running tests and creating fixtures

**Testing Checkpoint**: Full E2E test suite passes with pre-indexed database

---

## Execution Order

```
MAPCLI-1000 (BackendType - PREREQUISITE)
    │
    ▼
MAPCLI-1001 (Foundation) ─────────┐
    │                             │
    ▼                             ▼
MAPCLI-1002 (Daemon)        MAPCLI-1003 (Detection)
    │                             │
    └──────────┬──────────────────┘
               ▼
        MAPCLI-1004 (Commands)
               │
               ▼
        MAPCLI-1005 (Testing)
```

**Rationale**:
1. **MAPCLI-1000 must come first** - provides `BackendType` enum all other tickets depend on
2. MAPCLI-1001 provides factory pattern for main.rs
3. MAPCLI-1002 and MAPCLI-1003 can run in parallel after 1001
4. MAPCLI-1004 depends on 1001 (factory pattern established)
5. MAPCLI-1005 verifies everything works together

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Breaking PostgreSQL functionality | All tests must pass without `--features sqlite` |
| Indexer coupling prevents SQLite indexing | **Deferred to Phase 2** - MVP focuses on search/status/daemon |
| Status module direct connection | **MAPCLI-1004 explicitly includes status.rs refactor** |
| SQLite performance issues | Document single-writer limitation, disable parallel mode |
| Missing trait methods | VectorStore trait already complete from VECSTORE |

## Agent Assignments

| Ticket | Primary Agent | Secondary Agent |
|--------|--------------|-----------------|
| MAPCLI-1000 | rust-indexer-engineer | - |
| MAPCLI-1001 | rust-indexer-engineer | - |
| MAPCLI-1002 | rust-indexer-engineer | - |
| MAPCLI-1003 | rust-indexer-engineer | - |
| MAPCLI-1004 | rust-indexer-engineer | - |
| MAPCLI-1005 | integration-tester | unit-test-runner |

## Success Criteria

### Phase 1 (MVP) Complete When:
1. ✅ `crewchief-maproom search --repo x --query y` returns results from SQLite
2. ✅ `crewchief-maproom status` shows correct statistics for SQLite
3. ✅ `crewchief-maproom serve` daemon works with SQLite backend (all search modes)
4. ✅ `crewchief-maproom db cleanup-stale` works with SQLite
5. ✅ `status.rs` no longer creates its own PostgreSQL connection
6. ✅ All existing PostgreSQL tests pass
7. ✅ E2E test script passes with pre-indexed SQLite database
8. ✅ No regression in PostgreSQL functionality

### Phase 2 (Future) Complete When:
1. ✅ `crewchief-maproom scan` works with SQLite backend
2. ✅ `crewchief-maproom upsert` works with SQLite backend
3. ✅ `crewchief-maproom watch` works with SQLite backend
4. ✅ Indexer abstracted to `&dyn VectorStore`

### Definition of Done per Ticket:
1. Implementation compiles without warnings
2. `cargo test` passes (PostgreSQL)
3. `cargo test --features sqlite` passes (SQLite)
4. `cargo clippy` clean
5. Acceptance criteria verified
6. Committed with conventional commit message

---

## Phase 2: Future Work (Indexer Abstraction)

**Note**: These items are explicitly deferred from Phase 1 MVP.

### Future Ticket: Indexer Abstraction

**Summary**: Refactor indexer module to accept `&dyn VectorStore` instead of `tokio_postgres::Client`

**Scope**:
- Refactor `scan_worktree()` to accept `&dyn VectorStore`
- Refactor `upsert_files()` to accept `&dyn VectorStore`
- Refactor `watch_worktree()` to accept `&dyn VectorStore`
- Enable scan/upsert/watch commands for SQLite backend

**Rationale for Deferral**:
- Indexer functions contain ~1000 lines of PostgreSQL-coupled logic
- Full abstraction is a separate project-sized effort
- MVP delivers value without indexing (search existing data)

### Future Ticket: Embedding Pipeline Abstraction

**Summary**: Abstract embedding generation to work with both backends

**Scope**:
- Refactor `generate-embeddings` command
- Abstract embedding storage/retrieval
- Enable vector search for SQLite
