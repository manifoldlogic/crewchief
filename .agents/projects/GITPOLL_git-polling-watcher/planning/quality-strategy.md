# Quality Strategy: Git Polling File Watcher

## Testing Philosophy

This project replaces critical infrastructure (file watching). Testing must ensure:
1. **Correctness**: Events are emitted for actual file changes
2. **Reliability**: No false positives/negatives under edge cases
3. **Performance**: Polling completes within acceptable time
4. **Integration**: Works seamlessly with existing downstream components

## Test Categories

### 1. Unit Tests (High Priority)

#### Git Status Parsing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_modified_staged() {
        let output = "M  src/main.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(state.files.get(Path::new("src/main.rs")), Some(&FileStatus::Modified));
    }

    #[test]
    fn test_parse_modified_unstaged() {
        let output = " M src/main.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(state.files.get(Path::new("src/main.rs")), Some(&FileStatus::Modified));
    }

    #[test]
    fn test_parse_untracked() {
        let output = "?? new-file.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(state.files.get(Path::new("new-file.rs")), Some(&FileStatus::New));
    }

    #[test]
    fn test_parse_deleted() {
        let output = " D deleted.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(state.files.get(Path::new("deleted.rs")), Some(&FileStatus::Deleted));
    }

    #[test]
    fn test_parse_renamed() {
        let output = "R  old.rs -> new.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(
            state.files.get(Path::new("new.rs")),
            Some(&FileStatus::Renamed { from: PathBuf::from("old.rs") })
        );
    }

    #[test]
    fn test_parse_multiple_statuses() {
        let output = "M  modified.rs\n?? untracked.rs\n D deleted.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(state.files.len(), 3);
    }

    #[test]
    fn test_parse_empty_output() {
        let output = "";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert!(state.files.is_empty());
    }

    #[test]
    fn test_parse_path_with_spaces() {
        let output = " M \"path with spaces/file.rs\"\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert!(state.files.contains_key(Path::new("path with spaces/file.rs")));
    }
}
```

#### State Diff Logic

```rust
#[test]
fn test_diff_new_file() {
    let old = GitState::empty();
    let new = GitState::with_file("new.rs", FileStatus::New);
    let events = old.diff(&new);
    assert_eq!(events, vec![FileEvent::Modified(PathBuf::from("new.rs"))]);
}

#[test]
fn test_diff_deleted_file() {
    let old = GitState::with_file("deleted.rs", FileStatus::Clean);
    let new = GitState::empty();
    let events = old.diff(&new);
    assert_eq!(events, vec![FileEvent::Deleted(PathBuf::from("deleted.rs"))]);
}

#[test]
fn test_diff_modified_file() {
    let old = GitState::with_file("file.rs", FileStatus::Clean);
    let new = GitState::with_file("file.rs", FileStatus::Modified);
    let events = old.diff(&new);
    assert_eq!(events, vec![FileEvent::Modified(PathBuf::from("file.rs"))]);
}

#[test]
fn test_diff_renamed_file() {
    let old = GitState::with_file("old.rs", FileStatus::Clean);
    let new = GitState::with_file("new.rs", FileStatus::Renamed { from: PathBuf::from("old.rs") });
    let events = old.diff(&new);
    assert_eq!(events, vec![FileEvent::Renamed(PathBuf::from("old.rs"), PathBuf::from("new.rs"))]);
}

#[test]
fn test_diff_no_changes() {
    let old = GitState::with_file("file.rs", FileStatus::Clean);
    let new = GitState::with_file("file.rs", FileStatus::Clean);
    let events = old.diff(&new);
    assert!(events.is_empty());
}
```

### 2. Integration Tests (Critical)

These tests use actual git repositories:

```rust
// tests/git_poller_integration.rs

#[tokio::test]
async fn test_poller_detects_new_file() {
    let repo = TempGitRepo::new();
    let (mut poller, mut rx) = GitPoller::new(repo.path(), Default::default()).unwrap();

    // Start polling
    let handle = tokio::spawn(async move { poller.run().await });

    // Create a new file
    repo.create_file("new-file.rs", "content");

    // Wait for event
    let event = timeout(Duration::from_secs(5), rx.recv()).await.unwrap().unwrap();
    assert!(matches!(event, FileEvent::Modified(p) if p.ends_with("new-file.rs")));

    // Cleanup
    poller.shutdown().await;
    handle.await.unwrap();
}

#[tokio::test]
async fn test_poller_detects_modification() {
    let repo = TempGitRepo::new();
    repo.create_and_commit_file("existing.rs", "initial");

    let (mut poller, mut rx) = GitPoller::new(repo.path(), Default::default()).unwrap();
    let handle = tokio::spawn(async move { poller.run().await });

    // Modify the file
    repo.modify_file("existing.rs", "modified");

    let event = timeout(Duration::from_secs(5), rx.recv()).await.unwrap().unwrap();
    assert!(matches!(event, FileEvent::Modified(p) if p.ends_with("existing.rs")));

    poller.shutdown().await;
    handle.await.unwrap();
}

#[tokio::test]
async fn test_poller_detects_deletion() {
    let repo = TempGitRepo::new();
    repo.create_and_commit_file("to-delete.rs", "content");

    let (mut poller, mut rx) = GitPoller::new(repo.path(), Default::default()).unwrap();
    let handle = tokio::spawn(async move { poller.run().await });

    // Delete the file
    repo.delete_file("to-delete.rs");

    let event = timeout(Duration::from_secs(5), rx.recv()).await.unwrap().unwrap();
    assert!(matches!(event, FileEvent::Deleted(p) if p.ends_with("to-delete.rs")));

    poller.shutdown().await;
    handle.await.unwrap();
}

#[tokio::test]
async fn test_poller_detects_rename() {
    let repo = TempGitRepo::new();
    repo.create_and_commit_file("old-name.rs", "content");

    let config = GitPollerConfig {
        detect_renames: true,
        ..Default::default()
    };
    let (mut poller, mut rx) = GitPoller::new(repo.path(), config).unwrap();
    let handle = tokio::spawn(async move { poller.run().await });

    // Rename the file
    repo.rename_file("old-name.rs", "new-name.rs");

    let event = timeout(Duration::from_secs(5), rx.recv()).await.unwrap().unwrap();
    assert!(matches!(event, FileEvent::Renamed(old, new)
        if old.ends_with("old-name.rs") && new.ends_with("new-name.rs")));

    poller.shutdown().await;
    handle.await.unwrap();
}
```

### 3. Error Handling Tests

```rust
#[tokio::test]
async fn test_poller_not_git_repo() {
    let temp_dir = tempfile::tempdir().unwrap();
    let result = GitPoller::new(temp_dir.path().to_path_buf(), Default::default());
    assert!(matches!(result, Err(GitPollerError::NotGitRepository { .. })));
}

#[tokio::test]
async fn test_poller_survives_git_lock() {
    let repo = TempGitRepo::new();
    let (mut poller, _rx) = GitPoller::new(repo.path(), Default::default()).unwrap();

    // Simulate git lock (rebase in progress)
    repo.create_git_lock();

    // Should not panic, should log and retry
    let result = poller.poll_once().await;
    assert!(result.is_ok()); // May return empty, but shouldn't fail

    repo.remove_git_lock();
}

#[tokio::test]
async fn test_poller_handles_timeout() {
    let repo = TempGitRepo::new();
    let config = GitPollerConfig {
        git_timeout: Duration::from_millis(1), // Impossibly short
        ..Default::default()
    };

    let (mut poller, _rx) = GitPoller::new(repo.path(), config).unwrap();

    // Should handle timeout gracefully
    let result = poller.poll_once().await;
    // May succeed or fail depending on system speed, but shouldn't panic
}
```

### 4. Performance Tests

```rust
#[tokio::test]
#[ignore] // Run with --ignored for performance tests
async fn test_polling_performance_large_repo() {
    // Use actual large repo or create synthetic one
    let repo_path = PathBuf::from("/path/to/large/repo");

    let start = Instant::now();
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&repo_path)
        .output()
        .unwrap();
    let git_time = start.elapsed();

    let start = Instant::now();
    let _state = GitState::from_git_status(
        std::str::from_utf8(&output.stdout).unwrap(),
        &repo_path,
    ).unwrap();
    let parse_time = start.elapsed();

    println!("Git status time: {:?}", git_time);
    println!("Parse time: {:?}", parse_time);

    // Should complete in reasonable time
    assert!(git_time < Duration::from_secs(5));
    assert!(parse_time < Duration::from_millis(100));
}
```

## Test Utilities

### TempGitRepo Helper

```rust
/// Helper for creating temporary git repositories in tests.
pub struct TempGitRepo {
    dir: TempDir,
}

impl TempGitRepo {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
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
            .args(["commit", "-m", "add file"])
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
}
```

## Acceptance Criteria

### Must Pass

1. All unit tests for git status parsing
2. All integration tests with temp git repos
3. No "too many open files" errors on large repos
4. Events match existing `FileEvent` interface exactly

### Should Pass

1. Polling completes within configured interval
2. Graceful handling of git failures
3. Memory usage stays bounded

### Nice to Have

1. Performance benchmarks documented
2. Edge case coverage (unicode paths, symlinks)

## Manual Testing Checklist

- [ ] Run on this codebase (crewchief), observe no FD errors
- [ ] Create/modify/delete files, verify events received
- [ ] Rename file, verify rename event
- [ ] Run during git rebase, verify no crashes
- [ ] Test with configured poll interval (1s, 5s, 10s)
- [ ] Verify .gitignored files don't trigger events
- [ ] Test with untracked files enabled/disabled

## CI Integration

```yaml
# Add to existing test workflow
- name: Run git poller tests
  run: cargo test -p crewchief-maproom git_poller

- name: Run git poller integration tests
  run: cargo test -p crewchief-maproom --test git_poller_integration
```

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Git status format changes | Pin to `--porcelain` (stable format) |
| Slow git on network FS | Configurable timeout, skip cycle on timeout |
| Unicode path handling | Test with unicode paths explicitly |
| State corruption | State is transient, lost state just triggers re-scan |
