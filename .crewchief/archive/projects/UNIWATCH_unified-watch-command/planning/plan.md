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

## Codebase State Notes

The following functions/structs exist in `indexer/mod.rs` but are currently **private** and marked
`#[allow(dead_code)]`. They were prepared for this feature but not exported during the SQLite migration:

- `setup_head_watcher()` - EXISTS, needs `pub` export
- `DebouncedHandler` - EXISTS, needs `pub` export
- `BranchSwitchEvent` - EXISTS, needs `pub` export

**The `handle_branch_switch()` function was REMOVED** during the IDXABS-2001 SQLite migration
(see `indexer/mod.rs:702-705`). It must be reimplemented in Phase 3.

## Implementation Phases

### Phase 0: Module Exports (Prerequisite)

**Objective**: Export required components from indexer module so they can be used in main.rs.

**Tasks**:

1. **Export components from indexer/mod.rs**
   - Add `pub` visibility to `setup_head_watcher()` function
   - Add `pub` visibility to `DebouncedHandler` struct and its `new()`, `should_handle()` methods
   - Add `pub` visibility to `BranchSwitchEvent` struct
   - Remove `#[allow(dead_code)]` annotations from all three
   - Add re-exports in `indexer/mod.rs`: `pub use self::{DebouncedHandler, BranchSwitchEvent, setup_head_watcher}`
   - Location: `crates/maproom/src/indexer/mod.rs`

**Verification**: `cargo check` should show these items are now used (no dead_code warnings)

### Phase 1: Dynamic Worktree State

**Objective**: Track worktree_id dynamically so it can change at runtime.

**Tasks**:

1. **Add dynamic worktree tracking to main.rs**
   - Import `use std::sync::{Arc, RwLock};` (NOT tokio::sync - see architecture.md)
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

**NOTE**: This is NEW code. The original `handle_branch_switch()` was removed during IDXABS-2001.
See architecture.md for full implementation specification.

**Tasks**:

1. **Implement handle_branch_switch() function (NEW)**
   - Create new async function in `main.rs` (not indexer module)
   - Check debounce using `DebouncedHandler::should_handle()` (2 second window)
   - Read new branch name using `get_current_branch()`
   - Handle detached HEAD: if branch == "HEAD", use 8-char commit SHA
   - Check if branch actually changed (skip if same)
   - Get/create worktree record using `SqliteStore::get_or_create_worktree()`
   - Update `Arc<RwLock>` state variables (brief lock hold)
   - Trigger `incremental_update()` for new branch (log warning on error, don't crash)
   - Emit `BranchSwitchEvent` NDJSON to stdout
   - Location: `main.rs` Commands::Watch section (inline helper function)

### Phase 4: Testing

**Objective**: Verify the unified watch works correctly.

**Tasks**:

1. **Enable disabled UNIWATCH tests (rust-indexer-engineer)**
   - Several tests in `indexer/mod.rs` are disabled with `#[cfg(disabled_postgresql_test)]`
   - These need to be updated for SQLite and re-enabled:
     - `test_worktree_tracking_state_initialization` (UNIWATCH-1002)
     - `test_handle_branch_switch_updates_state` (UNIWATCH-2001)
     - `test_handle_branch_switch_skips_if_same_branch` (UNIWATCH-2001)
     - `test_event_loop_handles_both_file_and_head_events` (UNIWATCH-3002)
   - Already working tests (keep as-is):
     - `test_debounced_handler_prevents_rapid_events` (UNIWATCH-1003)
     - `test_branch_switch_event_serialization` (UNIWATCH-2002)
     - `test_dual_watchers_initialize` (UNIWATCH-3001)

2. **Create integration tests**
   - Test: Complete branch switch workflow
   - Test: Rapid branch switches are debounced
   - Test: File changes during branch switch
   - Test: Detached HEAD state handling
   - Test: --worktree flag backward compatibility
   - Location: `tests/unified_watch_test.rs`

3. **Update E2E test script for SQLite**
   - Replace PostgreSQL commands (`psql`) with SQLite equivalents
   - Update database path to `~/.maproom/maproom.db`
   - Use maproom CLI commands for cleanup where possible
   - Add branch switch scenarios
   - Location: `tests/e2e/test_unified_watch_workflow.sh`

4. **Manual testing checklist**
   - Start watch, edit file, verify indexed
   - Switch branch, verify detection and NDJSON output
   - Edit file on new branch, verify correct worktree
   - Switch back, verify state update
   - Rapid switches (3x in 2s), verify only final branch indexed
   - Detached HEAD checkout, verify SHA-based worktree created

## File Changes

| File | Changes |
|------|---------|
| `crates/maproom/src/indexer/mod.rs` | ~10 lines: add `pub` visibility, remove dead_code |
| `crates/maproom/src/main.rs` | ~80 lines: modified Commands::Watch + new handle_branch_switch |
| `crates/maproom/tests/unified_watch_test.rs` | ~200 lines new |
| `crates/maproom/tests/e2e/test_unified_watch_workflow.sh` | ~50 lines modified for SQLite |

## Existing Code to Reuse

These components exist in `crates/maproom/src/indexer/mod.rs` but need export (Phase 0):

| Component | Location | Purpose | Status |
|-----------|----------|---------|--------|
| `setup_head_watcher()` | line ~668 | Creates notify watcher for .git/HEAD | Needs `pub` export |
| `DebouncedHandler` | line ~33 | Rate limits rapid events | Needs `pub` export |
| `BranchSwitchEvent` | line ~112 | NDJSON struct for VSCode integration | Needs `pub` export |

These components are already public and ready to use:

| Component | Location | Purpose |
|-----------|----------|---------|
| `get_current_branch()` | git/mod.rs | Reads current branch from .git/HEAD |
| `incremental_update()` | incremental/mod.rs | Re-indexes files in worktree |
| `get_or_create_worktree()` | db/sqlite/mod.rs | Creates/retrieves worktree record |

## Success Criteria

- [ ] `maproom watch` detects branch switches within 2 seconds
- [ ] File changes after switch index to the new worktree
- [ ] Rapid branch switches (< 2s apart) are debounced
- [ ] `BranchSwitchEvent` NDJSON emitted on branch change
- [ ] No regressions to existing file watching
- [ ] All tests pass

## Agent Assignments

| Phase | Agent | Notes |
|-------|-------|-------|
| Phase 0 (Module Exports) | rust-indexer-engineer | Simple visibility changes |
| Phase 1-3 (Implementation) | rust-indexer-engineer | Core feature work |
| Phase 4 Task 1 (Enable tests) | rust-indexer-engineer | Requires Rust expertise to rewrite for SQLite |
| Phase 4 Tasks 2-3 (Integration Tests) | integration-tester | After implementation complete |
| Phase 4 Task 4 (Manual Testing) | verify-ticket | Before commit |
| Commit | commit-ticket | After all verification passes |
