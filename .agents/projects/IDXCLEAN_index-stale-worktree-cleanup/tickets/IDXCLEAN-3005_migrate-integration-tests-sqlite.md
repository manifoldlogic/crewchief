# Ticket: IDXCLEAN-3005: Migrate Integration Tests to SQLite

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary

Migrate cleanup integration tests from PostgreSQL to SQLite to match current database implementation.

## Background

The integration tests were originally written for PostgreSQL using `tokio_postgres::Client` connections. However, Maproom has migrated to SQLite exclusively. All cleanup integration tests are currently marked `#[ignore = "requires PostgreSQL database"]` and cannot run.

**Evidence:**
- `crates/maproom/tests/cleanup_detection_test.rs:207` - All tests ignored
- `crates/maproom/tests/cleanup_deletion_test.rs:286` - All tests ignored
- `crates/maproom/tests/cleanup_cli_test.rs:27` - All tests ignored
- `crates/maproom/src/db/cleanup.rs` - Implementation uses `SqliteStore`

**Impact:**
- 15+ critical safety tests cannot execute
- Deletion safety is unverified at integration level
- Quality-strategy.md specifies these tests as critical verification

## Acceptance Criteria

- [ ] `cleanup_detection_test.rs` rewritten to use `SqliteStore` instead of `tokio_postgres`
- [ ] `cleanup_deletion_test.rs` rewritten to use `SqliteStore` with `chunk_worktrees` junction table
- [ ] `cleanup_cli_test.rs` rewritten to use `SqliteStore`
- [ ] All `#[ignore = "requires PostgreSQL database"]` annotations removed
- [ ] Test fixtures use in-memory SQLite databases (`SqliteStore::new_test()`)
- [ ] Multi-worktree chunk tests use `chunk_worktrees` junction table (not JSONB arrays)
- [ ] All 15+ integration tests pass with `cargo test --test cleanup`
- [ ] CI/CD passes with cleanup tests enabled

## Technical Requirements

### Test Fixture Migration

**OLD (PostgreSQL):**
```rust
async fn setup_test_db() -> Client {
    let db_url = setup_temp_postgres().await;
    let (client, _) = tokio_postgres::connect(&db_url, NoTls).await.unwrap();
    client
}
```

**NEW (SQLite):**
```rust
async fn setup_test_db() -> SqliteStore {
    SqliteStore::new_test().await.expect("create test store")
}
```

### Multi-worktree Chunk Association

**OLD (JSONB arrays):**
```rust
let chunk_id = db.insert_chunk(Chunk {
    worktree_ids: json!([worktree_a_id, worktree_b_id]),
    // ...
}).await.unwrap();
```

**NEW (Junction table):**
```rust
let chunk_id = store.insert_chunk(Chunk { /* ... */ }).await.unwrap();
store.add_chunk_worktree(chunk_id, worktree_a_id).await.unwrap();
store.add_chunk_worktree(chunk_id, worktree_b_id).await.unwrap();
```

### Key Test Cases to Migrate

1. **Detection Tests:**
   - `test_detects_stale_worktree` - Worktree with non-existent path detected
   - `test_preserves_valid_worktree` - Valid worktrees not detected
   - `test_handles_permission_denied` - Permission errors treated as exists
   - `test_parallel_detection_performance` - 100 worktrees in <1 second

2. **Deletion Tests:**
   - `test_deletes_only_stale_worktrees` - Valid worktrees preserved
   - `test_transaction_rollback_on_error` - Error causes rollback
   - `test_multi_worktree_chunk_preserved` - Chunk in 2 worktrees, delete 1, chunk preserved
   - `test_single_worktree_chunk_garbage_collected` - Orphan chunk deleted

3. **CLI Tests:**
   - `test_cli_default_is_dry_run` - Default mode doesn't delete
   - `test_cli_confirm_actually_deletes` - --confirm flag works

## Implementation Notes

1. **Use existing SqliteStore test helpers:**
   - `SqliteStore::new_test()` creates in-memory database
   - Migrations run automatically on store creation

2. **Junction table API:**
   - `store.add_chunk_worktree(chunk_id, worktree_id)` - Associate chunk
   - `store.get_chunk_worktrees(chunk_id)` - Get worktree IDs
   - `store.remove_chunk_worktree(chunk_id, worktree_id)` - Disassociate

3. **No changes to implementation code required:**
   - `cleanup.rs` module already uses SqliteStore
   - Only test files need migration

### Running Tests

```bash
# Run all cleanup integration tests
cargo test --test cleanup_detection_test
cargo test --test cleanup_deletion_test
cargo test --test cleanup_cli_test

# Run all cleanup tests
cargo test cleanup

# Verify CI would pass
cargo test --all-targets
```

## Dependencies

- IDXCLEAN-1001 (Stale Detection Module) - Must be complete
- IDXCLEAN-1002 (Safe Deletion Module) - Must be complete

## Risk Assessment

- **Risk**: Test migration introduces bugs
  - **Mitigation**: Maintain same test logic, only change database layer

- **Risk**: SqliteStore API differs from expected
  - **Mitigation**: Check existing test patterns in codebase

## Files/Packages Affected

- `crates/maproom/tests/cleanup_detection_test.rs` - Rewrite for SQLite
- `crates/maproom/tests/cleanup_deletion_test.rs` - Rewrite for SQLite
- `crates/maproom/tests/cleanup_cli_test.rs` - Rewrite for SQLite

## Estimated Effort

6-8 hours
