# Ticket: INCRSCAN-2001: Create Integration Tests for Scan Modes

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
Create comprehensive integration tests covering all scan modes: unchanged tree skip, changed tree scan, force flag override, first-time scan, and concurrent scans. Tests verify the critical path of incremental scanning works correctly.

## Background
From quality-strategy.md: "Critical paths must be tested: skip decision logic, state persistence, error handling. Integration tests use real database and temp git repos to verify end-to-end behavior."

The skip logic (INCRSCAN-1001) and state persistence (INCRSCAN-1002) are core features that must work reliably. False skips would be catastrophic (missing code changes), and false scans waste resources. These tests provide confidence the feature works correctly.

After implementing tree SHA checking and state persistence in Phase 1, we need comprehensive integration tests to verify all scan modes work correctly and state persists properly across different scenarios.

## Acceptance Criteria

- [ ] **test_unchanged_tree_skip:** Create repo with 10 files, commit, scan once. Scan again (same tree SHA). Expect: Early return before file walk, duration < 100ms, log "No changes detected"

- [ ] **test_changed_tree_scan:** Scan repo (tree abc123), modify file, commit (tree def456). Scan again. Expect: Full scan executes, all files processed, state updated to def456

- [ ] **test_force_flag_override:** Scan repo, no changes. Scan with --force flag. Expect: Full scan despite no changes, log "Force flag enabled", state updated

- [ ] **test_first_scan_state_creation:** Clean database (no worktree_index_state). Scan repo. Expect: Full scan executes, state record created, last_tree_sha populated

- [ ] **test_concurrent_scans:** Same repo, run two scans concurrently (spawn threads). Expect: Both complete successfully, state is consistent, ON CONFLICT DO UPDATE handles race

## Technical Requirements

### 1. Test File Structure
- Create `crates/maproom/tests/incremental_scan_integration.rs`
- Use `#[tokio::test]` for async tests
- Setup test database (`maproom_test`) before each test
- Clean state between tests

### 2. Git Repository Setup
- Create temp git repo in `/tmp` or system temp directory
- Add test files and commit
- Manipulate tree for different scenarios (modify files, create new commits)
- Clean up repos after tests

### 3. Database Setup
- Use test database connection
- Apply migrations before tests
- Clear `worktree_index_state` between tests
- Verify state after each test

### 4. Test Utilities
- Helper function to create test git repo
- Helper function to setup test database
- Helper function to run scan command
- Helper function to query index state

### 5. Coverage
- All critical paths from quality-strategy.md
- Positive cases (skip, scan, force)
- Edge cases (first-time, concurrent)
- State verification after each scenario

## Implementation Notes

**IMPORTANT: Scan Function Return Types**

The `scan_worktree()` and `scan_worktree_parallel()` functions return `Result<()>`, NOT a struct with statistics. To verify that files were processed or skipped, tests must:

1. Use ProgressTracker and check `files_processed()` / `chunks_processed()` getter methods (added in CRITICAL-2 fix)
2. Query the database to verify state was persisted
3. Measure timing for skip scenarios (< 100ms for skips)

All test templates below have been updated to use this approach.

### Test Template Structure

```rust
#[tokio::test]
async fn test_unchanged_tree_skip() {
    // Setup test database
    let db = setup_test_db().await;

    // Create temp git repo
    let temp_repo = create_test_repo().await;
    add_test_files(&temp_repo, 10).await;
    let commit1 = git_commit(&temp_repo, "Initial commit").await;

    // Create progress tracker to monitor scan activity
    let progress1 = ProgressTracker::new(OutputMode::Minimal);

    // First scan: should process all files
    let start = Instant::now();
    scan_worktree(
        &db,
        "test",
        "main",
        &temp_repo,
        &commit1,
        4, // concurrency
        None, // languages
        None, // exclude
        Some(&progress1), // progress tracker
    ).await.unwrap();

    // Verify files were processed by checking progress tracker
    // Note: scan functions return Result<()>, not stats, so we use ProgressTracker
    assert!(progress1.files_processed() > 0, "First scan should process files");

    // Verify state saved in database
    let state = get_index_state(&db, "test", "main").await.unwrap();
    assert!(state.last_tree_sha.is_some(), "State should have tree SHA after scan");
    let first_tree = state.last_tree_sha.unwrap();

    // Second scan: should skip (no changes)
    let progress2 = ProgressTracker::new(OutputMode::Minimal);
    let start = Instant::now();
    scan_worktree(
        &db,
        "test",
        "main",
        &temp_repo,
        &commit1,
        4,
        None,
        None,
        Some(&progress2),
    ).await.unwrap();
    let duration = start.elapsed();

    // Verify skip happened by checking:
    // 1. No files were processed (progress tracker shows 0)
    // 2. Scan was very fast (< 100ms)
    assert_eq!(progress2.files_processed(), 0, "Second scan should skip (no files processed)");
    assert!(duration < Duration::from_millis(100), "Skip should be fast");

    // Verify state unchanged (still same tree SHA)
    let state2 = get_index_state(&db, "test", "main").await.unwrap();
    assert_eq!(state2.last_tree_sha.unwrap(), first_tree, "Tree SHA should not change");

    // Cleanup
    cleanup_test_repo(&temp_repo).await;
    cleanup_test_db(&db).await;
}
```

### Helper Functions Required

```rust
async fn setup_test_db() -> Client {
    // Connect to maproom_test database
    // Apply migrations
    // Return client
}

async fn create_test_repo() -> PathBuf {
    // Create temp directory
    // git init
    // Configure test user
    // Return path
}

async fn add_test_files(repo: &Path, count: usize) {
    // Create N test files with code
    // Use TypeScript or Rust files
}

async fn git_commit(repo: &Path, message: &str) -> String {
    // git add .
    // git commit -m message
    // Return commit SHA
}

async fn get_index_state(db: &Client, repo: &str, worktree: &str) -> Result<IndexState> {
    // Query worktree_index_state table
    // Return state record
}

async fn cleanup_test_repo(repo: &Path) {
    // Remove temp directory
}

async fn cleanup_test_db(db: &Client) {
    // DELETE FROM worktree_index_state WHERE repo = 'test'
    // Clean up test data
}
```

### Testing Strategy
- Run: `cargo test incremental_scan_integration`
- Run: `cargo test incremental_scan_integration -- --nocapture` (for debugging)
- All tests must pass before moving to Phase 3

### Priority & Complexity
- **Priority:** P0 (must verify implementation works)
- **Complexity:** Medium
- **Estimated Time:** 2-3 hours

## Dependencies
- **INCRSCAN-1001** - tree-sha-check-skip-logic (must be implemented)
- **INCRSCAN-1002** - state-persistence (must be implemented)

## Risk Assessment

- **Risk:** Tests flaky due to timing
  - **Mitigation:** Use generous timeouts, don't test exact durations

- **Risk:** Tests leave temp files
  - **Mitigation:** Use Drop trait for cleanup, ensure cleanup runs

- **Risk:** Database state leaks between tests
  - **Mitigation:** Truncate tables in setup, use unique test data

## Files/Packages Affected
- `crates/maproom/tests/incremental_scan_integration.rs` - Main test file (NEW)
- `crates/maproom/tests/test_helpers/mod.rs` - Shared test utilities (NEW, optional)
