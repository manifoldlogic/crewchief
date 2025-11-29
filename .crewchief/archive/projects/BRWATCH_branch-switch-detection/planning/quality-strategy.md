# Quality Strategy: Automatic Branch Switch Detection

## Testing Philosophy

**Core principle**: Reliability is paramount - must detect 100% of branch switches

Testing priorities:
1. **Detection reliability** - Never miss a switch
2. **Resource efficiency** - Idle CPU/memory acceptable
3. **Error resilience** - Continue running despite errors
4. **Cross-platform** - Works on Linux/macOS/Windows

## Test Pyramid

- **50% Unit tests** - Branch parsing, event handling
- **40% Integration tests** - Full workflow, error scenarios
- **10% E2E tests** - Long-running stability, performance

## Critical Path Tests

1. ✅ `test_watcher_detects_all_switches` - Reliability
2. ✅ `test_handler_continues_after_error` - Resilience
3. ✅ `test_idle_resource_usage` - Efficiency
4. ✅ `test_rapid_switching` - Concurrency

## Unit Tests

### 1. Branch Name Parsing

**File**: `crates/maproom/src/watcher.rs`

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_branch_ref() {
        let content = "ref: refs/heads/main\n";
        let branch = get_branch_from_head_content(content).unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_parse_feature_branch() {
        let content = "ref: refs/heads/feature/auth-system\n";
        let branch = get_branch_from_head_content(content).unwrap();
        assert_eq!(branch, "feature/auth-system");
    }

    #[test]
    fn test_parse_detached_head() {
        let content = "abc123def456789012345678901234567890abcd\n";
        let branch = get_branch_from_head_content(content).unwrap();
        assert_eq!(branch, "abc123de"); // Short SHA
    }

    #[test]
    fn test_parse_invalid_format() {
        let content = "invalid format";
        let result = get_branch_from_head_content(content);
        assert!(result.is_err());
    }
}
```

### 2. Event Handling

```rust
#[tokio::test]
async fn test_file_event_triggers_handler() {
    let temp_repo = create_test_git_repo();
    let (tx, rx) = channel();

    let watcher = setup_watcher(&temp_repo, tx);

    // Simulate git checkout (change HEAD file)
    modify_git_head(&temp_repo, "feature");

    // Wait for event
    let event = timeout(Duration::from_secs(5), rx.recv()).await;
    assert!(event.is_ok(), "Should receive file change event");
}
```

## Integration Tests

### 3. Full Workflow Tests

**File**: `crates/maproom/tests/watcher_integration.rs`

```rust
#[tokio::test]
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

#[tokio::test]
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

    // Verify all branches indexed
    for branch in &["feature-1", "feature-2", "feature-3", "main"] {
        let chunks = get_chunks_by_worktree(&pool, branch).await.unwrap();
        assert!(!chunks.is_empty(), "{} should be indexed", branch);
    }

    watcher_handle.abort();
}
```

### 4. Error Resilience Tests

```rust
#[tokio::test]
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

#[tokio::test]
async fn test_retry_on_transient_error() {
    // Mock transient error (network timeout, etc.)
    let pool = get_mock_pool_with_transient_errors();
    let repo = create_test_repo();

    let mut watcher = BranchWatcher::new(repo.clone(), pool).unwrap();

    // This should retry and succeed
    let result = watcher.handle_branch_switch().await;

    assert!(result.is_ok(), "Should succeed after retries");
}
```

## E2E Tests

### 5. CLI Command Tests

**File**: `crates/maproom/tests/cli_e2e.rs`

```rust
#[tokio::test]
async fn test_watch_command_lifecycle() {
    let repo = create_test_repo();

    // Start watch command
    let mut child = Command::new("maproom")
        .args(["watch", "--repo", repo.to_str().unwrap()])
        .spawn()
        .unwrap();

    // Give it time to start
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify it's running
    assert!(child.try_wait().unwrap().is_none(), "Should be running");

    // Switch branch
    git_checkout(&repo, "feature");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Graceful shutdown (Ctrl+C)
    child.kill().unwrap();
    let exit_status = child.wait().unwrap();

    // Should exit cleanly
    assert!(!exit_status.success()); // Killed returns non-zero, but clean
}
```

### 6. Long-Running Stability Test

```rust
#[tokio::test]
#[ignore] // Run manually, takes 10 minutes
async fn test_long_running_stability() {
    let pool = get_test_pool().await;
    let repo = create_test_repo();

    let mut watcher = BranchWatcher::new(repo.clone(), pool).unwrap();
    let watcher_handle = tokio::spawn(async move {
        watcher.start().await
    });

    // Run for 10 minutes with periodic branch switches
    for _ in 0..100 {
        git_checkout(&repo, "main");
        tokio::time::sleep(Duration::from_secs(3)).await;

        git_checkout(&repo, "feature");
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    // Should still be running
    assert!(!watcher_handle.is_finished(), "Should run for extended period");

    watcher_handle.abort();
}
```

## Performance Tests

### 7. Resource Usage Benchmarks

**File**: `crates/maproom/benches/watcher_performance.rs`

```rust
fn benchmark_idle_cpu(c: &mut Criterion) {
    c.bench_function("watcher_idle_cpu", |b| {
        b.iter(|| {
            // Measure CPU while watcher idle
            let usage = measure_cpu_usage_for_duration(Duration::from_secs(10));
            assert!(usage < 0.05, "CPU should be <5% while idle");
        });
    });
}

fn benchmark_detection_latency(c: &mut Criterion) {
    c.bench_function("detection_latency", |b| {
        b.iter(|| {
            let start = Instant::now();

            // Change .git/HEAD
            modify_git_head(&repo, "feature");

            // Wait for event
            rx.recv();

            let latency = start.elapsed();
            assert!(latency < Duration::from_secs(1), "Detection should be <1s");
        });
    });
}
```

**Success criteria**:
- Idle CPU: <5%
- Idle memory: <20MB
- Detection latency: <1s
- Update latency: <1 minute (via incremental update)

## Manual Testing Checklist

Before marking complete:

- [ ] Start `maproom watch` on test repository
- [ ] Switch between 5 different branches rapidly
- [ ] Verify all branches indexed (query each)
- [ ] Leave watcher running for 10 minutes idle
- [ ] Check CPU/memory usage (via `top` or `htop`)
- [ ] Trigger Ctrl+C, verify graceful shutdown
- [ ] Check logs for any errors or warnings
- [ ] Test on Linux (primary target)
- [ ] Test on macOS (if available)
- [ ] Test with repository that has 100+ branches

## Acceptance Criteria

Project is complete when:

1. ✅ All unit tests pass
2. ✅ All integration tests pass
3. ✅ E2E workflow tests pass
4. ✅ Performance benchmarks met (<5% CPU, <20MB RAM)
5. ✅ Manual testing checklist complete
6. ✅ 100% detection reliability (1000 switches, 0 missed)
7. ✅ Graceful error handling (doesn't crash)
8. ✅ Cross-platform (Linux + macOS tested)

**Any failure** → Return to implementation
