# UNIWATCH: Unified Watch Command

**Slug**: UNIWATCH
**Status**: Ready for Implementation
**Scope**: Add runtime branch detection to existing watch command

## Problem

The `maproom watch` command detects the current branch at **startup**, but does not detect branch switches at **runtime**. When users run `git checkout feature`, the watch command continues indexing files to the original worktree.

**Before** (current behavior):
```bash
maproom watch                    # Auto-detects "main" at startup
# ... user runs: git checkout feature
# ... user edits src/auth.rs
# Files indexed to "main" worktree ❌ (should be "feature")
```

**After** (target behavior):
```bash
maproom watch                    # Auto-detects "main" at startup
# ... user runs: git checkout feature
# Watch detects switch, updates worktree_id
# ... user edits src/auth.rs
# Files indexed to "feature" worktree ✓
```

## Solution

Add `.git/HEAD` file watching to the existing `tokio::select!` event loop. When HEAD changes:
1. Detect new branch name
2. Update worktree_id dynamically
3. Re-index the new branch
4. Emit NDJSON event for VSCode extension

## Scope

### In Scope
- Runtime branch switch detection
- Dynamic worktree_id tracking
- NDJSON events for branch switches
- Debouncing rapid switches
- Integration tests

### Out of Scope
- Multi-repository watching
- New CLI commands
- Database schema changes
- Performance optimizations

## Planning Documents

| Document | Purpose |
|----------|---------|
| [plan.md](planning/plan.md) | Implementation phases and tasks |
| [architecture.md](planning/architecture.md) | Technical design and code structure |
| [quality-strategy.md](planning/quality-strategy.md) | Test strategy and acceptance criteria |

## Key Files

| File | Role |
|------|------|
| `crates/maproom/src/main.rs:1105-1216` | Watch command handler (modify) |
| `crates/maproom/src/indexer/mod.rs` | Contains reusable components |
| `crates/maproom/tests/unified_watch_test.rs` | Integration tests (create) |

## Existing Components to Reuse

| Component | Location | Purpose |
|-----------|----------|---------|
| `setup_head_watcher()` | indexer/mod.rs:668 | Creates .git/HEAD watcher |
| `DebouncedHandler` | indexer/mod.rs:33 | Rate limits rapid events |
| `BranchSwitchEvent` | indexer/mod.rs:112 | NDJSON event struct |
| `get_current_branch()` | git/mod.rs | Reads current branch |

## Success Criteria

- [ ] Branch switches detected within 2 seconds
- [ ] File changes after switch index to correct worktree
- [ ] Rapid switches (< 2s) are debounced
- [ ] BranchSwitchEvent NDJSON emitted
- [ ] No regressions to file watching
- [ ] All tests pass

## Agents

| Phase | Agent |
|-------|-------|
| Implementation | rust-indexer-engineer |
| Integration Tests | integration-tester |
| Verification | verify-ticket |
| Commit | commit-ticket |

## Next Steps

1. Run `/create-project-tickets UNIWATCH` to generate tickets
2. Execute tickets sequentially using rust-indexer-engineer
3. Verify with integration-tester
4. Manual testing before merge
