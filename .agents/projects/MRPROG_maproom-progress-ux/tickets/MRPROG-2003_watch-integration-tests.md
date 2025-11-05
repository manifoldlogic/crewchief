# Ticket: MRPROG-2003: Write integration tests for watch output modes

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create integration tests that verify both minimal and verbose output modes for the watch command. Tests should spawn the watch command, trigger file changes, and verify the output format matches expectations.

## Background
Integration tests ensure the watch command works correctly in realistic scenarios with actual file system events. Unlike unit tests, these verify the complete workflow from CLI invocation through file change detection to output formatting.

This is pragmatic integration testing: verify the core workflows (minimal and verbose modes) work correctly, not exhaustive coverage of every edge case.

This ticket implements Phase 2 (Watch Minimal Output) testing requirements from the MRPROG project plan, ensuring the watch command minimal and verbose modes work correctly in real-world scenarios.

## Acceptance Criteria
- [ ] Integration test file created: `crates/maproom/tests/watch_integration.rs`
- [ ] Test helper: `setup_test_repo()` creates temporary git repo with files
- [ ] Test: Watch with minimal output (default) shows compact 3-line format
- [ ] Test: Watch with --verbose shows detailed file-by-file output
- [ ] Test: Multiple file changes show multiple dots in minimal mode
- [ ] Test: Timing information appears in both modes
- [ ] All tests pass: `cargo test --test watch_integration`
- [ ] Tests clean up temporary repositories

## Technical Requirements

### Test File Structure

Create `crates/maproom/tests/watch_integration.rs` with the following structure:

```rust
// crates/maproom/tests/watch_integration.rs

use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn setup_test_repo() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize git repo
    Command::new("git")
        .arg("init")
        .current_dir(&repo_path)
        .assert()
        .success();

    // Create initial file
    fs::write(repo_path.join("test.txt"), "initial").unwrap();

    Command::new("git")
        .args(&["add", "."])
        .current_dir(&repo_path)
        .assert()
        .success();

    Command::new("git")
        .args(&["commit", "-m", "initial"])
        .current_dir(&repo_path)
        .assert()
        .success();

    (temp_dir, repo_path)
}

#[test]
fn test_watch_minimal_output() {
    let (_temp, repo_path) = setup_test_repo();

    // Spawn watch command
    let mut watch_cmd = Command::cargo_bin("maproom")
        .unwrap()
        .arg("watch")
        .arg("--path")
        .arg(&repo_path)
        .spawn()
        .unwrap();

    // Wait for watcher to start
    thread::sleep(Duration::from_secs(2));

    // Modify file to trigger change
    fs::write(repo_path.join("test.txt"), "changed").unwrap();

    // Wait for re-index
    thread::sleep(Duration::from_secs(2));

    // Kill watch command
    watch_cmd.kill().unwrap();
    let output = watch_cmd.wait_with_output().unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Verify minimal output format
    assert!(stdout.contains("file(s) changed"), "Should show change count");
    assert!(stdout.contains("Indexing:"), "Should show indexing label");
    assert!(stdout.contains("Done in"), "Should show completion timing");

    // Verify NOT verbose
    assert!(!stdout.contains("Re-indexing..."), "Should not show verbose messages");
}

#[test]
fn test_watch_verbose_output() {
    let (_temp, repo_path) = setup_test_repo();

    let mut watch_cmd = Command::cargo_bin("maproom")
        .unwrap()
        .arg("watch")
        .arg("--path")
        .arg(&repo_path)
        .arg("--verbose")
        .spawn()
        .unwrap();

    thread::sleep(Duration::from_secs(2));
    fs::write(repo_path.join("test.txt"), "changed").unwrap();
    thread::sleep(Duration::from_secs(2));

    watch_cmd.kill().unwrap();
    let output = watch_cmd.wait_with_output().unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Verify verbose output
    assert!(stdout.contains("Detected changes"), "Should show verbose messages");
    assert!(stdout.contains("Re-indexing"), "Should show re-indexing message");
    assert!(stdout.contains("Index updated"), "Should show completion message");
}

#[test]
fn test_watch_multiple_files() {
    let (_temp, repo_path) = setup_test_repo();

    // Create additional files
    fs::write(repo_path.join("file1.txt"), "content1").unwrap();
    fs::write(repo_path.join("file2.txt"), "content2").unwrap();

    let mut watch_cmd = Command::cargo_bin("maproom")
        .unwrap()
        .arg("watch")
        .arg("--path")
        .arg(&repo_path)
        .spawn()
        .unwrap();

    thread::sleep(Duration::from_secs(2));

    // Modify multiple files
    fs::write(repo_path.join("file1.txt"), "changed1").unwrap();
    fs::write(repo_path.join("file2.txt"), "changed2").unwrap();

    thread::sleep(Duration::from_secs(3));

    watch_cmd.kill().unwrap();
    let output = watch_cmd.wait_with_output().unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Verify multiple dots for multiple files
    assert!(stdout.contains("2 file(s) changed") || stdout.contains("files changed"));

    // Should contain dots (at least 2)
    let dot_count = stdout.matches('.').count();
    assert!(dot_count >= 2, "Should show dot per file");
}
```

### Dependencies to Add

Add to `crates/maproom/Cargo.toml` under `[dev-dependencies]`:

```toml
assert_cmd = "2.0"
predicates = "3.0"
tempfile = "3.8"
```

## Implementation Notes

1. Create `crates/maproom/tests/` directory if it doesn't exist
2. Create `watch_integration.rs` file with the test structure above
3. Add dev-dependencies to `crates/maproom/Cargo.toml`
4. Implement `setup_test_repo()` helper with temp directory and git init
5. Write `test_watch_minimal_output()` test
6. Write `test_watch_verbose_output()` test
7. Write `test_watch_multiple_files()` test
8. Run tests: `cargo test --test watch_integration`
9. Verify cleanup (temp directories removed automatically via TempDir drop)

### Testing Strategy

- Run tests locally: `cargo test --test watch_integration`
- Run in CI to verify cross-platform compatibility
- If tests are flaky, increase sleep durations to 3-4 seconds
- Use generous timeouts to account for slower CI environments

## Dependencies

**BLOCKED BY:**
- MRPROG-2001: Needs watch_worktree minimal mode implementation
- MRPROG-2002: Needs --verbose flag wiring

**REQUIRES:**
- Existing watch command infrastructure
- File system watcher functionality

## Risk Assessment

- **Risk**: File system watch timing might be flaky in CI environments
  - **Mitigation**: Use generous sleep durations (2-3 seconds), can increase if needed

- **Risk**: Temp directory cleanup might fail on Windows
  - **Mitigation**: TempDir auto-cleanup on drop handles this; tests will fail if cleanup fails

- **Risk**: Git commands might not be available in test environment
  - **Mitigation**: Tests will skip gracefully if git is not available

- **Risk**: Process spawning/killing might behave differently across platforms
  - **Mitigation**: Use assert_cmd which handles cross-platform process management

## Files/Packages Affected

**CREATE:**
- `crates/maproom/tests/watch_integration.rs`

**MODIFY:**
- `crates/maproom/Cargo.toml` (add dev-dependencies)

**ESTIMATED EFFORT:** 2-3 hours

## References

- Quality strategy: `.agents/projects/MRPROG_maproom-progress-ux/planning/quality-strategy.md` (Integration Tests section)
- Project plan: `.agents/projects/MRPROG_maproom-progress-ux/planning/plan.md` (Phase 2, Task 3)
