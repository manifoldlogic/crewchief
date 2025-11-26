# Analysis: MAPCLI - Maproom CLI Abstraction

## Problem Definition

The `crewchief-maproom` CLI binary and daemon currently use direct PostgreSQL database connections (`db::connect()`, `db::create_pool()`) for all operations. This tight coupling prevents using the SQLite backend that was implemented in the VECSTORE project.

**Current State**:
- `main.rs` calls `db::connect()` directly returning `tokio_postgres::Client`
- Daemon uses `PgPool` (deadpool-postgres pool)
- All CLI commands use `&Client` or `&PgPool` parameters
- Query functions in `db/queries.rs` use PostgreSQL-specific syntax
- No runtime database backend selection

**Desired State**:
- CLI/daemon use `VectorStore` trait abstraction
- Backend selected via `get_store()` factory based on URL
- Same commands work with SQLite or PostgreSQL
- Zero-config operation with SQLite default

## Context

This project is Phase 2 of the SQLite Integration effort, following the successful completion of VECSTORE (VectorStore trait completion). VECSTORE established the `VectorStore` trait with implementations for both PostgreSQL and SQLite. MAPCLI applies this abstraction to the CLI and daemon layers.

**Dependency**: VECSTORE (completed 2025-11-26)

## Existing Solutions in Codebase

### Factory Pattern (Implemented in VECSTORE)

`crates/maproom/src/db/factory.rs`:
```rust
pub async fn get_store() -> anyhow::Result<Arc<dyn VectorStore>> {
    let url = get_database_url()?;
    if url.starts_with("postgres") {
        Ok(Arc::new(PostgresStore::connect().await?))
    } else if url.starts_with("sqlite") {
        Ok(Arc::new(SqliteStore::connect(&url).await?))
    }
}
```

### Current CLI Architecture

`main.rs` directly creates database connections:
```rust
let client = db::connect().await?;  // Returns tokio_postgres::Client
indexer::scan_worktree(&client, ...)?;
```

### Current Daemon Architecture

`daemon/mod.rs`:
```rust
struct DaemonState {
    pool: PgPool,  // Hardcoded PostgreSQL pool
    embedding_service: EmbeddingService,
}
```

## Research Findings

### Direct PostgreSQL Usage Analysis

Identified locations using direct PostgreSQL connections:

1. **main.rs** (~20 locations):
   - `db::connect()` calls for each command
   - `db::create_pool()` for parallel scan mode
   - Direct `client.query_one()` calls in VectorSearch command

2. **daemon/mod.rs** (~10 locations):
   - `DaemonState.pool: PgPool`
   - `state.pool.get().await?` for connections
   - Direct SQL queries in `execute_search()`

3. **Indexer functions** (indirect):
   - `scan_worktree(&client, ...)` takes `&impl GenericClient`
   - `upsert_files(&client, ...)` same pattern
   - These could accept trait-based stores

### Status Module Coupling (CRITICAL)

**File**: `crates/maproom/src/status.rs` (lines 28-34)

The status module creates its **own PostgreSQL connection**, completely bypassing the factory pattern:

```rust
pub async fn get_status(...) -> Result<StatusResponse> {
    let database_url = env::var("MAPROOM_DATABASE_URL")?;
    let (client, connection) = tokio_postgres::connect(&database_url, tokio_postgres::NoTls).await?;
    // ...
}
```

**Problems**:
1. Uses `tokio_postgres::connect()` directly instead of factory
2. Queries use PostgreSQL-specific JSONB operators (`@>`) that won't work with SQLite:
   ```sql
   COUNT(DISTINCT c.id) FILTER (WHERE c.worktree_ids @> jsonb_build_array(w.id))
   ```
3. Not parameterized for different backends

**Impact**: `status` command will fail completely with SQLite backend.

**Resolution**: Refactor `status.rs` to accept `Arc<dyn VectorStore>` and use existing trait methods (`list_repos()`, `list_worktrees()`).

### Indexer Function Coupling (CRITICAL)

**File**: `crates/maproom/src/indexer/mod.rs`

The indexer functions have **explicit PostgreSQL type parameters**:

```rust
// Line 459 - Sequential scan
pub async fn scan_worktree(
    client: &Client,  // tokio_postgres::Client
    repo: &str,
    worktree: &str,
    root: &Path,
    commit: &str,
    // ...
)

// Line 717 - File upsert
pub async fn upsert_files(
    client: &Client,  // tokio_postgres::Client
    repo: &str,
    // ...
)

// Line 1076 - File watcher
pub async fn watch_worktree(
    _client: &Client,  // tokio_postgres::Client
    repo: &str,
    // ...
)

// Line 259 - Parallel scan
pub async fn scan_worktree_parallel(
    pool: &PgPool,  // deadpool_postgres::Pool
    // ...
)
```

**Impact**: These functions cannot accept SQLite connections. They require `tokio_postgres::Client` or `PgPool`.

**Options**:
1. **Option A**: Create duplicate SQLite indexer functions - significant work, cleaner separation
2. **Option B**: Defer scan/upsert/watch for SQLite - smaller MVP, achievable now
3. **Option C**: Refactor indexer to accept `&dyn VectorStore` - biggest change, cleanest result

**Decision**: **Option B for MVP**. The indexer abstraction is substantial work (6+ tickets). MVP will support daemon/search/status/cleanup with SQLite. Indexer commands remain PostgreSQL-only until Phase 2.

### Trait Method Coverage

The VectorStore trait now covers all major operations:
- ✅ Repository/worktree management
- ✅ File/chunk insertion
- ✅ Embedding storage
- ✅ FTS/vector/hybrid search
- ✅ Context assembly
- ✅ Index state persistence
- ✅ Cleanup operations

**Missing for full CLI support** (deferred to Phase 2):
- Indexer integration (currently uses raw queries)
- Migration command (PostgreSQL-specific)
- Generate-embeddings command (uses raw queries)

### Backend Detection Strategy

Current `get_database_url()` in `connection.rs`:
1. Check `MAPROOM_DATABASE_URL` env var
2. Check `~/.config/crewchief/maproom.json` config file
3. Default to PostgreSQL URL

Proposed SQLite-first detection:
1. Check `MAPROOM_DATABASE_URL` env var
2. Check `~/.maproom/maproom.db` exists
3. Default to `~/.maproom/maproom.db` (auto-create)

## Impact Analysis

### Commands Requiring Changes

| Command | Current Implementation | Change Required | MVP Scope |
|---------|----------------------|-----------------|-----------|
| `db migrate` | Direct SQL migrations | Skip for SQLite (auto-migrates) | ✅ Phase 1 |
| `db cleanup-stale` | Uses `StaleWorktreeDetector` | Use trait `detect_stale_worktrees()` | ✅ Phase 1 |
| `scan` | `db::connect()` + `indexer::scan_worktree()` | **Requires indexer abstraction** | ⏸️ Phase 2 |
| `upsert` | `db::connect()` + `indexer::upsert_files()` | **Requires indexer abstraction** | ⏸️ Phase 2 |
| `watch` | `db::connect()` + `indexer::watch_worktree()` | **Requires indexer abstraction** | ⏸️ Phase 2 |
| `search` | `db::search_chunks_fts()` | Use trait `search_chunks_fts()` | ✅ Phase 1 |
| `vector-search` | Direct SQL queries | Use trait `search_chunks_vector()` | ✅ Phase 1 |
| `status` | `status::get_status()` (own connection!) | **Complete refactor needed** | ✅ Phase 1 |
| `generate-embeddings` | Direct SQL queries | PostgreSQL-only for MVP | ⏸️ Phase 2 |
| `serve` (daemon) | `PgPool` + direct queries | Use `get_store()` | ✅ Phase 1 |

### Non-Blocking Items

Some features are PostgreSQL-specific and need graceful handling:
- `migrate` commands - SQLite auto-migrates
- `migrate markdown/rollback/verify` - PostgreSQL-only for now
- Advanced embedding pipeline - Can be deferred
- Indexer commands (scan/upsert/watch) - Deferred to Phase 2

## Constraints

1. **Binary Compatibility**: CLI binary must work with both backends
2. **Feature Flags**: SQLite requires `--features sqlite` at compile time
3. **Indexer Coupling**: `indexer/` module has tight PostgreSQL coupling - **blocking for scan/upsert/watch**
4. **Status Module**: Creates own connection - **requires complete refactor**
5. **Embedding Service**: Independent of database backend
6. **Parallel Scan**: Uses `PgPool` which doesn't apply to SQLite

## Scope Boundaries

### Phase 1 Scope (MVP)
- Add `BackendType` enum and `backend_type()` to VectorStore trait
- Update `main.rs` to use `get_store()` factory for supported commands
- Refactor `status.rs` to use VectorStore trait methods
- Update daemon to use `Arc<dyn VectorStore>`
- Update search/vector-search commands to use trait methods
- Update `db cleanup-stale` to use trait cleanup methods
- Handle `db migrate` gracefully for SQLite (skip with message)

### Phase 2 Scope (Future - Indexer Abstraction)
- Refactor indexer to accept `&dyn VectorStore`
- Enable scan/upsert/watch commands for SQLite
- Abstract embedding generation pipeline

### Out of Scope
- Parallel scan with SQLite (single-writer limitation)
- Migration command abstraction
- Embedding pipeline full abstraction

## Success Metrics

### Phase 1 (MVP)
1. `crewchief-maproom search` returns results from SQLite database
2. `crewchief-maproom status` shows correct counts for SQLite
3. `crewchief-maproom serve` (daemon) works with SQLite backend
4. `crewchief-maproom db cleanup-stale` detects stale worktrees in SQLite
5. All existing PostgreSQL functionality unchanged
6. No compile errors without `--features sqlite`

### Phase 2 (Future)
1. `crewchief-maproom scan` works with `MAPROOM_DATABASE_URL=sqlite://...`
2. `crewchief-maproom upsert` works with SQLite
3. `crewchief-maproom watch` works with SQLite
