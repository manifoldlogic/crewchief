# Ticket: GITPOLL-2901: Integration Tests with Temp Git Repos

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

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

Implement integration tests that use actual git repositories to verify end-to-end behavior of the git polling system. These tests validate that file changes are correctly detected and events are emitted.

## Background

While unit tests verify parsing and diffing logic, integration tests ensure the complete system works: git commands execute correctly, state tracking works across polls, and events flow through to consumers.

Reference: [quality-strategy.md](../planning/quality-strategy.md) - Integration Tests section

## Acceptance Criteria

- [x] `TempGitRepo` test helper implemented
- [x] Test: new file creation detected
- [x] Test: file modification detected
- [x] Test: file deletion detected
- [x] Test: file rename detected
- [x] Test: non-git directory returns error
- [x] Test: recovery from git lock (updated: git status works with lock, test verifies this)
- [x] All tests pass with `cargo test -p crewchief-maproom --test git_poller_integration`

## Technical Requirements

### TempGitRepo Helper

Create in `crates/maproom/tests/helpers/temp_git_repo.rs`:

```rust
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Helper for creating temporary git repositories in tests.
pub struct TempGitRepo {
    dir: TempDir,
}

impl TempGitRepo {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("failed to create temp dir");

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .expect("failed to init git repo");

        // Configure git user for commits
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .expect("failed to configure git");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir.path())
            .output()
            .expect("failed to configure git");

        Self { dir }
    }

    pub fn path(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }

    pub fn create_file(&self, name: &str, content: &str) {
        let path = self.dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    pub fn create_and_commit_file(&self, name: &str, content: &str) {
        self.create_file(name, content);
        Command::new("git")
            .args(["add", name])
            .current_dir(self.dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", &format!("add {}", name)])
            .current_dir(self.dir.path())
            .output()
            .unwrap();
    }

    pub fn modify_file(&self, name: &str, content: &str) {
        std::fs::write(self.dir.path().join(name), content).unwrap();
    }

    pub fn delete_file(&self, name: &str) {
        std::fs::remove_file(self.dir.path().join(name)).unwrap();
    }

    pub fn rename_file(&self, old: &str, new: &str) {
        std::fs::rename(
            self.dir.path().join(old),
            self.dir.path().join(new),
        ).unwrap();
    }

    pub fn create_git_lock(&self) {
        std::fs::write(
            self.dir.path().join(".git/index.lock"),
            "lock"
        ).unwrap();
    }

    pub fn remove_git_lock(&self) {
        let _ = std::fs::remove_file(self.dir.path().join(".git/index.lock"));
    }

    pub fn stage_file(&self, name: &str) {
        Command::new("git")
            .args(["add", name])
            .current_dir(self.dir.path())
            .output()
            .unwrap();
    }

    pub fn stage_rename(&self, old: &str, new: &str) {
        Command::new("git")
            .args(["mv", old, new])
            .current_dir(self.dir.path())
            .output()
            .unwrap();
    }
}
```

### Integration Test File

Create `crates/maproom/tests/git_poller_integration.rs`:

```rust
mod helpers;

use std::time::Duration;
use tokio::time::timeout;
use crewchief_maproom::incremental::{
    GitPoller, GitPollerConfig, FileEvent
};
use helpers::temp_git_repo::TempGitRepo;

#[tokio::test]
async fn test_poller_detects_new_file() {
    let repo = TempGitRepo::new();
    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100), // Fast for testing
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();

    let handle = tokio::spawn(async move { poller.run().await });

    // Create a new file
    repo.create_file("new-file.rs", "fn main() {}");

    // Wait for event with timeout
    let event = timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timeout waiting for event")
        .expect("channel closed");

    assert!(matches!(event, FileEvent::Modified(p) if p.ends_with("new-file.rs")));

    // Cleanup
    let _ = shutdown.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_poller_detects_modification() {
    let repo = TempGitRepo::new();
    repo.create_and_commit_file("existing.rs", "initial content");

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100),
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();
    let handle = tokio::spawn(async move { poller.run().await });

    // Modify the file
    repo.modify_file("existing.rs", "modified content");

    let event = timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    assert!(matches!(event, FileEvent::Modified(p) if p.ends_with("existing.rs")));

    let _ = shutdown.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_poller_detects_deletion() {
    let repo = TempGitRepo::new();
    repo.create_and_commit_file("to-delete.rs", "content");

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100),
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();
    let handle = tokio::spawn(async move { poller.run().await });

    // Delete the file
    repo.delete_file("to-delete.rs");

    let event = timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    assert!(matches!(event, FileEvent::Deleted(p) if p.ends_with("to-delete.rs")));

    let _ = shutdown.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_poller_detects_rename() {
    let repo = TempGitRepo::new();
    repo.create_and_commit_file("old-name.rs", "content");

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100),
        detect_renames: true,
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();
    let handle = tokio::spawn(async move { poller.run().await });

    // Rename using git mv to ensure git detects it as rename
    repo.stage_rename("old-name.rs", "new-name.rs");

    let event = timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    assert!(matches!(event, FileEvent::Renamed(old, new)
        if old.ends_with("old-name.rs") && new.ends_with("new-name.rs")));

    let _ = shutdown.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_poller_not_git_repo() {
    let temp_dir = tempfile::tempdir().unwrap();
    let result = GitPoller::new(temp_dir.path().to_path_buf(), Default::default());
    assert!(result.is_err());
}

#[tokio::test]
async fn test_poller_survives_git_lock() {
    let repo = TempGitRepo::new();

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100),
        ..Default::default()
    };

    let (mut poller, _rx, _shutdown) = GitPoller::new(repo.path(), config).unwrap();

    // Create git lock (simulates rebase/merge in progress)
    repo.create_git_lock();

    // Should not panic, should return Ok with empty or skip
    let result = poller.poll_once().await;
    assert!(result.is_ok());

    repo.remove_git_lock();
}

#[tokio::test]
async fn test_poller_multiple_files_single_cycle() {
    let repo = TempGitRepo::new();
    repo.create_and_commit_file("file1.rs", "content1");
    repo.create_and_commit_file("file2.rs", "content2");

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(500), // Longer to batch changes
        ..Default::default()
    };

    let (mut poller, mut rx, shutdown) = GitPoller::new(repo.path(), config).unwrap();
    let handle = tokio::spawn(async move { poller.run().await });

    // Modify both files before poll
    repo.modify_file("file1.rs", "modified1");
    repo.modify_file("file2.rs", "modified2");

    // Collect events
    let mut events = Vec::new();
    for _ in 0..2 {
        if let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
            events.push(event);
        }
    }

    assert_eq!(events.len(), 2);

    let _ = shutdown.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_poller_ignores_gitignored_files() {
    let repo = TempGitRepo::new();

    // Create .gitignore
    repo.create_and_commit_file(".gitignore", "ignored.rs\n");

    let config = GitPollerConfig {
        poll_interval: Duration::from_millis(100),
        ..Default::default()
    };

    let (mut poller, _rx, _shutdown) = GitPoller::new(repo.path(), config).unwrap();

    // Create ignored file
    repo.create_file("ignored.rs", "content");

    // Poll once
    let events = poller.poll_once().await.unwrap();

    // Should not see the ignored file
    assert!(!events.iter().any(|e| match e {
        FileEvent::Modified(p) | FileEvent::Deleted(p) => p.ends_with("ignored.rs"),
        FileEvent::Renamed(_, p) => p.ends_with("ignored.rs"),
    }));
}
```

### Test Module Structure

```
crates/maproom/tests/
├── git_poller_integration.rs
└── helpers/
    ├── mod.rs
    └── temp_git_repo.rs
```

`helpers/mod.rs`:
```rust
pub mod temp_git_repo;
```

## Implementation Notes

### Test Timing

- Use short poll intervals for fast tests (100ms)
- Use generous timeouts (5s) to avoid flaky tests
- Consider CI environment may be slower

### Cleanup

Tests should clean up properly:
- Use `TempDir` for automatic cleanup
- Always signal shutdown before test ends
- Use `tokio::select!` with timeout for waiting on events

### Git Configuration

Tests need minimal git config to work:
- User email and name required for commits
- Don't rely on global git config

## Dependencies

- GITPOLL-2001: FileWatcher integration
- GITPOLL-2002: WorktreeWatcher integration

## Risk Assessment

- **Risk**: Tests may be flaky due to timing
  - **Mitigation**: Use generous timeouts, fast poll intervals. Consider retry logic.

- **Risk**: Git behavior varies by version
  - **Mitigation**: Use `--porcelain` format which is stable. Document minimum git version.

- **Risk**: CI environment may not have git
  - **Mitigation**: Skip tests with `#[ignore]` attribute if git not available (unlikely)

## Files/Packages Affected

- `crates/maproom/tests/git_poller_integration.rs` (NEW)
- `crates/maproom/tests/helpers/mod.rs` (NEW)
- `crates/maproom/tests/helpers/temp_git_repo.rs` (NEW)
