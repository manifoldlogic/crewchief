# Ticket: IDXABS-1001: Delete PostgreSQL Database Files

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (deletion ticket, expected cargo check errors)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- This ticket involves file deletion only; `cargo check` will show expected errors
- "Tests pass" is N/A for this ticket - next ticket (1002) will address compile errors
- Run `cargo check` to verify expected missing import errors

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Remove all PostgreSQL-specific files from the `db/` module to begin the SQLite-only migration.

## Background
The maproom crate currently maintains two database backends (PostgreSQL and SQLite) with complex abstraction layers. Per the SQLite-only migration plan, we're removing PostgreSQL entirely to simplify the codebase.

This is the first ticket in Phase 1 (Delete PostgreSQL Code) and is a prerequisite for all subsequent refactoring work.

**Reference**: Phase 1 of `planning/plan.md` - "Delete PostgreSQL Code"

## Acceptance Criteria
- [x] `db/postgres/` directory is deleted (entire directory)
- [x] `db/pool.rs` is deleted
- [x] `db/queries.rs` is deleted (28 PostgreSQL-specific query functions)
- [x] `db/factory.rs` is deleted (backend switching logic)
- [x] `db/materialized_views.rs` is deleted
- [x] `cargo check` shows expected missing import errors (not unexpected errors)
- [x] No PostgreSQL-specific files remain in `db/`

## Technical Requirements
- Delete files completely (not comment out)
- Do not modify any other files in this ticket
- Expected errors from `cargo check`: missing imports for deleted modules
- Document which errors appear so next ticket knows what to fix

## Implementation Notes

### Files to Delete
```
crates/maproom/src/db/
├── postgres/           # DELETE entire directory
├── pool.rs             # DELETE
├── queries.rs          # DELETE
├── factory.rs          # DELETE
└── materialized_views.rs # DELETE
```

### Expected Cargo Check Output
After deletion, `cargo check` will fail with errors like:
- `cannot find module postgres`
- `unresolved import db::pool`
- `cannot find function get_store_with_type`
- Errors in files that import from deleted modules

This is expected and will be fixed in subsequent tickets.

### Verification
Run after deletion:
```bash
# Verify files are gone
ls -la crates/maproom/src/db/

# Expect compilation errors
cargo check --bin crewchief-maproom 2>&1 | head -50
```

## Dependencies
- None (first ticket in the project)

## Risk Assessment
- **Risk**: Accidentally delete files that should be kept
  - **Mitigation**: Only delete files explicitly listed in scope
  - **Mitigation**: Git provides recovery if needed
- **Risk**: Some PostgreSQL code might be in unexpected locations
  - **Mitigation**: This ticket focuses on db/ only; later tickets handle other modules

## Files/Packages Affected
Files to DELETE:
- `crates/maproom/src/db/postgres/` (entire directory)
- `crates/maproom/src/db/pool.rs`
- `crates/maproom/src/db/queries.rs`
- `crates/maproom/src/db/factory.rs`
- `crates/maproom/src/db/materialized_views.rs`
