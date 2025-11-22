# Ticket: UNIWATCH-3001: Add .git/HEAD Watcher to watch_worktree() Initialization

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
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Integrate the setup_head_watcher() function into watch_worktree() initialization, creating the head event channel and watcher alongside the existing file watcher.

## Background
We now have setup_head_watcher() function (from UNIWATCH-1001) and need to call it during watch_worktree() initialization. This creates the .git/HEAD watcher and event channel that will feed into our modified event loop.

This ticket implements Phase 3 (Event Loop Integration) of the UNIWATCH project, which aims to wire up event handling and async coordination for both file and branch events.

## Acceptance Criteria
- [ ] Calculate .git/HEAD path from root parameter
- [ ] Create tokio::mpsc::channel for head events (capacity: 10)
- [ ] Call setup_head_watcher() with git_head path and channel sender
- [ ] Store watcher handle for cleanup (add to function scope variables)
- [ ] Gracefully handle setup failures (log warning, continue with file watching only)
- [ ] Integration test `test_dual_watchers_initialize()` passes
- [ ] Both file and head watchers start successfully

## Technical Requirements
- Location: `crates/maproom/src/indexer/mod.rs` (in watch_worktree function, after WorktreeWatcher creation)
- Add around line 820 (after `watcher.start()?`)
- Approximately 20 lines of modifications
- Use same error handling pattern as file watcher
- Watcher handle should be moved into processor_task scope for cleanup
- Channel capacity: 10 (branch switches are infrequent events)

## Implementation Notes
Add .git/HEAD watcher initialization after WorktreeWatcher start:

```rust
// After WorktreeWatcher start (around line 824)
watcher.start()?;
info!(/* existing log */);

// NEW: Add .git/HEAD watcher
let git_head = root_abs.join(".git/HEAD");
let (head_tx, mut head_rx) = tokio::sync::mpsc::channel(10);

let _head_watcher = match setup_head_watcher(&git_head, head_tx) {
    Ok(watcher) => Some(watcher),
    Err(e) => {
        warn!("Failed to watch .git/HEAD: {}. Branch detection disabled.", e);
        None
    }
};
```

**Key implementation details:**
- Use `root_abs.join(".git/HEAD")` to construct path
- Create tokio mpsc channel with capacity of 10
- Store watcher handle with underscore prefix to indicate it's held for cleanup
- Use `match` for graceful error handling
- Log warning and continue if HEAD watcher setup fails
- File watching continues even if branch watching fails

## Dependencies
- **UNIWATCH-1001** - setup_head_watcher function must exist
- **UNIWATCH-1002** - current_branch and current_worktree_id state must exist

## Risk Assessment
- **Risk**: .git/HEAD might not exist (bare repos, corrupted repos)
  - **Mitigation**: Graceful error handling, log warning, continue file watching only
- **Risk**: Channel capacity too small might drop events
  - **Mitigation**: Use capacity of 10 (branch switches are infrequent)
- **Risk**: Watcher handle cleanup might not work correctly
  - **Mitigation**: Follow same pattern as file watcher, let Drop trait handle cleanup

## Files/Packages Affected
- `crates/maproom/src/indexer/mod.rs` (~20 lines of modifications around line 820-824)
