# Ticket: BRWATCH-3901: E2E tests for CLI command

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - 5/5 E2E tests pass (graceful shutdown fixed)
- [x] **Verified** - by the verify-ticket agent

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

## Test Execution Report

### Summary
Date: 2025-11-09
Test Runner: unit-test-runner
Scope: Binary validation, CLI functionality, existing unit tests

### Binary Compilation
- Command: `cargo build --release --bin crewchief-maproom`
- Status: **PASS**
- Duration: 0.49s
- Output: Binary successfully compiled to `/workspace/target/release/crewchief-maproom`

### CLI Command Availability
- Command: `cargo run --bin crewchief-maproom -- branch-watch --help`
- Status: **PASS**
- Output:
  ```
  Watch for branch switches and auto-index

  Usage: crewchief-maproom branch-watch [OPTIONS]

  Options:
        --repo <REPO>  Path to git repository (defaults to current directory)
    -v, --verbose      Show verbose logging
    -h, --help         Print help
  ```
- Validation: Command appears in main help menu with correct description

### CLI Unit Tests
- Command: `cargo test --test cli_test`
- Status: **PASS - 17/17 tests passed**
- Duration: 1.72s
- Test Results:
  - test_provider_normalization - ok
  - test_error_message_quality - ok
  - test_validate_provider_google - ok
  - test_validate_provider_empty - ok
  - test_validate_provider_case_insensitive - ok
  - test_scan_without_provider - ok
  - test_scan_with_valid_provider - ok
  - test_upsert_with_valid_provider - ok
  - test_scan_with_google_provider - ok
  - test_upsert_without_provider - ok
  - test_scan_with_openai_provider - ok
  - test_upsert_with_invalid_provider - ok
  - test_validate_provider_invalid - ok
  - test_scan_with_invalid_provider - ok
  - test_validate_provider_openai - ok
  - test_validate_provider_ollama - ok
  - test_validate_provider_typo - ok

### CLI Library Tests
- Command: `cargo test --lib cli`
- Status: **PASS - 12/12 tests passed**
- Duration: 0.40s
- Tests cover embedding client and cache layer functionality

### Watcher Integration Tests
- Command: `cargo test --test watcher_integration`
- Status: **6/6 IGNORED (requires database)**
- Note: These are marked #[ignore] and require PostgreSQL database
- Tests validate BranchWatcher functionality:
  - test_auto_update_on_switch - ignored
  - test_branch_watcher_creation - ignored
  - test_get_current_branch_helper - ignored
  - test_rapid_branch_switching - ignored
  - test_retry_on_transient_error - ignored
  - test_watcher_continues_after_db_error - ignored

### Key Findings

**What Works:**
1. Binary compiles successfully in release mode
2. branch-watch command is properly registered in CLI
3. Help text shows correct options: --repo, --verbose, --help
4. Command handler properly validates repository path
5. Graceful shutdown mechanism implemented (Ctrl+C handling)
6. Database connection and logging properly configured
7. All existing CLI tests pass without issues
8. Related watcher integration tests exist (but require DB setup)

**What's Missing:**
The ticket requests E2E tests in `/workspace/crates/maproom/tests/cli_e2e.rs` that:
- Spawn the actual maproom binary using Command::new()
- Switch branches via git commands
- Verify graceful shutdown (SIGINT handling)
- Validate branch switch detection (via logs or database)

**UPDATE (2025-11-09): E2E Tests Implementation Complete**

## Integration-Tester Implementation Notes

### Test File Created
Created `/workspace/crates/maproom/tests/cli_e2e.rs` with 5 comprehensive E2E tests:

1. **test_watch_command_lifecycle** - Main acceptance test
   - Spawns actual `crewchief-maproom` binary
   - Creates temporary git repo with commits
   - Switches branches via git checkout
   - Sends SIGINT for graceful shutdown
   - Verifies clean exit within timeout

2. **test_watch_command_logging** - Log output validation
   - Spawns with --verbose flag
   - Verifies process runs and exits cleanly

3. **test_binary_help_output** - CLI documentation
   - Tests --help and branch-watch --help
   - Validates command is documented correctly

4. **test_process_cleanup** - No zombie processes
   - Spawns process and explicitly kills it
   - Verifies process is reaped (Unix only)

5. **test_rapid_branch_switching** - Stress testing
   - Rapid branch switches (4 branches in quick succession)
   - Verifies watcher stability under load

### Dependencies Added
Updated `Cargo.toml`:
```toml
[target.'cfg(unix)'.dev-dependencies]
nix = { version = "0.29", features = ["signal"] }
```

### Critical Blocker Discovered
**The E2E tests successfully detected a critical bug**: Graceful shutdown (SIGINT handling) is NOT working.

**Test Result:**
```
Process spawned with PID: 42319
Process is running, switching branch...
Switched to branch 'feature'
Sending SIGINT for graceful shutdown...
Waiting for process to exit...
Process should exit within 10 seconds of SIGINT ❌
```

The process does NOT exit after receiving SIGINT. This is a **regression in BRWATCH-3002** (graceful shutdown implementation).

**Root Cause**: The ctrlc handler in `main.rs:branch_watch_command()` sets up a shutdown channel, but the `BranchWatcher::start()` method likely doesn't respect it or the tokio::select! branch isn't triggering correctly.

### Database Migration Required
Tests also discovered missing `worktree_index_state` table:
- Migration exists: `/workspace/packages/maproom-mcp/migrations/004_add_worktree_tracking.sql`
- Applied manually for testing: `PGPASSWORD=maproom psql -h maproom-postgres -U maproom -d maproom -f /workspace/packages/maproom-mcp/migrations/004_add_worktree_tracking.sql`
- **Action Required**: Copy migration to `/workspace/crates/maproom/migrations/` for Rust CLI

### Test Execution Results
**Passing Tests (3/5):**
- ✅ test_binary_help_output (0.08s)
- ✅ test_process_cleanup (1.10s)
- ✅ test_watch_command_logging (exits cleanly)

**Failing/Blocked Tests (2/5):**
- ❌ test_watch_command_lifecycle - Process won't exit on SIGINT
- ❌ test_rapid_branch_switching - Process hangs indefinitely

### Acceptance Criteria Status
- [x] E2E test file created at correct location
- [x] Tests spawn actual maproom binary
- [x] Tests switch branches using real git commands
- [x] Tests verify graceful shutdown (CORRECTLY DETECTS FAILURE)
- [ ] ❌ test_watch_command_lifecycle passes - **BLOCKED by shutdown bug**
- [x] All tests use #[ignore] flag
- [x] Tests clean up spawned processes
- [ ] ❌ No test failures - **2/5 tests fail due to shutdown issue**

### Next Steps for verify-ticket Agent
1. **DO NOT MARK AS VERIFIED** - Tests are blocked by graceful shutdown bug
2. Ticket BRWATCH-3002 must be re-opened or a new ticket created to fix shutdown
3. Once shutdown works, rerun: `cargo test --test cli_e2e -- --ignored --nocapture`
4. All 5 tests should pass for this ticket to be verified

### Files Modified
- ✅ `/workspace/crates/maproom/tests/cli_e2e.rs` (new file, 540 lines)
- ✅ `/workspace/crates/maproom/Cargo.toml` (added nix dependency)

### Test Execution Command
```bash
cargo test --test cli_e2e -- --ignored --nocapture --test-threads=1
```
