# Ticket: UNIWATCH-1002: Add Dynamic Worktree Tracking State

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

Add Arc<RwLock> state variables to track current branch and worktree_id dynamically, initialized from function parameters.

## Background

Currently, watch_worktree() determines the worktree once at startup and never changes it. We need to track the current worktree_id dynamically so that when a branch switch is detected, we can update which worktree file changes are indexed to.

This is part of Phase 1 (Foundation) of the UNIWATCH project, establishing the state management infrastructure needed for dynamic branch detection.

## Acceptance Criteria

- [ ] `Arc<RwLock<String>>` for `current_branch` created and initialized
- [ ] `Arc<RwLock<i32>>` for `current_worktree_id` created and initialized
- [ ] Initialization uses existing `get_or_create_repo()` and `get_or_create_worktree()` functions
- [ ] Arc/RwLock pattern matches existing maproom code style
- [ ] Unit test `test_worktree_tracking_initialization()` passes

## Technical Requirements

- Location: `crates/maproom/src/indexer/mod.rs` (in watch_worktree function, after pool creation)
- Use `std::sync::Arc` and `std::sync::RwLock` (not tokio versions)
- Initialize with values from function parameters (repo, worktree, root)
- Approximately 15 lines of modifications
- No unsafe code
- No clippy warnings

## Implementation Notes

Add the tracking state after pool creation (around line 810 in the watch_worktree function):

```rust
// After pool creation (around line 810)
let current_branch = Arc::new(RwLock::new(worktree.to_string()));
let current_worktree_id = Arc::new(RwLock::new({
    let (repo_id, _) = get_or_create_repo(&pool, repo).await?;
    let (worktree_id, _) = get_or_create_worktree(&pool, repo_id, worktree, root).await?;
    worktree_id
}));
```

**Key considerations:**
- Use std::sync versions (not tokio) to match existing codebase patterns
- Initialize from function parameters to ensure correct starting state
- RwLock allows multiple readers or one writer, appropriate for this use case
- Arc enables sharing across tasks without lifetime issues

## Dependencies

None (can be done in parallel with UNIWATCH-1001)

## Risk Assessment

- **Risk**: RwLock deadlocks if lock ordering is inconsistent
  - **Mitigation**: Always acquire locks in same order, document lock acquisition pattern in comments

- **Risk**: RwLock poisoning if panic occurs while holding lock
  - **Mitigation**: Use unwrap_or_else with logging for lock acquisition, never panic while holding lock

## Files/Packages Affected

- `crates/maproom/src/indexer/mod.rs` (approximately 15 modifications)
