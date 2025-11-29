# Ticket: INCRSCAN-1004: Create Error Handling Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

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
Create integration tests for error scenarios: git command failures, database query failures, and state update failures. Verify the system safely falls back to full scans on errors and handles non-fatal state update errors correctly.

## Background
From quality-strategy.md: "Error Handling (Should Test) - Git failures don't crash scan, Database errors don't crash scan, State update failures are non-fatal, All errors log clearly."

From architecture.md: "Error Handling Strategy - Fail-Safe Defaults: When in doubt, do a full scan. Never skip incorrectly."

The fail-safe design is critical to correctness. If git or database operations fail, the system must default to scanning (not skipping), ensuring no code changes are missed. State update failures should be non-fatal since the scan itself succeeded.

After implementing core functionality (INCRSCAN-1001, INCRSCAN-1002) and basic integration tests (INCRSCAN-2001), we need tests specifically for error conditions to ensure safe fallback behavior.

## Acceptance Criteria

- [ ] **test_git_failure_fallback:** Setup invalid git environment (remove .git directory or use non-git path). Attempt scan. Expect: Warning logged, full scan executes (fallback), scan completes successfully without crash

- [ ] **test_db_query_failure:** Setup database error scenario (disconnect database or query non-existent table). Attempt scan with state query. Expect: Warning logged, full scan executes (safe fallback), scan completes successfully

- [ ] **test_state_update_failure:** Setup state update failure (make worktree_index_state table read-only or mock failure). Complete successful scan, attempt state update. Expect: Warning logged, scan returns success (non-fatal error), user informed of state update issue

## Technical Requirements

### 1. Test File Structure
- Create `crates/maproom/tests/scan_error_handling.rs`
- Use `#[tokio::test]` for async tests
- Mock or simulate error conditions
- Verify fallback behavior and logging
- Share test helpers with other integration tests if possible

### 2. Error Simulation Approaches
- **Git errors:** Use invalid repo path or corrupt .git directory
- **DB query errors:** Disconnect database mid-query, drop tables, or use mocking
- **State update errors:** Make table read-only or use permission-based failures

### 3. Verification Points
- Scan continues without crashing (returns Ok result)
- Full scan executes as fallback (files_processed > 0)
- Appropriate warnings logged (check tracing output)
- User-friendly error messages in logs

### 4. Test Utilities
- Reuse helper functions from `incremental_scan_integration.rs` if available
- Create error simulation helpers as needed
- Ensure proper cleanup in all test scenarios

## Implementation Notes

### Test 1: Git Failure Fallback

**Goal:** Verify system falls back to full scan when git operations fail.

```rust
#[tokio::test]
async fn test_git_failure_fallback() {
    let db = setup_test_db().await;

    // Create directory without .git (not a git repo)
    let fake_repo = create_temp_dir();
    add_test_files(&fake_repo, 5);

    // Attempt scan - should fallback to full scan despite git error
    let result = scan_worktree(
        &db,
        "test",
        "main",
        &fake_repo,
        "HEAD",
        4,
        None,
        None,
        None,
    ).await;

    // Verify: scan completes successfully (doesn't crash)
    assert!(result.is_ok(), "Scan should complete despite git error");

    // Verify: full scan executed (files were processed)
    let scan_result = result.unwrap();
    assert!(scan_result.files_processed > 0, "Should process files in fallback");

    // Verify: appropriate warning logged
    // Check tracing output for "Git tree SHA retrieval failed" or similar

    cleanup_temp_dir(&fake_repo);
}
```

### Test 2: Database Query Failure

**Goal:** Verify system falls back to full scan when database state query fails.

```rust
#[tokio::test]
async fn test_db_query_failure() {
    let db = setup_test_db().await;
    let temp_repo = create_test_repo().await;
    add_test_files(&temp_repo, 5);
    git_commit(&temp_repo, "Initial").await;

    // Drop worktree_index_state table to simulate query failure
    db.execute("DROP TABLE IF EXISTS maproom.worktree_index_state", &[])
        .await
        .unwrap();

    // Attempt scan - should fallback to full scan
    let result = scan_worktree(
        &db,
        "test",
        "main",
        &temp_repo,
        "HEAD",
        4,
        None,
        None,
        None,
    ).await;

    // Verify: scan completes despite database error
    assert!(result.is_ok(), "Scan should complete despite DB query failure");

    // Verify: full scan executed (safe fallback)
    let scan_result = result.unwrap();
    assert!(scan_result.files_processed > 0, "Should process files in fallback");

    // Restore table for other tests
    db::migrate(&db).await.unwrap();

    cleanup_test_repo(&temp_repo).await;
}
```

### Test 3: State Update Failure (Non-Fatal)

**Goal:** Verify state update failures don't fail the scan (non-fatal error handling).

**IMPORTANT:** The original approach used REVOKE/GRANT to simulate errors, but this requires database superuser permissions and may not be portable. Below are **three alternative approaches** in order of preference:

**Approach A: Drop Table (Recommended - Most Portable)**
```rust
#[tokio::test]
async fn test_state_update_failure() {
    let db = setup_test_db().await;
    let temp_repo = create_test_repo().await;
    add_test_files(&temp_repo, 5);
    git_commit(&temp_repo, "Initial").await;

    // First scan to create worktree/repo records
    let progress1 = ProgressTracker::new(OutputMode::Minimal);
    scan_worktree(&db, "test", "main", &temp_repo, "HEAD", 4, None, None, Some(&progress1))
        .await
        .unwrap();

    // Drop worktree_index_state table to simulate state update failure
    // This will cause the state update to fail, but scan should still succeed
    db.execute("DROP TABLE IF EXISTS maproom.worktree_index_state", &[])
        .await
        .unwrap();

    // Second scan should complete successfully despite state update failure
    let progress2 = ProgressTracker::new(OutputMode::Minimal);
    let result = scan_worktree(
        &db,
        "test",
        "main",
        &temp_repo,
        "HEAD",
        4,
        None,
        None,
        Some(&progress2),
    ).await;

    // Verify: scan succeeds despite state update failure (non-fatal)
    assert!(
        result.is_ok(),
        "Scan should succeed even if state update fails (non-fatal)"
    );

    // Verify: files were processed (check progress tracker)
    assert!(progress2.files_processed() > 0, "Scan should process files");

    // Verify: warning logged about state update failure
    // Check tracing output for "Failed to update index state" or similar

    // Restore table for cleanup (re-run migrations)
    db::migrate(&db).await.unwrap();

    cleanup_test_repo(&temp_repo).await;
}
```

**Approach B: Invalid Connection (Alternative)**
```rust
#[tokio::test]
async fn test_state_update_failure_via_invalid_connection() {
    // This approach tests state update errors by using an invalid connection
    // for the state update step. This is closer to real-world network failures.

    let db = setup_test_db().await;
    let temp_repo = create_test_repo().await;
    add_test_files(&temp_repo, 5);
    git_commit(&temp_repo, "Initial").await;

    // Note: This test would require modifying the scan command to inject
    // a connection failure for state update testing. May not be practical
    // for integration tests. Consider as a manual test or code review item.
}
```

**Approach C: Manual Testing (Fallback)**
If automated testing proves too complex:
1. Run scan normally to populate state
2. Manually disconnect database mid-scan
3. Verify scan completes with warning (not error)
4. Document findings in ticket verification

### Alternative Approaches

If mocking database failures proves complex:
- Use invalid connection strings (simulate network failure)
- Test error handling code paths directly (unit tests)
- Check log output for proper error messages
- Manual testing with disconnected database

If permission manipulation doesn't work:
- Mock the state update function at the module level
- Test error handling through code inspection
- Verify logging behavior in isolation

### Testing Strategy

**Run Tests:**
```bash
# Run all error handling tests
cargo test scan_error_handling

# Run with log output visible
cargo test scan_error_handling -- --nocapture

# Run with debug logging
RUST_LOG=debug cargo test scan_error_handling -- --nocapture

# Run specific test
cargo test test_git_failure_fallback
```

**Verify:**
- All 3 tests pass
- No panics or crashes
- Appropriate warnings in logs
- Fallback behavior confirmed

## Dependencies
- **INCRSCAN-1001** (tree-sha-check-skip-logic) - Must be implemented
- **INCRSCAN-1002** (state-persistence) - Must be implemented
- **INCRSCAN-2001** (integration-tests-scan-modes) - Should pass first to confirm core functionality works

## Risk Assessment

- **Risk:** Hard to simulate database failures in tests
  - **Mitigation (UPDATED):** Use portable table dropping approach (Approach A) rather than permission manipulation. If that's complex, use manual testing (Approach C). WARNING-4 addressed.

- **Risk:** Tests might not catch all error paths
  - **Mitigation:** Code review of error handling logic, focus on critical paths identified in quality-strategy.md

- **Risk:** Cleanup failures leave test database in bad state
  - **Mitigation:** Always restore tables via migrations in cleanup blocks. Use `defer` patterns or Drop trait for guaranteed cleanup

- **Risk:** Tests may not be portable across systems
  - **Mitigation (FIXED):** Updated to use DROP TABLE approach which doesn't require superuser permissions and works on all PostgreSQL installations

## Files/Packages Affected
- `crates/maproom/tests/scan_error_handling.rs` - New test file for error scenarios
- `crates/maproom/tests/test_helpers/mod.rs` - Shared test utilities (if exists, reuse helpers)

**Note:** This is a Phase 2 (Testing & Verification) ticket. Based on the phase-based numbering system, this might be more appropriately numbered INCRSCAN-2002, but INCRSCAN-1004 has been assigned per request.
