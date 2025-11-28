# Architecture: Unified Watch Command

## Overview

Add runtime branch switch detection to the watch command by integrating `.git/HEAD` file watching into the existing event loop.

## Prerequisites: Module Exports (Phase 0)

Before implementation, the following components must be exported from `indexer/mod.rs`:

| Component | Current State | Required Action |
|-----------|---------------|-----------------|
| `setup_head_watcher()` | `#[allow(dead_code)]`, private | Add `pub`, export in `mod.rs` |
| `DebouncedHandler` | `#[allow(dead_code)]`, private | Add `pub`, export in `mod.rs` |
| `BranchSwitchEvent` | `#[allow(dead_code)]`, private | Add `pub`, export in `mod.rs` |

These components exist but were marked dead_code during the PostgreSQL→SQLite migration.
They must be made public before use in `main.rs`.

## Current Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Commands::Watch Handler                       │
│                     (main.rs:1105-1216)                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Startup                                                     │
│     └── Auto-detect branch via get_current_branch() ✅          │
│     └── Create worktree_id (HARDCODED, never changes) ❌        │
│                                                                 │
│  2. MultiWatcher                                                │
│     └── Watches repository files for changes ✅                 │
│     └── Sends FileEvent to event_rx channel ✅                  │
│                                                                 │
│  3. Event Loop (tokio::select!)                                │
│     ├── Ctrl+C → shutdown ✅                                    │
│     └── FileEvent → incremental_update(worktree_id) ✅          │
│         └── Uses HARDCODED worktree_id from startup ❌          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Problem**: When user runs `git checkout feature`, the watch command doesn't know. Files continue to be indexed to the original worktree.

## Target Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Commands::Watch Handler                       │
│                     (main.rs:1105-1216)                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Startup                                                     │
│     └── Auto-detect branch via get_current_branch()             │
│     └── Create Arc<RwLock<i64>> for worktree_id  [NEW]          │
│     └── Create Arc<RwLock<String>> for branch    [NEW]          │
│                                                                 │
│  2. MultiWatcher (existing)                                     │
│     └── Watches repository files for changes                    │
│     └── Sends FileEvent to event_rx channel                     │
│                                                                 │
│  3. HEAD Watcher [NEW]                                          │
│     └── Watches .git/HEAD for branch switches                   │
│     └── Sends notify::Event to head_rx channel                  │
│     └── Uses existing setup_head_watcher() from indexer         │
│                                                                 │
│  4. Event Loop (tokio::select!)                                │
│     ├── Ctrl+C → shutdown                                       │
│     ├── FileEvent → incremental_update(*worktree_id.read())     │
│     │   └── Reads DYNAMIC worktree_id [CHANGED]                 │
│     └── HeadEvent → handle_branch_switch() [NEW]                │
│         ├── Detect new branch                                   │
│         ├── Update worktree_id and branch locks                 │
│         ├── Trigger incremental_update for new branch           │
│         └── Emit BranchSwitchEvent NDJSON                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Key Components

### 1. Dynamic Worktree State

Replace static variables with thread-safe mutable state.

**Important:** Use `std::sync::RwLock`, NOT `tokio::sync::RwLock`. This matches the existing
pattern in the codebase (see `config/hot_reload.rs`, `cache/system.rs`, `embedding/cache.rs`).
The lock is held briefly (copy value, drop lock, then use), so blocking is acceptable.

```rust
use std::sync::{Arc, RwLock};

// BEFORE (line 1149)
let worktree_id = store.get_or_create_worktree(...).await?;

// AFTER
let worktree_id: Arc<RwLock<i64>> = Arc::new(RwLock::new(
    store.get_or_create_worktree(...).await?
));
let current_branch: Arc<RwLock<String>> = Arc::new(RwLock::new(worktree.clone()));
```

### 2. HEAD Watcher

Use the existing `setup_head_watcher()` function from `indexer/mod.rs`:

```rust
// Initialize HEAD watcher (before event loop)
let git_head = watch_path.join(".git/HEAD");
let (head_tx, mut head_rx) = tokio::sync::mpsc::channel(10);
let _head_watcher = crewchief_maproom::indexer::setup_head_watcher(&git_head, head_tx)?;
```

### 3. Modified Event Loop

Add HEAD events to the existing `tokio::select!`:

```rust
loop {
    tokio::select! {
        _ = signal::ctrl_c() => { /* shutdown */ }

        Some(event) = event_rx.recv() => {
            // Read dynamic worktree_id
            let wt_id = *worktree_id.read().unwrap();
            incremental_update(&store, wt_id, &watch_path).await?;
        }

        Some(_head_event) = head_rx.recv() => {
            // NEW: Handle branch switch
            handle_branch_switch(
                &watch_path,
                &store,
                &repo,
                &current_branch,
                &worktree_id,
            ).await?;
        }
    }
}
```

### 4. Branch Switch Handler

**NOTE:** This function must be NEWLY IMPLEMENTED. The original `handle_branch_switch()` was
removed during the IDXABS-2001 SQLite migration (see `indexer/mod.rs:702-705`). It previously
used PostgreSQL's `PgPool` and must be reimplemented using `SqliteStore`.

The function should be implemented inline in `main.rs` (not in indexer module) to keep
event loop handling logic together:

```rust
/// Handle a branch switch detected by the HEAD watcher.
///
/// NEW IMPLEMENTATION for SQLite (original PostgreSQL version removed in IDXABS-2001).
async fn handle_branch_switch(
    watch_path: &Path,
    store: &SqliteStore,
    repo: &str,
    repo_id: i64,  // Pass repo_id to avoid extra lookup
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

    // 2. Check if actually changed (skip if same branch)
    let old_branch = current_branch.read().unwrap().clone();
    let old_wt_id = *worktree_id.read().unwrap();
    if old_branch == new_branch {
        return Ok(()); // Same branch, skip
    }

    // 3. Get/create worktree record using SqliteStore
    let watch_path_str = watch_path.to_string_lossy().to_string();
    let new_wt_id = store.get_or_create_worktree(repo_id, &new_branch, &watch_path_str).await?;
    let worktree_created = new_wt_id != old_wt_id; // Simplified check

    // 4. Update state (brief lock hold)
    {
        *current_branch.write().unwrap() = new_branch.clone();
        *worktree_id.write().unwrap() = new_wt_id;
    }

    // 5. Re-index new branch
    if let Err(e) = incremental_update(&store, new_wt_id, watch_path).await {
        tracing::warn!("Incremental update after branch switch failed: {}", e);
        // Continue anyway - don't block on indexing errors
    }

    // 6. Emit NDJSON event to stdout (for VSCode extension consumption)
    let event = BranchSwitchEvent {
        event_type: "branch_switched",
        timestamp: chrono::Utc::now().to_rfc3339(),
        repo: repo.to_string(),
        old_branch,
        new_branch,
        old_worktree_id: old_wt_id,
        new_worktree_id: new_wt_id,
        worktree_created,
    };
    println!("{}", serde_json::to_string(&event)?);

    Ok(())
}
```

## Event Flow

```
User runs: git checkout feature
    ↓
.git/HEAD modified: "ref: refs/heads/feature"
    ↓
notify crate detects change
    ↓
setup_head_watcher sends to head_rx
    ↓
tokio::select! receives HeadEvent
    ↓
handle_branch_switch() called
    ├── get_current_branch() → "feature"
    ├── Check old_branch != new_branch
    ├── get_or_create_worktree() → worktree_id=42
    ├── Update Arc<RwLock> state
    ├── incremental_update(42, path)
    └── Emit BranchSwitchEvent NDJSON
    ↓
Future file events use worktree_id=42 ✓
```

## Debouncing

Use the existing `DebouncedHandler` from `indexer/mod.rs` to prevent rapid branch switches from causing excessive indexing:

```rust
let debouncer = DebouncedHandler::new(Duration::from_secs(2));

// In handle_branch_switch()
if !debouncer.should_handle() {
    tracing::debug!("Debouncing rapid branch switch");
    return Ok(());
}
```

## NDJSON Event

The `BranchSwitchEvent` struct already exists in `indexer/mod.rs`:

```json
{
  "type": "branch_switched",
  "timestamp": "2025-01-16T10:30:00Z",
  "repo": "myproject",
  "old_branch": "main",
  "new_branch": "feature",
  "old_worktree_id": 1,
  "new_worktree_id": 42,
  "worktree_created": false
}
```

## Error Handling

| Scenario | Behavior |
|----------|----------|
| `.git/HEAD` unreadable | Log warning, continue file watching |
| Database error in `get_or_create_worktree()` | Log warning, keep using old worktree_id, do NOT crash |
| Database error in `incremental_update()` | Log warning, continue (indexing failure is non-fatal) |
| Detached HEAD state | Use 8-character commit SHA as branch name (via `git rev-parse --short=8 HEAD`) |
| Rapid branch switches | Debounce with 2-second window, process only final state |

### Error Recovery Strategy

The watch command should be resilient to transient errors:

1. **Database errors**: Log and continue. The next successful operation will restore correct state.
2. **Git errors**: Log and continue. Branch detection will retry on next HEAD event.
3. **Indexing errors**: Non-fatal. Files may be temporarily unindexed but will be caught on next change.

```rust
// Error handling pattern
match handle_branch_switch(...).await {
    Ok(()) => { /* success */ }
    Err(e) => {
        tracing::warn!("Branch switch handling failed: {}. Continuing with previous state.", e);
        // Continue with old worktree_id - don't crash the watch command
    }
}
```

## Detached HEAD Handling

When the repository is in detached HEAD state (e.g., after `git checkout <commit-sha>`):

1. **Detection**: `get_current_branch()` returns "HEAD" when detached
2. **Branch Name**: Use 8-character commit SHA via `git rev-parse --short=8 HEAD`
3. **Worktree Creation**: Create worktree record with SHA as name (e.g., "abc12345")
4. **Indexing**: Index normally to the detached worktree

```rust
// Detached HEAD handling
let branch_name = get_current_branch(watch_path)?;
let effective_branch = if branch_name == "HEAD" {
    // Detached HEAD - use short SHA
    get_short_commit_sha(watch_path)? // Returns 8-char SHA like "abc12345"
} else {
    branch_name
};
```

**Note**: Frequent detached HEAD checkouts may create many worktree records. Consider
periodic cleanup via `db cleanup-stale` command.

## Event Ordering Strategy

To prevent race conditions between file events and branch switch events:

1. **tokio::select! Ordering**: File events and HEAD events are processed sequentially
   by the event loop - they cannot interleave within a single iteration.

2. **Atomic State Update**: worktree_id update is atomic (single write lock).

3. **Event Queue Behavior**: If a file event arrives while branch switch is processing,
   it waits in the channel until the next `select!` iteration.

4. **Worst Case**: A file event processed with "wrong" worktree_id is acceptable -
   the file will be re-indexed on next change with correct worktree_id.

```
Timeline (sequential, no interleaving):
  t0: FileEvent arrives in event_rx
  t1: HeadEvent arrives in head_rx
  t2: select! picks FileEvent (processes with old worktree_id)
  t3: select! picks HeadEvent (updates worktree_id)
  t4: Future FileEvents use new worktree_id ✓
```

## Thread Safety

- `Arc<RwLock<i64>>` for worktree_id allows concurrent reads, exclusive writes
- Lock held briefly (copy value, drop lock, then use)
- Same pattern used throughout maproom codebase

## Existing Components to Reuse

| Component | Location | Status | Action Required |
|-----------|----------|--------|-----------------|
| `setup_head_watcher()` | `indexer/mod.rs:668` | Private, `#[allow(dead_code)]` | **Export: add `pub`, re-export in mod.rs** |
| `DebouncedHandler` | `indexer/mod.rs:33` | Private, `#[allow(dead_code)]` | **Export: add `pub`, re-export in mod.rs** |
| `BranchSwitchEvent` | `indexer/mod.rs:112` | Private, `#[allow(dead_code)]` | **Export: add `pub`, re-export in mod.rs** |
| `get_current_branch()` | `git/mod.rs` | **Public, ready to use** | None |
| `incremental_update()` | `incremental/mod.rs` | **Public, ready to use** | None |
| `get_or_create_worktree()` | `db/sqlite/mod.rs` | **Public, ready to use** | None |

### NDJSON Output Destination

The `BranchSwitchEvent` should be emitted to **stdout** (not stderr). This is correct for:
- VSCode extension consumption (reads stdout for NDJSON events)
- CLI piping patterns (`watch | jq`)
- Consistency with existing NDJSON patterns in maproom

Regular logging uses `tracing` which outputs to stderr, keeping NDJSON events separate.
