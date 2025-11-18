# Ticket: IDXCLEAN-3001: Integration Tests for Detection Accuracy

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive integration tests verifying correct identification of stale vs. valid worktrees using real PostgreSQL database and filesystem fixtures.

## Background
Detection accuracy is critical for preventing data loss. This ticket implements the test suite that validates the StaleWorktreeDetector can correctly distinguish between worktrees that should be deleted (stale) and those that must be preserved (valid).

This ticket implements Phase 3 - Integration Testing and Safety Validation from plan.md (lines 395-428), specifically ticket IDXCLEAN-3001. The test strategy is detailed in quality-strategy.md Path 1 - Detection Accuracy (lines 47-109).

## Acceptance Criteria
- [ ] Test: Detects worktree with non-existent path
- [ ] Test: Does not detect worktree with valid path
- [ ] Test: Handles multiple stale worktrees correctly
- [ ] Test: Empty database returns no stale worktrees
- [ ] Test: Permission denied treated as exists (worktree preserved)
- [ ] Test: Handles special characters in paths
- [ ] All tests pass with clear assertions

## Technical Requirements
- Use real PostgreSQL test database (not mocks) connected via TEST_DATABASE_URL
- Use tempfile crate for temporary directory fixtures
- Tests must be idempotent (can run multiple times without side effects)
- Clean up test data after each test (both database and filesystem)
- Use tokio::test for async tests
- Follow existing test patterns in crates/maproom/tests/
- Each test should use a unique repo_id to avoid conflicts
- Use transaction rollback or explicit cleanup for database state

## Implementation Notes
Create a new test file `crates/maproom/tests/cleanup_detection_test.rs` that imports and tests the StaleWorktreeDetector module.

Test structure should follow this pattern (from quality-strategy.md lines 71-108):

```rust
#[tokio::test]
async fn test_detects_stale_worktree() {
    // Setup: Create temp dir, insert worktree with non-existent path
    // Action: Run detector.find_stale_worktrees()
    // Assert: Stale worktree is detected
}
```

Key test scenarios:
1. **Stale detection**: Insert worktree with path that doesn't exist, verify it's detected
2. **Valid preservation**: Insert worktree with path that exists, verify it's NOT detected
3. **Multiple worktrees**: Mix of stale and valid, verify correct filtering
4. **Empty database**: No worktrees returns empty list
5. **Permission errors**: Paths that exist but are inaccessible are treated as valid
6. **Special characters**: Paths with spaces, unicode, etc.

Each test should:
- Use a unique `repo_id` (e.g., UUID or test-specific string)
- Create temporary directories using tempfile::TempDir
- Insert test data into maproom.worktrees table
- Call detector and verify results
- Clean up both database and filesystem

## Dependencies
- IDXCLEAN-1001 (StaleWorktreeDetector module implementation)

## Risk Assessment
- **Risk**: Tests may interfere with each other if using shared database state
  - **Mitigation**: Use unique repo_id per test and explicit cleanup
- **Risk**: Test directories may not be cleaned up on test failure
  - **Mitigation**: Use tempfile::TempDir which auto-cleans on drop
- **Risk**: Permission tests may behave differently across platforms
  - **Mitigation**: Document platform-specific behavior, skip if needed

## Files/Packages Affected
- `crates/maproom/tests/cleanup_detection_test.rs` (new test file)
- `crates/maproom/Cargo.toml` (dev-dependencies: tempfile if not present)

## Planning References
- `/workspace/.agents/projects/IDXCLEAN_index-stale-worktree-cleanup/planning/plan.md` (lines 395-428)
- `/workspace/.agents/projects/IDXCLEAN_index-stale-worktree-cleanup/planning/quality-strategy.md` (lines 47-109)
