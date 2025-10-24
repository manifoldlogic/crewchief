# Ticket: INC_INDEX-2001: File Watcher Implementation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement the file system watcher component for incremental indexing using the notify crate. This component detects file changes in real-time, applies debouncing logic, respects ignore patterns from .gitignore, and sends filtered file events through a message channel for downstream processing.

## Background
This ticket is part of Phase 2 (Week 2, Task 1) of the INC_INDEX incremental indexing pipeline. The file watcher is the entry point of the incremental indexing system, responsible for capturing file system events and filtering them before they trigger index updates. Without this component, the system cannot respond to file changes and must rely on full re-indexing.

The watcher needs to be efficient (low CPU usage when idle), fast (detect changes within 2 seconds), and smart (respect ignore patterns to avoid indexing irrelevant files like node_modules or build artifacts).

## Acceptance Criteria
- [ ] File changes detected within 2 seconds of modification
- [ ] CPU usage remains below 1% when idle (no file changes)
- [ ] Debouncing logic working correctly with 500ms delay
- [ ] Ignore patterns from .gitignore are respected
- [ ] All event types handled: Write, Create, Remove, Rename
- [ ] Events sent through mpsc channel to change detector
- [ ] Unit tests pass with 100% coverage of core logic
- [ ] Integration tests demonstrate real file system watching

## Technical Requirements

### Core Dependencies
- `notify` crate with `RecommendedWatcher` backend
- `tokio::sync::mpsc` for event channel communication
- `ignore` crate for .gitignore pattern matching (or manual `GlobSet` implementation)
- `tokio` for async runtime

### Event Types to Handle
1. **Write** - File content modified
2. **Create** - New file created
3. **Remove** - File deleted
4. **Rename** - File moved or renamed (from → to paths)

### Debouncing Strategy
- 500ms delay (configurable)
- Coalesce multiple events for same file within window
- Only emit final event after quiet period

### Ignore Patterns
- Read from .gitignore in watched directory
- Default patterns:
  - `*.log`
  - `.git/**`
  - `node_modules/**`
  - `dist/**`
  - `target/**`
  - `.crewchief/**`

### Channel Communication
- Use `mpsc::channel` with bounded capacity (e.g., 1000 events)
- Send `FileEvent` enum variants:
  ```rust
  pub enum FileEvent {
      Modified(PathBuf),
      Deleted(PathBuf),
      Renamed(PathBuf, PathBuf), // (from, to)
  }
  ```

## Implementation Notes

### Architecture Reference
See `/workspace/crewchief_context/maproom/INC_INDEX/INC_INDEX_ARCHITECTURE.md` lines 14-47 for detailed architecture.

### Key Design Points
1. **FileWatcher Struct**:
   ```rust
   pub struct FileWatcher {
       watcher: RecommendedWatcher,
       tx: mpsc::Sender<FileEvent>,
       ignore_patterns: GlobSet,
   }
   ```

2. **RecursiveMode**: Use `RecursiveMode::Recursive` to watch entire directory tree

3. **Event Processing Loop**: Run in dedicated thread or async task, continuously receiving and filtering events

4. **Ignore Pattern Matching**: Check each path against `GlobSet` before sending event:
   ```rust
   if !self.should_ignore(&path) {
       self.tx.send(FileEvent::Modified(path))?;
   }
   ```

5. **Error Handling**:
   - Log watch errors but don't crash
   - Handle channel send failures gracefully
   - Retry on transient file system errors

### Testing Strategy
- **Unit Tests**:
  - Mock file events
  - Test ignore pattern matching
  - Verify debouncing logic
  - Test event type conversions

- **Integration Tests**:
  - Create temp directory
  - Start watcher
  - Modify files
  - Assert events received within 2s
  - Verify ignored files don't trigger events

### Performance Considerations
- Use `RecommendedWatcher` which selects optimal backend per OS (inotify on Linux, FSEvents on macOS)
- Keep ignore patterns compiled in `GlobSet` for fast matching
- Use bounded channel to prevent memory bloat if processing lags
- Minimize allocations in hot path (event loop)

## Dependencies
- **Prerequisite Tickets**:
  - INC_INDEX-1002 (change detection component) - must exist to receive FileEvent messages

- **External Dependencies**:
  - notify crate (~0.6 or latest stable)
  - ignore crate (for .gitignore parsing) or globset
  - tokio for async/channels

## Risk Assessment
- **Risk**: High event volume could overwhelm the channel if debouncing fails
  - **Mitigation**: Use bounded channel with back-pressure, implement aggressive debouncing, add metrics/logging to detect overflow

- **Risk**: Cross-platform differences in file system event APIs (inotify vs FSEvents vs ReadDirectoryChangesW)
  - **Mitigation**: Use notify crate's cross-platform abstraction, test on multiple OSes, rely on RecommendedWatcher auto-selection

- **Risk**: Ignore patterns not matching correctly, leading to indexing of large directories (node_modules)
  - **Mitigation**: Comprehensive test suite for ignore patterns, validation during startup, fallback to sensible defaults

- **Risk**: Debouncing delay too long causes perceived lag; too short causes excessive processing
  - **Mitigation**: Make debounce delay configurable, start with 500ms based on architecture doc, allow tuning

## Files/Packages Affected

### New Files to Create
- `crates/maproom/src/incremental/watcher.rs` - FileWatcher implementation
- `crates/maproom/src/incremental/events.rs` - FileEvent enum and types
- `crates/maproom/src/incremental/ignore.rs` - Ignore pattern handling
- `crates/maproom/src/incremental/mod.rs` - Module exports (if not exists)
- `crates/maproom/tests/incremental/watcher_test.rs` - Unit tests
- `crates/maproom/tests/integration/watcher_integration_test.rs` - Integration tests

### Existing Files to Modify
- `crates/maproom/Cargo.toml` - Add notify, ignore/globset dependencies
- `crates/maproom/src/lib.rs` - Export incremental module (if needed)

### Configuration Files
- Update architecture doc or config schema to document watcher configuration options (debounce_ms, ignore_patterns)
