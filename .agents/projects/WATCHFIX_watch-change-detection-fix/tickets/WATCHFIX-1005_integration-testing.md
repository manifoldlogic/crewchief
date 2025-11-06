# Ticket: WATCHFIX-1005: Write Integration Tests for Watch Command Fix

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Write comprehensive integration tests that verify the watch command fix works end-to-end: multiple files modified simultaneously are correctly detected, classified as Modified (not New), and successfully re-indexed with updated database timestamps.

## Background
The bug was discovered when modifying 3 files simultaneously - all were detected but zero were indexed. Integration tests must reproduce this scenario and verify it now works. These tests provide confidence the fix is complete and prevent regressions.

This ticket implements Phase 5 (Integration Testing) from the WATCHFIX project plan, ensuring the fixes from WATCHFIX-1002 and WATCHFIX-1003 work correctly in real-world scenarios.

## Acceptance Criteria
- [ ] Multi-file test passes: 3 files modified simultaneously, all 3 re-indexed with updated timestamps
- [ ] Single file test passes: 1 file modified, correctly classified as Modified
- [ ] New file test passes or documented as limitation (if file record creation unclear)
- [ ] Test utilities are reusable for future watch tests
- [ ] All tests run in < 10 seconds total
- [ ] Tests use real PostgreSQL database (Docker)
- [ ] Tests clean up after themselves (no leftover data/temp files)

## Technical Requirements

**Files to Create:**
- `crates/maproom/tests/watch_integration.rs` (~200 lines)
- `crates/maproom/tests/test_utils.rs` (~150 lines, if doesn't exist)

**Test 1: Multi-File Modification**
```rust
#[tokio::test]
async fn test_watch_multi_file_modification() {
    // Setup
    let pool = setup_test_db().await;
    let temp_dir = create_test_repo().await;
    seed_files(&pool, &temp_dir, vec!["src/a.rs", "src/b.rs", "src/c.rs"]).await;

    let start_time = Utc::now();

    // Start watch in background
    let watch_handle = start_watch(&pool, &temp_dir).await;

    // Modify all 3 files
    modify_file(&temp_dir.join("src/a.rs"), "// comment a").await;
    modify_file(&temp_dir.join("src/b.rs"), "// comment b").await;
    modify_file(&temp_dir.join("src/c.rs"), "// comment c").await;

    // Wait for processing (debounce + processing time)
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Assert all 3 files re-indexed
    assert_file_indexed_after(&pool, "src/a.rs", start_time).await;
    assert_file_indexed_after(&pool, "src/b.rs", start_time).await;
    assert_file_indexed_after(&pool, "src/c.rs", start_time).await;

    // Cleanup
    watch_handle.abort();
    cleanup_test_db(&pool).await;
}
```

**Test 2: Single File Modification**
```rust
#[tokio::test]
async fn test_watch_single_file_modified() {
    // Similar setup, modify 1 file, verify classified as Modified
    // Verify file exists in DB, modification detected, chunks updated
}
```

**Test 3: New File (Optional/TODO)**
```rust
#[tokio::test]
#[ignore] // Or skip if file record creation unclear
async fn test_watch_new_file() {
    // Create new file, verify indexed as New
    // NOTE: May need to clarify who creates file records during watch
}
```

**Test Utilities Required** (`test_utils.rs`):
- `setup_test_db()` - Create & migrate test database
- `cleanup_test_db()` - Truncate all tables
- `create_test_repo()` - Create temp directory with git repo structure
- `seed_files()` - Insert file records and initial chunks
- `modify_file()` - Helper to modify file content
- `assert_file_indexed_after()` - Query chunks, assert timestamp > start_time
- `start_watch()` - Spawn watch command in background task

**Database Configuration:**
- Use Docker PostgreSQL: `postgresql://maproom:maproom@localhost:5432/maproom`
- Assume database is running (docker-compose already started)
- Consider using separate test database to avoid conflicts

**Testing Environment:**
- Use `tempfile` crate for temporary directories
- Set `RUST_LOG=info` for tests to see watch output
- Use `tokio::time::timeout()` to prevent hanging tests
- Ensure proper cleanup on test failure (use Drop trait or defer patterns)

## Implementation Notes

**Testing Philosophy:**
- Focus on happy path (multi-file success scenario)
- Don't test every edge case (unit tests cover that)
- Prioritize confidence over exhaustive coverage
- Integration tests verify the fix works end-to-end

**Key Technical Considerations:**

1. **Test Database Setup:**
   - Tests must not interfere with each other
   - Consider test isolation strategies (separate DB, transaction rollback, or table truncation)
   - Ensure migrations run before tests

2. **Watch Command Execution:**
   - Spawn watch as background tokio task
   - Handle graceful shutdown (abort watch task in cleanup)
   - Use channels to monitor watch events if needed

3. **Timing and Synchronization:**
   - Account for file system debouncing (default 500ms in notify-debouncer-full)
   - Add buffer time for processing (total 5 seconds should be safe)
   - Use timeouts to prevent hanging tests

4. **File System Operations:**
   - Use `tempfile::TempDir` for automatic cleanup
   - Create realistic file structure (git repo with src/ directory)
   - Ensure file modifications trigger file system events

5. **Assertions:**
   - Query database to verify chunks were updated
   - Compare timestamps (indexed_at > start_time)
   - Verify chunk content reflects modifications

**Potential Issues:**
- CI environment may have different timing characteristics
- Docker database may not be available in CI (ensure GitHub Actions has postgres service)
- File system events may behave differently on different platforms

## Dependencies
- **WATCHFIX-1002** (processor_task fix being tested)
- **WATCHFIX-1003** (processor path handling being tested)
- Docker PostgreSQL must be running
- Database migrations must be available

## Risk Assessment

- **Risk**: Database setup in CI might fail
  - **Mitigation**: Ensure postgres service configured in GitHub Actions workflow, document setup requirements

- **Risk**: Tests might be flaky due to timing issues
  - **Mitigation**: Use generous timeouts (5+ seconds), add retries if needed, log timing information

- **Risk**: Unclear how new files get file records during watch
  - **Mitigation**: Document as limitation if needed, mark new file test as `#[ignore]` until clarified

- **Risk**: Tests interfere with each other if run in parallel
  - **Mitigation**: Use separate test databases or ensure proper cleanup between tests

- **Risk**: Temp directories not cleaned up on test failure
  - **Mitigation**: Use `tempfile::TempDir` which auto-cleans on Drop

## Files/Packages Affected
- **CREATE**: `crates/maproom/tests/watch_integration.rs` (~200 lines)
- **CREATE or MODIFY**: `crates/maproom/tests/test_utils.rs` (~150 lines)
- **POSSIBLY MODIFY**: `crates/maproom/Cargo.toml` (add test dependencies: `tempfile`, possibly `test-context`)

## Planning References
- `/workspace/.agents/projects/WATCHFIX_watch-change-detection-fix/planning/quality-strategy.md` - Multi-File Processing test section
- `/workspace/.agents/projects/WATCHFIX_watch-change-detection-fix/planning/plan.md` - Phase 5 deliverables
- `/workspace/.agents/projects/WATCHFIX_watch-change-detection-fix/planning/analysis.md` - Test Scenario section (evidence of bug)

## Estimated Effort
8 hours

## Priority
HIGH - Required to verify fix works and prevent regressions
