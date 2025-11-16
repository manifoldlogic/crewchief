# Analysis: Unified Watch Command

## Problem Definition

### Current State

Maproom has TWO separate watch commands:

1. **`watch --repo X --worktree Y --path Z`**
   - Monitors file system changes using `notify` crate
   - Triggers incremental indexing via `incremental_update()`
   - **Bug**: Worktree name is set once at startup and never updates
   - Location: `crates/maproom/src/indexer/mod.rs::watch_worktree()`

2. **`branch-watch --repo Z`**
   - Monitors `.git/HEAD` file for branch switches
   - Triggers full `incremental_update()` on detection
   - **Does not** watch file changes
   - Location: `crates/maproom/src/watcher.rs::BranchWatcher`

### Why This Is Broken

**Scenario**: Developer using `watch` with normal workflow

```bash
# Start watching on main branch
maproom watch --repo myproject --worktree main --path .

# Switch to feature branch
git checkout feature-auth

# Edit files
vim src/auth.rs
# save...

# ❌ PROBLEM: Changes indexed to "main" instead of "feature-auth"!
```

The `watch` command doesn't know the branch switched. It keeps using the original `--worktree main` argument.

### Why It Was Split Originally

Looking at archived project `BRWATCH_branch-switch-detection`:

**Original design philosophy**:
- `watch` = File watching (within a worktree)
- `branch-watch` = Branch switching (across worktrees)
- Separation of concerns

**This made sense for**:
- CLI power users who manually control both
- Server environments running both as separate services
- Testing each component independently

**This breaks down for**:
- VSCode extensions that need single process
- Individual developers who just want to "watch and forget"
- Real-world workflows with frequent branch switching

## Current Implementation Details

### File Watching (`watch` command)

**Entry point**: `crates/maproom/src/main.rs` Commands::Watch

```rust
Commands::Watch {
    repo,
    worktree,
    path,
    throttle,
} => {
    let (repo_name, branch_name, _) = get_git_info(&path)?;
    let repo = repo.unwrap_or(repo_name);
    let worktree = worktree.unwrap_or(branch_name);  // ← Determined ONCE

    indexer::watch_worktree(&client, &repo, &worktree, &path, &throttle).await?;
}
```

**Implementation**: `crates/maproom/src/indexer/mod.rs::watch_worktree()`
- Uses `WorktreeWatcher` which wraps `FileWatcher`
- FileWatcher uses `notify::RecommendedWatcher`
- Events flow: File change → FileEvent → IndexingEvent → IncrementalProcessor
- **Worktree ID is fixed at creation time**

### Branch Watching (`branch-watch` command)

**Entry point**: `crates/maproom/src/main.rs` Commands::BranchWatch

**Implementation**: `crates/maproom/src/watcher.rs::BranchWatcher`

```rust
pub struct BranchWatcher {
    repo_path: PathBuf,
    client: Client,
}

impl BranchWatcher {
    pub async fn start(&mut self, shutdown_rx: Option<Receiver<()>>) -> Result<()> {
        // Watch .git/HEAD
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(&git_head, RecursiveMode::NonRecursive)?;

        // On .git/HEAD change:
        if let Ok(event) = rx.recv() {
            self.handle_branch_switch().await?;
        }
    }

    async fn handle_branch_switch(&mut self) -> Result<()> {
        let current_branch = get_current_branch(&self.repo_path)?;
        let worktree_id = get_or_create_worktree(..., &current_branch, ...).await?;

        // Re-index entire branch
        incremental_update(&self.client, worktree_id, &self.repo_path).await?;
    }
}
```

**Key insight**: `BranchWatcher` already knows how to:
- Detect branch switches
- Get current branch name
- Trigger incremental updates

## Existing Solutions (Industry)

### Watchman (Facebook)
- Single unified file watcher
- Detects directory changes, file changes, git state changes
- Trigger-based actions

**Lesson**: Don't split file watching from git watching

### Jest --watch
- Single watch mode
- Detects both file changes and git branch changes
- Automatically re-runs relevant tests

**Lesson**: Users expect "watch" to mean "watch everything relevant"

### Nodemon
- Watches file changes
- Can detect git branch via custom scripts
- Single unified command

**Lesson**: Additional watchers are opt-in, not required

## Root Cause Analysis

**Why the split happened**:
1. BRANCHX project added worktree-aware indexing
2. BRWATCH project added branch detection
3. Both were implemented as separate concerns
4. No unified "developer watch" use case was considered

**Why it's a problem now**:
1. VSCode extension needs single long-running process
2. Developers expect "watch" to just work
3. Manual orchestration of two processes is error-prone

## Requirements for Unified Solution

### Functional Requirements
1. Single `watch` command
2. Detect file changes (current capability)
3. Detect branch switches (add from `branch-watch`)
4. Update target worktree when branch switches
5. Continue watching after branch switch

### Non-Functional Requirements
1. Performance: No degradation vs current implementation
2. Resource usage: Single watcher process, not two
3. Reliability: Don't miss file changes during branch switch
4. Backward compatibility: Existing `watch` args still work

### Out of Scope
1. Watching multiple branches simultaneously
2. Watching multiple repositories
3. Desktop notifications
4. WebSocket status updates

## Technical Constraints

### Must Preserve
- `WorktreeWatcher` architecture (multi-worktree capable)
- `IncrementalProcessor` pipeline
- NDJSON event output format
- Database schema (no migrations needed)

### Can Modify
- `watch_worktree()` function implementation
- Worktree ID routing logic
- Event handling in watch loop

### Must Add
- `.git/HEAD` file watching alongside file watching
- Branch detection logic
- Worktree update mechanism

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Miss file events during branch switch | High | Medium | Queue events, process after switch |
| Race conditions (file change + branch switch) | Medium | Low | Event debouncing, proper ordering |
| Increased CPU usage (two watchers) | Low | Low | Both use OS events, minimal overhead |
| Breaking existing `watch` usage | High | Low | Maintain backward compatibility |
| Complexity from merging two systems | Medium | Medium | Keep separation internally, unify interface |

## Success Criteria

### Must Have
- [ ] Single `watch` command detects file changes
- [ ] Single `watch` command detects branch switches
- [ ] File changes after switch indexed to correct worktree
- [ ] No manual restart needed on branch switch
- [ ] Backward compatible with existing flags

### Nice to Have
- [ ] NDJSON events for branch switches
- [ ] Graceful handling of rapid branch switches
- [ ] Deprecation warning for `branch-watch` command

## Research Findings

### From BRANCHX Project
- `incremental_update()` is fast enough for branch switches (<1 minute typical)
- Tree SHA optimization makes "no changes" instant (<100ms)
- Worktree tracking already in database schema

### From BRWATCH Project
- `.git/HEAD` watching is lightweight (<1% CPU)
- `notify` crate handles file watching efficiently
- Debouncing prevents duplicate events

### Key Insight
**We don't need two commands.** The `watch` command can monitor both:
1. Repository files (for changes)
2. `.git/HEAD` file (for branch switches)

Both use the same underlying `notify` watcher infrastructure.

## Recommended Approach

**Extend `watch` command to:**
1. Start two file watchers:
   - Main watcher: Repository files
   - HEAD watcher: `.git/HEAD` file
2. When `.git/HEAD` changes:
   - Detect new branch name
   - Get/create worktree ID
   - Update routing so future file events go to new worktree
   - Trigger incremental update for new branch
3. Continue watching files as normal

**Implementation location**: Modify `crates/maproom/src/indexer/mod.rs::watch_worktree()`

**Preserve**: All of `BranchWatcher` logic for branch detection

**Result**: Single command, zero manual intervention
