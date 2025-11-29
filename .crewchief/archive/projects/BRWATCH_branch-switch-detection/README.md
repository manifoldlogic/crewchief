# BRWATCH: Automatic Branch Switch Detection

**Status**: ✅ **COMPLETED**
**Slug**: BRWATCH
**Completion Date**: 2025-11-09
**Dependencies**: BRANCHX (Branch-Aware Indexing)
**Blocks**: None (Final project in sequence)

## Problem Statement

After BRANCHX, we have:
✅ Worktree-specific indexing
✅ Incremental updates (fast!)
✅ Tree SHA optimization

But developers must manually run:
```bash
git checkout feature
maproom scan --worktree feature  # Manual step!
```

**Goal**: Automatically detect branch switches and trigger incremental updates

## Proposed Solution

**Watch `.git/HEAD` for changes** and auto-trigger incremental updates:

```bash
# Developer workflow
git checkout feature

# Behind the scenes (automatic):
# 1. .git/HEAD file changes
# 2. File watcher detects change
# 3. Get new branch name
# 4. Trigger incremental_update()
# 5. Index updated in <1 minute
# 6. Developer can immediately search new branch
```

## Success Metrics

- **Detection speed**: <1 second to detect branch switch
- **Update speed**: <1 minute for typical branch switch (via incremental update)
- **Reliability**: 100% detection rate (no missed switches)
- **Resource usage**: <5% CPU when idle

## Architecture

### File Watching

**Strategy**: Use `notify` crate to watch `.git/HEAD`

```rust
use notify::{Watcher, RecursiveMode, watcher};

async fn watch_for_branch_switches(repo_path: &Path) -> Result<()> {
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_secs(1))?;

    let git_head = repo_path.join(".git/HEAD");
    watcher.watch(&git_head, RecursiveMode::NonRecursive)?;

    while let Ok(event) = rx.recv() {
        match event {
            DebouncedEvent::Write(_) | DebouncedEvent::Create(_) => {
                handle_branch_switch(repo_path).await?;
            }
            _ => {}
        }
    }

    Ok(())
}
```

### Branch Switch Handler

```rust
async fn handle_branch_switch(repo_path: &Path) -> Result<()> {
    let current_branch = get_current_branch(repo_path)?;

    info!("Branch switch detected: {}", current_branch);

    let pool = get_pool().await?;
    let worktree_id = get_or_create_worktree(&pool, &current_branch).await?;

    // Trigger incremental update (from BRANCHX)
    let stats = incremental_update(&pool, worktree_id, repo_path).await?;

    info!("Index updated: {} chunks processed, {} embeddings generated",
          stats.chunks_processed, stats.embeddings_generated);

    Ok(())
}
```

## Implementation Phases

### Phase 1: File Watcher (Day 1)
- Implement `.git/HEAD` watcher
- Detect file changes
- Extract current branch name
- Unit tests for watcher

**Deliverable**: Branch switches detected

### Phase 2: Auto-Update Logic (Day 2)
- Integrate with incremental_update
- Handle errors gracefully
- Add debouncing (avoid multiple triggers)
- Integration tests

**Deliverable**: Auto-updates working

### Phase 3: CLI Command (Day 3)
- `maproom watch --repo <path>` command
- Run as background process
- Graceful shutdown
- Logging and metrics

**Deliverable**: User-facing command

### Phase 4: Documentation (Day 4)
- Usage guide
- Troubleshooting
- Performance tuning
- Buffer for issues

## Testing Strategy

### Unit Tests
- `test_detect_branch_switch` - Watcher triggers on .git/HEAD change
- `test_extract_branch_name` - Parse branch from HEAD file
- `test_debounce_multiple_changes` - Don't trigger multiple times

### Integration Tests
- `test_auto_update_on_switch` - Full workflow
- `test_watcher_handles_errors` - Error scenarios
- `test_concurrent_switches` - Rapid switching

### E2E Tests
- Manual: Switch branches rapidly, verify all indexed
- Performance: CPU usage while idle
- Reliability: 1000 switches, 0 missed

## CLI Usage

### Start Watching

```bash
maproom watch --repo /path/to/repo

# Output:
# [INFO] Watching for branch switches in /path/to/repo
# [INFO] Current branch: main (already indexed)
# [INFO] Waiting for changes...
```

### Branch Switch Flow

```bash
# Terminal 1: Watch running
maproom watch --repo myproject

# Terminal 2: Developer switches branches
git checkout feature-auth

# Terminal 1: Auto-update
# [INFO] Branch switch detected: feature-auth
# [INFO] Triggering incremental update...
# [INFO] Processing 150 changed files...
# [INFO] Index updated: 7,500 chunks, 1,200 new embeddings
# [INFO] Cost: $0.024
# [INFO] Waiting for changes...
```

## Developer Experience

### Before (Manual)
```bash
git checkout feature
maproom scan --worktree feature  # Must remember!
# Wait 5 minutes...
```

### After (Automatic)
```bash
git checkout feature
# Auto-updates in background (<1 minute)
# Can immediately search feature branch
```

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| File watcher fails | High | Log errors, auto-restart |
| Rapid branch switching | Medium | Debouncing, queue updates |
| CPU usage while idle | Low | Efficient polling, tested |
| Missed detection | High | Comprehensive testing |

## Dependencies

**Requires**: BRANCHX complete
- Uses `incremental_update()` API
- Uses `get_or_create_worktree()`
- Uses worktree-specific indexing

**New dependencies**: `notify` crate (file watching)

## Technology Choices

### Why `notify` Crate?

**Alternatives**:
1. **Polling** - Check .git/HEAD every N seconds
   - Con: Wastes CPU, delays detection
2. **Git hooks** - Use post-checkout hook
   - Con: Requires user to install hook
3. **notify crate** ✅
   - Pro: Native file system events (efficient)
   - Pro: Cross-platform (Linux, macOS, Windows)
   - Pro: Battle-tested (many users)

### Why .git/HEAD?

**Alternatives**:
1. **Watch all .git/** - Too many events
2. **Poll `git rev-parse`** - Wasteful
3. **.git/HEAD** ✅
   - Changes on every branch switch
   - Single file to watch
   - Lightweight

## Performance Characteristics

### Idle State
- CPU: <1% (file watcher uses OS events)
- Memory: ~10MB (watcher + channel)
- Disk: 0 I/O (event-driven)

### Branch Switch
- Detection: <1 second (OS file event)
- Update: <1 minute (incremental, from BRANCHX)
- Total: ~1 minute from `git checkout` to indexed

## Acceptance Criteria

- [ ] File watcher detects all branch switches
- [ ] Auto-update triggers incremental_update
- [ ] CLI command works (`maproom watch`)
- [ ] Error handling graceful (logs, continues)
- [ ] CPU usage <5% while idle
- [ ] No missed detections (100% reliability)
- [ ] Documentation complete
- [ ] Manual testing successful

**Timeline**: 3-4 days (1 buffer day)

## Optional Enhancements (Future)

**Not in MVP**:
- Desktop notifications on index complete
- Progress bar during update
- Multiple repository watching
- WebSocket API for live status

**Rationale**: Core functionality first, polish later

---

**Next Steps**: Generate tickets using `/create-project-tickets BRWATCH`
