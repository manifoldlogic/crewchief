# Analysis: Indexer SQLite Migration

## 1. Problem Definition

The `crewchief-maproom` CLI currently maintains two database backends (PostgreSQL and SQLite) with complex abstraction layers. This creates:

1. **Maintenance burden** - Two implementations to keep in sync
2. **Complexity** - `VectorStore` trait, factory pattern, backend switching
3. **Blocked commands** - Indexing commands explicitly reject SQLite despite SQLite implementation existing

### Current State

The codebase has:
- **183 PostgreSQL-specific references** across 33 files
- **Dual implementation** - `PostgresStore` and `SqliteStore` both implement `VectorStore`
- **Hardcoded blockers** - `main.rs` rejects SQLite for scan/upsert/watch
- **Feature flags** - SQLite requires `--features sqlite`

### Goal

**Remove PostgreSQL entirely.** SQLite is sufficient for:
- Zero-configuration operation (no Docker/external services)
- Single-file database (easy backup, portability)
- Adequate performance for code search use case
- sqlite-vec for vector similarity
- FTS5 for full-text search

## 2. What to Delete

### Files to Remove
```
crates/maproom/src/db/postgres/       # PostgreSQL implementation
crates/maproom/src/db/pool.rs         # PostgreSQL connection pooling
crates/maproom/src/db/queries.rs      # PostgreSQL-specific queries
crates/maproom/src/db/factory.rs      # Backend switching (not needed)
crates/maproom/src/db/materialized_views.rs  # PostgreSQL materialized views
```

### Code to Remove
- `BackendType` enum and all checks
- `get_store_with_type()` function
- All `tokio_postgres` imports and usage
- Feature flag `#[cfg(feature = "sqlite")]` guards
- SQLite blockers in main.rs

## 3. What to Simplify

### Database Module (`db/mod.rs`)
```rust
// Before: Complex trait + factory pattern
pub trait VectorStore: Send + Sync { ... }
pub fn get_store() -> Arc<dyn VectorStore> { ... }

// After: Direct SQLite store
pub use sqlite::SqliteStore;
pub async fn connect() -> anyhow::Result<SqliteStore> {
    SqliteStore::new_from_env().await
}
```

### Main.rs Command Handlers
```rust
// Before: Backend switching with blockers
let (store, backend_type) = get_store_with_type().await?;
if backend_type == BackendType::SQLite {
    anyhow::bail!("...");
}

// After: Direct SQLite usage
let store = db::connect().await?;
indexer::scan_worktree(&store, ...).await?;
```

### Indexer Module
```rust
// Before: &Client (PostgreSQL)
pub async fn scan_worktree(client: &Client, ...) -> Result<ScanStats>

// After: &SqliteStore
pub async fn scan_worktree(store: &SqliteStore, ...) -> Result<ScanStats>
```

## 4. What to Keep

### SQLite Implementation (db/sqlite/)
- `mod.rs` - SqliteStore implementation
- `schema.rs` - Schema DDL
- `migrations.rs` - Migration system
- `embeddings.rs` - Embedding storage
- `vector.rs` - sqlite-vec integration
- `fts.rs` - FTS5 search
- `hybrid.rs` - Hybrid search
- `graph.rs` - Graph traversal

### Core Functionality
- `indexer/` - Refactor to use SqliteStore
- `embedding/` - Refactor to use SqliteStore
- `search/` - Refactor to use SqliteStore
- `context/` - Refactor to use SqliteStore

## 5. Scope

### In Scope
1. Delete PostgreSQL-specific code
2. Remove `VectorStore` trait abstraction (use concrete type)
3. Refactor all modules to use `SqliteStore` directly
4. Remove feature flags (SQLite is the only backend)
5. Update Cargo.toml dependencies
6. Update tests

### Out of Scope
1. New features
2. Performance optimizations
3. VSCode extension changes
4. Embedding provider changes

## 6. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Breaking PostgreSQL users | N/A | N/A | Intentional removal |
| Missing SQLite features | Low | High | SqliteStore already implements all methods |
| Embedding pipeline gaps | Medium | Medium | Implement 3 missing methods in ticket 2002 |
| Test failures | Medium | Medium | Run tests incrementally |
| Performance regression | Low | Low | SQLite adequate for use case |

**Note:** The embedding pipeline (`embedding/pipeline.rs`) requires 3 methods not yet in SqliteStore:
- `get_chunks_needing_embeddings_count()`
- `copy_existing_embeddings_from_cache()`
- `fetch_chunks_needing_embeddings()`

These will be implemented as part of ticket 2002 (Refactor Embedding Pipeline).

### Rollback Strategy

If critical issues are discovered during implementation:
1. **Git revert** - All changes are committed per-ticket, enabling easy revert
2. **Feature branch** - Work on separate branch until stable
3. **Incremental testing** - Run tests after each phase before proceeding

## 7. Success Criteria

```bash
# These commands work without --features sqlite:
cargo run --bin crewchief-maproom -- scan --path /repo
cargo run --bin crewchief-maproom -- upsert --paths src/main.rs
cargo run --bin crewchief-maproom -- watch
cargo run --bin crewchief-maproom -- generate-embeddings
cargo run --bin crewchief-maproom -- search "function"

# All tests pass:
cargo test

# Database created at:
~/.maproom/maproom.db  # Default location
```

## 8. Estimated Effort

| Component | Effort |
|-----------|--------|
| Delete PostgreSQL code | 2-3 hours |
| Simplify db/mod.rs | 2-3 hours |
| Refactor indexer module | 3-4 hours |
| Refactor embedding pipeline | 2-3 hours |
| Refactor search module | 2-3 hours |
| Refactor context module | 1-2 hours |
| Update main.rs | 2-3 hours |
| Update Cargo.toml | 1 hour |
| Fix tests | 3-4 hours |
| **Total** | **18-26 hours** |
