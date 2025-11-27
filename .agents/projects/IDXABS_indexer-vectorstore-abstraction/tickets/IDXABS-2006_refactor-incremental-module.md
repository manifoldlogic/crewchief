# Ticket: IDXABS-2006: Refactor Incremental Module

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- All incremental module tests must pass

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update ALL incremental module files to use SqliteStore instead of PostgreSQL's `&Client`. The incremental module handles file watching and incremental updates, which is critical for the `watch` command.

## Background
The `incremental/` module was missed in the original ticket planning. It contains 8 PostgreSQL references across 3 files that must be refactored for SQLite-only operation.

**Reference**: Phase 2, Ticket 2006 of `planning/plan.md`

**PostgreSQL reference analysis**:
- `incremental/edge_updater.rs` - 4 references
- `incremental/processor.rs` - 1 reference
- `incremental/tree_sha_update.rs` - 3 references

## Acceptance Criteria
- [ ] All incremental tests pass
- [ ] No `&Client` references in incremental/
- [ ] No `tokio_postgres` imports in incremental/
- [ ] Verification: `grep -r "tokio_postgres\|&Client" crates/maproom/src/incremental/` returns nothing
- [ ] `watch` command works correctly (uses incremental module)

## Technical Requirements
- Replace all `&Client` parameters with `&SqliteStore`
- Update function signatures throughout the module
- Replace raw SQL queries with SqliteStore method calls
- If SqliteStore lacks required methods, implement them in `db/sqlite/mod.rs`
- Maintain existing functionality for incremental updates

## Implementation Notes

### Files to Update

#### `incremental/edge_updater.rs` (4 refs)
- Handles updating chunk edges during incremental indexing
- Look for `tokio_postgres::Client` parameter types
- Update edge update queries to use SqliteStore methods

#### `incremental/processor.rs` (1 ref)
- Main incremental processing logic
- Update database interaction to use SqliteStore

#### `incremental/tree_sha_update.rs` (3 refs)
- Handles tree SHA updates for change detection
- Update SHA comparison and storage queries

### Pattern to Follow
```rust
// Before
pub async fn update_edges(client: &Client, ...) -> Result<()>

// After
pub fn update_edges(store: &SqliteStore, ...) -> Result<()>
```

### SqliteStore Methods
Check if these methods exist in SqliteStore, add if needed:
- `update_chunk_edges()`
- `update_tree_sha()`
- `get_tree_sha()`
- Edge/relationship update methods

## Dependencies
- IDXABS-1001 through IDXABS-1003 (Phase 1 complete)
- IDXABS-2001 (Indexer module refactored - may share patterns)

## Risk Assessment
- **Risk**: Watch command functionality breaks
  - **Mitigation**: Test watch command explicitly after changes
  - **Mitigation**: Preserve exact behavior of incremental update logic
- **Risk**: Missing SqliteStore methods
  - **Mitigation**: Add methods as needed, following existing patterns
- **Risk**: Performance regression in file watching
  - **Mitigation**: SQLite single-writer is acceptable for incremental updates

## Files/Packages Affected
Files to MODIFY:
- `crates/maproom/src/incremental/edge_updater.rs`
- `crates/maproom/src/incremental/processor.rs`
- `crates/maproom/src/incremental/tree_sha_update.rs`
- `crates/maproom/src/incremental/mod.rs` (if needed for exports)

Files to potentially MODIFY (add methods):
- `crates/maproom/src/db/sqlite/mod.rs`
