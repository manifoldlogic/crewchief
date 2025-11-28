# UNIWATCH: Unified Watch Command

**Slug**: UNIWATCH
**Status**: Ready for Implementation (Updated 2025-01-28)
**Scope**: Add runtime branch detection to existing watch command

## Updates from Project Review

Planning documents updated to reflect current codebase state:
- **Phase 0 added**: Module exports required before implementation
- **handle_branch_switch()**: Must be newly implemented (was removed in IDXABS-2001)
- **Tests**: 4 disabled PostgreSQL tests identified for SQLite migration
- **E2E script**: Migration from PostgreSQL to SQLite documented

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

Components in `indexer/mod.rs` that exist but need export (Phase 0):

| Component | Location | Status |
|-----------|----------|--------|
| `setup_head_watcher()` | indexer/mod.rs:668 | **Needs `pub` export** |
| `DebouncedHandler` | indexer/mod.rs:33 | **Needs `pub` export** |
| `BranchSwitchEvent` | indexer/mod.rs:112 | **Needs `pub` export** |

Components already public and ready to use:

| Component | Location | Purpose |
|-----------|----------|---------|
| `get_current_branch()` | git/mod.rs | Reads current branch |
| `incremental_update()` | incremental/mod.rs | Re-indexes worktree files |
| `get_or_create_worktree()` | db/sqlite/mod.rs | Database worktree management |

## Success Criteria

- [ ] Branch switches detected within 2 seconds
- [ ] File changes after switch index to correct worktree
- [ ] Rapid switches (< 2s) are debounced
- [ ] BranchSwitchEvent NDJSON emitted
- [ ] No regressions to file watching
- [ ] All tests pass

## Agents

| Phase | Agent | Notes |
|-------|-------|-------|
| Phase 0 (Exports) | rust-indexer-engineer | Simple visibility changes |
| Phases 1-3 (Implementation) | rust-indexer-engineer | Core feature work |
| Phase 4 (Enable tests) | rust-indexer-engineer | SQLite test migration |
| Phase 4 (Integration Tests) | integration-tester | New integration tests |
| Verification | verify-ticket | Manual testing checklist |
| Commit | commit-ticket | After verification passes |

## Next Steps

1. Run `/create-project-tickets UNIWATCH` to generate tickets
2. Execute tickets sequentially using rust-indexer-engineer
3. Verify with integration-tester
4. Manual testing before merge
