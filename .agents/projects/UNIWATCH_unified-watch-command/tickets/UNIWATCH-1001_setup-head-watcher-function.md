# Ticket: UNIWATCH-1001: Create setup_head_watcher() Function

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

Implement a helper function to set up notify::RecommendedWatcher for monitoring .git/HEAD file changes, with proper channel bridging from sync to async.

## Background

The current `watch` command only monitors repository files but doesn't detect branch switches. We need to watch .git/HEAD to detect when the user runs `git checkout`. This ticket implements the infrastructure to watch that file using the notify crate, bridging the synchronous file watcher events to tokio's async channels.

This is the foundation task for Phase 1 (Foundation) of the UNIWATCH project, which aims to add .git/HEAD watching infrastructure to enable branch detection in the unified watch command.

## Acceptance Criteria

- [x] Function `setup_head_watcher(git_head: &Path, tx: tokio::sync::mpsc::Sender<notify::Event>) -> Result<notify::RecommendedWatcher>` compiles and works
- [x] Function creates notify::RecommendedWatcher that watches the provided git_head path
- [x] Channel bridging task spawned to convert std::sync::mpsc to tokio::mpsc
- [x] Returns watcher handle that can be dropped for cleanup
- [x] Unit test `test_setup_head_watcher_creates_bridge()` passes

## Technical Requirements

- Location: `crates/maproom/src/indexer/mod.rs` (add new function before watch_worktree)
- Use `notify::recommended_watcher()` for file watching
- Use `RecursiveMode::NonRecursive` (only watch the file itself)
- Bridge pattern: std::sync::mpsc::channel() → tokio::spawn → tokio::mpsc::Sender
- Approximately 30 lines of new code
- No clippy warnings

## Implementation Notes

Reference existing BranchWatcher in `src/watcher.rs` for notify usage patterns. The bridging task should loop on `sync_rx.recv()` and send to async `tx.send(event).await`. Handle send errors gracefully (channel closed = exit task). Return the watcher so caller can drop it for cleanup.

**Expected code structure:**
```rust
fn setup_head_watcher(
    git_head: &Path,
    tx: tokio::sync::mpsc::Sender<notify::Event>
) -> Result<notify::RecommendedWatcher> {
    // Create sync channel for notify crate
    let (sync_tx, sync_rx) = std::sync::mpsc::channel();

    // Create watcher
    let mut watcher = notify::recommended_watcher(move |res| {
        if let Ok(event) = res {
            let _ = sync_tx.send(event);
        }
    })?;

    // Watch the .git/HEAD file
    watcher.watch(git_head, RecursiveMode::NonRecursive)?;

    // Bridge sync to async
    tokio::spawn(async move {
        while let Ok(event) = sync_rx.recv() {
            if tx.send(event).await.is_err() {
                break; // Channel closed
            }
        }
    });

    Ok(watcher)
}
```

## Dependencies

None (foundation ticket)

## Risk Assessment

- **Risk**: Channel bridging might deadlock if not implemented correctly
  - **Mitigation**: Use same pattern as WorktreeWatcher internally uses

- **Risk**: Watcher might not detect symlink changes to .git/HEAD
  - **Mitigation**: notify crate handles symlinks by default, test with actual git checkout

## Files/Packages Affected

- `crates/maproom/src/indexer/mod.rs` (approximately 30 new lines)

## Implementation Notes

**Implementation completed successfully:**

1. **Function added**: `setup_head_watcher()` added to `/workspace/crates/maproom/src/indexer/mod.rs` at line 794, before `watch_worktree()` function
2. **Function signature**: Matches specification exactly - takes `git_head: &Path` and `tx: tokio::sync::mpsc::Sender<notify::Event>`, returns `Result<notify::RecommendedWatcher>`
3. **Channel bridging**: Implemented sync → async bridging using `std::sync::mpsc::channel()` and `tokio::spawn()`
4. **Watch mode**: Uses `RecursiveMode::NonRecursive` to watch only the file, not the directory
5. **Error handling**: Returns errors from watcher creation and watch setup via `?` operator
6. **Clean exit**: Bridging task exits when async channel is closed (receiver dropped)
7. **Documentation**: Comprehensive doc comments with example usage

**Test added**: `test_setup_head_watcher_creates_bridge()` at line 1224
- Verifies watcher creation succeeds
- Verifies proper cleanup when watcher is dropped
- Test passes in 0.11s

**Test execution output:**
```
running 1 test
test indexer::tests::test_setup_head_watcher_creates_bridge ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 779 filtered out; finished in 0.11s
```

**Clippy status**: One expected warning about unused function (dead code), which is correct since this is a foundation function for future tickets. No other issues.

**Code structure**: ~30 lines for function, ~40 lines for comprehensive test, follows existing patterns from `BranchWatcher` in `src/watcher.rs`.
