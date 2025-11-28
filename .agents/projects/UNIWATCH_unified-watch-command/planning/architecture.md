# Architecture: Unified Watch Command

## Overview

Add runtime branch switch detection to the watch command by integrating `.git/HEAD` file watching into the existing event loop.

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

Replace static variables with thread-safe mutable state:

```rust
// BEFORE (line 1149)
let worktree_id = store.get_or_create_worktree(...).await?;

// AFTER
let worktree_id = Arc::new(RwLock::new(
    store.get_or_create_worktree(...).await?
));
let current_branch = Arc::new(RwLock::new(worktree.clone()));
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

New function to handle branch switches:

```rust
async fn handle_branch_switch(
    watch_path: &Path,
    store: &SqliteStore,
    repo: &str,
    current_branch: &Arc<RwLock<String>>,
    worktree_id: &Arc<RwLock<i64>>,
) -> Result<()> {
    // 1. Detect new branch
    let new_branch = get_current_branch(watch_path)?;

    // 2. Check if actually changed
    let old_branch = current_branch.read().unwrap().clone();
    if old_branch == new_branch {
        return Ok(()); // Same branch, skip
    }

    // 3. Get/create worktree record
    let repo_id = store.get_repo_id(repo).await?;
    let new_wt_id = store.get_or_create_worktree(repo_id, &new_branch, ...).await?;

    // 4. Update state
    *current_branch.write().unwrap() = new_branch.clone();
    *worktree_id.write().unwrap() = new_wt_id;

    // 5. Re-index new branch
    incremental_update(&store, new_wt_id, watch_path).await?;

    // 6. Emit NDJSON event
    let event = BranchSwitchEvent {
        event_type: "branch_switched",
        old_branch,
        new_branch,
        // ...
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
| Database connection lost | Log error, retry on next event |
| Detached HEAD state | Use short commit SHA as branch name |
| Rapid branch switches | Debounce, process only final state |

## Thread Safety

- `Arc<RwLock<i64>>` for worktree_id allows concurrent reads, exclusive writes
- Lock held briefly (copy value, drop lock, then use)
- Same pattern used throughout maproom codebase

## Existing Components to Reuse

| Component | Location | Status |
|-----------|----------|--------|
| `setup_head_watcher()` | `indexer/mod.rs:668` | Exists, ready to use |
| `DebouncedHandler` | `indexer/mod.rs:33` | Exists, ready to use |
| `BranchSwitchEvent` | `indexer/mod.rs:112` | Exists, ready to use |
| `get_current_branch()` | `git/mod.rs` | Exists, ready to use |
| `incremental_update()` | `incremental/mod.rs` | Exists, ready to use |
