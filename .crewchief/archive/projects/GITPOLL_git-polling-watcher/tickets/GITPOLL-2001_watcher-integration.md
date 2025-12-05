# Ticket: GITPOLL-2001: Integrate GitPoller into watcher.rs

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

Modify `watcher.rs` to use `GitPoller` instead of the notify-based `RecommendedWatcher`. The `FileWatcher` facade should maintain the same public interface for backward compatibility.

## Background

The current `watcher.rs` creates a `RecommendedWatcher` with `RecursiveMode::Recursive`, which causes file descriptor exhaustion on large repositories. Replacing it with `GitPoller` eliminates this problem while maintaining the same event interface.

Reference: [architecture.md](../planning/architecture.md) - WorktreeWatcher Integration section

## Acceptance Criteria

- [x] `FileWatcher` uses `GitPoller` internally instead of `notify::RecommendedWatcher`
- [x] Public interface unchanged (`watch()`, `stop()` methods if present)
- [x] `WatcherConfig` updated with git polling configuration options
- [x] Existing `FileEvent` emission preserved (drop-in replacement)
- [x] Existing code compiles without modification (backward compatible)

## Technical Requirements

### Current Implementation Reference

The current implementation in `watcher.rs` likely looks like:
```rust
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    // ...
}
```

### New Implementation

Replace with:
```rust
pub struct FileWatcher {
    poller_shutdown: watch::Sender<bool>,
    poller_handle: JoinHandle<Result<(), GitPollerError>>,
    // ...
}

impl FileWatcher {
    pub fn new(
        root: PathBuf,
        config: WatcherConfig,
    ) -> Result<(Self, mpsc::Receiver<FileEvent>), WatcherError> {
        let poller_config = GitPollerConfig::from(&config);
        let (poller, event_rx, shutdown_tx) = GitPoller::new(root, poller_config)?;

        let handle = tokio::spawn(async move {
            poller.run().await
        });

        Ok((
            Self {
                poller_shutdown: shutdown_tx,
                poller_handle: handle,
            },
            event_rx,
        ))
    }

    pub async fn stop(self) -> Result<(), WatcherError> {
        // Signal shutdown
        let _ = self.poller_shutdown.send(true);
        // Wait for poller to finish
        self.poller_handle.await??;
        Ok(())
    }
}
```

### WatcherConfig Updates

Add git polling configuration while preserving existing fields:
```rust
pub struct WatcherConfig {
    // Existing fields (keep for backward compatibility)
    pub debounce_ms: u64,
    pub channel_capacity: usize,

    // New git polling config
    pub poll_interval_ms: u64,      // default: 3000
    pub include_untracked: bool,    // default: true
    pub detect_renames: bool,       // default: true
    pub git_timeout_ms: u64,        // default: 10000
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 100,
            channel_capacity: 1000,
            poll_interval_ms: 3000,
            include_untracked: true,
            detect_renames: true,
            git_timeout_ms: 10000,
        }
    }
}

impl From<&WatcherConfig> for GitPollerConfig {
    fn from(config: &WatcherConfig) -> Self {
        Self {
            poll_interval: Duration::from_millis(config.poll_interval_ms),
            include_untracked: config.include_untracked,
            detect_renames: config.detect_renames,
            git_timeout: Duration::from_millis(config.git_timeout_ms),
            ..Default::default()
        }
    }
}
```

### Error Type Updates

Update `WatcherError` to include git poller errors:
```rust
#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("not a git repository: {0}")]
    NotGitRepository(PathBuf),

    #[error("git poller error: {0}")]
    GitPollerError(#[from] GitPollerError),

    #[error("watcher task failed")]
    TaskFailed,

    // Keep existing variants if any
}
```

## Implementation Notes

### Preserving Interface

The key constraint is backward compatibility. Downstream code that creates a `FileWatcher` and receives `FileEvent`s should work without modification.

Check existing usage patterns:
```rust
// Current pattern (should still work):
let (watcher, event_rx) = FileWatcher::new(root, config)?;

// In a task:
while let Some(event) = event_rx.recv().await {
    match event {
        FileEvent::Modified(path) => { ... }
        FileEvent::Deleted(path) => { ... }
        FileEvent::Renamed(old, new) => { ... }
    }
}
```

### Debouncing Consideration

The current implementation may have debouncing logic. With git polling:
- Native debouncing is unnecessary (polling is inherently debounced)
- Keep `debounce_ms` config for backward compatibility but it may be unused
- Document that debouncing is implicit with polling

### Import Cleanup

Remove `notify` imports from watcher.rs:
```rust
// REMOVE:
// use notify::{RecommendedWatcher, RecursiveMode, Watcher};

// ADD:
use crate::incremental::git_poller::{GitPoller, GitPollerConfig, GitPollerError};
```

## Dependencies

- GITPOLL-1001: GitState module
- GITPOLL-1002: GitPoller module
- GITPOLL-1901: Unit tests (validates core logic)

## Risk Assessment

- **Risk**: Breaking existing consumers of FileWatcher
  - **Mitigation**: Keep exact same public interface. Add new config fields with defaults.

- **Risk**: Different timing behavior (instant → polled)
  - **Mitigation**: Document latency change. 3s polling is acceptable for watch command use case.

- **Risk**: Existing tests may expect notify behavior
  - **Mitigation**: Update tests in GITPOLL-2901 to work with polling

## Files/Packages Affected

- `crates/maproom/src/incremental/watcher.rs` (MODIFY)
- `crates/maproom/src/incremental/mod.rs` (may need export updates)
