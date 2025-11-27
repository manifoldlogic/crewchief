# UNIWATCH: Unified Watch Command

**Status**: Blocked - Depends on IDXABS-6003
**Slug**: UNIWATCH
**Timeline**: 1-2 days (after IDXABS completes)
**Scope**: Minimal - modify existing watch_worktree(), no new components
**Blocked By**: IDXABS-6003 (watch command must work first)

## Problem Statement

Currently, maproom requires TWO separate commands to watch a repository effectively:

1. **`watch`** - Watches file changes, but hardcodes the worktree at startup
2. **`branch-watch`** - Detects branch switches and re-indexes

**The problem**: When you switch branches, `watch` keeps indexing to the old branch name. This is unusable for real-world development where developers frequently switch branches.

**Root cause**: The commands were designed for CLI power users who manually orchestrate both processes, not for VSCode extensions or developers who want "watch and forget" behavior.

## Proposed Solution

**Create a single unified `watch` command that:**
- Watches file changes for incremental indexing (existing capability)
- Detects git branch switches automatically (from `branch-watch`)
- Updates the target worktree dynamically when branches change (new glue)
- Eliminates the need for running `branch-watch` separately

**Architecture**: Reuse existing components, add event router for worktree tracking.

## Developer Experience

**Before** (broken):
```bash
# Terminal 1
maproom watch --repo myproject --worktree main

# Terminal 2
maproom branch-watch --repo .

# Problem: If you switch branches, watch still indexes to "main"
```

**After** (unified):
```bash
# Single terminal
maproom watch

# Zero manual intervention:
git checkout feature-auth  # ✓ Detected, re-indexed
vim src/auth.rs           # ✓ Changes indexed to feature-auth
git checkout main          # ✓ Detected, switched back
vim README.md              # ✓ Changes indexed to main
```

**Result**: Zero manual commands. Just code.

## Success Metrics

### Functional
- Single `watch` command handles both file changes and branch switches
- Branch switches detected within 1-2 seconds
- Worktree updates automatically (no manual restart)
- File changes after switch go to correct worktree
- Backward compatible with existing `watch` usage

### Non-Functional
- CPU usage <2% idle (minimal overhead vs current)
- Memory usage <20MB (vs ~35MB for two processes)
- No file events lost during branch transitions
- Graceful error recovery

## Project Scope

### In Scope
✅ Unify `watch` and `branch-watch` into single command
✅ Auto-detect current branch (no `--worktree` flag needed)
✅ Thread-safe worktree tracking during transitions
✅ Backward compatibility (existing flags still work)
✅ Deprecation warning for `branch-watch` command
✅ Comprehensive test suite
✅ Documentation updates

### Out of Scope
❌ Performance improvements (reuse existing)
❌ New features (notifications, progress bars, WebSocket API)
❌ Multi-repository watching
❌ VSCode extension changes (separate project)
❌ Database schema changes

## Planning Documents

### Core Planning
- **[analysis.md](planning/analysis.md)** - Problem analysis, current implementation, industry solutions
- **[architecture.md](planning/architecture.md)** - Simplified approach modifying existing watch_worktree()
- **[quality-strategy.md](planning/quality-strategy.md)** - Test pyramid, coverage goals, acceptance criteria
- **[security-review.md](planning/security-review.md)** - Security assessment (no new risks introduced)
- **[plan.md](planning/plan.md)** - 5-phase implementation timeline (1-2 days)
- **[project-review.md](planning/project-review.md)** - Critical review that led to architectural simplification
- **[REVISION_SUMMARY.md](planning/REVISION_SUMMARY.md)** - Why we simplified from UnifiedWatcher to modified watch_worktree()

### Key Insights from Planning

**From Analysis**:
- Current split was design choice for separation of concerns
- Breaks down for integrated tools (VSCode) and "watch and forget" workflows
- Both commands use same underlying `notify` crate infrastructure

**From Architecture**:
- ✅ **Modify existing watch_worktree()** instead of creating new infrastructure
- ✅ **~150 lines of changes** vs 400+ lines of new code
- ✅ **Add .git/HEAD watching** to existing event loop
- ✅ **Dynamic worktree tracking** with Arc<RwLock> for thread safety
- ✅ **tokio::select!** for event multiplexing in single loop
- ✅ **NDJSON events** for VSCode extension compatibility

**From Quality Strategy**:
- 15 tests total: 8 unit, 4 integration, 1 E2E, 1 manual
- Focus on critical paths (branch switch workflow, race conditions)
- Manual testing checklist for real-world validation

**From Security Review**:
- ✅ No new attack surfaces introduced
- ✅ Reuses existing security-reviewed components
- ✅ Proper error handling, thread safety, parameterized queries
- ✅ Approved for release (refactoring only)

**From Implementation Plan**:
- 5 phases over **1-2 days**
- **15 tickets**
- **~295 lines total** (modifications + new code)
- Simpler integration, lower risk
- Rollback plan: Revert function modifications if issues found

**From Project Review**:
- Critical review identified simpler approach
- Chose to modify existing code vs creating new infrastructure
- 26% less code, 33% faster timeline, lower risk vs original proposal

## Relevant Agents

### Primary Implementation
- **rust-indexer-engineer** - Implement UnifiedWatcher, CLI integration, documentation

### Testing & Verification
- **unit-test-runner** - Execute test suite, report failures (no fixes)
- **integration-tester** - Create and run E2E tests (bash scripts)
- **verify-ticket** - Manual testing, acceptance criteria verification

### Process
- **commit-ticket** - Create conventional commits for completed work

## Next Steps

1. **Review planning documents** for completeness
2. **Generate tickets**: Run `/create-project-tickets UNIWATCH`
3. **Execute sequentially**: Foundation → Logic → Event Loop → CLI → Testing
4. **Manual testing** before merge
5. **Update VSCode extension** to use unified command (separate project)

## Dependencies

**Project Dependencies**:
- **IDXABS-6003** - The basic watch command must be implemented for SQLite before UNIWATCH can enhance it

**Required** (existing):
- `notify` crate - File watching
- `tokio` - Async runtime
- `SqliteStore` - Database (PostgreSQL removed)
- Existing `IncrementalProcessor` component (must be implemented in IDXABS-6002)

**No new dependencies required** ✅

## Prerequisite Work (IDXABS Project)

Before starting UNIWATCH tickets, the following IDXABS tickets must be complete:

1. **IDXABS-6001** - Migrate tests to SQLite (validation capability)
2. **IDXABS-6002** - Implement incremental module (core functionality)
3. **IDXABS-6003** - Implement basic watch command (foundation for UNIWATCH)

**Current IDXABS Status**: The incremental module is stubbed (functions return Ok/empty values). The watch command prints an error. Tests don't compile due to PostgreSQL references.

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Race conditions (file + branch) | Medium | High | RwLock synchronization, integration tests |
| File events lost during switch | Low | High | Event queueing, proper channel handling |
| Breaking existing usage | Low | High | Backward compatibility, deprecation warnings |
| Complexity from merging | Medium | Medium | Keep internal separation, unify only interface |

## Acceptance Criteria

**Ready to ship when**:
- [ ] All unit tests pass (10 tests)
- [ ] All integration tests pass (5 tests)
- [ ] E2E tests pass (2 bash scripts)
- [ ] Manual testing checklist complete
- [ ] No clippy warnings
- [ ] Security review passed
- [ ] Documentation updated
- [ ] Backward compatibility verified
- [ ] VSCode extension tested with new command

**Timeline**: 1-2 days from ticket generation to merge
