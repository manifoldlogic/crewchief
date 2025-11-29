# Analysis: Unified Watch Command

## Problem Statement

The `maproom watch` command auto-detects the current branch at startup, but does not detect branch switches at runtime. This causes files to be indexed to the wrong worktree after a `git checkout`.

## Current Behavior

```bash
maproom watch                    # Detects "main" at startup
# worktree_id = 1 (set once, never changes)

git checkout feature             # User switches branches
vim src/auth.rs                  # User edits file
# File indexed to worktree_id=1 (main) ❌
```

**Root Cause**: `worktree_id` is set at line 1149 of `main.rs` and passed to `incremental_update()` without ever being updated.

## Target Behavior

```bash
maproom watch                    # Detects "main" at startup
# worktree_id = Arc<RwLock<1>>

git checkout feature             # User switches branches
# .git/HEAD change detected
# worktree_id updated to 42 (feature)

vim src/auth.rs                  # User edits file
# File indexed to worktree_id=42 (feature) ✓
```

## Why This Matters

1. **VSCode Extension**: Needs single long-running process that handles both file and branch changes
2. **Developer Experience**: Users expect "watch" to just work without manual intervention
3. **Data Correctness**: Files indexed to wrong worktree pollute search results

## Technical Approach

Add `.git/HEAD` file watching to the existing `tokio::select!` event loop:

1. **Dynamic State**: Wrap `worktree_id` in `Arc<RwLock<i64>>`
2. **HEAD Watcher**: Use existing `setup_head_watcher()` from indexer module
3. **Event Handler**: Add branch for HEAD events in select! loop
4. **Branch Switch**: Detect change, update state, re-index, emit NDJSON

## Constraints

### Must Preserve
- Existing file watching behavior
- NDJSON event format
- CLI backward compatibility (--worktree flag still works)
- Database schema (no migrations)

### Can Modify
- main.rs Commands::Watch handler
- Event loop structure
- worktree_id storage

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Race condition (file + branch) | Medium | High | Event queuing, proper ordering |
| File events lost during switch | Low | High | Process queued events after switch |
| Rapid switches thrash index | Medium | Low | Debouncing (2 second window) |
| Breaking existing usage | Low | High | Backward compatible, deprecation warnings |

## Success Metrics

- Branch switch detection: < 2 seconds
- No file events lost during transition
- No regressions to existing file watching
- All tests pass
