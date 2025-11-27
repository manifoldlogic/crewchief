# Ticket: IDXABS-2001: Refactor Indexer Module

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Run `cargo check` to verify indexer module compiles
- Full test suite may not pass until all Phase 2 tickets complete
- Focus on compilation, not runtime tests

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Update `indexer/mod.rs` and related files to use `&SqliteStore` instead of `&Client` (tokio_postgres), removing parallel scanning complexity.

## Background
The indexer module currently uses PostgreSQL's `&Client` type for database operations. With the SQLite-only migration, all functions need to use `&SqliteStore` directly and call its methods instead of raw SQL.

**Reference**: Phase 2, Ticket 2001 of `planning/plan.md` - "Refactor Indexer Module"
**Architecture**: See `planning/architecture.md` - Section 4.1 "indexer/mod.rs"

## Acceptance Criteria
- [ ] `scan_worktree(&store, ...)` compiles with `&SqliteStore` parameter
- [ ] `upsert_files(&store, ...)` compiles with `&SqliteStore` parameter
- [ ] `watch_worktree(&store, ...)` compiles with `&SqliteStore` parameter (if exists)
- [ ] No `&Client` references remain in `indexer/` directory
- [ ] No `tokio_postgres` imports in `indexer/` directory
- [ ] `scan_worktree_parallel` is removed or merged into `scan_worktree`
- [ ] `indexer/parallel.rs` is deleted or merged
- [ ] `cargo check` passes for indexer module

## Technical Requirements
- Change function signatures from `&Client` to `&SqliteStore`
- Replace `db::*` function calls with `store.*` method calls
- SqliteStore methods to use (already exist):
  - `store.get_or_create_repo()`
  - `store.upsert_file()`
  - `store.get_tree_sha()`
  - `store.set_tree_sha()`
  - etc.
- Remove parallel scanning (SQLite is single-writer)

## Implementation Notes

### Function Signature Changes
```rust
// Before
use tokio_postgres::Client;
pub async fn scan_worktree(client: &Client, ...) -> Result<ScanStats>

// After
use crate::db::SqliteStore;
pub async fn scan_worktree(store: &SqliteStore, ...) -> Result<ScanStats>
```

### Method Call Changes
```rust
// Before
let repo_id = db::get_or_create_repo(client, repo, root).await?;
db::upsert_file(client, ...).await?;

// After
let repo_id = store.get_or_create_repo(repo, root).await?;
store.upsert_file(&file_record).await?;
```

### Files in indexer/ Directory
- `mod.rs` - Main refactoring target
- `parallel.rs` - DELETE or merge into mod.rs
- Helper functions - Update to use SqliteStore

### Verification
```bash
# Check indexer module compiles
cargo check -p crewchief-maproom --lib 2>&1 | grep -E "indexer|Client"

# Verify no Client references
grep -r "tokio_postgres\|&Client" crates/maproom/src/indexer/
# Should return nothing
```

## Dependencies
- IDXABS-1001 (Delete PostgreSQL Database Files)
- IDXABS-1002 (Simplify db/mod.rs)
- IDXABS-1003 (Update Cargo.toml)

## Risk Assessment
- **Risk**: SqliteStore missing required methods
  - **Mitigation**: SqliteStore already implements most VectorStore methods
  - **Mitigation**: Add simple wrapper methods if needed
- **Risk**: Parallel scanning was providing important functionality
  - **Mitigation**: SQLite WAL mode handles concurrent reads
  - **Mitigation**: Single-threaded scan is simpler and adequate
- **Risk**: Different error types between PostgreSQL and SQLite
  - **Mitigation**: SqliteStore already returns anyhow::Result

## Files/Packages Affected
Files to MODIFY:
- `crates/maproom/src/indexer/mod.rs` - Change `&Client` to `&SqliteStore`
- Helper functions in indexer module

Files to DELETE:
- `crates/maproom/src/indexer/parallel.rs` (if exists, merge content first)
