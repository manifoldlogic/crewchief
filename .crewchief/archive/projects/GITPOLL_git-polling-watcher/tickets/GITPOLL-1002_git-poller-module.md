# Ticket: GITPOLL-1002: Implement GitPoller Module

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

Implement the `GitPoller` module that runs the polling loop, executes `git status`, and emits `FileEvent`s when changes are detected. This is the main runtime component of the git polling system.

## Background

The GitPoller replaces the notify-based file watcher with periodic git status polling. It uses the GitState module (GITPOLL-1001) to track and compare repository state across poll cycles.

Reference: [architecture.md](../planning/architecture.md) - GitPoller Component section

## Acceptance Criteria

- [x] `GitPollerConfig` struct with configurable poll interval, timeout, retry settings
- [x] `GitPoller` struct that runs polling loop using tokio
- [x] `poll_once()` method executes single poll cycle (for testing)
- [x] `run()` async method runs continuous polling loop
- [x] `shutdown()` triggers graceful termination (via watch channel)
- [x] Proper error handling with retry logic for transient failures
- [x] Validates git repository on creation (returns error for non-git dirs)

## Technical Requirements

- Create new file: `crates/maproom/src/incremental/git_poller.rs`
- Export from `crates/maproom/src/incremental/mod.rs`
- Use `tokio::sync::mpsc` for event channel
- Use `tokio::sync::watch` for shutdown signaling (alternative to CancellationToken)
- Use `tokio::process::Command` for async git execution
- Use `tokio::time::timeout` for git command timeout

### GitPollerConfig

```rust
#[derive(Debug, Clone)]
pub struct GitPollerConfig {
    /// Polling interval (default: 3 seconds)
    pub poll_interval: Duration,

    /// Include untracked files (default: true)
    pub include_untracked: bool,

    /// Detect renames (default: true)
    pub detect_renames: bool,

    /// Timeout for git command (default: 10 seconds)
    pub git_timeout: Duration,

    /// Number of retries on git failure (default: 3)
    pub retry_count: u32,

    /// Delay between retries (default: 1 second)
    pub retry_delay: Duration,
}

impl Default for GitPollerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(3),
            include_untracked: true,
            detect_renames: true,
            git_timeout: Duration::from_secs(10),
            retry_count: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}
```

### GitPoller Struct

```rust
pub struct GitPoller {
    root: PathBuf,
    config: GitPollerConfig,
    previous_state: GitState,
    event_tx: mpsc::Sender<FileEvent>,
    shutdown_rx: watch::Receiver<bool>,
}

impl GitPoller {
    pub fn new(
        root: PathBuf,
        config: GitPollerConfig,
    ) -> Result<(Self, mpsc::Receiver<FileEvent>, watch::Sender<bool>), GitPollerError>;

    pub async fn run(&mut self) -> Result<(), GitPollerError>;

    pub async fn poll_once(&mut self) -> Result<Vec<FileEvent>, GitPollerError>;
}
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum GitPollerError {
    #[error("git command failed: {stderr}")]
    GitExecutionError { stderr: String },

    #[error("git command timed out after {timeout:?}")]
    GitTimeout { timeout: Duration },

    #[error("not a git repository: {path}")]
    NotGitRepository { path: PathBuf },

    #[error("git operation in progress")]
    GitOperationInProgress,

    #[error("failed to parse git status: {reason}")]
    ParseError { line: String, reason: String },

    #[error("channel closed")]
    ChannelClosed,
}
```

### Git Command Execution

```rust
async fn run_git_status(&self) -> Result<String, GitPollerError> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.args(["status", "--porcelain"]);

    if self.config.detect_renames {
        cmd.arg("-M");
    }

    cmd.current_dir(&self.root);

    let output = tokio::time::timeout(
        self.config.git_timeout,
        cmd.output()
    )
    .await
    .map_err(|_| GitPollerError::GitTimeout { timeout: self.config.git_timeout })?
    .map_err(|e| GitPollerError::GitExecutionError { stderr: e.to_string() })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        // Check for specific errors
        if stderr.contains("not a git repository") {
            return Err(GitPollerError::NotGitRepository { path: self.root.clone() });
        }
        if stderr.contains(".git/index.lock") {
            return Err(GitPollerError::GitOperationInProgress);
        }
        return Err(GitPollerError::GitExecutionError { stderr });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

### Polling Loop

```rust
pub async fn run(&mut self) -> Result<(), GitPollerError> {
    let mut interval = tokio::time::interval(self.config.poll_interval);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                match self.poll_once().await {
                    Ok(events) => {
                        for event in events {
                            if self.event_tx.send(event).await.is_err() {
                                return Err(GitPollerError::ChannelClosed);
                            }
                        }
                    }
                    Err(GitPollerError::GitOperationInProgress) => {
                        // Skip this cycle, try next
                        tracing::debug!("git operation in progress, skipping poll cycle");
                    }
                    Err(e) => {
                        tracing::warn!("git poll error: {}", e);
                        // Continue polling, may recover
                    }
                }
            }
            _ = self.shutdown_rx.changed() => {
                if *self.shutdown_rx.borrow() {
                    tracing::info!("git poller shutting down");
                    break;
                }
            }
        }
    }

    Ok(())
}
```

## Implementation Notes

### Initial State

On first poll, `previous_state` is empty (Default::default()). This means:
- All dirty files in first poll emit Modified events
- This is acceptable - triggers initial indexing of changed files
- Document this behavior

### Git Repository Validation

Check on creation:
```rust
pub fn new(root: PathBuf, config: GitPollerConfig) -> Result<...> {
    // Verify it's a git repository
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(&root)
        .output()?;

    if !output.status.success() {
        return Err(GitPollerError::NotGitRepository { path: root });
    }

    // ... rest of initialization
}
```

### Retry Logic

For transient failures:
```rust
async fn poll_with_retry(&mut self) -> Result<Vec<FileEvent>, GitPollerError> {
    let mut last_error = None;

    for attempt in 0..self.config.retry_count {
        match self.poll_once_inner().await {
            Ok(events) => return Ok(events),
            Err(GitPollerError::GitOperationInProgress) => {
                // Don't retry, just skip
                return Err(GitPollerError::GitOperationInProgress);
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < self.config.retry_count - 1 {
                    tokio::time::sleep(self.config.retry_delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}
```

## Dependencies

- GITPOLL-1001: GitState module (must be completed first)

## Risk Assessment

- **Risk**: Git command hangs on slow filesystems
  - **Mitigation**: Configurable timeout with sensible default (10s)

- **Risk**: High CPU from polling too frequently
  - **Mitigation**: Default 3s interval, configurable. Document performance characteristics.

- **Risk**: Channel backpressure if consumer is slow
  - **Mitigation**: Use bounded channel with reasonable capacity. Log warnings if approaching capacity.

## Files/Packages Affected

- `crates/maproom/src/incremental/git_poller.rs` (NEW)
- `crates/maproom/src/incremental/mod.rs` (export new module)
- `crates/maproom/Cargo.toml` (may need tokio features)
