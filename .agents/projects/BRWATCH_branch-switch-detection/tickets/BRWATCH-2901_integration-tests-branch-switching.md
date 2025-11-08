# Ticket: BRWATCH-2901: Integration tests for branch switching

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Execute comprehensive integration tests for Phase 2 branch switch handler, validating the full workflow from file event to database update and error resilience.

## Background
This is a critical path test ticket for Phase 2. From quality-strategy.md lines 84-187, integration tests represent 40% of the test pyramid and validate:
1. Full workflow (switch → detect → update)
2. Error handling (DB errors, transient failures)
3. Concurrent/rapid switches

**Critical Tests** (from quality-strategy.md line 22):
- ✅ `test_handler_continues_after_error` - Resilience (CRITICAL 2)
- ✅ `test_rapid_switching` - Concurrency (CRITICAL 4)

**Planning Reference**: `/workspace/.agents/projects/BRWATCH_branch-switch-detection/planning/quality-strategy.md` - Lines 84-187

## Acceptance Criteria
- [ ] `test_auto_update_on_switch` passes - Full workflow validation
- [ ] `test_rapid_branch_switching` passes - Concurrency handling (CRITICAL 4)
- [ ] `test_watcher_continues_after_db_error` passes - Error resilience (CRITICAL 2)
- [ ] `test_retry_on_transient_error` passes - Retry logic validation
- [ ] All tests run with `--ignored` flag (integration tests)
- [ ] Tests clean up database records after execution
- [ ] No test failures or panics
- [ ] Performance acceptable (<2s per test excluding indexing time)

## Technical Requirements
- Create integration test file: `/workspace/crates/maproom/tests/watcher_integration.rs`
- Use `#[tokio::test]` and `#[ignore]` annotations
- Require database connection via DATABASE_URL
- Use test utilities:
  - `create_test_repo()` - Temporary git repository
  - `git_checkout(repo, branch)` - Simulate git checkout
  - `get_chunks_by_worktree(pool, worktree_name)` - Verify indexing
- Run tests: `cargo test --test watcher_integration -- --ignored --nocapture`
- Clean up test data after each test

## Implementation Notes

From quality-strategy.md lines 90-146:

### Test 1: Full Workflow
```rust
#[tokio::test]
#[ignore]
async fn test_auto_update_on_switch() {
    let pool = get_test_pool().await;
    let repo = create_test_repo();

    // Start watcher in background
    let watcher_handle = tokio::spawn(async move {
        let mut watcher = BranchWatcher::new(repo.clone(), pool.clone()).unwrap();
        watcher.start().await
    });

    // Give watcher time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Switch branch via git
    git_checkout(&repo, "feature");

    // Wait for auto-update
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify feature branch indexed
    let chunks = get_chunks_by_worktree(&pool, "feature").await.unwrap();
    assert!(!chunks.is_empty(), "Feature branch should be indexed");

    // Cleanup
    watcher_handle.abort();
}
```

### Test 2: Rapid Switching (CRITICAL 4)
```rust
#[tokio::test]
#[ignore]
async fn test_rapid_branch_switching() {
    let pool = get_test_pool().await;
    let repo = create_test_repo();

    let mut watcher = BranchWatcher::new(repo.clone(), pool.clone()).unwrap();
    let watcher_handle = tokio::spawn(async move {
        watcher.start().await
    });

    // Rapid switches
    for branch in &["feature-1", "feature-2", "feature-3", "main"] {
        git_checkout(&repo, branch);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Wait for all updates
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Verify all branches indexed (or at least final branch due to debouncing)
    for branch in &["feature-1", "feature-2", "feature-3", "main"] {
        let chunks = get_chunks_by_worktree(&pool, branch).await.unwrap();
        // Note: Due to debouncing, early branches might not be indexed
        // At minimum, "main" (last switch) should be indexed
    }

    watcher_handle.abort();
}
```

### Test 3: Error Resilience (CRITICAL 2)
```rust
#[tokio::test]
#[ignore]
async fn test_watcher_continues_after_db_error() {
    let pool = get_test_pool().await;
    let repo = create_test_repo();

    let mut watcher = BranchWatcher::new(repo.clone(), pool.clone()).unwrap();
    let watcher_handle = tokio::spawn(async move {
        watcher.start().await
    });

    // Simulate DB error by closing pool
    pool.close().await;

    // Switch branch (will fail to index)
    git_checkout(&repo, "feature");
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Watcher should still be running (not crashed)
    assert!(!watcher_handle.is_finished(), "Watcher should continue despite error");

    watcher_handle.abort();
}
```

### Test 4: Retry Logic
```rust
#[tokio::test]
#[ignore]
async fn test_retry_on_transient_error() {
    // Mock transient error (would require mocking framework)
    // For now, test retry logic manually or with controlled failure injection

    // This test verifies:
    // 1. First attempt fails with transient error
    // 2. Retry happens after 2s delay
    // 3. Second attempt succeeds
    // 4. Logs show retry attempts

    // Implementation requires mock database or controlled failure injection
    // Skip for MVP, validate manually with logging
}
```

### Test Utilities
```rust
use tempdir::TempDir;
use std::process::Command;

fn create_test_repo() -> PathBuf {
    let temp_dir = TempDir::new("test-repo").unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // git init
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Configure git
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create initial commit
    std::fs::write(repo_path.join("README.md"), "# Test").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    repo_path
}

fn git_checkout(repo_path: &Path, branch: &str) {
    Command::new("git")
        .args(["checkout", "-b", branch])
        .current_dir(repo_path)
        .output()
        .unwrap();
}

async fn get_chunks_by_worktree(pool: &PgPool, worktree_name: &str) -> Result<Vec<Chunk>> {
    // Query chunks for specific worktree
    // Implementation depends on BRANCHX schema
    todo!("Query chunks WHERE worktree_ids @> [worktree_id]")
}
```

## Dependencies
- BRWATCH-2001 complete (handle_branch_switch)
- BRWATCH-2002 complete (error handling, retry logic)
- BRWATCH-2003 complete (debouncing)
- **BRANCHX complete** (incremental_update, database schema)
- **DATABASE_URL** environment variable set

## Risk Assessment
- **Risk**: Tests flaky due to timing dependencies
  - **Mitigation**: Use generous timeouts, retry flaky tests, verify deterministic behavior
- **Risk**: Git operations fail in test environment
  - **Mitigation**: Use tempdir, ensure git is installed, handle initialization errors
- **Risk**: Database cleanup incomplete causing test pollution
  - **Mitigation**: Truncate test tables before each test, use unique repo names

## Files/Packages Affected
- `/workspace/crates/maproom/tests/watcher_integration.rs` (new file with integration tests)
