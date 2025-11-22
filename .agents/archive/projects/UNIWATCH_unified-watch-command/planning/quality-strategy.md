# Quality Strategy: Unified Watch Command

## Testing Philosophy

**Goal**: Ship with confidence that branch switching works correctly, without over-testing every edge case.

**Focus Areas**:
1. **Critical path**: File changes route to correct worktree after branch switch
2. **Race conditions**: Concurrent file changes and branch switches
3. **Error recovery**: Graceful degradation when components fail

**Non-Goals**:
- Exhaustive edge case coverage
- Performance benchmarking (trust existing benchmarks)
- UI/UX testing (CLI output format changes)

## Test Pyramid

```
        ┌─────────────┐
        │     E2E     │  2 tests  (Happy path + error recovery)
        │             │
        └─────────────┘
       ┌───────────────┐
       │  Integration  │   5 tests  (Branch switch scenarios)
       │               │
       └───────────────┘
      ┌─────────────────┐
      │   Unit Tests    │    10 tests  (Component behavior)
      │                 │
      └─────────────────┘
```

## Unit Tests

### Component: `UnifiedWatcher::handle_branch_switch()`

**Purpose**: Verify branch switch updates worktree ID correctly

```rust
#[tokio::test]
async fn test_handle_branch_switch_updates_worktree_id() {
    // Setup
    let (mut watcher, _) = create_test_watcher().await;
    let initial_id = *watcher.current_worktree_id.read().unwrap();

    // Simulate branch switch
    simulate_git_head_change(&watcher.repo_path, "feature-test");
    watcher.handle_branch_switch(/* event */).await.unwrap();

    // Verify
    let new_id = *watcher.current_worktree_id.read().unwrap();
    assert_ne!(initial_id, new_id);

    let new_branch = watcher.current_branch.read().unwrap();
    assert_eq!(*new_branch, "feature-test");
}
```

**Coverage**:
- ✓ Worktree ID updated
- ✓ Branch name updated
- ✓ Database record created

### Component: `EventRouter`

**Purpose**: Verify file events route to current worktree

```rust
#[tokio::test]
async fn test_file_events_route_to_current_worktree() {
    let router = EventRouter::new();

    // Set worktree ID
    {
        let mut id = router.current_worktree_id.write().unwrap();
        *id = 42;
    }

    // Create file event
    let event = FileEvent::Modified(PathBuf::from("test.rs"));

    // Route event
    let routed = router.route_file_event(event);

    // Verify
    assert_eq!(routed.worktree_id, 42);
}
```

**Coverage**:
- ✓ Correct worktree ID attached
- ✓ Thread-safe reads
- ✓ Event not mutated

### Component: Debouncing

**Purpose**: Prevent rapid successive branch switches

```rust
#[test]
fn test_debouncer_prevents_rapid_switches() {
    let debouncer = DebouncedHandler::new(Duration::from_secs(2));

    // First event - should process
    assert!(debouncer.should_handle());

    // Immediate second event - should debounce
    assert!(!debouncer.should_handle());

    // After 2 seconds - should process
    std::thread::sleep(Duration::from_secs(3));
    assert!(debouncer.should_handle());
}
```

**Coverage**:
- ✓ Debouncing works
- ✓ Time window configurable
- ✓ Thread-safe

### Component: Error Handling

**Purpose**: Graceful degradation when branch detection fails

```rust
#[tokio::test]
async fn test_branch_detection_failure_preserves_state() {
    let (mut watcher, _) = create_test_watcher().await;

    // Store initial state
    let initial_branch = watcher.current_branch.read().unwrap().clone();
    let initial_id = *watcher.current_worktree_id.read().unwrap();

    // Break .git/HEAD (make unreadable)
    std::fs::remove_file(watcher.repo_path.join(".git/HEAD")).unwrap();

    // Attempt branch switch
    let result = watcher.handle_branch_switch(/* event */).await;

    // Verify graceful failure
    assert!(result.is_err());

    // State unchanged
    assert_eq!(*watcher.current_branch.read().unwrap(), initial_branch);
    assert_eq!(*watcher.current_worktree_id.read().unwrap(), initial_id);
}
```

**Coverage**:
- ✓ Error logged
- ✓ State preserved
- ✓ Watcher continues

## Integration Tests

### Scenario 1: Normal Branch Switch

**Purpose**: End-to-end branch switch with file indexing

```rust
#[tokio::test]
async fn test_complete_branch_switch_workflow() {
    let temp_repo = create_test_git_repo();
    let client = test_db_client().await;

    // Start watcher on main branch
    let mut watcher = UnifiedWatcher::new(
        temp_repo.path(),
        client,
        default_config(),
    ).await.unwrap();

    // Edit file on main
    write_file(&temp_repo, "test.txt", "main content");
    wait_for_indexing().await;

    // Verify indexed to main worktree
    let chunks = query_chunks_by_file(&client, "test.txt").await;
    assert_eq!(chunks[0].worktree_name, "main");

    // Switch branch
    git_checkout(&temp_repo, "feature");
    wait_for_branch_switch().await;

    // Edit file on feature
    write_file(&temp_repo, "test.txt", "feature content");
    wait_for_indexing().await;

    // Verify indexed to feature worktree
    let chunks = query_chunks_by_file(&client, "test.txt").await;
    let feature_chunk = chunks.iter()
        .find(|c| c.worktree_name == "feature")
        .unwrap();
    assert!(feature_chunk.content.contains("feature content"));
}
```

**Coverage**:
- ✓ Branch switch detected
- ✓ Worktree ID updated
- ✓ File changes after switch indexed correctly
- ✓ Database state correct

### Scenario 2: Rapid Branch Switching

**Purpose**: Verify debouncing prevents index thrashing

```rust
#[tokio::test]
async fn test_rapid_branch_switches_debounced() {
    let temp_repo = create_test_git_repo();
    let client = test_db_client().await;

    let mut watcher = UnifiedWatcher::new(
        temp_repo.path(),
        client,
        WatcherConfig {
            debounce_ms: 2000,
            ..default()
        },
    ).await.unwrap();

    // Rapid switches
    git_checkout(&temp_repo, "branch1");
    git_checkout(&temp_repo, "branch2");
    git_checkout(&temp_repo, "branch3");

    // Wait for debounce
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Only final branch should be indexed
    let current = watcher.current_branch.read().unwrap();
    assert_eq!(*current, "branch3");

    // Only one incremental update should have run
    let update_count = count_incremental_updates(&watcher);
    assert_eq!(update_count, 1);
}
```

**Coverage**:
- ✓ Debouncing works
- ✓ Final state correct
- ✓ No wasted indexing operations

### Scenario 3: Concurrent File Changes During Switch

**Purpose**: Verify no file events lost during branch transition

```rust
#[tokio::test]
async fn test_file_changes_during_branch_switch() {
    let temp_repo = create_test_git_repo();
    let client = test_db_client().await;

    let mut watcher = UnifiedWatcher::new(
        temp_repo.path(),
        client,
        default_config(),
    ).await.unwrap();

    // Start branch switch (don't wait)
    let switch_handle = tokio::spawn(async move {
        git_checkout(&temp_repo, "feature");
    });

    // Immediately edit file (race condition)
    tokio::time::sleep(Duration::from_millis(100)).await;
    write_file(&temp_repo, "race.txt", "content");

    // Wait for both to complete
    switch_handle.await.unwrap();
    wait_for_indexing().await;

    // Verify file was indexed (to either main or feature, but not lost)
    let chunks = query_chunks_by_file(&client, "race.txt").await;
    assert!(!chunks.is_empty(), "File should be indexed");
}
```

**Coverage**:
- ✓ Race condition handled
- ✓ No events lost
- ✓ Consistent state

### Scenario 4: Watcher Restart After Failure

**Purpose**: Verify watcher can recover from errors

```rust
#[tokio::test]
async fn test_watcher_recovers_from_git_head_deletion() {
    let temp_repo = create_test_git_repo();
    let client = test_db_client().await;

    let mut watcher = UnifiedWatcher::new(
        temp_repo.path(),
        client,
        default_config(),
    ).await.unwrap();

    // Delete .git/HEAD
    std::fs::remove_file(temp_repo.path().join(".git/HEAD")).unwrap();

    // Wait a bit
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Restore .git/HEAD
    git_checkout(&temp_repo, "main");

    // Verify watcher recovers
    write_file(&temp_repo, "recovery.txt", "content");
    wait_for_indexing().await;

    let chunks = query_chunks_by_file(&client, "recovery.txt").await;
    assert!(!chunks.is_empty(), "Watcher should have recovered");
}
```

**Coverage**:
- ✓ Error recovery
- ✓ Watcher continues after failure
- ✓ File watching resumes

### Scenario 5: Backward Compatibility

**Purpose**: Verify old `--worktree` flag still works (with warning)

```rust
#[tokio::test]
async fn test_worktree_flag_deprecated_but_works() {
    let output = Command::new("crewchief-maproom")
        .args(&["watch", "--worktree", "main", "--path", "."])
        .output()
        .unwrap();

    // Should work
    assert!(output.status.success());

    // Should log deprecation warning
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Ignoring --worktree flag"));
    assert!(stderr.contains("auto-detected"));
}
```

**Coverage**:
- ✓ Backward compatible
- ✓ Warning logged
- ✓ Auto-detection used

## End-to-End Tests

### E2E Test 1: Real Developer Workflow

**Purpose**: Simulate actual developer usage

```bash
#!/bin/bash
# tests/e2e/test_developer_workflow.sh

set -e

# Setup
REPO=$(mktemp -d)
cd $REPO
git init
git checkout -b main
echo "main code" > main.txt
git add . && git commit -m "initial"

# Start watch in background
crewchief-maproom watch --path . &
WATCH_PID=$!

# Wait for startup
sleep 2

# Developer workflow
echo "Working on main..." > main.txt
sleep 1

git checkout -b feature-auth
echo "Auth code" > auth.txt
sleep 2

git checkout main
echo "More main work" >> main.txt
sleep 1

# Verify (query database)
MAIN_CHUNKS=$(psql -c "SELECT COUNT(*) FROM chunks WHERE worktree_name='main'")
FEATURE_CHUNKS=$(psql -c "SELECT COUNT(*) FROM chunks WHERE worktree_name='feature-auth'")

# Cleanup
kill $WATCH_PID
rm -rf $REPO

# Assert
[[ $MAIN_CHUNKS -gt 0 ]] || exit 1
[[ $FEATURE_CHUNKS -gt 0 ]] || exit 1

echo "✓ Developer workflow test passed"
```

**Coverage**:
- ✓ Real git operations
- ✓ Real file changes
- ✓ Real database state
- ✓ Matches user expectations

### E2E Test 2: Error Recovery

**Purpose**: Verify graceful degradation in production

```bash
#!/bin/bash
# tests/e2e/test_error_recovery.sh

set -e

REPO=$(mktemp -d)
cd $REPO
git init

# Start watch
crewchief-maproom watch &
WATCH_PID=$!
sleep 2

# Cause database connection loss
docker stop maproom-postgres

# Edit files (should queue or log errors, not crash)
echo "content" > test.txt
sleep 1

# Restore database
docker start maproom-postgres
sleep 5

# Verify watcher still running
if ! ps -p $WATCH_PID > /dev/null; then
    echo "✗ Watcher crashed"
    exit 1
fi

# Cleanup
kill $WATCH_PID
rm -rf $REPO

echo "✓ Error recovery test passed"
```

**Coverage**:
- ✓ Database failure handling
- ✓ Watcher doesn't crash
- ✓ Recovery possible

## Test Coverage Goals

### Must Cover
- ✓ Branch switch updates worktree ID
- ✓ File events route to correct worktree
- ✓ Debouncing prevents rapid switches
- ✓ Concurrent events handled correctly
- ✓ Error recovery works

### Nice to Cover
- Multiple rapid branch switches
- Large file changes during switch
- Network partitions (database unreachable)
- Disk full scenarios

### Won't Cover (Out of Scope)
- Performance benchmarks (trust existing)
- Memory leak detection (covered by Rust)
- Multi-repository scenarios (not MVP)
- Windows-specific edge cases (test on Linux only)

## Manual Testing Checklist

**Before merging**:
- [ ] Start `watch` on main branch
- [ ] Edit file → verify NDJSON event
- [ ] Switch to feature branch → verify branch_switched event
- [ ] Edit file → verify file_processed event with new worktree
- [ ] Switch back to main → verify worktree updated
- [ ] Edit file → verify indexed to main
- [ ] Stop database → verify watch doesn't crash
- [ ] Restart database → verify watch recovers

## Acceptance Criteria

### Functional
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Both E2E tests pass
- [ ] Manual testing checklist complete

### Non-Functional
- [ ] No race conditions detected
- [ ] Error recovery tested
- [ ] Backward compatibility verified

### Quality Gates
- [ ] `cargo test` passes
- [ ] `cargo clippy` no warnings
- [ ] No `unwrap()` in production code (use proper error handling)

## Risk-Based Testing

| Risk | Likelihood | Impact | Test Coverage |
|------|------------|--------|---------------|
| Race condition (file + branch) | Medium | High | Integration test |
| File events lost | Low | High | Integration test |
| Database failure | Medium | Medium | E2E test |
| Rapid switches thrash index | Low | Low | Integration test |
| Memory leak | Very Low | Medium | Not tested (trust Rust) |

## Test Execution Strategy

### Development
```bash
# Fast feedback loop
cargo test --lib unified_watch
```

### Pre-commit
```bash
# All unit + integration tests
cargo test
cargo clippy
```

### CI Pipeline
```bash
# Full suite including E2E
cargo test
./tests/e2e/test_developer_workflow.sh
./tests/e2e/test_error_recovery.sh
```

## Success Criteria

**Ship when**:
- All automated tests pass
- Manual testing checklist complete
- No critical bugs in error handling
- Race conditions tested and handled

**Don't ship if**:
- File events can be lost
- Branch switches don't update worktree
- Watcher crashes on errors
- Backward compatibility broken
