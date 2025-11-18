# Implementation Plan: Worktree-Scoped Search

## Project Overview

**Goal:** Make Maproom search default to current worktree instead of searching all worktrees, dramatically improving result relevance and search performance.

**Scope:** Add auto-detection of current git branch and use it as the default search scope in MCP search tool.

**Duration:** 4-5 days (1 week sprint)

**Agents:** TypeScript specialists, MCP server specialists, testing specialists

## Phases

### Phase 1: Git Utilities Enhancement (Day 1)

**Objective:** Add git branch detection capability to existing git utilities

**Agent:** TypeScript/MCP specialist

**Tasks:**
1. Install `lru-cache` dependency: `cd packages/maproom-mcp && pnpm add lru-cache`
2. Add `getCurrentBranch()` function to `packages/maproom-mcp/src/utils/git.ts`
3. Implement LRU caching for branch detection (60s TTL)
4. Handle edge cases (detached HEAD, not in git repo)
5. Write unit tests for git utilities

**Deliverables:**
- `lru-cache` package installed in dependencies
- `getCurrentBranch(cwd?: string): Promise<string>` function
- Cache implementation with TTL
- Unit tests passing

**Acceptance Criteria:**
- [ ] `lru-cache` dependency installed in package.json
- [ ] `getCurrentBranch()` returns correct branch name
- [ ] Handles detached HEAD state gracefully
- [ ] Returns clear error when not in git repo
- [ ] Cache reduces git subprocess calls by >95%
- [ ] All unit tests passing

**Files Modified:**
- `packages/maproom-mcp/src/utils/git.ts`
- `packages/maproom-mcp/tests/unit/git.test.ts` (new)

**Dependencies:** None

**Risk:** LOW - Isolated utility function

### Phase 2: Worktree Resolution Logic (Day 2)

**Objective:** Implement three-tier worktree resolution (explicit > auto > fallback)

**Agent:** TypeScript/MCP specialist

**Tasks:**
1. Add `resolveWorktreeId()` function to search handler
2. Implement tier 1: explicit parameter handling
3. Implement tier 2: auto-detection via `getCurrentBranch()`
4. Implement tier 3: fallback to main, then all
5. Add `lookupWorktreeId()` with LRU caching (5min TTL)
6. Write unit tests for resolution logic

**Deliverables:**
- `resolveWorktreeId()` function with three-tier logic
- `lookupWorktreeId()` with database query and caching
- Comprehensive error handling and fallback
- Unit tests passing

**Acceptance Criteria:**
- [ ] Explicit parameter always takes priority
- [ ] Auto-detection works when parameter is omitted
- [ ] Fallback to main when current branch not indexed
- [ ] Fallback to all when main not indexed
- [ ] Clear error messages for each failure mode
- [ ] All unit tests passing

**Files Modified:**
- `packages/maproom-mcp/src/index.ts` (search handler)
- `packages/maproom-mcp/tests/unit/worktree-resolution.test.ts` (new)

**Dependencies:** Phase 1 (git utilities)

**Risk:** MEDIUM - Core logic, needs careful testing

### Phase 3: Integration with Search Tool (Day 3)

**Objective:** Wire up worktree resolution to search tool handler

**Agent:** TypeScript/MCP specialist

**Tasks:**
1. Modify search tool handler to call `resolveWorktreeId()`
2. Pass resolved worktree_id to Rust search executors
3. Add metadata to search results (auto_detected, fallback, etc.)
4. Add helpful hints/messages for fallback scenarios
5. Update MCP tool schema documentation
6. Write integration tests

**Deliverables:**
- Search handler integrated with worktree resolution
- Result metadata includes worktree scoping information
- Helpful error messages for users
- Integration tests passing

**Acceptance Criteria:**
- [ ] Search defaults to current worktree when parameter omitted
- [ ] Explicit parameter still works (backward compatible)
- [ ] Passing `null` searches all worktrees (power user)
- [ ] Result metadata shows which worktree was searched
- [ ] Helpful hints shown when fallback occurs
- [ ] All integration tests passing

**Files Modified:**
- `packages/maproom-mcp/src/index.ts` (search tool handler)
- `packages/maproom-mcp/tests/integration/worktree-scoping.test.ts` (new)

**Dependencies:** Phase 2 (worktree resolution)

**Risk:** MEDIUM - Integration point, affects user experience

### Phase 4: Testing and Validation (Day 4)

**Objective:** Comprehensive testing across all scenarios and edge cases using Vitest test framework

**Agent:** Testing specialist

**Tasks:**
1. Create test fixtures (SQL setup script, git repository setup)
2. Run full integration test suite (using existing `vitest.config.ts`)
3. Test happy path (auto-detection works)
4. Test explicit override scenarios
5. Test fallback scenarios (branch not indexed)
6. Test error scenarios (git fails, db fails)
7. Test performance (caching, latency)
8. Test backward compatibility (existing code)
9. Test on Linux + macOS platforms (minimum)
10. Manual testing checklist

**Deliverables:**
- Test fixtures (database and git repository)
- Complete test suite (unit + integration) using Vitest
- Performance benchmarks
- Manual testing report (Linux + macOS)
- Bug fixes from testing

**Acceptance Criteria:**
- [ ] Test fixtures created and documented
- [ ] All unit tests passing (100% coverage of core logic)
- [ ] All integration tests passing (end-to-end scenarios)
- [ ] Performance targets met (<50ms search with cache)
- [ ] No breaking changes to existing tests
- [ ] Tested on Linux + macOS (minimum)
- [ ] Manual testing checklist complete
- [ ] All bugs found during testing are fixed

**Files Modified:**
- Various test files (using Vitest framework)
- Test fixtures in `tests/fixtures/`
- Bug fixes as needed

**Dependencies:** Phase 3 (integration complete)

**Risk:** LOW - Testing phase, no new features

### Phase 5: Documentation and Release (Day 5)

**Objective:** Document new behavior and prepare for release

**Agent:** Documentation specialist / Technical lead

**Tasks:**
1. Update MCP tool documentation for search
2. Add examples of auto-detection to docs
3. Document how to search all worktrees (pass `null`)
4. Update CHANGELOG
5. Security checklist completion
6. Final code review
7. Merge to main branch

**Deliverables:**
- Updated documentation
- CHANGELOG entry
- Security review complete
- Code merged to main

**Acceptance Criteria:**
- [ ] Documentation clearly explains new default behavior
- [ ] Examples show common use cases
- [ ] Security checklist complete
- [ ] Code review approved by 1+ reviewers
- [ ] All CI/CD checks passing
- [ ] Merged to main

**Files Modified:**
- `packages/maproom-mcp/README.md`
- `CHANGELOG.md`
- Any documentation files

**Dependencies:** Phase 4 (testing complete)

**Risk:** LOW - Documentation and process

## Agent Assignments

### Primary Agent: TypeScript/MCP Specialist

**Responsibilities:**
- Phase 1: Git utilities
- Phase 2: Worktree resolution
- Phase 3: Search integration

**Skills Required:**
- TypeScript/Node.js expertise
- MCP server architecture understanding
- Git command knowledge
- PostgreSQL query optimization

**Estimated Effort:** 3 days

### Secondary Agent: Testing Specialist

**Responsibilities:**
- Phase 4: Comprehensive testing
- Writing integration tests
- Performance benchmarking
- Manual testing execution

**Skills Required:**
- Jest/Vitest testing frameworks
- Integration testing
- Performance testing
- Test data setup

**Estimated Effort:** 1 day

### Supporting Agent: Documentation Specialist

**Responsibilities:**
- Phase 5: Documentation updates
- Examples and tutorials
- CHANGELOG maintenance

**Skills Required:**
- Technical writing
- Markdown
- MCP tool documentation

**Estimated Effort:** 0.5 days

## Milestones and Checkpoints

### Milestone 1: Core Utilities Complete (End of Day 1)

**Checkpoint:**
- [ ] `getCurrentBranch()` implemented and tested
- [ ] Caching working correctly
- [ ] Unit tests passing
- [ ] Demo: Show branch detection working

**Go/No-Go Decision:** If git utilities don't work reliably, address issues before proceeding

### Milestone 2: Resolution Logic Complete (End of Day 2)

**Checkpoint:**
- [ ] `resolveWorktreeId()` implemented
- [ ] All three tiers working (explicit > auto > fallback)
- [ ] Database lookup with caching working
- [ ] Unit tests passing
- [ ] Demo: Show resolution tiers in action

**Go/No-Go Decision:** If resolution logic is flaky, fix before integration

### Milestone 3: Search Integration Complete (End of Day 3)

**Checkpoint:**
- [ ] Search handler using worktree resolution
- [ ] Results include correct metadata
- [ ] Error messages helpful and clear
- [ ] Integration tests passing
- [ ] Demo: End-to-end search with auto-detection

**Go/No-Go Decision:** If integration has issues, debug before testing phase

### Milestone 4: Testing Complete (End of Day 4)

**Checkpoint:**
- [ ] All tests passing (unit + integration)
- [ ] Performance benchmarks met
- [ ] Manual testing complete
- [ ] No critical bugs outstanding
- [ ] Demo: Show test coverage and results

**Go/No-Go Decision:** If critical bugs found, fix before release

### Milestone 5: Ready to Ship (End of Day 5)

**Checkpoint:**
- [ ] Documentation complete
- [ ] Security review approved
- [ ] Code review approved
- [ ] CI/CD passing
- [ ] Merged to main
- [ ] Demo: Show complete feature working

**Go/No-Go Decision:** If any checkpoint fails, delay release

## Risk Management

### Risk 1: Git Detection Fails in Some Environments

**Probability:** MEDIUM
**Impact:** HIGH (core functionality broken)

**Mitigation:**
- Test across multiple environments (Linux, macOS, Windows)
- Implement robust fallback (search all if detection fails)
- Clear error messages guide users to workarounds

**Contingency:** If git detection proves unreliable, make it opt-in rather than default

### Risk 2: Performance Regression

**Probability:** LOW
**Impact:** MEDIUM (slower searches)

**Mitigation:**
- Benchmark before and after implementation
- Cache aggressively to minimize overhead
- Monitor performance during testing

**Contingency:** If searches are slower, optimize caching or make auto-detection opt-in

### Risk 3: Breaking Existing Integrations

**Probability:** LOW
**Impact:** HIGH (users' code breaks)

**Mitigation:**
- Comprehensive backward compatibility testing
- Only change behavior when parameter is omitted
- Document migration path clearly

**Contingency:** If breaking changes detected, add feature flag to enable/disable

### Risk 4: Database Query Performance

**Probability:** LOW
**Impact:** LOW (minor latency increase)

**Mitigation:**
- Use indexed columns for lookups
- Cache database results aggressively
- Benchmark query performance

**Contingency:** If queries are slow, add database indexes or optimize query

## Testing Checkpoints

### Unit Test Coverage

**Target:** >90% coverage for new code

**Critical Functions:**
- `getCurrentBranch()` - 100% coverage (all edge cases)
- `resolveWorktreeId()` - 100% coverage (all tiers)
- `lookupWorktreeId()` - 100% coverage (cache + query)

### Integration Test Scenarios

**Required Scenarios:**
1. Auto-detection works (happy path)
2. Explicit override works
3. Fallback to main when branch not indexed
4. Fallback to all when main not indexed
5. Search all with explicit `null`
6. Cache hit rate >95%
7. Performance within budget (<50ms)

### Manual Test Checklist

**Pre-Release Manual Tests:**
1. Search in main branch
2. Search in feature branch
3. Search with explicit worktree parameter
4. Search with `worktree: null`
5. Switch to new branch (not indexed)
6. Detached HEAD state
7. Not in git repository
8. Multiple searches (verify caching)

## Success Metrics

### Implementation Metrics

- [ ] All phases complete on schedule
- [ ] All tests passing (unit + integration)
- [ ] Code review approved
- [ ] Security review approved

### Quality Metrics

- [ ] Test coverage >90% for new code
- [ ] Zero breaking changes to existing tests
- [ ] Performance: search latency <50ms with cache
- [ ] Performance: cache hit rate >95%

### User Experience Metrics

- [ ] Search defaults to current worktree
- [ ] Results are relevant (from user's context)
- [ ] Error messages are clear and helpful
- [ ] Backward compatibility preserved

### Performance Metrics

- [ ] Search latency: <50ms (cached)
- [ ] Search latency: <150ms (cold cache)
- [ ] Memory overhead: <100 KB
- [ ] Cache hit rate: >95%

## Rollback Plan

### If Critical Issue Found Post-Merge

**Steps:**
1. Create hotfix branch from last known good commit
2. Revert worktree scoping changes
3. Deploy hotfix to users
4. Fix issues in separate branch
5. Re-test thoroughly
6. Re-deploy when stable

**Trigger Conditions:**
- Search completely broken for users
- Critical security vulnerability discovered
- Performance degradation >500ms per search
- Data corruption or loss

### Feature Flag Approach (Alternative)

**If available:**
1. Add feature flag: `ENABLE_WORKTREE_SCOPING`
2. Default to `false` initially (opt-in)
3. Gradually enable for more users
4. Monitor metrics and errors
5. Full rollout when stable

**Note:** Feature flags not currently in scope, but recommended for future

## Communication Plan

### Stakeholder Updates

**Who:** Project lead, team members, users

**When:** End of each phase, major milestones, issues discovered

**Format:**
- Daily: Slack updates on progress
- End of each phase: Demo/review session
- Issues: Immediate notification

### Documentation Updates

**Internal:**
- Update `.agents/projects/WTSRCH_*/` with progress notes
- Document any deviations from plan
- Track decisions and rationale

**External:**
- Update package README
- Update CHANGELOG
- Add migration guide (if needed)

## Post-Launch Monitoring

### Metrics to Track (First Week)

**Usage Metrics:**
- % of searches using auto-detection
- % of searches using explicit worktree
- % of searches using `null` (all worktrees)

**Performance Metrics:**
- Average search latency
- Cache hit rate
- Memory usage

**Error Metrics:**
- % of searches falling back to main
- % of searches with git detection failures
- Error types and frequency

### Success Criteria (After 1 Week)

- [ ] No critical bugs reported
- [ ] Cache hit rate >90% (target 95%)
- [ ] Search latency <100ms average
- [ ] Fallback rate <5% of searches
- [ ] Positive user feedback

## Dependencies

### External Dependencies

**Git:** Must be available on user's system
- **Risk:** User doesn't have git installed
- **Mitigation:** Graceful fallback, clear error message

**PostgreSQL:** Database must be running and accessible
- **Risk:** Database connection issues
- **Mitigation:** Existing connection pooling, error handling

**Node.js:** Runtime environment
- **Risk:** Version incompatibility
- **Mitigation:** Specify minimum version in package.json

### Internal Dependencies

**Existing Code:**
- `utils/git.ts` - Extend with new functions
- `src/index.ts` - Modify search handler
- Rust search executors - Already support worktree filtering (no changes)

**Infrastructure:**
- Test database for integration tests
- CI/CD pipeline for automated testing

## Conclusion

This plan provides a clear, phased approach to implementing worktree-scoped search:

**Week at a Glance:**
- **Day 1:** Git utilities
- **Day 2:** Resolution logic
- **Day 3:** Search integration
- **Day 4:** Testing
- **Day 5:** Documentation and release

**Key Success Factors:**
1. Robust error handling and fallbacks
2. Comprehensive testing (unit + integration)
3. Backward compatibility preserved
4. Clear documentation and communication

**Risk Mitigation:**
- Early checkpoints to catch issues
- Fallback strategies for each risk
- Rollback plan if critical issues arise

**Next Steps:**
1. Review and approve plan
2. Assign agents to phases
3. Begin Phase 1 implementation
4. Track progress daily
5. Adjust plan as needed

**Estimated Timeline:** 4-5 days (1 sprint)

**Confidence Level:** HIGH - Clear scope, low risk, proven patterns
