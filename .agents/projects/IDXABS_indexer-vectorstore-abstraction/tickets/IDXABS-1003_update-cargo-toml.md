# Ticket: IDXABS-1003: Update Cargo.toml

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

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
- [ ] `tokio-postgres` dependency removed
- [ ] `pgvector` dependency removed (if present)
- [ ] `deadpool-postgres` dependency removed (if present)
- [ ] `deadpool` dependency removed (if present)
- [ ] `postgres-types` dependency removed (if present)
- [ ] `default = ["postgres"]` removed from features
- [ ] `postgres = []` feature removed
- [ ] `sqlite = ["rusqlite", "r2d2", "r2d2_sqlite"]` feature removed (no longer optional)
- [ ] `rusqlite` is a required dependency (not optional)
- [ ] `r2d2` is a required dependency (not optional)
- [ ] `r2d2_sqlite` is a required dependency (not optional)
- [ ] `cargo build -p crewchief-maproom` succeeds (for dependency resolution)
- [ ] `cargo tree -p crewchief-maproom | grep -i postgres` returns nothing
- [ ] Document list of compilation errors remaining for Phase 2

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
