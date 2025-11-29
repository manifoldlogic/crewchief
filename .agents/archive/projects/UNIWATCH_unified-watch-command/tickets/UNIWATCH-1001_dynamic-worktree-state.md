# Ticket: UNIWATCH-1001: Add Dynamic Worktree State Tracking

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (visibility changes only, cargo check passes)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Replace the static `worktree_id` variable in the watch command handler with thread-safe mutable state using `Arc<RwLock<i64>>` to enable dynamic updates when branches switch at runtime.

## Background
Currently, `worktree_id` is set once at startup (main.rs:1149) and never changes. When users run `git checkout`, files continue indexing to the original worktree. This ticket enables runtime modification of `worktree_id` by wrapping it in thread-safe state.

**Plan Reference:** Phase 1 - Dynamic Worktree State

## Acceptance Criteria
- [x] `worktree_id` wrapped in `Arc<RwLock<i64>>` in Commands::Watch handler
- [x] `current_branch` (branch name) wrapped in `Arc<RwLock<String>>`
- [x] Event handler reads `worktree_id` from the lock (copy value, drop lock, then use)
- [x] Uses `std::sync::RwLock`, NOT `tokio::sync::RwLock`
- [x] `cargo check -p crewchief-maproom` passes
- [x] Existing watch functionality unchanged (file indexing still works)

## Technical Requirements
- Import: `use std::sync::{Arc, RwLock};` (NOT tokio::sync)
- Transform at ~line 1149:
  ```rust
  // BEFORE
  let worktree_id = store.get_or_create_worktree(...).await?;

  // AFTER
  let worktree_id: Arc<RwLock<i64>> = Arc::new(RwLock::new(
      store.get_or_create_worktree(...).await?
  ));
  let current_branch: Arc<RwLock<String>> = Arc::new(RwLock::new(worktree.clone()));
  ```
- Update event handler to read from lock:
  ```rust
  let wt_id = *worktree_id.read().unwrap();
  incremental_update(&store, wt_id, &watch_path).await?;
  ```
- Lock pattern: copy value, drop lock immediately, then use the copy

## Implementation Notes
Use `std::sync::RwLock` (not `tokio::sync::RwLock`) to match existing codebase patterns. See:
- `config/hot_reload.rs`
- `cache/system.rs`
- `embedding/cache.rs`

The lock is held very briefly (only to copy a primitive i64 or clone a String), so blocking is acceptable and preferred for consistency.

**Key file:** `crates/maproom/src/main.rs` - Commands::Watch handler at lines 1105-1216

## Dependencies
- UNIWATCH-0001 (module exports must be complete first)

## Risk Assessment
- **Risk**: Low - state wrapping is a standard Rust pattern
  - **Mitigation**: Keep lock hold times minimal (copy and drop)
- **Risk**: Could break existing file watching if lock pattern is wrong
  - **Mitigation**: Manual test file watching after change

## Files/Packages Affected
- `crates/maproom/src/main.rs` (~15 lines modified in Commands::Watch)
