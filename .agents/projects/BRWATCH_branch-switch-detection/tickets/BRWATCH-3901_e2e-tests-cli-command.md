# Ticket: BRWATCH-3901: E2E tests for CLI command

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Execute end-to-end tests for the `maproom watch` CLI command, validating the full user workflow from command invocation to graceful shutdown.

## Background
This is a critical path test ticket for Phase 3. From quality-strategy.md lines 189-223, E2E tests represent 10% of the test pyramid and validate:
1. CLI command lifecycle
2. Long-running stability
3. Real git operations

These tests use the actual `maproom` binary and real git repositories to ensure the user experience works end-to-end.

**Planning Reference**: `/workspace/.agents/projects/BRWATCH_branch-switch-detection/planning/quality-strategy.md` - Lines 189-223

## Acceptance Criteria
- [ ] `test_watch_command_lifecycle` passes - Command starts, runs, stops
- [ ] Test spawns actual `maproom watch` process
- [ ] Test switches branches using real git commands
- [ ] Test verifies graceful shutdown (Ctrl+C → clean exit)
- [ ] Test validates watcher detected branch switch (via logs or database)
- [ ] All tests run with `--ignored` flag (slow E2E tests)
- [ ] Tests clean up spawned processes (no zombies)
- [ ] No test failures or hangs

## Technical Requirements
- Create E2E test file: `/workspace/crates/maproom/tests/cli_e2e.rs`
- Use `std::process::Command` to spawn maproom binary
- Use `#[tokio::test]` and `#[ignore]` annotations
- Require test repository with git initialized
- Run tests: `cargo test --test cli_e2e -- --ignored --nocapture`
- Kill child processes on test completion
- Timeout tests to prevent hangs (max 30s per test)

## Implementation Notes

From quality-strategy.md lines 196-223:

### Test: CLI Command Lifecycle
```rust
#[tokio::test]
#[ignore]
async fn test_watch_command_lifecycle() {
    let repo = create_test_repo();

    // Start watch command
    let mut child = Command::new("maproom")
        .args(["watch", "--repo", repo.to_str().unwrap()])
        .env("DATABASE_URL", get_test_database_url())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn maproom watch");

    // Give it time to start
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify it's running
    assert!(child.try_wait().unwrap().is_none(), "Process should be running");

    // Switch branch
    git_checkout(&repo, "feature");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Graceful shutdown (send SIGINT for Ctrl+C simulation)
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        let pid = Pid::from_raw(child.id() as i32);
        kill(pid, Signal::SIGINT).unwrap();
    }

    #[cfg(not(unix))]
    {
        // On Windows, use taskkill or CtrlC event
        child.kill().unwrap();
    }

    // Wait for graceful exit
    let status = tokio::time::timeout(
        Duration::from_secs(5),
        async { child.wait().await }
    ).await;

    assert!(status.is_ok(), "Process should exit within 5 seconds");

    // Check exit code (0 for graceful shutdown)
    // Note: SIGINT may result in non-zero exit on some systems
    // let exit_code = status.unwrap().unwrap().code();
}
```

### Test Utilities
```rust
use tempdir::TempDir;
use std::process::{Command, Stdio};

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

fn get_test_database_url() -> String {
    std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for E2E tests")
}
```

### Validating Branch Switch Detection

Option 1: Parse stdout for log messages
```rust
let stdout = String::from_utf8(child.stdout.unwrap().bytes().collect()).unwrap();
assert!(stdout.contains("Branch switch detected: feature"));
```

Option 2: Query database for indexed chunks
```rust
let pool = PgPool::connect(&get_test_database_url()).await.unwrap();
let chunks = get_chunks_by_worktree(&pool, "feature").await.unwrap();
assert!(!chunks.is_empty(), "Feature branch should be indexed");
```

### Cleanup
```rust
impl Drop for TestRunner {
    fn drop(&mut self) {
        // Kill any remaining child processes
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}
```

## Dependencies
- BRWATCH-3001 complete (watch command)
- BRWATCH-3002 complete (graceful shutdown)
- BRWATCH-3003 complete (logging)
- Compiled `maproom` binary available in target/debug or target/release
- DATABASE_URL environment variable set
- git installed in test environment

## Risk Assessment
- **Risk**: Tests hang if process doesn't exit
  - **Mitigation**: Use tokio::timeout, kill process in Drop handler
- **Risk**: Platform-specific signal handling differences
  - **Mitigation**: Use conditional compilation for Unix vs Windows
- **Risk**: Binary not found or not compiled
  - **Mitigation**: Run `cargo build` before tests, clear error message if missing

## Files/Packages Affected
- `/workspace/crates/maproom/tests/cli_e2e.rs` (new file with E2E tests)
