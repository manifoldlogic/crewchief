# Project: Incremental Scan Completion

**Status:** Ready for Implementation
**Priority:** P0 (Blocking genetic optimizer)
**Complexity:** Low
**Timeline:** 1-2 days

## Problem Statement

The Maproom indexer's incremental scanning feature is incomplete, causing severe performance degradation. When scanning worktrees with identical code (same git tree SHA), the system unnecessarily updates ~474K chunks, taking 2-3 hours instead of seconds.

**Root Cause:** The `worktree_index_state` table is never populated after scans complete, so every scan is treated as first-time indexing.

**Impact:**
- Genetic optimizer unusable (24+ hours for 12 worktrees)
- Wasted API costs for redundant embeddings
- Poor developer experience

## Proposed Solution

Add tree SHA checking and state persistence to the scan command—a surgical fix to existing infrastructure.

**Approach:**
1. Get git tree SHA before scanning
2. Query `worktree_index_state` for last indexed SHA
3. If unchanged (and not `--force`): skip scan, return immediately
4. After scan completes: update state with new SHA and stats
5. Handle all errors gracefully (fallback to full scan)

**Expected Improvement:** 10,000x speedup for unchanged worktrees (2-3 hours → 5-10ms)

## Architecture Highlights

### Minimal Changes
- Touch only scan command handler in `main.rs`
- Use existing functions (`get_git_tree_sha`, `update_index_state`)
- No schema changes (table exists from migration 0020)

### Fail-Safe Design
- Any error → fallback to full scan (never skip incorrectly)
- State update errors are non-fatal (logged only)
- Backward compatible (existing behavior preserved with `--force`)

### Observable
- Clear logging for every decision (skip, scan, fallback)
- Metrics tracked (files/chunks processed)
- Debug information for troubleshooting

## Scope

### In Scope
- ✅ Tree SHA check before scan
- ✅ Skip logic for unchanged worktrees
- ✅ State persistence after scan
- ✅ Error handling (safe fallbacks)
- ✅ Integration tests for all modes
- ✅ Manual validation (genetic optimizer)

### Out of Scope
- ❌ True incremental scanning (`git diff-tree` integration)
- ❌ Refactoring `scan_worktree()` for pluggable discovery
- ❌ Parallel tree SHA checks
- ❌ Remote state caching

**Rationale:** Ship minimal fix now (10,000x speedup), defer complex refactoring until we have real-world usage data.

## Success Criteria

1. **Unchanged worktrees skip scanning** (< 1 second check)
2. **Changed worktrees process correctly** (full scan as before)
3. **Force flag works** (overrides skip logic)
4. **State persists** (`worktree_index_state` populated)
5. **Genetic optimizer completes** (12 worktrees < 2 minutes)
6. **Zero false skips** (correctness maintained)

## Tickets

| ID | Description | Priority | Estimate |
|----|-------------|----------|----------|
| INCRSCAN-1001 | Add tree SHA check and skip logic | P0 | 2-3h |
| INCRSCAN-1002 | Add state persistence after scan | P0 | 1-2h |
| INCRSCAN-1003 | Create integration tests | P0 | 2-3h |
| INCRSCAN-1004 | Create error handling tests | P1 | 1-2h |
| INCRSCAN-1005 | Manual validation (genetic optimizer) | P0 | 30m |
| INCRSCAN-1006 | Documentation and changelog | P1 | 1h |

**Total Estimate:** 8-12 hours (1-2 days)

## Agents

**Primary:** `rust-indexer-engineer`
- Implements tree SHA check and state persistence
- Adds error handling
- Updates documentation

**Secondary:** `integration-tester`
- Creates comprehensive test suite
- Validates all scan modes
- Tests error conditions

**Validation:** Manual (genetic optimizer execution)

## Planning Documents

Comprehensive planning completed:

1. **[analysis.md](planning/analysis.md)**
   - Problem definition and root cause analysis
   - Current state assessment
   - Industry solutions research
   - Performance impact analysis

2. **[architecture.md](planning/architecture.md)**
   - System design (before/after flows)
   - Component integration points
   - Data flow and decision logic
   - Error handling strategy

3. **[quality-strategy.md](planning/quality-strategy.md)**
   - Risk-based testing approach
   - Integration test plan
   - Manual validation procedures
   - Performance benchmarks

4. **[security-review.md](planning/security-review.md)**
   - Threat model and attack surface
   - Security analysis by component
   - Mitigation strategies
   - Sign-off and approval

5. **[plan.md](planning/plan.md)**
   - Phased execution plan
   - Ticket breakdown with acceptance criteria
   - Dependencies and timeline
   - Risk management and rollback

## Quick Start

**For Implementer:**
1. Read `planning/architecture.md` (design overview)
2. Start with `INCRSCAN-1001` (tree SHA check)
3. Then `INCRSCAN-1002` (state persistence)
4. Run tests (`INCRSCAN-1003`, `INCRSCAN-1004`)
5. Validate with genetic optimizer (`INCRSCAN-1005`)

**For Reviewer:**
1. Check integration tests pass
2. Run genetic optimizer (verify < 2 min)
3. Inspect state table after scan
4. Review error handling paths

## Key Insights

### Why This is Simple

**Existing Infrastructure:**
- ✅ `worktree_index_state` table (created)
- ✅ `get_last_indexed_tree()` function (tested)
- ✅ `update_index_state()` function (tested)
- ✅ `get_git_tree_sha()` function (exists)

**What's Missing:**
- ❌ Calling these functions from scan command

**Solution:** 20 lines of glue code in the right place.

### Why Not the Full Refactor?

**INCREMENTAL_INTEGRATION_NOTE.md proposed:**
- Refactor `scan_worktree()` for pluggable file discovery
- Integrate `git diff-tree` for changed file detection
- Adapt progress tracking for dynamic lists
- Complex architectural changes

**This Project Instead:**
- Add tree SHA check at command level (skip entire scan if unchanged)
- Same correctness, 90% of performance benefit
- 8 hours vs weeks of work
- Ship value now, defer complexity

### Performance Expectations

| Scenario | Current | After | Improvement |
|----------|---------|-------|-------------|
| Unchanged worktree | 2-3 hours | 5-10ms | 720,000x |
| Changed worktree | Variable | Same + 10ms | Negligible |
| First-time scan | Variable | Same + 10ms | Negligible |
| Genetic optimizer (12 worktrees) | 24+ hours | < 2 minutes | 720x |

## Risk Assessment

**Low Risk Project:**
- Uses existing tested functions
- Safe fallback on any error (full scan)
- No schema changes
- Backward compatible
- Easy rollback (just revert commits)

**Primary Risk:** False skips (correctness issue)

**Mitigation:** Extensive testing, fail-safe defaults, manual validation

## Future Enhancements

**Phase 2 (Separate Project):**
- Integrate `git diff-tree` for true incremental (process only changed files)
- Refactor scan pipeline for pluggable file discovery
- Achieve 100x speedup for small changes (vs current 10,000x for no changes)

**Phase 3 (Separate Project):**
- Remote state caching (distributed teams)
- Predictive indexing (index on branch create)
- Smart embedding cache (reuse across branches)

## Go-Live Checklist

- [ ] All P0 tests pass
- [ ] Manual validation successful (genetic optimizer < 2 min)
- [ ] Code reviewed
- [ ] Documentation complete
- [ ] CHANGELOG updated
- [ ] Merged to main
- [ ] Binaries rebuilt

## Questions?

**For architecture questions:** See `planning/architecture.md`
**For testing approach:** See `planning/quality-strategy.md`
**For security concerns:** See `planning/security-review.md`
**For implementation plan:** See `planning/plan.md`

---

**Ready to implement. No blockers. Let's ship it.**
