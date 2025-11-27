# Architecture Revision Summary

**Note**: This document explains the architectural evolution that occurred during planning. The current planning documents (architecture.md, plan.md) reflect the simplified approach described below.

## What Changed During Planning

After reviewing the initial architecture proposal and examining the existing codebase in detail, we identified a **significantly simpler approach** that achieves the same goals with less code and lower risk.

## Key Discovery

The existing `watch_worktree()` function in `crates/maproom/src/indexer/mod.rs` already contains ~80% of the infrastructure we need:
- WorktreeWatcher for file monitoring
- Event processing loop
- IncrementalProcessor integration
- Database connection pool
- Error handling and recovery

**We don't need to create a new UnifiedWatcher component** - we just need to add .git/HEAD watching to the existing event loop.

## Comparison

### Original Architecture
- **New struct**: UnifiedWatcher (300-400 lines)
- **New module**: unified_watch.rs
- **Approach**: Create new component, wire to CLI
- **Integration**: Replace watch_worktree() function
- **Testing**: Test new component from scratch

### Revised Architecture
- **New struct**: None
- **New module**: None
- **Approach**: Modify existing watch_worktree() function (~150 lines)
- **Integration**: No API change, just internal modifications
- **Testing**: Test modifications to proven code

## Impact Metrics

| Metric | Original | Revised | Improvement |
|--------|----------|---------|-------------|
| **Lines of Code** | 400+ | 295 | 26% reduction |
| **New Components** | 2 structs, 1 module | 0 | Much simpler |
| **Timeline** | 2-3 days | 1-2 days | 33% faster |
| **Tickets** | 17 | 15 | 12% fewer |
| **Risk Level** | Medium | Low | Lower risk |
| **Integration Complexity** | New function | Modify existing | Simpler |

## Technical Approach

### What We're Adding

1. **setup_head_watcher()** function (~30 lines)
   - Creates notify watcher for .git/HEAD
   - Bridges sync to async channels
   - Returns watcher handle

2. **handle_branch_switch()** function (~40 lines)
   - Extracts branch name
   - Updates database worktree record
   - Updates shared state (Arc<RwLock>)
   - Triggers incremental_update()
   - Emits NDJSON event

3. **Dynamic worktree tracking** (~15 lines)
   - Arc<RwLock<String>> for current_branch
   - Arc<RwLock<i32>> for current_worktree_id
   - Thread-safe state sharing

4. **Modified event loop** (~40 lines)
   - Change while-let to tokio::select!
   - Handle both file and HEAD events
   - Read dynamic worktree_id

5. **CLI updates** (~20 lines)
   - Auto-detect branch if --worktree not provided
   - Deprecation warning for --worktree flag

**Total: ~145 new lines + ~100 modified lines = ~245 lines of actual code**

### What We're Reusing

- WorktreeWatcher (proven component)
- Event processing pipeline (proven)
- Database queries (proven, parameterized)
- Error handling patterns (proven)
- Shutdown coordination (proven)
- NDJSON event infrastructure (proven)

## Why This is Better

### 1. Less Code = Fewer Bugs
26% less code means 26% fewer places for bugs to hide.

### 2. Reuses Proven Infrastructure
All the hard problems (file watching, event processing, database, error handling) are already solved and tested.

### 3. Simpler Review
Changes visible in unified diff instead of entirely new component.

### 4. Backward Compatible
Function signature doesn't change - existing usage continues to work.

### 5. Faster Implementation
1-2 days instead of 2-3 days due to less code to write and test.

### 6. Lower Risk
Modifying existing code is lower risk than creating new parallel infrastructure.

### 7. Easier Rollback
Just revert function modifications instead of removing entire new module.

## Files Modified

### Primary Changes
1. **`crates/maproom/src/indexer/mod.rs`**
   - Add setup_head_watcher() function
   - Add handle_branch_switch() function
   - Modify watch_worktree() initialization
   - Modify event loop to use tokio::select!
   - Update event processing to use dynamic worktree_id
   - **Total: ~200 lines modified/added**

### Secondary Changes
2. **`crates/maproom/src/main.rs`**
   - Auto-detect branch in Commands::Watch
   - Add deprecation warnings
   - **Total: ~20 lines modified**

3. **`crates/maproom/CLAUDE.md`**
   - Update watch command documentation
   - Document NDJSON events
   - **Total: ~50 lines modified**

**No new files required!**

## Testing Impact

### Fewer Tests Needed
- Original: 17 tests (10 unit, 5 integration, 2 E2E)
- Revised: 15 tests (8 unit, 4 integration, 1 E2E, 1 manual)

### Simpler Test Cases
- No need to test new component initialization
- No need to test component integration
- Just test the modifications to existing code

### Same Coverage
Despite fewer tests, we achieve the same critical path coverage because we're testing modifications to proven code rather than entirely new infrastructure.

## Architecture Decision Record

**Decision**: Adopt simplified approach (modify watch_worktree) over original approach (create UnifiedWatcher)

**Context**:
- Original plan proposed new UnifiedWatcher component
- Code review revealed existing watch_worktree() has most needed infrastructure
- Creating new component adds unnecessary complexity

**Consequences**:
- ✅ 26% less code to write and maintain
- ✅ 33% faster implementation timeline
- ✅ Lower risk (modifying existing vs creating new)
- ✅ Simpler testing strategy
- ✅ Backward compatible (no API changes)
- ✅ Easier code review process
- ⚠️ watch_worktree() function becomes slightly longer (~200 lines vs ~150)
- ⚠️ Less separation of concerns (but acceptable for this refactoring)

**Status**: Approved - proceed with simplified approach

## Current State

All planning documents have been updated to reflect the simplified approach:

- **architecture.md** - Contains the simplified approach (modify watch_worktree)
- **plan.md** - Contains the 15-ticket implementation timeline
- **quality-strategy.md** - Remains valid (test counts slightly reduced)
- **security-review.md** - Remains valid (no new attack surfaces)

## Next Steps

1. ✅ Identified simpler approach through code review
2. ✅ Updated architecture.md with simplified design
3. ✅ Updated plan.md with 15-ticket timeline
4. ✅ Updated README.md to reflect simplified approach
5. ⏳ Run `/create-project-tickets UNIWATCH` to generate tickets
6. ⏳ Implement using simplified approach
7. ⏳ Verify using same acceptance criteria

## Acknowledgment

This revision was identified through the `/review-project` process, which caught the architectural complexity before implementation began. This demonstrates the value of critical review before execution.

**Time saved by catching this early**: ~1 day of implementation + ~2 hours of testing = ~10 hours

**Technical debt avoided**: ~150 lines of unnecessary infrastructure that would need long-term maintenance

## Questions?

See:
- **architecture-revised.md** for detailed technical design
- **plan-revised.md** for updated implementation timeline
- **project-review.md** for the analysis that led to this revision
