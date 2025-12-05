# Ticket: GITPOLL-3002: HEAD Commit Detection

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Add HEAD commit detection to GitPoller to catch files that are created and committed within the polling interval. Currently, if a file is created and committed between polls, it never appears in `git status --porcelain` and is missed.

## Background

The git polling approach detects dirty files via `git status --porcelain`. However, this misses files that are:
1. Created and committed within a single poll interval
2. Modified and committed within a single poll interval

By also tracking `git rev-parse HEAD`, we can detect when commits occur and use `git diff --name-only` to find what changed.

## Acceptance Criteria

- [x] GitPoller tracks previous HEAD commit hash
- [x] On each poll, check if HEAD changed
- [x] If HEAD changed, run `git diff --name-only <old>..<new>` to get changed files
- [x] Emit Modified events for files changed in commits
- [x] Handle initial state (no previous HEAD) gracefully
- [x] Handle force-push/reset scenarios (old HEAD may not exist)
- [x] Unit tests for HEAD change detection
- [x] Integration test: create file, commit, verify event received

## Technical Requirements

### GitPoller Changes

Update `crates/maproom/src/incremental/git_poller.rs`:

```rust
pub struct GitPoller {
    // ... existing fields ...

    /// Previous HEAD commit for detecting commits between polls.
    previous_head: Option<String>,
}

impl GitPoller {
    /// Get current HEAD commit hash.
    async fn get_head(&self) -> Result<String, GitPollerError> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.root)
            .output()
            .await?;

        if !output.status.success() {
            // May fail on empty repo (no commits yet)
            return Err(GitPollerError::GitExecutionError {
                stderr: String::from_utf8_lossy(&output.stderr).to_string()
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get files changed between two commits.
    async fn get_commit_changes(&self, old: &str, new: &str) -> Result<Vec<PathBuf>, GitPollerError> {
        let output = Command::new("git")
            .args(["diff", "--name-only", &format!("{}..{}", old, new)])
            .current_dir(&self.root)
            .output()
            .await?;

        if !output.status.success() {
            // Old commit may not exist (force push, shallow clone)
            // Fall back to treating all files in new commit as changed
            return Ok(vec![]);
        }

        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| self.root.join(line.trim()))
            .collect();

        Ok(files)
    }

    async fn poll_once_inner(&mut self) -> Result<Vec<FileEvent>, GitPollerError> {
        let mut events = Vec::new();

        // Check for HEAD changes (commits)
        if let Ok(current_head) = self.get_head().await {
            if let Some(ref prev_head) = self.previous_head {
                if &current_head != prev_head {
                    // HEAD changed - commits occurred
                    let changed_files = self.get_commit_changes(prev_head, &current_head).await?;
                    for file in changed_files {
                        events.push(FileEvent::Modified(file));
                    }
                }
            }
            self.previous_head = Some(current_head);
        }

        // Existing git status polling...
        let output = self.run_git_status().await?;
        let new_state = GitState::from_git_status(&output)?;
        let status_events = self.previous_state.diff(&new_state);
        self.previous_state = new_state;

        events.extend(status_events);
        Ok(events)
    }
}
```

### Edge Cases

1. **Empty repository**: `git rev-parse HEAD` fails - handle gracefully, skip HEAD tracking
2. **First poll**: No previous HEAD - just record current HEAD, no diff
3. **Force push/reset**: Old HEAD may not exist - fall back to empty diff or full scan
4. **Shallow clone**: History may be limited - handle diff failures gracefully
5. **Detached HEAD**: Works the same, HEAD still points to a commit

### Test Cases

1. Create file, commit within poll interval - should detect
2. Modify file, commit within poll interval - should detect
3. Multiple commits within poll interval - should detect all changed files
4. Empty repo (no commits) - should not crash
5. Force push scenario - should handle gracefully

## Dependencies

- GITPOLL-1002: GitPoller module (base implementation)

## Risk Assessment

- **Risk**: Performance impact of additional git command
  - **Mitigation**: `git rev-parse HEAD` is very fast (<5ms). Diff only runs when HEAD changes.

- **Risk**: Duplicate events (file in both status and commit diff)
  - **Mitigation**: Consumers should deduplicate by path. Could also dedupe in GitPoller.

- **Risk**: Force push breaks diff
  - **Mitigation**: Catch diff failure, log warning, continue with status-only events.

## Files/Packages Affected

- `crates/maproom/src/incremental/git_poller.rs` (modify)
- `crates/maproom/tests/git_poller_integration.rs` (add tests)
