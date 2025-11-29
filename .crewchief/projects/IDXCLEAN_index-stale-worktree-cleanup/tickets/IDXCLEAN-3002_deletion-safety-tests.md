# Ticket: IDXCLEAN-3002: Integration Tests for Deletion Safety and Multi-Worktree Protection

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive integration tests verifying safe deletion with transaction integrity and multi-worktree chunk protection. This test suite ensures that only stale worktrees are deleted, multi-worktree chunks are preserved when only one worktree is deleted, single-worktree chunks are garbage collected correctly, and transactions roll back on errors.

## Background
This is the most critical test suite in the IDXCLEAN project. It must verify the core safety guarantees of the cleanup system:

1. **Selective deletion**: Only stale worktrees are deleted, never valid ones
2. **Multi-worktree chunk safety**: Chunks shared by multiple worktrees must be preserved when only one worktree is deleted
3. **Garbage collection**: Chunks belonging to only one worktree must be deleted when that worktree is cleaned up
4. **Transaction integrity**: Errors must trigger rollback to prevent partial state

These tests validate the implementation of IDXCLEAN-1002 (WorktreeCleaner module).

**References:**
- `plan.md`: Phase 3 - Integration Testing and Safety Validation, ticket IDXCLEAN-3002 (lines 431-466)
- `quality-strategy.md`: Path 2 - Deletion Safety (lines 111-184)
- `quality-strategy.md`: Scenario 4 - Multi-Worktree Chunk Safety (lines 425-468)
- `quality-strategy.md`: Scenario 5 - Garbage Collection Accuracy (lines 472-512)

## Acceptance Criteria
- [x] Test: Deletes only stale worktrees (not valid ones)
- [x] Test: Transaction rollback on error (failure mid-transaction reverts all changes)
- [x] Test: **Multi-worktree chunk safety** (chunk in 2 worktrees, delete 1, verify chunk preserved with updated worktree_ids)
- [x] Test: **Single-worktree garbage collection** (chunk in 1 worktree, delete it, verify chunk deleted)
- [x] Test: Array-based removal updates `worktree_ids` correctly (JSONB array manipulation)
- [x] Test: Dry-run mode makes no database changes
- [x] Test: Audit logging captures all deletions with proper context
- [x] Test: Handles concurrent operations safely
- [x] All tests pass

## Technical Requirements
- Real PostgreSQL database with `chunks.worktree_ids` as JSONB arrays
- Verify array manipulation: `worktree_ids - 'X'::TEXT` correctly removes worktree ID
- Verify garbage collection trigger: `jsonb_array_length(worktree_ids) = 0` causes chunk deletion
- Test fixtures must create realistic multi-worktree scenarios (2+ worktrees sharing chunks)
- Transaction tests must inject failures (invalid IDs, constraint violations) to verify rollback
- Use `tracing_subscriber::test` or equivalent to capture and verify audit log entries
- Tests must verify database state before and after operations
- Tests must handle async operations correctly (tokio runtime)

## Implementation Notes
**File to create:** `crates/maproom/tests/cleanup_deletion_test.rs`

**Key test patterns from quality-strategy.md:**

1. **Safe deletion test** (lines 136-161):
   - Create 1 valid worktree and 2 stale worktrees
   - Delete only the stale ones
   - Verify stale worktrees removed, valid worktree preserved

2. **Transaction rollback test** (lines 163-183):
   - Create 2 stale worktrees, make second have invalid ID
   - Attempt deletion (should fail)
   - Verify first worktree NOT deleted (rollback worked)

3. **Multi-worktree chunk safety test** (lines 426-468):
   - Create 2 worktrees (1 valid, 1 stale) sharing a chunk
   - Verify chunk has both worktree IDs in `worktree_ids` array
   - Delete stale worktree
   - **Critical**: Verify chunk still exists with only the valid worktree ID

4. **Garbage collection test** (lines 473-511):
   - Create stale worktree with chunk that belongs ONLY to that worktree
   - Verify chunk exists before deletion
   - Delete stale worktree
   - **Critical**: Verify chunk is deleted (garbage collected)

**Database setup:**
```rust
async fn setup_test_db() -> Database {
    // Create test database connection
    // Run migrations to ensure schema is current
    // Return Database instance
}

async fn create_valid_worktree(db: &Database, name: &str) -> i64 {
    // Insert worktree with path that exists (use temp dir)
}

async fn create_stale_worktree(db: &Database, path: &str) -> i64 {
    // Insert worktree with non-existent path
}
```

**Audit logging verification:**
- Use `tracing_subscriber::fmt()::with_test_writer()` to capture logs
- Parse log output to verify deletion events logged
- Verify context includes: worktree ID, path, chunk counts

## Dependencies
- IDXCLEAN-1002 (WorktreeCleaner module) - **MUST be completed first**
- PostgreSQL test database with pgvector extension
- Test database must support transactions and JSONB operations

## Risk Assessment
- **Risk**: Tests may pass without actually verifying JSONB array manipulation
  - **Mitigation**: Explicitly verify `worktree_ids` array contents before/after operations

- **Risk**: Transaction rollback tests might not catch partial state issues
  - **Mitigation**: Test multiple failure points (mid-transaction, constraint violations, network errors)

- **Risk**: Race conditions in concurrent operation tests
  - **Mitigation**: Use explicit transaction isolation levels and timing controls

- **Risk**: Test database state pollution between tests
  - **Mitigation**: Each test creates its own database or uses transactions that rollback

## Files/Packages Affected
**Files Created:**
- `crates/maproom/tests/cleanup_deletion_test.rs` - New integration test file

**Files Referenced:**
- `crates/maproom/src/cleanup/cleaner.rs` - WorktreeCleaner implementation being tested
- `crates/maproom/src/database/mod.rs` - Database operations used in tests
- `crates/maproom/src/cleanup/types.rs` - StaleWorktree and CleanupReport types

**Packages:**
- `tokio` - Async runtime for tests
- `sqlx` - Database operations
- `tracing-subscriber` - Log capture for audit trail verification
- `tempfile` - Creating temporary directories for valid worktree paths
