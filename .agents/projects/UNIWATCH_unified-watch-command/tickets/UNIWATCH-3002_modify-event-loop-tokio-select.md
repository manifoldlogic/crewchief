# Ticket: UNIWATCH-3002: Modify Event Loop to Use tokio::select! for Dual Event Sources

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
Replace the current while-let event loop with tokio::select! to handle both file events and head events in a single async loop.

## Background
Currently, watch_worktree() has a single event loop processing file events. We need to modify it to process BOTH file events (from event_rx) and head events (from head_rx) using tokio::select! for multiplexing.

This ticket implements Phase 3 (Event Loop Integration) of the UNIWATCH project, enabling the event loop to handle both file system changes and branch switches in a unified async workflow.

## Acceptance Criteria
- [ ] Replace `while let Some(event) = event_rx.recv()` with `loop { tokio::select! {} }`
- [ ] Add branch for file events: `Some(indexing_event) = event_rx.recv() =>`
- [ ] Add branch for head events: `Some(head_event) = head_rx.recv() =>`
- [ ] File event processing logic unchanged (just indented inside select branch)
- [ ] Head event branch calls handle_branch_switch() with debouncing
- [ ] Shutdown handling still works (handle None from both channels)
- [ ] Integration test `test_event_loop_handles_both_sources()` passes

## Technical Requirements
- Location: `crates/maproom/src/indexer/mod.rs` (in processor_task spawn, around line 847)
- Approximately 50 lines of modifications
- Use tokio::select! macro for event multiplexing
- Both branches must be async-safe
- Preserve all existing error handling
- Add DebouncedHandler initialization at start of task
- Use Duration::from_secs(2) for debounce interval

## Implementation Notes
Transform the current while-let loop into tokio::select! with dual event sources:

```rust
// BEFORE (line 847):
let processor_task = tokio::spawn(async move {
    while let Some(indexing_event) = event_rx.recv().await {
        // ... existing processing logic
    }
});

// AFTER:
let processor_task = tokio::spawn(async move {
    let debouncer = DebouncedHandler::new(Duration::from_secs(2));

    loop {
        tokio::select! {
            Some(indexing_event) = event_rx.recv() => {
                // EXISTING: All current file event processing logic here
                // (lines 848-950 remain unchanged, just indented)
            }
            Some(_head_event) = head_rx.recv() => {
                // NEW: Branch switch handling with debouncing
                if !debouncer.should_handle() {
                    debug!("Debouncing rapid branch switch");
                    continue;
                }

                if let Err(e) = handle_branch_switch(
                    &root_clone,
                    &current_branch_clone,
                    &current_worktree_id_clone,
                    &pool_clone,
                    &repo_clone,
                ).await {
                    error!("Branch switch handling failed: {}", e);
                }
            }
            else => break, // Both channels closed
        }
    }

    info!("Event processing loop exited");
});
```

**Key implementation details:**
- Initialize DebouncedHandler at the start of the spawned task
- File event branch contains ALL existing processing logic (lines 848-950)
- Head event branch checks debouncer before processing
- Use `_head_event` prefix to indicate event content is not currently used
- `else` branch handles graceful shutdown when both channels close
- All Arc clones must be created before moving into task
- Preserve existing logging throughout

## Dependencies
- **UNIWATCH-1001** - setup_head_watcher creates head_rx
- **UNIWATCH-1003** - DebouncedHandler for rate limiting
- **UNIWATCH-2001** - handle_branch_switch function
- **UNIWATCH-3001** - head_rx channel must exist

## Risk Assessment
- **Risk**: Complex select! logic might have subtle bugs
  - **Mitigation**: Copy proven patterns from tokio examples, use standard select! syntax
- **Risk**: Event processing order might cause race conditions
  - **Mitigation**: Use proper Arc<RwLock> synchronization, document ordering assumptions
- **Risk**: Forgetting to clone Arc references before moving into task
  - **Mitigation**: Clone all Arc references at the same location before spawning
- **Risk**: Holding locks across await points could cause deadlocks
  - **Mitigation**: Ensure handle_branch_switch follows lock discipline (acquire, read, drop, then await)

## Files/Packages Affected
- `crates/maproom/src/indexer/mod.rs` (~50 lines of modifications around line 847-950)
