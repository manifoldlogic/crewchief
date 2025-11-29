# Analysis: Automatic Branch Switch Detection

## Problem Definition

**Current workflow** (post-BRANCHX):
```bash
git checkout feature        # Switch branch
maproom scan --worktree feature  # Manual indexing step
```

**Problems**:
1. **Manual step**: Developers must remember to run scan
2. **Friction**: Breaks flow, context switch
3. **Out-of-sync**: Search results don't match current branch
4. **Cognitive load**: "Did I index this branch?"

## User Experience Gap

**Expectation**: "I switched branches, search should show code from this branch"
**Reality**: "I need to manually trigger indexing, wait, then search"

This violates principle of **least surprise** - tools should work automatically.

## Industry Solutions

### JetBrains IntelliJ
- **Approach**: Background indexing on file changes
- **Trigger**: File system watcher
- **UX**: Indexing indicator in status bar

### VS Code
- **Approach**: Extension host watches workspace
- **Trigger**: Workspace file changes
- **UX**: Background process, no user action

### Sourcegraph
- **Approach**: Poll git refs every N seconds
- **Trigger**: Timer-based
- **UX**: Server-side, transparent to user

## Our Approach: File Watching

**Why file watching**:
- **Immediate**: OS-level events, <1s detection
- **Efficient**: No polling waste, CPU idle when no changes
- **Reliable**: File system events guaranteed by OS
- **Cross-platform**: Works on Linux, macOS, Windows

**What to watch**: `.git/HEAD`
- Changes on every branch switch
- Single file (not entire .git/)
- Content = current branch ref

## Git HEAD File Explained

### What is .git/HEAD?

```bash
cat .git/HEAD
# On branch main:
ref: refs/heads/main

# On branch feature:
ref: refs/heads/feature

# Detached HEAD:
abc123def456...  # Commit SHA
```

**Key property**: File changes on every `git checkout`

### Why This Works

```bash
# Before
cat .git/HEAD  # ref: refs/heads/main

git checkout feature

# After
cat .git/HEAD  # ref: refs/heads/feature (FILE CHANGED!)
```

**File watcher** sees change → triggers handler → indexes new branch

## Technical Constraints

### 1. File Watcher Limitations

**Challenges**:
- Must run as background process (not one-shot command)
- Requires keeping process alive
- Must handle watcher errors gracefully

**Solution**: Long-running CLI command (`maproom watch`)

### 2. Rapid Branch Switching

**Scenario**:
```bash
git checkout feature-1  # Trigger index
git checkout feature-2  # Trigger index (1st still running!)
git checkout feature-3  # Trigger index (2 running!)
```

**Problem**: Multiple concurrent indexing operations

**Solution**: Debouncing or queueing
```rust
// Option 1: Debounce (wait for calm)
// Option 2: Queue (process sequentially)
// Option 3: Cancel previous (latest wins)
```

**Choice**: Queue with cancellation (balance correctness & responsiveness)

### 3. Error Handling

**Scenarios**:
- Database connection lost
- Git repository corrupted
- File watcher crashes
- Disk full during indexing

**Strategy**: Log, continue watching
```rust
loop {
    match handle_branch_switch().await {
        Ok(_) => info!("Index updated"),
        Err(e) => {
            error!("Index failed: {}", e);
            // Continue watching (don't crash)
        }
    }
}
```

## Success Criteria

1. **Automatic detection**: No manual scan needed
2. **Fast**: <1 minute from switch to indexed
3. **Reliable**: 100% detection (no missed switches)
4. **Efficient**: <5% CPU while idle
5. **Robust**: Handles errors, continues running

## Out of Scope

**Not included**:
- Watch file changes within branch → Use IDE for this
- Auto-commit detection → Only branch switches
- Remote branch tracking → Only local checkouts
- GUI notifications → CLI only for MVP

**Why**: Focus on core value (auto-indexing on branch switch)

## Dependencies

**Requires**: BRANCHX complete
- `incremental_update()` function
- `get_or_create_worktree()` function
- Worktree-specific indexing working

**New dependencies**:
- `notify` crate (v5.0+) for file watching
- Async runtime already exists (tokio)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Watcher misses events | Low | High | Use reliable notify crate, test extensively |
| Resource leak | Medium | Medium | Graceful shutdown, cleanup on exit |
| Rapid switching | Medium | Low | Debouncing/queueing |
| Cross-platform issues | Low | Medium | Test on Linux/macOS/Windows |

## Next Steps

1. Design watcher architecture (architecture.md)
2. Plan error handling strategy (architecture.md)
3. Define test strategy (quality-strategy.md)
4. Create implementation plan (plan.md)
