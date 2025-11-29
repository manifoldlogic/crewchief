# Ticket: UNIWATCH-4002: Create Integration Tests for Unified Watch

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive integration tests that verify the complete branch switch workflow, including file indexing, NDJSON event emission, debouncing, and edge cases like detached HEAD state.

## Background
Integration tests validate that all components work together correctly: HEAD watcher detection, dynamic state updates, database operations, and NDJSON output. These tests use real git operations to simulate user workflows.

**Plan Reference:** Phase 4 - Testing, Tasks 2-3

## Acceptance Criteria
- [x] Test file created: `crates/maproom/tests/unified_watch_test.rs`
- [x] Test 1: Complete branch switch workflow (main → feature → verify indexing)
- [x] Test 2: Rapid branch switches are debounced (3 switches in <2s → only final processed)
- [x] Test 3: File changes during branch switch don't get lost
- [x] Test 4: Detached HEAD creates SHA-named worktree
- [x] Test 5: --worktree flag backward compatibility (warning shown, auto-detection used)
- [x] All 5 integration tests pass: `cargo test --test unified_watch_test -- --nocapture`

## Technical Requirements
**Test file location:** `crates/maproom/tests/unified_watch_test.rs`

### Test 1: Complete Branch Switch Workflow
```rust
#[tokio::test]
async fn test_complete_branch_switch_workflow() {
    // Setup: Create temp git repo with main and feature branches
    // 1. Start watch on main
    // 2. Edit file, verify indexed to main worktree
    // 3. git checkout feature
    // 4. Wait for detection (<2s)
    // 5. Edit file, verify indexed to feature worktree
    // 6. Verify BranchSwitchEvent NDJSON was emitted
}
```

### Test 2: Rapid Branch Switches Debounced
```rust
#[tokio::test]
async fn test_rapid_branch_switches_debounced() {
    // Setup: Create temp git repo with branches b1, b2, b3
    // 1. Start watch
    // 2. git checkout b1, b2, b3 rapidly (<2s total)
    // 3. Wait for debounce window (2s)
    // 4. Verify only b3 worktree is current
    // 5. Verify single BranchSwitchEvent for b3
}
```

### Test 3: File Changes During Branch Switch
```rust
#[tokio::test]
async fn test_file_changes_during_branch_switch() {
    // 1. Start watch on main
    // 2. Spawn: git checkout feature
    // 3. Immediately edit file
    // 4. Wait for events to settle
    // 5. Verify file is indexed (to either branch, but not lost)
}
```

### Test 4: Detached HEAD State
```rust
#[tokio::test]
async fn test_detached_head_creates_sha_worktree() {
    // Setup: Create temp git repo with commits
    // 1. Start watch on main
    // 2. git checkout <commit-sha> (detached HEAD)
    // 3. Wait for detection
    // 4. Verify BranchSwitchEvent has 8-char SHA as new_branch
    // 5. Edit file, verify indexed to SHA-named worktree
}
```

### Test 5: Backward Compatibility
```rust
#[tokio::test]
async fn test_worktree_flag_backward_compatible() {
    // 1. Run watch with --worktree flag
    // 2. Verify deprecation warning in stderr
    // 3. Verify auto-detection is used (not the flag value)
}
```

## Implementation Notes
- Use `tempfile` crate for temporary directories
- Use `git2` or `std::process::Command` for git operations
- Capture stdout to verify NDJSON events
- Consider using `tokio::time::timeout` to prevent hanging tests
- Tests should clean up temporary repos
- **IMPORTANT:** Each test must create a fresh `DebouncedHandler` instance to avoid state leakage between tests. Do not share debouncer instances across tests or use static/global state.

**Test utilities to consider:**
```rust
async fn create_test_repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    Command::new("git").args(["init"]).current_dir(&dir).output().unwrap();
    Command::new("git").args(["checkout", "-b", "main"]).current_dir(&dir).output().unwrap();
    // ... initial commit
    dir
}
```

## Dependencies
- UNIWATCH-3001 (full implementation must be complete)
- UNIWATCH-4001 (unit tests should pass first)

## Risk Assessment
- **Risk**: Tests are flaky due to timing dependencies
  - **Mitigation**: Use explicit waits and generous timeouts
- **Risk**: Tests leave behind temp git repos
  - **Mitigation**: Use tempfile which auto-cleans on drop

## Files/Packages Affected
- `crates/maproom/tests/unified_watch_test.rs` (~200 lines new)
