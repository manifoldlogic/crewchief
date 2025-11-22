# WTSRCH Ticket Index

**Project:** Worktree-Scoped Search
**Status:** Ready for Implementation
**Total Tickets:** 5
**Estimated Total Effort:** 12-18 hours (3-4 working days)

---

## Ticket Execution Order

Execute tickets **sequentially** in dependency order:

```
WTSRCH-1001 → WTSRCH-2001 → WTSRCH-3001 → WTSRCH-4001 → WTSRCH-5001
   (Day 1)       (Day 2)       (Day 3)       (Day 4)       (Day 5)
```

---

## Phase 1: Git Utilities Enhancement

### WTSRCH-1001: Implement Git Branch Detection with Caching
**Status:** 📋 Ready
**Agent:** typescript-engineer
**Effort:** 2-3 hours
**Dependencies:** None

**Deliverables:**
- Install `lru-cache` dependency (version ^10.0.0)
- `getCurrentBranch()` function in `src/utils/git.ts`
- LRU cache with 60s TTL for branch detection
- Unit tests in `tests/unit/git.test.ts`

**Acceptance Criteria:**
- [x] Dependency installed in package.json
- [x] Function returns correct branch name
- [x] Handles detached HEAD gracefully
- [x] Throws clear error when not in git repo
- [x] Cache reduces subprocess calls by >95%
- [x] All unit tests passing

**Files Modified:**
- `packages/maproom-mcp/package.json`
- `packages/maproom-mcp/src/utils/git.ts`
- `packages/maproom-mcp/tests/unit/git.test.ts` (NEW)

**Plan Reference:** Phase 1 (plan.md:14-47)

---

## Phase 2: Worktree Resolution Logic

### WTSRCH-2001: Implement Three-Tier Worktree Resolution Logic
**Status:** 📋 Ready (blocked by WTSRCH-1001)
**Agent:** typescript-nodejs-specialist
**Effort:** 3-4 hours
**Dependencies:** WTSRCH-1001 ✅

**Deliverables:**
- `resolveWorktreeId()` function with four-tier resolution
- `lookupWorktreeId()` function with database query and caching
- LRU cache with 5-minute TTL for worktree IDs
- Unit tests in `tests/unit/worktree-resolution.test.ts`

**Acceptance Criteria:**
- [x] Tier 1 (Explicit): Uses provided parameter
- [x] Tier 2 (Auto-detect): Calls getCurrentBranch()
- [x] Tier 3 (Fallback): Falls back to main
- [x] Tier 4 (Last resort): Falls back to null (all)
- [x] Database queries use parameterized statements
- [x] Cache implemented with 5-minute TTL
- [x] Clear error messages for each failure mode
- [x] Metadata includes mode, detected_branch, fallback info
- [x] All unit tests passing

**Files Modified:**
- `packages/maproom-mcp/src/index.ts`
- `packages/maproom-mcp/tests/unit/worktree-resolution.test.ts` (NEW)

**Plan Reference:** Phase 2 (plan.md:49-83)

---

## Phase 3: Search Integration

### WTSRCH-3001: Integrate Worktree Resolution into Search Tool Handler
**Status:** 📋 Ready (blocked by WTSRCH-2001)
**Agent:** typescript-mcp-engineer
**Effort:** 2-3 hours
**Dependencies:** WTSRCH-1001 ✅, WTSRCH-2001 ✅

**Deliverables:**
- Integration of `resolveWorktreeId()` into search handler
- Search result metadata (auto_detected, worktree, mode, hint)
- Helpful hint messages for fallback scenarios
- Integration tests in `tests/integration/worktree-scoping.test.ts`

**Acceptance Criteria:**
- [x] resolveWorktreeId() called before search execution
- [x] Undefined worktree parameter triggers auto-detection
- [x] Explicit parameter bypasses auto-detection
- [x] Resolved worktree ID passed to search executors
- [x] Metadata added to search results
- [x] Helpful hints for branch not indexed scenario
- [x] Integration tests passing
- [x] Existing tests continue to pass
- [x] No breaking changes to MCP schema

**Files Modified:**
- `packages/maproom-mcp/src/index.ts`
- `packages/maproom-mcp/tests/integration/worktree-scoping.test.ts` (NEW)

**Plan Reference:** Phase 3 (plan.md:85-119)

---

## Phase 4: Testing and Validation

### WTSRCH-4001: Comprehensive Testing and Validation
**Status:** 📋 Ready (blocked by WTSRCH-3001)
**Agent:** typescript-specialist (testing)
**Effort:** 4-6 hours
**Dependencies:** WTSRCH-1001 ✅, WTSRCH-2001 ✅, WTSRCH-3001 ✅

**Deliverables:**
- Test fixtures (database SQL, git repository setup)
- Integration test suite (8 core scenarios)
- Performance benchmarks (cache hit rate, latency, memory)
- Manual testing report (Linux + macOS, 10 scenarios)
- Bug fixes for any issues found

**Acceptance Criteria:**
- [x] Test fixtures created (database + git repo)
- [x] All unit tests passing
- [x] All integration tests passing
- [x] Happy path verified (auto-detection works)
- [x] Fallback verified (branch not indexed)
- [x] Explicit override verified
- [x] Search all verified (worktree: null)
- [x] Performance: <50ms latency, >95% cache hit rate
- [x] Manual testing complete (Linux + macOS)
- [x] Existing tests pass (zero breaking changes)
- [x] Bug fixes completed

**Files Modified:**
- `packages/maproom-mcp/tests/fixtures/` (NEW)
- `packages/maproom-mcp/tests/integration/worktree-scoping.test.ts`
- `packages/maproom-mcp/tests/unit/git.test.ts`
- `packages/maproom-mcp/tests/unit/worktree-resolution.test.ts`

**Plan Reference:** Phase 4 (plan.md:122-164)

---

## Phase 5: Documentation and Release

### WTSRCH-5001: Documentation and Release Preparation
**Status:** 📋 Ready (blocked by WTSRCH-4001)
**Agent:** documentation-specialist
**Effort:** 1-2 hours
**Dependencies:** WTSRCH-1001 ✅, WTSRCH-2001 ✅, WTSRCH-3001 ✅, WTSRCH-4001 ✅

**Deliverables:**
- Updated MCP tool documentation (search tool)
- CHANGELOG entry for v2.1.0
- Security checklist completion
- GitHub PR description
- Ready for code review and merge

**Acceptance Criteria:**
- [x] MCP tool documentation updated
- [x] Examples added (auto-detect, override, search all)
- [x] Troubleshooting section referenced
- [x] CHANGELOG.md updated (v2.1.0)
- [x] Security checklist completed
- [x] Code review ready
- [x] All CI/CD checks passing
- [x] Ready for merge to main

**Files Modified:**
- `packages/maproom-mcp/README.md`
- `packages/maproom-mcp/CHANGELOG.md`
- `.agents/projects/WTSRCH_worktree-scoped-search/planning/security-review.md`

**Plan Reference:** Phase 5 (plan.md:166-194)

---

## Dependency Graph

```
WTSRCH-1001 (Git Branch Detection)
    ↓
WTSRCH-2001 (Worktree Resolution) ← depends on 1001
    ↓
WTSRCH-3001 (Search Integration) ← depends on 1001, 2001
    ↓
WTSRCH-4001 (Testing) ← depends on 1001, 2001, 3001
    ↓
WTSRCH-5001 (Documentation) ← depends on 1001, 2001, 3001, 4001
```

**Critical Path:** All tickets are on the critical path (sequential dependencies)

---

## Ticket Workflow

For each ticket:

1. **Implementation** - Assign to primary agent, complete work
2. **Unit Testing** - Run unit tests (if applicable)
3. **Verification** - Use verify-ticket agent to check acceptance criteria
4. **Commit** - Use commit-ticket agent to create proper commit

**Commands:**
```bash
# Work on single ticket
/single-ticket WTSRCH-1001

# Or work through all tickets sequentially
/work-on-project WTSRCH
```

---

## Success Metrics

### Functional
- ✅ Search defaults to current worktree
- ✅ Explicit parameter works (backward compatible)
- ✅ Fallback works (branch not indexed)
- ✅ All tests passing

### Performance
- ✅ Search latency <50ms (with cache)
- ✅ Cache hit rate >95%
- ✅ Memory overhead <100 KB

### Quality
- ✅ 90% reduction in duplicate results
- ✅ Zero breaking changes
- ✅ Clear error messages
- ✅ Positive user feedback

---

## Risk Management

### Critical Risks

**Risk 1: Platform-specific git behavior**
- **Probability:** Medium
- **Impact:** High (feature doesn't work on some platforms)
- **Mitigation:** Test on Linux + macOS minimum, robust fallback logic
- **Owner:** WTSRCH-4001 (testing ticket)

**Risk 2: Breaking existing integrations**
- **Probability:** Low
- **Impact:** High (users' code breaks)
- **Mitigation:** Comprehensive backward compatibility testing
- **Owner:** WTSRCH-4001 (testing ticket)

**Risk 3: Performance regression**
- **Probability:** Low
- **Impact:** Medium (slower searches)
- **Mitigation:** Aggressive caching, performance benchmarks
- **Owner:** WTSRCH-4001 (testing ticket)

---

## Planning Document References

All tickets reference comprehensive planning documents:

- **README.md** - Project overview and problem statement
- **planning/analysis.md** - Deep problem analysis (~5000 words)
- **planning/architecture.md** - Solution design (~4500 words)
- **planning/plan.md** - 5-phase implementation plan
- **planning/quality-strategy.md** - Testing approach
- **planning/security-review.md** - Security assessment
- **planning/project-review.md** - Pre-implementation review
- **planning/review-updates.md** - Planning updates post-review

---

## Ticket Status Legend

- 📋 **Ready** - Can be started immediately
- 🚧 **In Progress** - Currently being worked on
- ✅ **Complete** - Finished and verified
- ⏸️ **Blocked** - Waiting on dependencies

---

## Notes

- All tickets use **Vitest** test framework (not Jest)
- Test on **Linux + macOS minimum** (Windows optional)
- **Backward compatibility** is critical - existing code must work unchanged
- **Security checklist** must be completed before merge
- **Performance targets** are success criteria, not nice-to-haves

---

**Last Updated:** 2025-11-18
**Next Action:** Begin with WTSRCH-1001 using `/single-ticket WTSRCH-1001`
