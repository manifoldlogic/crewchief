# Ticket: SQLITE-0001: Migration System

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement a versioned schema migration system for SQLite to enable safe schema upgrades without data loss.

## Background
The existing `init_schema()` in `schema.rs` only creates tables if not exists - there's no versioning or upgrade path. Users upgrading from SQLFIX need a proper migration system before any schema changes can be applied.

**CRITICAL**: This is a BLOCKING prerequisite. Phase 1+ cannot begin until this is complete.

Implements: Plan Phase 0 - Migration Infrastructure

## Acceptance Criteria
- [x] `schema_migrations` table tracks applied migrations with version, name, and timestamp
- [x] `MigrationRunner::current_version()` returns current schema version (0 if no migrations)
- [x] `MigrationRunner::migrate()` applies pending migrations in version order
- [x] `MigrationRunner::needs_migration()` returns true if pending migrations exist
- [x] Each migration runs in a transaction and rolls back cleanly on failure
- [x] Running migrations twice is safe (idempotent)
- [x] Tests pass: test_migration_fresh_database, test_migration_idempotent, test_migration_version_tracking, test_migration_rollback_on_failure

## Technical Requirements
- Create `crates/maproom/src/db/sqlite/migrations.rs` module
- Use embedded SQL (not separate files) for simplicity
- Wrap existing `init_schema()` as migration version 1
- Structure migrations as:
  ```rust
  struct Migration {
      version: i32,
      name: &'static str,
      up: &'static str,
      down: &'static str,  // Best-effort rollback
  }
  ```
- Preserve existing `spawn_blocking` async pattern
- Version 1 must be backward-compatible with SQLFIX databases

## Implementation Notes
```rust
// Schema for migration tracking
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

// Migration runner pattern
pub struct MigrationRunner<'a> {
    conn: &'a Connection,
}

impl<'a> MigrationRunner<'a> {
    pub fn current_version(&self) -> Result<i32>;
    pub fn migrate(&self) -> Result<()>;
    pub fn needs_migration(&self) -> bool;
    fn migrations() -> Vec<Migration>;
}
```

Integrate into SqliteStore:
```rust
impl SqliteStore {
    pub async fn migrate(&self) -> Result<()> {
        self.run(|conn| {
            let runner = MigrationRunner::new(conn);
            runner.migrate()
        }).await
    }
}
```

## Dependencies
- None (this is the first ticket in the project)

## Risk Assessment
- **Risk**: Migration fails mid-way, leaving database in inconsistent state
  - **Mitigation**: Each migration runs in a transaction; failed migration rolls back completely
- **Risk**: Existing SQLFIX databases incompatible with migration version 1
  - **Mitigation**: Version 1 wraps existing `init_schema()` with IF NOT EXISTS semantics

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/migrations.rs` (NEW)
- `crates/maproom/src/db/sqlite/mod.rs` (add migrate() method, export module)
- `crates/maproom/src/db/sqlite/schema.rs` (minor refactoring to support migrations)
