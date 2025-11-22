# Ticket: UNIWATCH-3003: Update Event Processing to Use Dynamic Worktree ID

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
Modify file event processing to read current_worktree_id dynamically instead of using the hardcoded worktree_id, ensuring file changes are indexed to the correct worktree after branch switches.

## Background
Currently, file events are tagged with a hardcoded worktree_id determined at startup. After implementing dynamic tracking, we need to read current_worktree_id when processing each file event so that files are indexed to whichever branch is currently checked out.

This ticket completes Phase 3 (Event Loop Integration) of the UNIWATCH project by ensuring file events use the dynamically tracked worktree ID that changes when branches are switched.

## Acceptance Criteria
- [ ] File event processing reads `*current_worktree_id.read().unwrap()` instead of using hardcoded value
- [ ] Database queries use the dynamic worktree_id
- [ ] Logging shows correct worktree_id
- [ ] No change to processing logic (only which worktree_id is used)
- [ ] Integration test `test_file_events_use_current_worktree()` passes

## Technical Requirements
- Location: `crates/maproom/src/indexer/mod.rs` (in event processing loop, around line 850+)
- Approximately 10 lines of modifications
- Clone Arc for use in spawned task
- Read lock must be dropped quickly (don't hold across await points)
- Use the read value immediately, don't cache across await boundaries

## Implementation Notes
The existing code creates WorktreeWatcher with a worktree_id at startup (line 818):

```rust
let worktree_id = format!("{}:{}", repo, worktree);
let (mut watcher, mut event_rx) = WorktreeWatcher::new(worktree_id.clone(), ...);
```

**Important**: This worktree_id is for WorktreeWatcher's internal logging only. For database operations, we need to use the dynamic current_worktree_id.

Modifications needed in the event processing loop:

```rust
// Clone Arc before moving into spawned task (around line 847)
let current_worktree_id_clone = current_worktree_id.clone();

// Inside event processing (where database queries happen):
// BEFORE:
// Uses hardcoded worktree_id from startup

// AFTER:
let worktree_id = *current_worktree_id_clone.read().unwrap();
// Use this worktree_id for database queries
```

**Key implementation details:**
- Clone `current_worktree_id` Arc before spawning processor_task
- Read the value at the point of use (when making database queries)
- Drop the lock immediately (use pattern: `let id = *lock.read().unwrap();`)
- Use the read value for all database operations in that event processing iteration
- Don't cache across await points - read fresh for each event

**Lock discipline:**
1. Acquire read lock
2. Copy the String value
3. Lock is dropped (automatic when read() guard goes out of scope)
4. Use the copied value in database queries

## Dependencies
- **UNIWATCH-1002** - current_worktree_id must exist
- **UNIWATCH-3002** - event loop must be modified first (select! structure must be in place)

## Risk Assessment
- **Risk**: Holding read lock across await points could cause deadlocks
  - **Mitigation**: Read value, drop lock immediately (use `*lock.read()` to copy the value), then use value
- **Risk**: Race condition if branch switches during file processing
  - **Mitigation**: Acceptable - file will be indexed to whichever branch was active when processing started. This is correct behavior.
- **Risk**: Unwrap might panic if lock is poisoned
  - **Mitigation**: Acceptable - if lock is poisoned, the system is in an invalid state and should panic

## Files/Packages Affected
- `crates/maproom/src/indexer/mod.rs` (~10 lines of modifications in event processing section)
