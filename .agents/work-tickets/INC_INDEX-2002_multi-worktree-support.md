# Ticket: INC_INDEX-2002: Multi-Worktree Support

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement multi-worktree support for the incremental indexing system, enabling simultaneous watching of multiple worktree directories with proper event isolation and concurrent watcher management.

## Background
The incremental indexing system needs to support CrewChief's multi-worktree architecture where multiple agents work in isolated worktrees simultaneously. Each worktree must be watched independently, with events properly tagged and isolated to ensure that file changes in one worktree don't affect the index state of another. This is Phase 2, Week 2, Task 2 of the incremental indexing implementation plan.

## Acceptance Criteria
- [ ] Multiple watchers can run simultaneously for different worktree paths
- [ ] Events are properly isolated by worktree (tagged with worktree_id)
- [ ] Concurrent watcher operation is stable under load
- [ ] Watcher lifecycle is properly managed (start, stop, restart)
- [ ] Watcher crashes are detected and trigger automatic restart
- [ ] Tests demonstrate concurrent multi-watcher operation
- [ ] Integration tests verify event isolation between worktrees

## Technical Requirements
- Support watching multiple worktree directory paths simultaneously
- Isolate file system events by worktree using worktree_id tagging
- Use separate notify instances for concurrent watchers to avoid cross-contamination
- Implement watcher lifecycle management: start, stop, restart operations
- Handle watcher crashes gracefully with automatic restart capability
- Provide status monitoring for all active watchers
- Ensure thread-safe concurrent access to watcher state

## Implementation Notes

### Multi-Watcher Architecture
Create a `MultiWatcher` component that manages multiple `WorktreeWatcher` instances:

```rust
struct MultiWatcher {
    watchers: HashMap<WorktreeId, WorktreeWatcher>,
    tx: mpsc::Sender<IndexingEvent>,
}
```

### Per-Worktree Watcher
Each `WorktreeWatcher` should:
- Maintain its own notify watcher instance
- Tag all events with its worktree_id
- Handle its own lifecycle (start/stop/restart)
- Report health status to MultiWatcher

### Event Isolation
Events must include worktree context:
```rust
struct IndexingEvent {
    worktree_id: WorktreeId,
    path: PathBuf,
    event_type: EventType,
    timestamp: SystemTime,
}
```

### Lifecycle Management
- Start: Initialize watcher for a new worktree path
- Stop: Clean shutdown of specific watcher
- Restart: Handle watcher failures with exponential backoff
- Health checks: Periodic verification that watchers are responsive

### Concurrency Considerations
- Use separate channels per watcher to avoid backpressure
- Implement proper error handling for channel send failures
- Consider rate limiting for high-frequency file changes
- Ensure graceful shutdown of all watchers

## Dependencies
- INC_INDEX-2001 (single watcher implementation) - Must be completed first to establish base watcher patterns and event handling

## Risk Assessment
- **Risk**: Channel congestion if multiple worktrees generate high-frequency events simultaneously
  - **Mitigation**: Implement rate limiting and event coalescing per worktree, use buffered channels with appropriate capacity

- **Risk**: Memory leaks from watcher instances not being properly cleaned up
  - **Mitigation**: Implement Drop trait for proper cleanup, use weak references where appropriate, add integration tests for lifecycle

- **Risk**: Race conditions in concurrent watcher access
  - **Mitigation**: Use tokio Mutex or RwLock for shared state, minimize critical sections, prefer message passing over shared memory

- **Risk**: Watcher failures cascading to affect other worktrees
  - **Mitigation**: Isolate watchers completely, handle errors per-watcher, ensure one failure doesn't stop others

## Files/Packages Affected
- `crates/maproom/src/incremental/multi_watcher.rs` - New file for multi-watcher management component
- `crates/maproom/src/incremental/worktree_watcher.rs` - New file for per-worktree watcher implementation
- `crates/maproom/src/incremental/mod.rs` - Update module exports to include new components
- `crates/maproom/tests/incremental/multi_watcher_test.rs` - New unit test file for multi-watcher functionality
- `crates/maproom/tests/incremental/integration_test.rs` - Integration tests for concurrent watcher scenarios
- `crates/maproom/Cargo.toml` - May need additional dependencies (tokio channels, etc.)
