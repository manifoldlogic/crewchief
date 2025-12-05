# Architecture: Git Polling File Watcher

## Overview

Replace the `notify`-based file watcher with a git status polling mechanism that detects file changes by periodically running `git status --porcelain` and comparing with previous state.

## Architecture Decision Records

### ADR-1: Git Status as Change Detection Source

**Decision**: Use `git status --porcelain` as the primary change detection mechanism.

**Rationale**:
- Git already tracks file state efficiently
- Automatically respects `.gitignore`
- No file descriptor limits
- Cross-platform consistent behavior

**Consequences**:
- 2-5 second latency instead of instant
- Requires git repository (graceful fallback needed)
- Simpler codebase

### ADR-2: State Comparison Approach

**Decision**: Maintain in-memory state of last known file status, compare on each poll.

**Alternatives Considered**:
1. Stateless (parse git status each time) - Inefficient, can't detect deletions reliably
2. File-based state persistence - Overkill, adds complexity
3. In-memory state - Simple, fast, loses state on restart (acceptable)

**Rationale**:
- Fast comparison (HashMap lookup)
- Memory usage proportional to tracked files (~1KB per 100 files)
- State loss on restart is fine (just triggers full re-index)

### ADR-3: Event Emission Strategy

**Decision**: Emit events compatible with existing `FileEvent` enum.

**Rationale**:
- Zero changes to downstream consumers (detector, processor)
- Drop-in replacement for notify-based watcher
- Existing debouncing logic still works

## Component Design

### New Components

```
crates/maproom/src/incremental/
├── git_poller.rs        # NEW: Git status polling implementation
├── git_state.rs         # NEW: State tracking and comparison
├── watcher.rs           # MODIFIED: Facade that uses GitPoller
└── ...
```

### GitPoller Component

```rust
/// Git-based file change poller.
///
/// Detects file changes by periodically running `git status --porcelain`
/// and comparing with previous state. Emits FileEvent for detected changes.
pub struct GitPoller {
    /// Root path of the git repository
    root: PathBuf,

    /// Configuration
    config: GitPollerConfig,

    /// Previous state from last poll
    previous_state: GitState,

    /// Channel to send file events
    event_tx: mpsc::Sender<FileEvent>,

    /// Shutdown signal
    shutdown: CancellationToken,
}

impl GitPoller {
    /// Create a new git poller for the given repository root.
    pub fn new(
        root: PathBuf,
        config: GitPollerConfig,
    ) -> Result<(Self, mpsc::Receiver<FileEvent>)>;

    /// Start the polling loop (runs until shutdown).
    pub async fn run(&mut self) -> Result<()>;

    /// Perform a single poll cycle (for testing).
    pub async fn poll_once(&mut self) -> Result<Vec<FileEvent>>;

    /// Trigger an immediate poll (for manual refresh).
    pub fn trigger_poll(&self);

    /// Graceful shutdown.
    pub async fn shutdown(&self);
}
```

### GitState Component

```rust
/// Represents the state of files in a git repository.
#[derive(Debug, Clone)]
pub struct GitState {
    /// Map of relative path -> FileStatus
    files: HashMap<PathBuf, FileStatus>,

    /// Timestamp when state was captured
    captured_at: Instant,
}

/// Status of a file in the git repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    /// File is tracked and unmodified
    Clean,

    /// File is modified (staged or unstaged)
    Modified,

    /// File is new (added or untracked)
    New,

    /// File is staged for deletion
    Deleted,

    /// File was renamed from another path
    Renamed { from: PathBuf },
}

impl GitState {
    /// Parse git status output into state.
    pub fn from_git_status(output: &str, root: &Path) -> Result<Self>;

    /// Compare with another state, return differences as FileEvents.
    pub fn diff(&self, other: &GitState) -> Vec<FileEvent>;
}
```

### Configuration

```rust
/// Configuration for the git poller.
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

## Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                        GitPoller                                 │
│                                                                  │
│  ┌──────────┐    ┌─────────────┐    ┌──────────────┐            │
│  │  Timer   │───▶│ git status  │───▶│ Parse Output │            │
│  │ (3 sec)  │    │ --porcelain │    │              │            │
│  └──────────┘    └─────────────┘    └──────┬───────┘            │
│                                            │                     │
│                                            ▼                     │
│  ┌──────────────┐    ┌──────────────────────────────┐           │
│  │ Previous     │◀──▶│ GitState::diff()             │           │
│  │ GitState     │    │ Compare old vs new state     │           │
│  └──────────────┘    └──────────────┬───────────────┘           │
│                                      │                           │
│                                      ▼                           │
│                      ┌───────────────────────────────┐          │
│                      │ Emit FileEvents               │          │
│                      │ - Modified(path)              │          │
│                      │ - Deleted(path)               │          │
│                      │ - Renamed(old, new)           │          │
│                      └───────────────┬───────────────┘          │
└──────────────────────────────────────┼──────────────────────────┘
                                       │
                                       ▼
                          ┌────────────────────────┐
                          │ mpsc::Receiver<FileEvent>
                          │ (to WorktreeWatcher)   │
                          └────────────────────────┘
```

## Integration Points

### WorktreeWatcher Integration

The `WorktreeWatcher` currently creates a `FileWatcher`. It will be modified to:

```rust
impl WorktreeWatcher {
    pub fn new(
        worktree_id: WorktreeId,
        path: PathBuf,
        config: WatcherConfig,
    ) -> Result<(Self, mpsc::Receiver<IndexingEvent>)> {
        // Create GitPoller instead of FileWatcher
        let (poller, event_rx) = GitPoller::new(path.clone(), config.into())?;

        // ... rest unchanged, converts FileEvent to IndexingEvent
    }
}
```

### Backward Compatibility

The `WatcherConfig` struct gains new fields with defaults:

```rust
pub struct WatcherConfig {
    // Existing (still used for debouncing)
    pub debounce_ms: u64,
    pub channel_capacity: usize,

    // New git polling config
    pub poll_interval_ms: u64,      // default: 3000
    pub include_untracked: bool,    // default: true
}
```

## Error Handling

### Git Command Failures

```rust
pub enum GitPollerError {
    /// Git command failed to execute
    GitExecutionError { stderr: String },

    /// Git command timed out
    GitTimeout { timeout: Duration },

    /// Not a git repository
    NotGitRepository { path: PathBuf },

    /// Git operation in progress (rebase, merge)
    GitOperationInProgress,

    /// Parse error in git status output
    ParseError { line: String, reason: String },
}
```

### Recovery Strategy

1. **Transient failures**: Retry with exponential backoff
2. **Persistent failures**: Log error, continue polling (may recover)
3. **Not a git repo**: Return error on creation, don't start polling
4. **Git operation in progress**: Skip this poll cycle, try next interval

## Performance Considerations

### Memory Usage

- `GitState` holds one `HashMap` entry per tracked file
- Entry size: ~100-200 bytes per file (PathBuf + enum)
- 10,000 files ≈ 2MB memory
- 100,000 files ≈ 20MB memory (acceptable)

### CPU Usage

- `git status` is highly optimized (uses filesystem stat cache)
- Parsing output: O(n) where n = number of changed files (usually small)
- State diff: O(n) HashMap operations (very fast)
- Total per poll: < 1ms CPU time for typical repos

### Git Status Optimization

```bash
# Use minimal output format
git status --porcelain

# Enable rename detection (adds ~10% overhead, worth it)
git status --porcelain -M

# Disable expensive operations we don't need
git -c core.preloadindex=true status --porcelain
```

## Testing Strategy

### Unit Tests

1. `GitState::from_git_status()` - Parse various output formats
2. `GitState::diff()` - Detect modifications, additions, deletions, renames
3. `GitPollerConfig` - Default values, validation

### Integration Tests

1. Create temp git repo, modify files, verify events emitted
2. Test rename detection
3. Test untracked file handling
4. Test recovery from git failures

### Manual Testing

1. Run on large repo (this codebase)
2. Verify no "too many open files" errors
3. Measure actual polling latency
4. Test during git operations (rebase, merge)

## Migration Path

### Phase 1: Add GitPoller (non-breaking)

- Add `git_poller.rs` and `git_state.rs`
- Add tests
- Don't change existing watcher yet

### Phase 2: Integration

- Modify `WorktreeWatcher` to use `GitPoller`
- Keep `FileWatcher` as fallback option
- Add configuration to choose implementation

### Phase 3: Cleanup

- Remove notify-based `FileWatcher`
- Simplify configuration
- Update documentation

## Future Enhancements

### Optional Native Watching for Small Repos

```rust
pub enum WatcherBackend {
    /// Git polling (default, always works)
    GitPolling,

    /// Native file watching (for small repos)
    Native,

    /// Auto-detect based on directory count
    Auto { threshold: usize },
}
```

### Watchman Integration

If user has Watchman installed, optionally use it:

```rust
impl GitPoller {
    fn detect_watchman() -> Option<WatchmanClient> {
        // Check if watchman is available
    }
}
```

### Hybrid Mode

For instant feedback on saves, combine:
- Git polling for comprehensive change detection
- Single-file native watch on currently-open files (from editor)
