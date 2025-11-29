# Ticket: GITPOLL-2002: Update WorktreeWatcher Integration

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

Update `WorktreeWatcher` to work with the refactored `FileWatcher` that now uses `GitPoller`. Ensure `IndexingEvent` emission continues to work correctly.

## Background

`WorktreeWatcher` is a higher-level abstraction that wraps `FileWatcher` and converts `FileEvent`s into `IndexingEvent`s for the indexing pipeline. Since `FileWatcher` now uses `GitPoller` internally, we need to verify `WorktreeWatcher` still works correctly and update any necessary plumbing.

Reference: [architecture.md](../planning/architecture.md) - WorktreeWatcher Integration section

## Acceptance Criteria

- [ ] `WorktreeWatcher` creates and uses updated `FileWatcher` with `GitPoller`
- [ ] `IndexingEvent` emission works correctly for all event types
- [ ] Configuration passes through correctly to `GitPoller`
- [ ] Shutdown/cleanup works properly
- [ ] No changes to `IndexingEvent` interface

## Technical Requirements

### WorktreeWatcher Structure

The `WorktreeWatcher` likely has this structure:
```rust
pub struct WorktreeWatcher {
    worktree_id: WorktreeId,
    path: PathBuf,
    file_watcher: FileWatcher,
    // ...
}
```

### Event Conversion

`WorktreeWatcher` converts `FileEvent` to `IndexingEvent`:
```rust
// FileEvent -> IndexingEvent mapping
FileEvent::Modified(path) -> IndexingEvent::FileChanged { worktree_id, path }
FileEvent::Deleted(path) -> IndexingEvent::FileDeleted { worktree_id, path }
FileEvent::Renamed(old, new) -> IndexingEvent::FileRenamed { worktree_id, old_path, new_path }
```

### Configuration Threading

Ensure `WatcherConfig` is properly converted:
```rust
impl WorktreeWatcher {
    pub fn new(
        worktree_id: WorktreeId,
        path: PathBuf,
        config: WatcherConfig,
    ) -> Result<(Self, mpsc::Receiver<IndexingEvent>), WatcherError> {
        // FileWatcher now internally uses GitPoller
        let (file_watcher, event_rx) = FileWatcher::new(path.clone(), config)?;

        let (indexing_tx, indexing_rx) = mpsc::channel(1000);

        // Spawn task to convert FileEvent to IndexingEvent
        let wt_id = worktree_id;
        tokio::spawn(async move {
            while let Some(file_event) = event_rx.recv().await {
                let indexing_event = match file_event {
                    FileEvent::Modified(path) => IndexingEvent::FileChanged {
                        worktree_id: wt_id,
                        path,
                    },
                    FileEvent::Deleted(path) => IndexingEvent::FileDeleted {
                        worktree_id: wt_id,
                        path,
                    },
                    FileEvent::Renamed(old, new) => IndexingEvent::FileRenamed {
                        worktree_id: wt_id,
                        old_path: old,
                        new_path: new,
                    },
                };
                if indexing_tx.send(indexing_event).await.is_err() {
                    break;
                }
            }
        });

        Ok((
            Self {
                worktree_id,
                path,
                file_watcher,
            },
            indexing_rx,
        ))
    }
}
```

### Verify IndexingEvent Types

Ensure existing `IndexingEvent` enum handles all cases:
```rust
pub enum IndexingEvent {
    FileChanged { worktree_id: WorktreeId, path: PathBuf },
    FileDeleted { worktree_id: WorktreeId, path: PathBuf },
    FileRenamed { worktree_id: WorktreeId, old_path: PathBuf, new_path: PathBuf },
    // ... other variants
}
```

## Implementation Notes

### Minimal Changes Expected

Since `FileWatcher` maintains the same interface, `WorktreeWatcher` changes should be minimal:
1. Verify it compiles with updated `FileWatcher`
2. Verify event conversion still works
3. Update any config handling if `WatcherConfig` changed

### Shutdown Handling

Ensure proper shutdown propagation:
```rust
impl WorktreeWatcher {
    pub async fn stop(self) -> Result<(), WatcherError> {
        self.file_watcher.stop().await
    }
}
```

### Testing Approach

After implementation, verify with:
1. Create `WorktreeWatcher` for test directory
2. Modify a file
3. Verify `IndexingEvent::FileChanged` received
4. Delete a file
5. Verify `IndexingEvent::FileDeleted` received

## Dependencies

- GITPOLL-2001: FileWatcher integration (must use updated FileWatcher)

## Risk Assessment

- **Risk**: IndexingEvent types may not match FileEvent types
  - **Mitigation**: Review both enums, ensure mapping is complete

- **Risk**: Event conversion task may have issues
  - **Mitigation**: Add logging for debugging, test with integration tests

## Files/Packages Affected

- `crates/maproom/src/incremental/worktree_watcher.rs` (MODIFY - likely minimal)
- May need to update tests in this file
