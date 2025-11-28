# Implementation Plan: Unified Watch Command

## Goal

Add runtime branch switch detection to the existing `watch` command so that when a user runs `git checkout`, the watch command automatically detects the switch and re-indexes to the correct worktree.

## Current State

The watch command in `main.rs:1105-1216` already:
- Auto-detects the current branch at **startup**
- Uses `MultiWatcher` for file change detection
- Uses `tokio::select!` event loop
- Shows deprecation warning for `--worktree` flag

**The Problem**: The `worktree_id` is set once at startup (line 1149) and never changes. When users switch branches with `git checkout`, files continue to be indexed to the original worktree.

## Implementation Phases

### Phase 1: Dynamic Worktree State

**Objective**: Track worktree_id dynamically so it can change at runtime.

**Tasks**:

1. **Add dynamic worktree tracking to main.rs**
   - Wrap `worktree_id` in `Arc<RwLock<i64>>` instead of plain `i64`
   - Wrap `worktree` (branch name) in `Arc<RwLock<String>>`
   - Update event handler to read current worktree_id from the lock
   - Location: `main.rs` Commands::Watch handler

### Phase 2: HEAD Watcher Integration

**Objective**: Add `.git/HEAD` file watching to detect branch switches.

**Tasks**:

1. **Initialize HEAD watcher in main.rs**
   - Calculate `.git/HEAD` path from watch_path
   - Create tokio channel for HEAD events
   - Call `setup_head_watcher()` from indexer module (already exists)
   - Store watcher handle for cleanup
   - Location: `main.rs` Commands::Watch, before event loop

2. **Add HEAD events to select! loop**
   - Add third branch to `tokio::select!` for `head_rx.recv()`
   - On HEAD event, call `handle_branch_switch()`
   - Location: `main.rs` Commands::Watch event loop

### Phase 3: Branch Switch Handler

**Objective**: Implement the logic that runs when a branch switch is detected.

**Tasks**:

1. **Implement handle_branch_switch() function**
   - Read new branch name using `get_current_branch()`
   - Check if branch actually changed (skip if same)
   - Get/create worktree record in database
   - Update Arc<RwLock> state variables
   - Trigger `incremental_update()` for new branch
   - Emit `BranchSwitchEvent` NDJSON to stdout
   - Apply debouncing (2 second window)
   - Location: `main.rs` or new helper module

### Phase 4: Testing

**Objective**: Verify the unified watch works correctly.

**Tasks**:

1. **Create integration tests**
   - Test: Complete branch switch workflow
   - Test: Rapid branch switches are debounced
   - Test: File changes during branch switch
   - Test: --worktree flag backward compatibility
   - Location: `tests/unified_watch_test.rs`

2. **Update E2E test script**
   - Verify existing script works with runtime detection
   - Add branch switch scenarios
   - Location: `tests/e2e/test_unified_watch_workflow.sh`

3. **Manual testing checklist**
   - Start watch, edit file, verify indexed
   - Switch branch, verify detection
   - Edit file on new branch, verify correct worktree
   - Switch back, verify state update

## File Changes

| File | Changes |
|------|---------|
| `crates/maproom/src/main.rs` | ~60 lines modified in Commands::Watch |
| `crates/maproom/tests/unified_watch_test.rs` | ~200 lines new |
| `crates/maproom/tests/e2e/test_unified_watch_workflow.sh` | ~50 lines modified |

## Existing Code to Reuse

These components already exist in `crates/maproom/src/indexer/mod.rs`:

| Component | Location | Purpose |
|-----------|----------|---------|
| `setup_head_watcher()` | line ~668 | Creates notify watcher for .git/HEAD |
| `DebouncedHandler` | line ~33 | Rate limits rapid events |
| `BranchSwitchEvent` | line ~112 | NDJSON struct for VSCode integration |
| `get_current_branch()` | git module | Reads current branch from .git/HEAD |

## Success Criteria

- [ ] `maproom watch` detects branch switches within 2 seconds
- [ ] File changes after switch index to the new worktree
- [ ] Rapid branch switches (< 2s apart) are debounced
- [ ] `BranchSwitchEvent` NDJSON emitted on branch change
- [ ] No regressions to existing file watching
- [ ] All tests pass

## Agent Assignments

| Phase | Agent |
|-------|-------|
| Phase 1-3 (Implementation) | rust-indexer-engineer |
| Phase 4 (Integration Tests) | integration-tester |
| Phase 4 (Manual Testing) | verify-ticket |
| Commit | commit-ticket |
