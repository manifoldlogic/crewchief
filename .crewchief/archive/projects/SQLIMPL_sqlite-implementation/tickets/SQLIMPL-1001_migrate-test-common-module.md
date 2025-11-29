# Ticket: SQLIMPL-1001: Migrate Test Common Module + Triage

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
Migrate the test common module from PostgreSQL to SQLite and triage all 35 test files to classify each as migrate, delete, or defer. This ticket establishes the foundation for all subsequent test migration work.

## Background
The maproom crate has 35 test files that reference PostgreSQL types (`tokio_postgres`, `PgPool`, `postgres::`). These tests don't compile because the PostgreSQL backend was removed. The common module (`tests/common/mod.rs`) provides shared test utilities and fixtures used by most tests.

This ticket implements Plan Phase 1, Ticket 1001: "Migrate Test Common Module + Triage".

## Acceptance Criteria
- [x] `tests/common/mod.rs` compiles with SQLite instead of PostgreSQL
- [x] In-memory database helper using `SqliteStore::connect(":memory:")` is functional
- [x] Test fixture patterns established for chunks, embeddings, and edges
- [x] Triage document created listing all 35 files with classification (migrate/delete/defer)
- [x] Common module compiles successfully (N/A: common is a module, not a test target)

## Technical Requirements
- Replace all `tokio_postgres` and `PgPool` imports with `rusqlite` and `SqliteStore`
- Use in-memory SQLite for test isolation: `SqliteStore::connect(":memory:")`
- Ensure migrations run automatically in test setup
- Create fixture helpers that match existing test patterns:
  ```rust
  pub fn test_store() -> SqliteStore {
      let store = SqliteStore::connect(":memory:").unwrap();
      // Migrations auto-run on connect
      store
  }

  pub fn sample_chunks() -> Vec<Chunk> { /* ... */ }
  pub fn sample_embeddings() -> Vec<Embedding> { /* ... */ }
  pub fn sample_edges() -> Vec<Edge> { /* ... */ }
  ```

## Implementation Notes

### Migration Pattern
```rust
// Before (PostgreSQL)
use tokio_postgres::Client;
use sqlx::postgres::PgPool;

pub async fn setup_test_db() -> PgPool {
    let pool = PgPool::connect(&url).await?;
    // ...
}

// After (SQLite)
use crewchief_maproom::db::sqlite::SqliteStore;

pub fn setup_test_db() -> SqliteStore {
    SqliteStore::connect(":memory:").expect("Failed to create test database")
}
```

### Triage Guidelines
For each of the 35 test files, determine:
1. **Migrate** - Test is valuable and should be converted to SQLite
2. **Delete** - Test is obsolete, tests removed functionality, or is redundant
3. **Defer** - Test requires stub implementations not yet done (Phase 2-4 work)

Create triage output in: `tests/TRIAGE.md`

### Files to Triage (35 total)
Reference: quality-strategy.md lists all files with complexity ratings

## Dependencies
- None (this is the first ticket)

## Risk Assessment
- **Risk**: Some test patterns may not translate directly to SQLite
  - **Mitigation**: Focus on compilation first; functional tests may fail initially
- **Risk**: Triage may reveal more issues than anticipated
  - **Mitigation**: Document issues but don't block; create follow-up tickets if needed

## Files/Packages Affected
- `crates/maproom/tests/common/mod.rs` (primary)
- `crates/maproom/tests/TRIAGE.md` (new - triage output)
