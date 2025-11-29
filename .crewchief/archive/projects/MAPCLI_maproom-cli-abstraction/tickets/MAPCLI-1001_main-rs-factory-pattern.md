# Ticket: MAPCLI-1001: Update main.rs to use get_store() Factory

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Replace direct `db::connect()` calls in main.rs with the `get_store()` factory pattern for MVP commands (search, status, cleanup). Add graceful error handling for PostgreSQL-only commands when using SQLite backend.

## Background
The CLI currently uses `db::connect()` which returns a `tokio_postgres::Client`, tightly coupling all commands to PostgreSQL. To support SQLite, we need to use the `get_store()` factory which returns `Arc<dyn VectorStore>` based on the database URL.

This ticket establishes the foundation for backend-agnostic CLI operations by modifying the entry point (main.rs) to use the VectorStore abstraction for MVP-scoped commands.

**Plan Reference**: Phase 1: Foundation (MAPCLI-1001) in plan.md

## Acceptance Criteria
- [ ] `get_store()` is used instead of `db::connect()` for search/status/cleanup commands
- [ ] Helper function `get_store_with_type()` exists for convenient backend detection
- [ ] PostgreSQL-only commands (scan, upsert, watch) show helpful error when SQLite is detected
- [ ] `db migrate` command skips for SQLite with informative message
- [ ] `--parallel` flag warns and falls back for SQLite backend
- [ ] Compilation succeeds without `--features sqlite`
- [ ] Compilation succeeds with `--features sqlite`
- [ ] All existing tests pass

## Technical Requirements
- Import `BackendType` and `get_store` from db module
- Create helper function for store retrieval with type
- Maintain backward compatibility with PostgreSQL
- Use `anyhow::bail!` for graceful error messages
- Keep existing `db::connect()` for scan/upsert/watch (PostgreSQL path)

## Implementation Notes

### Step 1: Add helper function
```rust
use crate::db::{factory::get_store, BackendType};

async fn get_store_with_type() -> anyhow::Result<(Arc<dyn VectorStore>, BackendType)> {
    let store = get_store().await?;
    let backend_type = store.backend_type();
    Ok((store, backend_type))
}
```

### Step 2: Update search command
```rust
Commands::Search { repo, query, limit, worktree, debug } => {
    let store = get_store().await?;
    let hits = store.search_chunks_fts(&repo, worktree.as_deref(), &query, limit as i64, debug).await?;
    // ... format and output results
}
```

### Step 3: Update PostgreSQL-only commands with graceful errors
```rust
Commands::Scan { path, repo, worktree, commit, parallel, .. } => {
    let store = get_store().await?;
    if store.backend_type() == BackendType::SQLite {
        anyhow::bail!(
            "The 'scan' command requires PostgreSQL backend.\n\
             SQLite support for indexing is coming in Phase 2.\n\
             Set MAPROOM_DATABASE_URL to a PostgreSQL connection string to use this command."
        );
    }
    // Proceed with existing PostgreSQL scan logic
    let client = db::connect().await?;
    // ...
}
```

### Step 4: Handle parallel flag for SQLite
```rust
if parallel {
    let store = get_store().await?;
    if store.backend_type() == BackendType::SQLite {
        eprintln!("Warning: --parallel flag ignored for SQLite backend (single-writer limitation)");
        // Fall through to sequential scan
    }
}
```

### Step 5: Update db migrate command
```rust
Commands::Db { command: DbCommand::Migrate { .. } } => {
    let store = get_store().await?;
    match store.backend_type() {
        BackendType::PostgreSQL => {
            let client = db::connect().await?;
            db::migrate(&client).await?;
            println!("Migration completed successfully");
        }
        BackendType::SQLite => {
            println!("SQLite database auto-migrates on connection - no action needed");
        }
    }
}
```

### Commands to Update
| Command | Action |
|---------|--------|
| `search` | Use `get_store()` and trait methods |
| `vector-search` | Use `get_store()` and trait methods |
| `status` | Will be updated in MAPCLI-1004 (depends on status.rs refactor) |
| `db cleanup-stale` | Use `get_store()` and trait methods |
| `db migrate` | Check backend type, skip for SQLite |
| `scan` | Check backend type, error for SQLite |
| `upsert` | Check backend type, error for SQLite |
| `watch` | Check backend type, error for SQLite |
| `serve` | Will be updated in MAPCLI-1002 (daemon ticket) |

## Dependencies
- **MAPCLI-1000**: BackendType enum must exist before this ticket can be implemented

## Risk Assessment
- **Risk**: Breaking existing PostgreSQL functionality
  - **Mitigation**: Keep `db::connect()` path for scan/upsert/watch; only add SQLite check before proceeding
- **Risk**: Incomplete command coverage
  - **Mitigation**: Focus on MVP commands; status/daemon handled in separate tickets

## Files/Packages Affected
- `crates/maproom/src/main.rs` - Primary modification target
  - Add imports for BackendType, get_store
  - Add get_store_with_type() helper
  - Update command handlers for search, cleanup, migrate
  - Add SQLite checks for scan, upsert, watch

## Testing
```bash
# Verify PostgreSQL path unchanged
MAPROOM_DATABASE_URL="postgresql://..." cargo run --bin crewchief-maproom -- search --repo test --query "function"

# Verify SQLite graceful error for scan
MAPROOM_DATABASE_URL="sqlite:///tmp/test.db" cargo run --features sqlite --bin crewchief-maproom -- scan --path .
# Should show: "The 'scan' command requires PostgreSQL backend..."

# Verify SQLite works for search (after database is populated)
MAPROOM_DATABASE_URL="sqlite:///tmp/test.db" cargo run --features sqlite --bin crewchief-maproom -- search --repo test --query "function"

# Verify db migrate skip for SQLite
MAPROOM_DATABASE_URL="sqlite:///tmp/test.db" cargo run --features sqlite --bin crewchief-maproom -- db migrate
# Should show: "SQLite database auto-migrates on connection..."

# Run all tests
cargo test
cargo test --features sqlite
```
