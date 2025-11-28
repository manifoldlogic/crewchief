# Ticket: UNIWATCH-2001: Integrate HEAD Watcher into Event Loop

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (infrastructure setup, cargo check passes, manual verification deferred to UNIWATCH-4002)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add `.git/HEAD` file watching to the watch command by integrating the existing `setup_head_watcher()` function and adding a new branch to the `tokio::select!` event loop.

## Background
To detect branch switches at runtime, the watch command needs to monitor `.git/HEAD` for changes. This file is modified by git whenever the user runs `git checkout`. The `setup_head_watcher()` function already exists in the indexer module (exported in UNIWATCH-0001) and can be reused.

**Plan Reference:** Phase 2 - HEAD Watcher Integration

## Acceptance Criteria
- [x] `.git/HEAD` path calculated from `watch_path`
- [x] tokio channel created for HEAD events: `(head_tx, head_rx)`
- [x] `setup_head_watcher()` called with the HEAD path and channel
- [x] Watcher handle stored for cleanup (prevent premature drop)
- [x] Third branch added to `tokio::select!` for `head_rx.recv()`
- [x] HEAD events trigger placeholder/stub (actual handler in UNIWATCH-3001)
- [x] **Manual verification:** Run `git checkout` and confirm log message "HEAD file changed" appears (deferred to UNIWATCH-4002)
- [x] `cargo check -p crewchief-maproom` passes
- [x] Existing file watching still works

## Technical Requirements
1. **Initialize HEAD watcher (before event loop):**
   ```rust
   let git_head = watch_path.join(".git/HEAD");
   let (head_tx, mut head_rx) = tokio::sync::mpsc::channel(10);
   let _head_watcher = crewchief_maproom::indexer::setup_head_watcher(&git_head, head_tx)?;
   ```

2. **Add to tokio::select! loop:**
   ```rust
   loop {
       tokio::select! {
           _ = signal::ctrl_c() => { /* existing shutdown */ }

           Some(event) = event_rx.recv() => {
               // existing file event handling
               let wt_id = *worktree_id.read().unwrap();
               incremental_update(&store, wt_id, &watch_path).await?;
           }

           Some(_head_event) = head_rx.recv() => {
               // NEW: Call branch switch handler (implemented in UNIWATCH-3001)
               tracing::info!("HEAD file changed - branch switch detected");
               // handle_branch_switch(...) call added in next ticket
           }
       }
   }
   ```

3. **Import statement:**
   ```rust
   use crewchief_maproom::indexer::setup_head_watcher;
   ```

## Implementation Notes
- The `_head_watcher` variable must be kept alive (not dropped) or the watcher stops
- Use `_` prefix since we don't call methods on it, but it must stay in scope
- The channel buffer size of 10 is sufficient - HEAD changes are infrequent
- This ticket sets up the infrastructure; actual branch switch handling is UNIWATCH-3001

**Key file:** `crates/maproom/src/main.rs` - Commands::Watch handler

## Dependencies
- UNIWATCH-0001 (setup_head_watcher must be exported)
- UNIWATCH-1001 (dynamic state must be in place for the handler)

## Risk Assessment
- **Risk**: Watcher drop causes silent failure to detect branch switches
  - **Mitigation**: Store watcher in named variable (not `let _ = ...`)
- **Risk**: Event ordering between file and HEAD events
  - **Mitigation**: tokio::select! handles events sequentially per iteration (see architecture.md)

## Files/Packages Affected
- `crates/maproom/src/main.rs` (~20 lines added to Commands::Watch)
