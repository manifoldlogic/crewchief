# Ticket: IDXABS-4001: Fix and Update Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- `cargo test -p crewchief-maproom` must pass
- Run full test suite and document any remaining issues
- All tests should use SqliteStore (no PostgreSQL tests)

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update all tests to use SqliteStore, remove feature flags, and delete PostgreSQL-specific tests.

## Background
With PostgreSQL removed, all tests need to use SqliteStore. Tests currently use `#[cfg(feature = "sqlite")]` guards that should be removed, and any PostgreSQL-specific tests should be deleted.

**Reference**: Phase 4, Ticket 4001 of `planning/plan.md` - "Fix and Update Tests"
**Quality Strategy**: See `planning/quality-strategy.md` (note: being updated for SQLite-only)

## Acceptance Criteria
- [ ] `cargo test -p crewchief-maproom` passes
- [ ] No `#[cfg(feature = "sqlite")]` guards in test code
- [ ] No PostgreSQL-specific tests remain
- [ ] Test helpers create `SqliteStore` (not generic store)
- [ ] All integration tests use in-memory or temp SQLite databases
- [ ] Test coverage maintained for critical paths:
  - [ ] Indexer tests (scan, upsert)
  - [ ] Search tests (FTS, vector, hybrid)
  - [ ] Embedding tests (pipeline, cache)
  - [ ] Context tests (graph, relationships)

## Technical Requirements
- Remove `#[cfg(feature = "sqlite")]` from all test code
- Delete any `#[cfg(not(feature = "sqlite"))]` PostgreSQL tests
- Update test helper functions:
  - `create_test_store()` → returns `SqliteStore`
  - Use `":memory:"` or `tempfile` for test databases
- Fix any tests broken by refactoring

## Implementation Notes

### Test Helper Updates
```rust
// Before
#[cfg(feature = "sqlite")]
pub async fn create_test_sqlite_store() -> SqliteStore { ... }

pub async fn create_test_postgres_client() -> Client { ... }

// After
pub async fn create_test_store() -> SqliteStore {
    let store = SqliteStore::new(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}
```

### Feature Flag Removal
```rust
// Before
#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_search_sqlite() {
    let store = create_test_sqlite_store().await;
    // ...
}

#[tokio::test]
async fn test_search_postgres() {
    let client = create_test_postgres_client().await;
    // ...
}

// After
#[tokio::test]
async fn test_search() {
    let store = create_test_store().await;
    // ...
}
```

### Test Locations
- `crates/maproom/src/` - Unit tests in modules
- `crates/maproom/tests/` - Integration tests
- Look for `#[cfg(test)]` modules

### Verification
```bash
# Run all tests
cargo test -p crewchief-maproom

# Check for remaining feature flags
grep -rn "cfg.*sqlite\|cfg.*feature" crates/maproom/src/ crates/maproom/tests/
# Should return nothing (or very minimal)

# Check for remaining PostgreSQL test code
grep -rn "postgres\|Client" crates/maproom/tests/
# Should return nothing
```

### Common Test Fixes

1. **Type mismatches**: Update function calls to pass `&store` instead of `&client`
2. **Missing methods**: Ensure SqliteStore has all methods tests expect
3. **Database state**: Use fresh in-memory database for each test
4. **Async runtime**: Ensure `#[tokio::test]` is used consistently

## Dependencies
- IDXABS-1001 through IDXABS-3001 (all previous tickets)

## Risk Assessment
- **Risk**: Tests reveal missing SqliteStore functionality
  - **Mitigation**: Add missing methods as discovered
  - **Mitigation**: Document any significant gaps
- **Risk**: Tests pass but behavior differs from PostgreSQL
  - **Mitigation**: E2E validation in ticket 4002
  - **Mitigation**: SQLite behavior is well-documented
- **Risk**: Flaky tests due to SQLite locking
  - **Mitigation**: Use in-memory databases (`:memory:`)
  - **Mitigation**: Each test gets own database

## Files/Packages Affected
Files to MODIFY:
- `crates/maproom/src/*/mod.rs` - Unit tests in modules
- `crates/maproom/tests/*.rs` - Integration tests
- `crates/maproom/src/lib.rs` - Test helpers if defined there

Files to potentially DELETE:
- Any `*_postgres_test.rs` files
- Any `tests/postgres/` directories
