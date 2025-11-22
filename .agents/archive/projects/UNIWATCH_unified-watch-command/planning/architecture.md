# Architecture: Unified Watch Command

## Executive Summary

**Approach**: Modify existing `watch_worktree()` function to add branch detection, achieving unified watching with ~150 lines of modifications instead of creating new infrastructure.

**Key Insight**: The existing `watch_worktree()` function already has all the pieces we need:
- WorktreeWatcher for file monitoring
- Event processing loop
- IncrementalProcessor integration
- Database connection pool

**What's Missing**: Just need to add .git/HEAD watching to the same event loop.

## Technical Approach

```rust
// MODIFY existing watch_worktree() function (~150 lines)
pub async fn watch_worktree(
    _client: &Client,
    repo: &str,
    worktree: &str,  // Optional - auto-detect if empty
    root: &Path,
    throttle: &str,
) -> anyhow::Result<()> {
    // EXISTING: WorktreeWatcher setup (lines 756-820)
    let (mut watcher, mut event_rx) = WorktreeWatcher::new(...);
    watcher.start()?;

    // NEW: Add .git/HEAD watcher (20 lines)
    let git_head = root.join(".git/HEAD");
    let (head_tx, mut head_rx) = tokio::sync::mpsc::channel(10);
    let head_watcher = setup_head_watcher(&git_head, head_tx)?;

    // NEW: Track current worktree dynamically (10 lines)
    let current_worktree = Arc::new(RwLock::new(worktree.to_string()));
    let current_worktree_id = Arc::new(RwLock::new(/* get from db */));

    // MODIFY: Event loop to handle both sources (50 lines)
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(file_event) = event_rx.recv() => {
                    // EXISTING file processing logic (lines 848-950)
                    // Just read current_worktree_id instead of hardcoded
                },
                Some(head_event) = head_rx.recv() => {
                    // NEW: Handle branch switch (30 lines)
                    handle_branch_switch(&current_worktree, &current_worktree_id).await;
                }
            }
        }
    });

    // EXISTING: Shutdown handling (lines 950+)
}
```

**Total**: ~150 lines of modifications to existing function

## Why This Works

### 1. **Reuses Proven Infrastructure**
- WorktreeWatcher already tags events correctly
- Event processing pipeline already exists
- Database pool already initialized
- Error handling already implemented

### 2. **Single Event Loop**
- No need to coordinate two separate watchers
- tokio::select! handles channel multiplexing
- Both event types in same async context
- Simpler shutdown coordination

### 3. **Minimal New Code**
- Just add .git/HEAD watching
- Just add worktree ID mutation
- Modify existing event handler to read dynamic worktree_id
- Everything else stays the same

### 4. **Integration is Trivial**
```rust
// In main.rs Commands::Watch handler
Commands::Watch { repo, worktree, path, throttle } => {
    let repo = repo.unwrap_or_else(|| get_git_info(&path).0);
    let worktree = worktree.unwrap_or_else(|| get_current_branch(&path));

    // Just call existing function - no API change needed!
    indexer::watch_worktree(&client, &repo, &worktree, &path, &throttle).await?;
}
```

## Detailed Design

### Component 1: .git/HEAD Watcher

**Purpose**: Detect branch switches

**Implementation** (20 lines):
```rust
fn setup_head_watcher(
    git_head: &Path,
    tx: tokio::sync::mpsc::Sender<notify::Event>
) -> Result<notify::RecommendedWatcher> {
    use notify::{Watcher, RecursiveMode};

    let (sync_tx, mut sync_rx) = std::sync::mpsc::channel();

    let mut watcher = notify::recommended_watcher(move |res| {
        if let Ok(event) = res {
            let _ = sync_tx.send(event);
        }
    })?;

    watcher.watch(git_head, RecursiveMode::NonRecursive)?;

    // Bridge std::sync::mpsc to tokio::mpsc
    tokio::spawn(async move {
        while let Ok(event) = sync_rx.recv() {
            let _ = tx.send(event).await;
        }
    });

    Ok(watcher)
}
```

**Why this works**:
- Reuses same notify::RecommendedWatcher as BranchWatcher
- Bridges sync channel to async (same pattern as WorktreeWatcher internally)
- Simple, tested approach

### Component 2: Dynamic Worktree Tracking

**Purpose**: Update worktree_id when branch changes

**Implementation** (10 lines):
```rust
// At start of watch_worktree()
let current_branch = Arc::new(RwLock::new(worktree.to_string()));
let current_worktree_id = Arc::new(RwLock::new({
    let (repo_id, _) = get_or_create_repo(&pool, repo).await?;
    let (worktree_id, _) = get_or_create_worktree(&pool, repo_id, worktree, root).await?;
    worktree_id
}));
```

**Thread Safety**:
- Arc allows sharing across event loop and handler tasks
- RwLock allows concurrent reads, exclusive writes
- Same pattern used throughout maproom codebase

### Component 3: Branch Switch Handler

**Purpose**: Update worktree when .git/HEAD changes

**Implementation** (30 lines):
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

    // Check if actually changed
    {
        let current = current_branch.read().unwrap();
        if *current == new_branch {
            return Ok(()); // False alarm or duplicate event
        }
    }

    info!("Branch switch detected: {} -> {}",
          current_branch.read().unwrap(), new_branch);

    // Get/create worktree record
    let (repo_id, _) = get_or_create_repo(pool, repo).await?;
    let (new_worktree_id, created) = get_or_create_worktree(
        pool, repo_id, &new_branch, repo_path
    ).await?;

    // Update tracking
    {
        let mut branch = current_branch.write().unwrap();
        *branch = new_branch;
    }
    {
        let mut id = current_worktree_id.write().unwrap();
        *id = new_worktree_id;
    }

    // Trigger incremental update
    incremental_update(pool, repo, &new_branch, repo_path).await?;

    info!("Switched to worktree_id={} (created={})", new_worktree_id, created);
    Ok(())
}
```

**Debouncing**: Reuse DebouncedHandler from watcher.rs (copy 20 lines)

### Component 4: Modified Event Processing

**Change**: Read dynamic worktree_id instead of using hardcoded

**Before** (line 818):
```rust
let worktree_id = format!("{}:{}", repo, worktree);
let (mut watcher, mut event_rx) = WorktreeWatcher::new(worktree_id.clone(), ...);
```

**After**:
```rust
// Start with initial worktree, but WorktreeWatcher's worktree_id is just for logging
let initial_worktree_id = format!("{}:{}", repo, worktree);
let (mut watcher, mut event_rx) = WorktreeWatcher::new(initial_worktree_id, ...);

// For actual database operations, read from current_worktree_id
let worktree_id = *current_worktree_id.read().unwrap();
```

**Impact**: ~10 line change in event processing loop

## Event Flow

```
User Action: git checkout feature
    ↓
.git/HEAD modified
    ↓
notify event → head_rx channel
    ↓
tokio::select! branch: head_rx.recv()
    ↓
handle_branch_switch()
    - get_current_branch() → "feature"
    - get_or_create_worktree() → worktree_id=42
    - Update current_worktree_id → 42
    - Call incremental_update()
    ↓
File changes detected
    ↓
FileWatcher → event_rx channel
    ↓
tokio::select! branch: event_rx.recv()
    ↓
Process with *current_worktree_id.read() → 42
    ↓
Indexed to correct worktree ✓
```

## Integration Strategy

### Phase 1: Add .git/HEAD watching (30 lines)
- setup_head_watcher() function
- Add to watch_worktree() initialization
- Bridge channels

### Phase 2: Add dynamic worktree tracking (20 lines)
- Arc<RwLock<String>> for branch
- Arc<RwLock<i32>> for worktree_id
- Initialize from parameters

### Phase 3: Add branch switch handler (30 lines)
- handle_branch_switch() function
- Call from event loop

### Phase 4: Modify event loop (50 lines)
- Change while let to loop + tokio::select!
- Add head_rx branch
- Read current_worktree_id instead of hardcoded

### Phase 5: Update CLI (20 lines)
- Auto-detect branch if --worktree not provided
- Deprecation warning if --worktree provided

**Total: ~150 lines of modifications/additions**

## NDJSON Event Format

### New Event: branch_switched

```json
{
  "type": "branch_switched",
  "timestamp": "2025-01-16T10:30:00Z",
  "repo": "crewchief",
  "old_branch": "main",
  "new_branch": "feature-auth",
  "old_worktree_id": 1,
  "new_worktree_id": 42,
  "worktree_created": false
}
```

**Implementation** (15 lines):
```rust
#[derive(Serialize)]
struct BranchSwitchEvent {
    #[serde(rename = "type")]
    event_type: &'static str,
    timestamp: String,
    repo: String,
    old_branch: String,
    new_branch: String,
    old_worktree_id: i32,
    new_worktree_id: i32,
    worktree_created: bool,
}

// Emit after handle_branch_switch()
let event = BranchSwitchEvent { ... };
println!("{}", serde_json::to_string(&event)?);
```

## Error Recovery

### Scenario 1: .git/HEAD Deleted
```rust
// In setup_head_watcher() - handle watch failure
if let Err(e) = watcher.watch(git_head, RecursiveMode::NonRecursive) {
    warn!("Failed to watch .git/HEAD: {}. Branch detection disabled.", e);
    // Continue with file watching only
}
```

### Scenario 2: Branch Detection Failure
```rust
// In handle_branch_switch()
if let Err(e) = get_current_branch(repo_path) {
    error!("Failed to detect branch: {}. Keeping current worktree.", e);
    return Ok(()); // Preserve state
}
```

### Scenario 3: Database Connection Loss
```rust
// Already handled in existing watch_worktree() code
// Event processing continues, errors logged
```

## Security Assessment

**No new security risks introduced**:
- ✅ Same components (just coordinated differently)
- ✅ Same database queries (parameterized)
- ✅ Same file operations (read-only)
- ✅ Same thread safety patterns (Arc<RwLock>)

See security-review.md for full analysis.

## Performance

**Expected**:
- CPU idle: <2% (same as current, just one additional file watcher)
- Memory: ~20MB (vs 35MB for two processes)
- Branch detection: <1 second
- No file events lost (buffered channels)

**Why this approach is efficient**:
- Single event loop (no context switching)
- Shared database pool (no connection overhead)
- Minimal synchronization points (just worktree_id updates)

## Testing Strategy

### Unit Tests (8 tests)
1. setup_head_watcher() creates watcher
2. handle_branch_switch() updates state
3. handle_branch_switch() calls incremental_update()
4. Dynamic worktree_id read in event processing
5. Debouncing prevents rapid switches
6. Error recovery preserves state
7. NDJSON event serialization
8. Channel bridging works correctly

### Integration Tests (4 tests)
1. Complete branch switch workflow
2. Rapid branch switches debounced
3. File changes during branch switch
4. Backward compatibility (--worktree flag)

### E2E Tests (1 test)
1. Developer workflow (bash script) - real git operations, real database

### Manual Testing (1 checklist)
- Real-world developer workflow validation
- Error scenarios
- NDJSON event verification

**Total: 15 tests**

## Migration Path

**None needed!** The function signature doesn't change:

```rust
// Before and After
pub async fn watch_worktree(
    _client: &Client,
    repo: &str,
    worktree: &str,
    root: &Path,
    throttle: &str,
) -> anyhow::Result<()>
```

**CLI usage**:
```bash
# Old way - still works
maproom watch --repo myproject --worktree main

# New way - auto-detects
maproom watch --repo myproject

# Both produce same result!
```

## Advantages

1. **Less Code**: ~150 lines of modifications vs creating new infrastructure
2. **Reuses Proven Components**: WorktreeWatcher, event processing, db pool
3. **Simpler Testing**: Fewer components to test
4. **Lower Risk**: Modifying existing vs creating new
5. **Easier Review**: Changes visible in unified diff
6. **Backward Compatible**: Same function signature
7. **Faster Implementation**: 1-2 days instead of 2-3 days
