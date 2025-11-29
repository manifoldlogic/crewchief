# Ticket: UNIWATCH-3001: Implement Branch Switch Handler

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (tests disabled, will be migrated in UNIWATCH-4001)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the `handle_branch_switch()` function that processes HEAD file changes, updates the dynamic worktree state, triggers re-indexing, and emits NDJSON events for VSCode extension integration.

## Background
**IMPORTANT:** This is NEW code. The original `handle_branch_switch()` function was REMOVED during the IDXABS-2001 SQLite migration (see indexer/mod.rs:702-705 comment). It previously used PostgreSQL's `PgPool` and must be reimplemented using `SqliteStore`.

When the HEAD watcher detects a branch switch, this handler:
1. Debounces rapid switches (2-second window)
2. Detects the new branch name
3. Handles detached HEAD state (uses 8-char SHA)
4. Updates worktree_id dynamically
5. Triggers re-indexing
6. Emits NDJSON event to stdout

**Plan Reference:** Phase 3 - Branch Switch Handler

## Acceptance Criteria
- [x] `handle_branch_switch()` async function implemented in main.rs
- [x] Debouncing works (rapid switches within 2s are coalesced)
- [x] New branch detected via `get_current_branch()`
- [x] Detached HEAD handled with 8-char commit SHA as branch name
- [x] Same-branch switches skipped (no-op if branch unchanged)
- [x] `get_or_create_worktree()` called with SqliteStore
- [x] `Arc<RwLock>` state updated with new worktree_id and branch
- [x] `incremental_update()` triggered for new branch (errors logged, not fatal)
- [x] `BranchSwitchEvent` NDJSON emitted to stdout
- [x] `cargo check -p crewchief-maproom` passes

## Technical Requirements
```rust
/// Handle a branch switch detected by the HEAD watcher.
///
/// NEW IMPLEMENTATION for SQLite (original PostgreSQL version removed in IDXABS-2001).
async fn handle_branch_switch(
    watch_path: &Path,
    store: &SqliteStore,
    repo: &str,
    repo_id: i64,
    current_branch: &Arc<RwLock<String>>,
    worktree_id: &Arc<RwLock<i64>>,
    debouncer: &DebouncedHandler,
) -> Result<()> {
    // 0. Check debounce (skip if rapid switch)
    if !debouncer.should_handle() {
        tracing::debug!("Debouncing rapid branch switch");
        return Ok(());
    }

    // 1. Detect new branch
    let new_branch = get_current_branch(watch_path)?;

    // 2. Handle detached HEAD
    let effective_branch = if new_branch == "HEAD" {
        get_short_commit_sha(watch_path)? // 8-char SHA
    } else {
        new_branch
    };

    // 3. Check if actually changed
    let old_branch = current_branch.read().unwrap().clone();
    let old_wt_id = *worktree_id.read().unwrap();
    if old_branch == effective_branch {
        return Ok(()); // Same branch, skip
    }

    // 4. Get/create worktree record
    let watch_path_str = watch_path.to_string_lossy().to_string();
    let new_wt_id = store.get_or_create_worktree(repo_id, &effective_branch, &watch_path_str).await?;
    let worktree_created = new_wt_id != old_wt_id;

    // 5. Update state (brief lock hold)
    {
        *current_branch.write().unwrap() = effective_branch.clone();
        *worktree_id.write().unwrap() = new_wt_id;
    }

    // 6. Re-index (log errors, don't crash)
    if let Err(e) = incremental_update(&store, new_wt_id, watch_path).await {
        tracing::warn!("Incremental update after branch switch failed: {}", e);
    }

    // 7. Emit NDJSON event to stdout
    let event = BranchSwitchEvent {
        event_type: "branch_switched",
        timestamp: chrono::Utc::now().to_rfc3339(),
        repo: repo.to_string(),
        old_branch,
        new_branch: effective_branch,
        old_worktree_id: old_wt_id,
        new_worktree_id: new_wt_id,
        worktree_created,
    };
    println!("{}", serde_json::to_string(&event)?);

    Ok(())
}
```

**Detached HEAD helper:**
```rust
fn get_short_commit_sha(path: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .current_dir(path)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git rev-parse: {}. Is git installed?", e))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "git rev-parse failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
```

**Note:** If git is not installed or fails, the error is logged and the function returns an error. The caller (handle_branch_switch) should catch this and continue with the old worktree_id rather than crashing.

**Error handling pattern:**
- Database errors: Log warning, continue with old worktree_id
- Git errors: Log warning, continue (retry on next event)
- Indexing errors: Non-fatal, continue watching

## Implementation Notes
- Function goes inline in main.rs (not indexer module) to keep event loop logic together
- Use existing `DebouncedHandler` from indexer module (2-second window)
- NDJSON goes to stdout (not stderr) for VSCode extension consumption
- Tracing output goes to stderr, keeping streams separate
- Lock hold time is minimal (copy/clone, then immediately drop)

**Async context note:** `get_current_branch()` and `get_short_commit_sha()` are synchronous functions called from async context. This is acceptable because:
- Both are fast file/subprocess operations (milliseconds)
- They don't hold resources that block async tasks
- The tokio runtime can handle brief blocking for these use cases
- Pattern matches existing codebase usage in watch command

**Required imports:**
```rust
use crewchief_maproom::indexer::{setup_head_watcher, DebouncedHandler, BranchSwitchEvent};
use crewchief_maproom::git::get_current_branch;
```

## Dependencies
- UNIWATCH-0001 (BranchSwitchEvent, DebouncedHandler must be exported)
- UNIWATCH-1001 (dynamic state variables must exist)
- UNIWATCH-2001 (HEAD watcher must be integrated)

## Risk Assessment
- **Risk**: Database error during get_or_create_worktree crashes watch
  - **Mitigation**: Log warning and continue with old worktree_id
- **Risk**: Rapid branch switches cause excessive re-indexing
  - **Mitigation**: DebouncedHandler with 2-second window
- **Risk**: Detached HEAD creates many worktree records over time
  - **Mitigation**: Document periodic cleanup via `db cleanup-stale`

## Files/Packages Affected
- `crates/maproom/src/main.rs` (~60 lines added: handle_branch_switch function + helper)
