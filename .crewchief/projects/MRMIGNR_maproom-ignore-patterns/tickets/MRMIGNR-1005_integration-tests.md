# Ticket: [MRMIGNR-1005]: Integration Tests for Scan and Watch

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add comprehensive integration tests to verify end-to-end .maproomignore behavior in both scan and watch operations, including pattern precedence and error handling.

## Background
Unit tests (MRMIGNR-1004) verify individual components work correctly. Integration tests verify the complete workflow:
- Scan excludes files matching .maproomignore during indexing
- Watch filters events matching .maproomignore patterns
- .gitignore and .maproomignore work independently
- Invalid patterns cause startup failures with clear errors

These tests provide confidence that scan and watch integrations (MRMIGNR-1002, MRMIGNR-1003) work correctly in real-world scenarios.

Reference: Testing Strategy (plan.md lines 185-199), Quality Strategy (quality-strategy.md lines 68-92)

## Acceptance Criteria
- [ ] Test `test_scan_respects_maproomignore()` passes - scan excludes files matching patterns
- [ ] Test `test_watch_filters_maproomignore_events()` passes - watch filters events based on patterns
- [ ] Test `test_invalid_patterns_fail_startup()` passes - both scan and watch fail with clear errors
- [ ] Test `test_gitignore_still_works()` passes - .gitignore and .maproomignore both apply independently
- [ ] All integration tests pass when run with `cargo test -p crewchief-maproom maproomignore_test`
- [ ] All existing integration tests still pass (no regression)
- [ ] Tests use real git repositories (tempfile::TempDir + git init)
- [ ] Tests verify database state or event emission (not just code execution)
- [ ] Code passes `cargo clippy -p crewchief-maproom` with no warnings
- [ ] Code formatted with `cargo fmt`

## Technical Requirements

**Location**: Create new file `crates/maproom/tests/maproomignore_test.rs`

**Test infrastructure**:
- Use `tempfile::TempDir` to create temporary git repositories
- Initialize repos with `git init` via `std::process::Command`
- Create realistic file structures (src/, test-fixtures/, etc.)
- Use actual scan/watch APIs (not mocked)
- Query database to verify indexing behavior

**Required test cases** (from quality-strategy.md lines 69-92):

```rust
#[test]
fn test_scan_respects_maproomignore() {
    // 1. Create temp repo with test-fixtures/ directory
    // 2. Add .maproomignore with "test-fixtures/**"
    // 3. Run scan operation
    // 4. Query database - verify test-fixtures/ files NOT present
    // 5. Verify other files (e.g., src/) ARE present
}

#[tokio::test]
async fn test_watch_filters_maproomignore_events() {
    // 1. Start watch with .maproomignore excluding "*.tmp"
    // 2. Create/modify test.tmp file
    // 3. Assert no indexing event emitted (check logs or event channel)
    // 4. Modify normal file (e.g., main.rs) - verify event processed
    // 5. Verify database updated for main.rs, NOT for test.tmp
}

#[test]
fn test_invalid_patterns_fail_startup() {
    // 1. Create .maproomignore with invalid pattern "[invalid"
    // 2. Attempt scan - verify returns Err with clear message
    // 3. Attempt watch startup - verify fails with clear message
    // 4. Verify error message mentions which pattern failed
}

#[test]
fn test_gitignore_still_works() {
    // 1. Create .gitignore excluding "*.secret"
    // 2. Create .maproomignore excluding "test/**"
    // 3. Create files: test/data.txt, src/secret.key, src/main.rs
    // 4. Run scan
    // 5. Verify: src/main.rs indexed, test/data.txt excluded, src/secret.key excluded
    // 6. Both patterns should apply independently
}
```

**Test data strategy** (from quality-strategy.md lines 160-177):

```rust
fn create_test_repo_with_maproomignore(patterns: &[&str]) -> TempDir {
    let dir = TempDir::new().unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&dir)
        .status()
        .unwrap();

    // Write .maproomignore
    let ignore_file = dir.path().join(".maproomignore");
    std::fs::write(&ignore_file, patterns.join("\n")).unwrap();

    // Create test file structure
    std::fs::create_dir_all(dir.path().join("test-fixtures")).unwrap();
    std::fs::write(dir.path().join("test-fixtures/data.sql"), "SELECT * FROM test").unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();

    dir
}
```

## Implementation Notes

**Integration test patterns**:
- Create realistic repository structures
- Use actual scan/watch APIs (not unit test stubs)
- Verify observable behavior (database state, events, errors)
- Clean up automatically (TempDir drops at test end)

**Database verification approach**:
- After scan, query database for specific file paths
- Verify excluded files NOT in index
- Verify included files ARE in index
- Use existing database query utilities

**Watch testing approach** (challenging due to async nature):
- Use tokio::test for async tests
- Start watcher in background task
- Send file change events
- Collect emitted IndexingEvents via channel
- Assert events match expectations

**Pattern precedence verification**:
- Create overlapping .gitignore and .maproomignore patterns
- Verify both exclusion sets apply (not mutually exclusive)
- Document precedence in test comments

**Order of work**:
1. Create `maproomignore_test.rs` file
2. Add helper functions for test repo creation
3. Implement scan integration test
4. Implement watch integration test (async)
5. Implement invalid pattern test
6. Implement gitignore independence test
7. Run tests and verify all pass
8. Run full integration test suite to check for regressions

## Dependencies
- **Prerequisite**: MRMIGNR-1002 (scan integration must work), MRMIGNR-1003 (watch integration must work)
- **Blocks**: MRMIGNR-1006 (documentation should reference passing integration tests)
- **External dependencies**: None (uses existing test infrastructure)

## Risk Assessment
- **Risk**: Watch test is flaky due to timing issues
  - **Impact**: Medium - CI failures, developer frustration
  - **Mitigation**: Use explicit synchronization (channels, barriers). Wait for expected events with timeout. Make assertions deterministic.

- **Risk**: Tests require specific git configuration
  - **Impact**: Low - tests fail in some environments
  - **Mitigation**: Set minimal git config in test setup (user.name, user.email). Use `git -c` for test-specific config.

- **Risk**: Database state checks are brittle
  - **Impact**: Medium - tests break when DB schema changes
  - **Mitigation**: Use high-level query utilities. Check for presence/absence of files, not specific database internals.

- **Risk**: Tests don't catch real-world usage patterns
  - **Impact**: High - bugs slip through to production
  - **Mitigation**: Base tests on critical paths from quality-strategy.md. Add manual smoke testing (MRMIGNR-1002, MRMIGNR-1003).

## Files/Packages Affected
- `crates/maproom/tests/maproomignore_test.rs` (new file)
- No production code changes in this ticket (test-only)

## Verification Notes
The verify-ticket agent should confirm:
1. File `crates/maproom/tests/maproomignore_test.rs` exists
2. All 4 specified test functions implemented
3. Tests create real git repositories (not mocked)
4. Scan test verifies database state (files present/absent)
5. Watch test verifies event filtering (async test)
6. Invalid pattern test verifies error messages
7. Gitignore independence test verifies both exclusion sets
8. All integration tests pass: `cargo test -p crewchief-maproom maproomignore_test`
9. Existing integration tests still pass (no regression)
10. No clippy warnings in test code
11. Test code formatted properly

**Test execution**:
```bash
# Run just the new integration tests
cargo test -p crewchief-maproom maproomignore_test -- --nocapture

# Expected output: all 4 tests pass
# test test_scan_respects_maproomignore ... ok
# test test_watch_filters_maproomignore_events ... ok
# test test_invalid_patterns_fail_startup ... ok
# test test_gitignore_still_works ... ok

# Run full integration test suite to check for regressions
cargo test -p crewchief-maproom --test '*'
```

**Coverage verification** (from quality-strategy.md lines 109-130):
- All critical paths from plan.md have integration tests
- Scan exclusion verified with database queries
- Watch filtering verified with event collection
- Error handling verified with startup failures
- Pattern precedence verified with overlapping patterns
