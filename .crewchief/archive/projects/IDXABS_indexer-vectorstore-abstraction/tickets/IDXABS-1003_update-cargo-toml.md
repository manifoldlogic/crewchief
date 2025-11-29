# Ticket: IDXABS-1003: Update Cargo.toml

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (dependency resolution verified, code errors expected until Phase 2)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- `cargo build` should succeed for the crate dependencies
- Full compilation may still fail due to code using deleted modules
- Verify Cargo.lock no longer contains PostgreSQL crates

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Remove PostgreSQL dependencies and feature flags from Cargo.toml, making SQLite dependencies required (not optional).

## Background
With PostgreSQL code deleted, the PostgreSQL dependencies are no longer needed. The `[features]` section for SQLite should also be removed since SQLite is now the only backend.

**Reference**: Phase 1, Ticket 1003 of `planning/plan.md` - "Update Cargo.toml"
**Architecture**: See `planning/architecture.md` - "Decision 2: Remove Feature Flags"

## Acceptance Criteria
- [x] `tokio-postgres` dependency removed
- [x] `pgvector` dependency removed (if present)
- [x] `deadpool-postgres` dependency removed (if present)
- [x] `deadpool` dependency removed (if present)
- [x] `postgres-types` dependency removed (if present)
- [x] `default = ["postgres"]` removed from features
- [x] `postgres = []` feature removed
- [x] `sqlite = ["rusqlite", "r2d2", "r2d2_sqlite"]` feature removed (no longer optional)
- [x] `rusqlite` is a required dependency (not optional)
- [x] `r2d2` is a required dependency (not optional)
- [x] `r2d2_sqlite` is a required dependency (not optional)
- [x] `cargo build -p crewchief-maproom` succeeds (for dependency resolution)
- [x] `cargo tree -p crewchief-maproom | grep -i postgres` returns nothing
- [x] Document list of compilation errors remaining for Phase 2

## Phase 2 Error Summary (82 errors)
Errors remaining that will be fixed in Phase 2 tickets:
- 17x `tokio_postgres` unresolved module (embedding/, context/, indexer/)
- 16x `tokio_postgres` unresolved import
- 10x `crate::db::PgPool` unresolved
- 5x `deadpool_postgres` unresolved module
- 5x each: `insert_chunk`, `get_or_create_worktree`, `get_or_create_repo` not found
- 4x `PgPool` type not found
- 3x each: `upsert_file`, `get_or_create_commit` not found
- 2x each: `insert_chunks_batch`, `find_chunk_by_symbol` not found
- 1x `queries` module, `VectorStore` trait, `pool` module references

## Technical Requirements
- Remove dependencies completely, not just comment out
- Ensure `rusqlite` keeps required features: `bundled`, `backup`
- Remove `optional = true` from rusqlite if present
- Remove any `default = []` features referencing sqlite

## Implementation Notes

### Dependencies to REMOVE
```toml
# Remove these from [dependencies]:
tokio-postgres = "..."
pgvector = "..."
deadpool-postgres = "..."
deadpool = "..."
postgres-types = "..."
```

### Dependencies to KEEP/MODIFY
```toml
# Keep rusqlite, remove optional flag
rusqlite = { version = "0.31", features = ["bundled", "backup"] }

# Keep r2d2 dependencies, remove optional flags
r2d2 = { version = "0.8" }
r2d2_sqlite = { version = "0.24" }

# sqlite-vec is statically linked via db/sqlite/mod.rs, not Cargo.toml
```

### Features Section Changes
```toml
# Current (incorrect):
[features]
default = ["postgres"]
profiling = ["puffin"]
sqlite = ["rusqlite", "r2d2", "r2d2_sqlite"]
postgres = []

# Target (correct):
[features]
profiling = ["puffin"]
# No default, no postgres, no sqlite feature - SQLite deps are required
```

### Verification
```bash
# Check dependencies resolve
cargo build -p crewchief-maproom --lib

# Verify no PostgreSQL in dependency tree
cargo tree -p crewchief-maproom 2>/dev/null | grep -i postgres
# Should return nothing

# Check Cargo.lock
grep -i "tokio-postgres\|pgvector\|deadpool-postgres" Cargo.lock
# Should return nothing
```

## Dependencies
- IDXABS-1001 (Delete PostgreSQL Database Files)
- IDXABS-1002 (Simplify db/mod.rs)

## Risk Assessment
- **Risk**: Other crates in workspace depend on PostgreSQL crates
  - **Mitigation**: Check if maproom-mcp or other packages need PostgreSQL
  - **Mitigation**: Only modify crewchief-maproom Cargo.toml
- **Risk**: rusqlite version incompatibility
  - **Mitigation**: Keep existing version, just remove optional flag
- **Risk**: Missing required features on rusqlite
  - **Mitigation**: Verify `bundled` and `backup` features are included

## Files/Packages Affected
Files to MODIFY:
- `crates/maproom/Cargo.toml` - Remove PostgreSQL deps, remove features section

Files to CHECK (may be updated automatically):
- `Cargo.lock` - Should remove PostgreSQL crates after `cargo update`
