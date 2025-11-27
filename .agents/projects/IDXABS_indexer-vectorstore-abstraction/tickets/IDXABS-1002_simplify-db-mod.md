# Ticket: IDXABS-1002: Simplify db/mod.rs and connection.rs

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (cargo check with --features sqlite shows reduced errors in other modules)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- `cargo check` should show reduced errors (trait usage errors remain)
- Full test suite won't pass until Phase 2 completes
- Verify `db::connect()` compiles and returns `SqliteStore`

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Remove the VectorStore trait abstraction and BackendType enum from `db/mod.rs`, simplify `db/connection.rs` to remove PostgreSQL fallback logic, and replace with direct SqliteStore exports and a simple `connect()` function.

## Background
With PostgreSQL files deleted (IDXABS-1001), the VectorStore trait abstraction is no longer needed. This ticket simplifies the database module to directly use SqliteStore.

**Reference**: Phase 1, Ticket 1002 of `planning/plan.md` - "Simplify db/mod.rs"
**Architecture**: See `planning/architecture.md` - "Decision 1: Remove VectorStore Trait"

## Acceptance Criteria
- [x] `VectorStore` trait definition is removed (~150 lines)
- [x] `BackendType` enum is removed
- [x] All `#[async_trait]` impl blocks for VectorStore are removed
- [x] `pub use sqlite::SqliteStore;` exports SqliteStore directly
- [x] `pub async fn connect() -> Result<SqliteStore>` function exists
- [x] PostgreSQL-related imports are removed
- [x] Feature flag conditionals (`#[cfg(feature = "sqlite")]`) are removed from db/mod.rs
- [x] `connection.rs` has no PostgreSQL references or fallback logic
- [x] `connection.rs` has no `#[cfg(feature = "sqlite")]` conditionals
- [x] `cargo check` shows fewer errors than after IDXABS-1001 (trait usage errors expected)

## Technical Requirements
- Remove trait definition, not just comment out
- The `connect()` function should:
  - Read `MAPROOM_DATABASE_URL` env var
  - Default to `~/.maproom/maproom.db` if not set
  - Return `SqliteStore` directly (not `Arc<dyn VectorStore>`)
- Keep sqlite module exports clean and simple

## Implementation Notes

### Current Structure (to remove)
```rust
// DELETE these from db/mod.rs:
pub trait VectorStore: Send + Sync {
    // ~150 lines of trait methods
}

pub enum BackendType {
    PostgreSQL,
    SQLite,
}

#[async_trait]
impl VectorStore for SqliteStore {
    // impl block
}

pub fn get_store() -> Arc<dyn VectorStore> { ... }
pub fn get_store_with_type() -> (Arc<dyn VectorStore>, BackendType) { ... }
```

### Target Structure
```rust
// db/mod.rs after simplification
pub mod sqlite;
pub use sqlite::SqliteStore;

pub async fn connect() -> anyhow::Result<SqliteStore> {
    let url = std::env::var("MAPROOM_DATABASE_URL")
        .unwrap_or_else(|_| {
            let home = dirs::home_dir().expect("No home directory");
            format!("sqlite://{}", home.join(".maproom/maproom.db").display())
        });
    SqliteStore::new(&url).await
}
```

### Verification
```bash
# Should compile db module
cargo check -p crewchief-maproom --lib 2>&1 | grep -c "error"

# Errors should be in OTHER modules (indexer, search, etc.)
# not in db/mod.rs itself
```

## Dependencies
- IDXABS-1001 (Delete PostgreSQL Database Files) - must be completed first

## Risk Assessment
- **Risk**: Breaking SqliteStore's existing interface
  - **Mitigation**: SqliteStore already exists; we're just removing the trait layer
- **Risk**: connect() function has different signature than expected
  - **Mitigation**: Verify SqliteStore::new() exists and accepts URL string
- **Risk**: Other modules still reference VectorStore trait
  - **Mitigation**: Expected - those will be fixed in Phase 2 tickets

## Files/Packages Affected
Files to MODIFY:
- `crates/maproom/src/db/mod.rs` - Remove trait, add connect()
- `crates/maproom/src/db/connection.rs` - Simplify to SQLite-only

Items to REMOVE from db/mod.rs:
- `VectorStore` trait definition
- `BackendType` enum
- `#[async_trait]` impl blocks
- `get_store()` and `get_store_with_type()` functions
- PostgreSQL imports
- Feature flag conditionals

Items to REMOVE/SIMPLIFY in connection.rs:
- PostgreSQL URL handling/fallback logic
- `#[cfg(feature = "sqlite")]` conditionals
- Any PostgreSQL connection string parsing
- Keep only SQLite-specific URL resolution
