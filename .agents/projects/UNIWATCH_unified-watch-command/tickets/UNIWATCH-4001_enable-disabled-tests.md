# Ticket: UNIWATCH-4001: Enable and Migrate Disabled Unit Tests

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - all 3 migrated tests pass, existing tests unaffected
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Enable the 3 UNIWATCH-prefixed unit tests that are currently disabled with `#[cfg(disabled_postgresql_test)]` by migrating them from PostgreSQL to SQLite.

## Background
During the IDXABS-2001 SQLite migration, several unit tests were disabled because they reference the old PostgreSQL-based infrastructure. These tests cover critical branch switching functionality and must be migrated to use SQLite before the feature can be considered complete.

**Plan Reference:** Phase 4 - Testing, Task 1

## Acceptance Criteria
- [x] `test_worktree_tracking_initialization` migrated to SQLite and enabled
- [x] `test_handle_branch_switch_updates_state` migrated to SQLite and enabled
- [x] `test_handle_branch_switch_skips_if_same_branch` migrated to SQLite and enabled
- [x] All `#[cfg(disabled_postgresql_test)]` annotations removed from these 3 tests
- [x] All 3 migrated tests pass: `cargo test -p crewchief-maproom -- --test-threads=1 test_worktree`
- [x] Existing working tests still pass (test_debounced_handler, test_branch_switch_event_serialization, test_dual_watchers, test_event_loop_handles_both_sources)

## Technical Requirements
**Tests to migrate (in indexer/mod.rs):**

| Test | Line | Current Issue | Migration Required |
|------|------|--------------|-------------------|
| `test_worktree_tracking_initialization` | 780 | Uses `crate::db::pool::create_pool()` | Use `SqliteStore::new()` with temp file |
| `test_handle_branch_switch_updates_state` | 935 | Calls removed `handle_branch_switch` | Call new function in main.rs |
| `test_handle_branch_switch_skips_if_same_branch` | 1103 | Calls removed `handle_branch_switch` | Call new function in main.rs |

**Note:** `test_event_loop_handles_both_sources` (line 1496) is NOT disabled and already works.

**Migration pattern:**
```rust
// BEFORE (PostgreSQL)
#[cfg(disabled_postgresql_test)]
#[tokio::test]
async fn test_worktree_tracking_initialization() {
    let pool = crate::db::pool::create_pool().await.unwrap();
    // ...
}

// AFTER (SQLite)
#[tokio::test]
async fn test_worktree_tracking_initialization() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = SqliteStore::new(&db_path).await.unwrap();
    // ...
}
```

**Tests already working (no changes needed):**
- `test_debounced_handler_prevents_rapid_events`
- `test_branch_switch_event_serialization`
- `test_dual_watchers_initialize`
- `test_event_loop_handles_both_sources` (line 1496)

## Implementation Notes
- Tests for `handle_branch_switch` may need adjustment since the function moved from indexer to main.rs
- Consider making the handler function signature testable (extract core logic if needed)
- Use `tempfile` crate for temporary SQLite databases in tests
- Tests should be independent and not share state

**Key file:** `crates/maproom/src/indexer/mod.rs` (test module section)

## Dependencies
- UNIWATCH-3001 (handle_branch_switch must be implemented for tests to call)

## Risk Assessment
- **Risk**: Test migration reveals bugs in new implementation
  - **Mitigation**: This is actually positive - tests are catching issues
- **Risk**: Tests tightly coupled to old PostgreSQL structure
  - **Mitigation**: Rewrite tests for new SQLite patterns rather than adapting

## Files/Packages Affected
- `crates/maproom/src/indexer/mod.rs` (test module section, ~100 lines modified)
