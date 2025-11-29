# Ticket: GITPOLL-1901: Unit Tests for Parsing and Diffing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

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

Implement comprehensive unit tests for the GitState parsing and state diff logic. These tests validate the core functionality without requiring actual git repositories.

## Background

The GitState module parses `git status --porcelain` output and compares states to detect changes. These operations must be thoroughly tested to ensure correct event emission.

Reference: [quality-strategy.md](../planning/quality-strategy.md) - Unit Tests section

## Acceptance Criteria

- [ ] Tests for all git status codes (M, A, D, R, ??, etc.)
- [ ] Tests for state diff logic (new, modified, deleted, renamed)
- [ ] Tests for path validation (absolute paths, `..` traversal rejected)
- [ ] Tests for edge cases (empty output, paths with spaces, unicode)
- [ ] Tests for quoted path parsing
- [ ] All tests pass with `cargo test -p crewchief-maproom git_state`

## Technical Requirements

Add tests in `crates/maproom/src/incremental/git_state.rs` (inline) or as separate test file.

### Git Status Parsing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_modified_staged() {
        let output = "M  src/main.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(state.get(Path::new("src/main.rs")), Some(&FileStatus::Modified));
    }

    #[test]
    fn test_parse_modified_unstaged() {
        let output = " M src/main.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(state.get(Path::new("src/main.rs")), Some(&FileStatus::Modified));
    }

    #[test]
    fn test_parse_modified_both() {
        let output = "MM src/main.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(state.get(Path::new("src/main.rs")), Some(&FileStatus::Modified));
    }

    #[test]
    fn test_parse_added_staged() {
        let output = "A  new-file.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(state.get(Path::new("new-file.rs")), Some(&FileStatus::New));
    }

    #[test]
    fn test_parse_untracked() {
        let output = "?? untracked.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(state.get(Path::new("untracked.rs")), Some(&FileStatus::New));
    }

    #[test]
    fn test_parse_deleted_staged() {
        let output = "D  deleted.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(state.get(Path::new("deleted.rs")), Some(&FileStatus::Deleted));
    }

    #[test]
    fn test_parse_deleted_unstaged() {
        let output = " D deleted.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(state.get(Path::new("deleted.rs")), Some(&FileStatus::Deleted));
    }

    #[test]
    fn test_parse_renamed() {
        let output = "R  old.rs -> new.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(
            state.get(Path::new("new.rs")),
            Some(&FileStatus::Renamed { from: PathBuf::from("old.rs") })
        );
    }

    #[test]
    fn test_parse_copied() {
        let output = "C  original.rs -> copy.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        // Treat copy as new file
        assert_eq!(state.get(Path::new("copy.rs")), Some(&FileStatus::New));
    }

    #[test]
    fn test_parse_multiple_files() {
        let output = "M  modified.rs\n?? untracked.rs\n D deleted.rs\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert_eq!(state.len(), 3);
        assert_eq!(state.get(Path::new("modified.rs")), Some(&FileStatus::Modified));
        assert_eq!(state.get(Path::new("untracked.rs")), Some(&FileStatus::New));
        assert_eq!(state.get(Path::new("deleted.rs")), Some(&FileStatus::Deleted));
    }

    #[test]
    fn test_parse_empty_output() {
        let output = "";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert!(state.is_empty());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let output = "   \n\n";
        let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
        assert!(state.is_empty());
    }
}
```

### Path Handling Tests

```rust
#[test]
fn test_parse_path_with_spaces() {
    let output = " M \"path with spaces/file.rs\"\n";
    let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
    assert!(state.get(Path::new("path with spaces/file.rs")).is_some());
}

#[test]
fn test_parse_path_with_unicode() {
    let output = " M src/\u{1F600}/emoji.rs\n";
    let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
    assert!(state.get(Path::new("src/\u{1F600}/emoji.rs")).is_some());
}

#[test]
fn test_parse_path_with_quotes_in_name() {
    // Git escapes quotes in paths
    let output = " M \"file\\\"quoted\\\".rs\"\n";
    let state = GitState::from_git_status(output, Path::new("/repo")).unwrap();
    assert!(state.get(Path::new("file\"quoted\".rs")).is_some());
}

#[test]
fn test_reject_absolute_path() {
    let output = " M /etc/passwd\n";
    let result = GitState::from_git_status(output, Path::new("/repo"));
    assert!(result.is_err());
}

#[test]
fn test_reject_path_traversal() {
    let output = " M ../outside/file.rs\n";
    let result = GitState::from_git_status(output, Path::new("/repo"));
    assert!(result.is_err());
}

#[test]
fn test_reject_hidden_traversal() {
    let output = " M foo/../bar/../../etc/passwd\n";
    let result = GitState::from_git_status(output, Path::new("/repo"));
    assert!(result.is_err());
}
```

### State Diff Tests

```rust
#[test]
fn test_diff_new_file() {
    let old = GitState::default();
    let mut new = GitState::default();
    new.insert(PathBuf::from("new.rs"), FileStatus::New);

    let events = old.diff(&new);
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], FileEvent::Modified(p) if p == Path::new("new.rs")));
}

#[test]
fn test_diff_deleted_file() {
    let mut old = GitState::default();
    old.insert(PathBuf::from("deleted.rs"), FileStatus::Clean);
    let new = GitState::default();

    let events = old.diff(&new);
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], FileEvent::Deleted(p) if p == Path::new("deleted.rs")));
}

#[test]
fn test_diff_modified_file() {
    let mut old = GitState::default();
    old.insert(PathBuf::from("file.rs"), FileStatus::Clean);
    let mut new = GitState::default();
    new.insert(PathBuf::from("file.rs"), FileStatus::Modified);

    let events = old.diff(&new);
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], FileEvent::Modified(p) if p == Path::new("file.rs")));
}

#[test]
fn test_diff_renamed_file() {
    let mut old = GitState::default();
    old.insert(PathBuf::from("old.rs"), FileStatus::Clean);
    let mut new = GitState::default();
    new.insert(PathBuf::from("new.rs"), FileStatus::Renamed { from: PathBuf::from("old.rs") });

    let events = old.diff(&new);
    // Should emit rename event
    assert!(events.iter().any(|e| matches!(e, FileEvent::Renamed(o, n) if o == Path::new("old.rs") && n == Path::new("new.rs"))));
}

#[test]
fn test_diff_no_changes() {
    let mut old = GitState::default();
    old.insert(PathBuf::from("file.rs"), FileStatus::Clean);
    let mut new = GitState::default();
    new.insert(PathBuf::from("file.rs"), FileStatus::Clean);

    let events = old.diff(&new);
    assert!(events.is_empty());
}

#[test]
fn test_diff_multiple_changes() {
    let mut old = GitState::default();
    old.insert(PathBuf::from("existing.rs"), FileStatus::Clean);
    old.insert(PathBuf::from("to-delete.rs"), FileStatus::Clean);

    let mut new = GitState::default();
    new.insert(PathBuf::from("existing.rs"), FileStatus::Modified);
    new.insert(PathBuf::from("new.rs"), FileStatus::New);

    let events = old.diff(&new);
    assert_eq!(events.len(), 3); // modified + deleted + new
}
```

### Config Tests

```rust
#[test]
fn test_config_default() {
    let config = GitPollerConfig::default();
    assert_eq!(config.poll_interval, Duration::from_secs(3));
    assert_eq!(config.git_timeout, Duration::from_secs(10));
    assert!(config.include_untracked);
    assert!(config.detect_renames);
}
```

## Implementation Notes

### Test Organization

Tests should be organized for clarity:
- Parsing tests grouped together
- Diff tests grouped together
- Path validation tests grouped together
- Use descriptive test names that indicate what's being tested

### Test Utilities

Create helper functions:
```rust
#[cfg(test)]
impl GitState {
    fn with_file(path: &str, status: FileStatus) -> Self {
        let mut state = Self::default();
        state.insert(PathBuf::from(path), status);
        state
    }

    fn insert(&mut self, path: PathBuf, status: FileStatus) {
        self.files.insert(path, status);
    }

    fn get(&self, path: &Path) -> Option<&FileStatus> {
        self.files.get(path)
    }

    fn len(&self) -> usize {
        self.files.len()
    }

    fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}
```

## Dependencies

- GITPOLL-1001: GitState module implementation
- GITPOLL-1002: GitPoller module (for config tests)

## Risk Assessment

- **Risk**: Tests may not cover all edge cases
  - **Mitigation**: Use property-based testing for path parsing if needed

- **Risk**: Platform-specific path behavior
  - **Mitigation**: Use `Path::new()` consistently, avoid string-based path manipulation

## Files/Packages Affected

- `crates/maproom/src/incremental/git_state.rs` (add tests)
- `crates/maproom/src/incremental/git_poller.rs` (add config tests)
