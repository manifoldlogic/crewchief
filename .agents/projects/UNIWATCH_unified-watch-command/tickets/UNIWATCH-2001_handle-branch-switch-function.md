# Ticket: UNIWATCH-2001: Implement handle_branch_switch() Function

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
Create the core branch switch handler that detects branch changes, updates database records, updates shared state, triggers re-indexing, and emits NDJSON events.

## Background
When `.git/HEAD` changes (user runs `git checkout`), we need to:
1. Detect the new branch name
2. Get/create the worktree database record
3. Update our shared state (current_branch, current_worktree_id)
4. Trigger incremental_update() to re-index for the new branch
5. Emit a branch_switched NDJSON event for the VSCode extension

This is the core logic that makes unified watching work. This ticket implements Phase 2 (Branch Switch Logic) from the UNIWATCH project plan, specifically the branch detection and state update functionality.

## Acceptance Criteria
- [ ] Function signature: `async fn handle_branch_switch(repo_path: &Path, current_branch: &Arc<RwLock<String>>, current_worktree_id: &Arc<RwLock<i32>>, pool: &PgPool, repo: &str) -> Result<()>`
- [ ] Uses `get_current_branch()` to extract branch name from `.git/HEAD`
- [ ] Early returns if branch hasn't actually changed (prevents unnecessary work)
- [ ] Calls `get_or_create_repo()` and `get_or_create_worktree()` to get database record
- [ ] Updates both `Arc<RwLock>` state variables with write locks
- [ ] Calls `incremental_update()` to trigger re-indexing
- [ ] Emits `branch_switched` NDJSON event to stdout
- [ ] Unit test `test_handle_branch_switch_updates_state()` passes
- [ ] Unit test `test_handle_branch_switch_skips_if_same_branch()` passes

## Technical Requirements
- Location: `crates/maproom/src/indexer/mod.rs` (new function before `watch_worktree`)
- Approximately 40 lines of new code
- Use proper lock acquisition pattern (acquire read lock, drop it, then acquire write lock if needed)
- Log branch switch with `info!()` macro
- Handle errors gracefully (preserve state on failure)
- Follow Rust best practices for async/await and error handling
- Use anyhow::Result for error handling
- Ensure thread-safe access to shared state

## Implementation Notes

Implementation approach:

```rust
async fn handle_branch_switch(
    repo_path: &Path,
    current_branch: &Arc<RwLock<String>>,
    current_worktree_id: &Arc<RwLock<i32>>,
    pool: &PgPool,
    repo: &str,
) -> Result<()> {
    // Get new branch name
    let new_branch = get_current_branch(repo_path)?;

    // Check if changed (read lock)
    {
        let current = current_branch.read().unwrap();
        if *current == new_branch {
            return Ok(()); // No change
        }
    }

    info!("Branch switch detected: {} -> {}",
          current_branch.read().unwrap(), new_branch);

    // Get/create worktree record
    let (repo_id, _) = get_or_create_repo(pool, repo).await?;
    let (new_worktree_id, created) = get_or_create_worktree(
        pool, repo_id, &new_branch, repo_path
    ).await?;

    // Update state (write locks)
    {
        let mut branch = current_branch.write().unwrap();
        *branch = new_branch.clone();
    }
    {
        let mut id = current_worktree_id.write().unwrap();
        *id = new_worktree_id;
    }

    // Trigger re-indexing
    incremental_update(pool, repo, &new_branch, repo_path).await?;

    info!("Switched to worktree_id={} (created={})", new_worktree_id, created);
    Ok(())
}
```

**Key Considerations**:
- Lock ordering: Always acquire read lock first, verify state, drop it, then acquire write lock to prevent deadlocks
- Error handling: If `incremental_update()` fails, log the error but consider whether to crash or continue watching
- State consistency: Capture old values before updating state (needed for NDJSON event in UNIWATCH-2002)
- Thread safety: Use proper RAII lock guards, minimize lock hold time

## Dependencies
- UNIWATCH-1002 (needs `Arc<RwLock>` state variables)
- Requires existing functions: `get_current_branch()`, `get_or_create_repo()`, `get_or_create_worktree()`, `incremental_update()`

## Risk Assessment
- **Risk**: Lock ordering issues could cause deadlocks
  - **Mitigation**: Always acquire read lock first, drop it, then acquire write lock; document lock ordering rules in code comments

- **Risk**: `incremental_update()` failures could crash the watcher
  - **Mitigation**: Add error handling with context using anyhow; consider logging errors and continuing watch rather than crashing

- **Risk**: Race conditions between branch detection and state updates
  - **Mitigation**: Use atomic operations (lock guards ensure atomicity); verify branch hasn't changed again after acquiring write lock

- **Risk**: Performance impact from repeated database calls
  - **Mitigation**: Early return optimization prevents unnecessary DB calls when branch hasn't changed

## Files/Packages Affected
- `crates/maproom/src/indexer/mod.rs` (~40 new lines for function implementation)
- Test file (location TBD based on existing test structure, likely `crates/maproom/src/indexer/tests.rs` or inline tests)
