# Quality Strategy: Worktree-Scoped Search

## Testing Philosophy

**Goal:** Ship with confidence that the worktree scoping works correctly across all scenarios, without over-engineering test infrastructure.

**MVP Mindset:**
- Tests prevent rework, not ceremonial checkboxes
- Focus on integration tests that prove end-to-end behavior
- Unit tests only where logic is complex or error-prone
- Manual testing for UX validation

## Critical Paths

### Path 1: Auto-Detection Happy Path

**User Story:** Developer working in `feature-auth` branch, searches for "authenticate", gets results only from that branch.

**Critical Behavior:**
1. `getCurrentBranch()` returns correct branch name
2. `lookupWorktreeId()` finds correct worktree ID
3. Search results only include chunks from detected worktree
4. Metadata indicates auto-detection succeeded

**Why Critical:** This is the 98% use case. If this doesn't work, project fails.

**Test Priority:** 🔴 Highest

### Path 2: Explicit Override

**User Story:** Developer wants to search `main` branch while working in `feature-auth`.

**Critical Behavior:**
1. Explicit `worktree: "main"` parameter overrides auto-detection
2. No git command is executed (skips auto-detect)
3. Search results only include chunks from main
4. Metadata indicates explicit mode

**Why Critical:** Power users need to override auto-detection reliably.

**Test Priority:** 🟡 High

### Path 3: Graceful Degradation

**User Story:** Developer switches to new branch that hasn't been indexed yet.

**Critical Behavior:**
1. Auto-detection finds branch name successfully
2. Worktree lookup fails (not in database)
3. Falls back to `main` worktree
4. Returns helpful error message with guidance
5. Search still returns results (from main)

**Why Critical:** Common scenario that must not break user flow.

**Test Priority:** 🟡 High

### Path 4: Search All Worktrees

**User Story:** Advanced user explicitly wants cross-worktree search.

**Critical Behavior:**
1. Explicit `worktree: null` parameter prevents auto-detection
2. No worktree filter applied to search
3. Results include chunks from all worktrees
4. Metadata indicates "all worktrees" mode

**Why Critical:** Ensures backward compatibility and power user access.

**Test Priority:** 🟢 Medium

## Test Strategy

### Integration Tests (Primary)

**Location:** `packages/maproom-mcp/tests/worktree-scoping.test.ts`

**Setup:**
- Real PostgreSQL database with test data
- Multiple indexed worktrees (main, feature-auth, feature-jwt)
- Git repository in known state

**Test Cases:**

```typescript
describe('Worktree-Scoped Search Integration', () => {
  describe('Auto-detection', () => {
    it('searches current worktree by default', async () => {
      // Setup: In feature-auth branch
      await gitCheckout('feature-auth')

      // Execute: Search without worktree parameter
      const results = await search({ repo: 'test-repo', query: 'auth' })

      // Verify: Only results from feature-auth
      expect(results.chunks.every(c => c.worktree === 'feature-auth')).toBe(true)
      expect(results.metadata.auto_detected).toBe(true)
      expect(results.metadata.worktree).toBe('feature-auth')
    })

    it('caches branch detection across requests', async () => {
      // Setup: Mock git to count calls
      const gitSpy = vi.spyOn(git, 'execGit')

      // Execute: Multiple searches
      await search({ repo: 'test-repo', query: 'auth' })
      await search({ repo: 'test-repo', query: 'validate' })

      // Verify: Git only called once (cached)
      expect(gitSpy).toHaveBeenCalledTimes(1)
    })
  })

  describe('Explicit override', () => {
    it('respects explicit worktree parameter', async () => {
      // Setup: In feature-auth branch
      await gitCheckout('feature-auth')

      // Execute: Search main explicitly
      const results = await search({
        repo: 'test-repo',
        worktree: 'main',
        query: 'auth'
      })

      // Verify: Only results from main (not feature-auth)
      expect(results.chunks.every(c => c.worktree === 'main')).toBe(true)
      expect(results.metadata.auto_detected).toBe(false)
      expect(results.metadata.mode).toBe('explicit')
    })

    it('searches all worktrees when worktree is null', async () => {
      // Setup: In feature-auth branch
      await gitCheckout('feature-auth')

      // Execute: Explicit null = search all
      const results = await search({
        repo: 'test-repo',
        worktree: null,
        query: 'auth'
      })

      // Verify: Results from multiple worktrees
      const worktrees = new Set(results.chunks.map(c => c.worktree))
      expect(worktrees.size).toBeGreaterThan(1)
      expect(results.metadata.mode).toBe('all')
    })
  })

  describe('Graceful degradation', () => {
    it('falls back to main when current branch not indexed', async () => {
      // Setup: In new-feature branch (not indexed)
      await gitCheckout('new-feature')

      // Execute: Search
      const results = await search({ repo: 'test-repo', query: 'auth' })

      // Verify: Falls back to main
      expect(results.chunks.every(c => c.worktree === 'main')).toBe(true)
      expect(results.metadata.fallback).toBe(true)
      expect(results.hint).toContain('new-feature')
      expect(results.hint).toContain('not indexed')
    })

    it('falls back when git command fails', async () => {
      // Setup: Mock git to fail
      vi.spyOn(git, 'getCurrentBranch').mockRejectedValue(new Error('git failed'))

      // Execute: Search
      const results = await search({ repo: 'test-repo', query: 'auth' })

      // Verify: Falls back to main
      expect(results.chunks.every(c => c.worktree === 'main')).toBe(true)
      expect(results.metadata.fallback_reason).toContain('Failed to detect')
    })

    it('searches all when main also not indexed', async () => {
      // Setup: Remove main worktree from database
      await db.query('DELETE FROM worktrees WHERE name = $1', ['main'])

      // Execute: Search
      const results = await search({ repo: 'test-repo', query: 'auth' })

      // Verify: Falls back to all worktrees
      expect(results.metadata.mode).toBe('all')
      expect(results.hint).toContain('No worktrees indexed')
    })
  })

  describe('Performance', () => {
    it('completes search within performance budget', async () => {
      // Setup: Warm cache
      await search({ repo: 'test-repo', query: 'auth' })

      // Execute: Measure second search
      const start = Date.now()
      await search({ repo: 'test-repo', query: 'validate' })
      const duration = Date.now() - start

      // Verify: Fast due to caching
      expect(duration).toBeLessThan(50) // <50ms with cache
    })

    it('cache expires after TTL', async () => {
      // Setup: Mock timers
      vi.useFakeTimers()

      // Execute: First search
      await search({ repo: 'test-repo', query: 'auth' })

      // Fast-forward past TTL
      vi.advanceTimersByTime(61000) // 61 seconds

      // Execute: Second search (cache expired)
      const gitSpy = vi.spyOn(git, 'execGit')
      await search({ repo: 'test-repo', query: 'validate' })

      // Verify: Git called again (cache miss)
      expect(gitSpy).toHaveBeenCalled()

      vi.useRealTimers()
    })
  })
})
```

### Unit Tests (Focused)

**Location:** `packages/maproom-mcp/tests/unit/worktree-resolution.test.ts`

**Purpose:** Test complex resolution logic in isolation

**Test Cases:**

```typescript
describe('resolveWorktreeId()', () => {
  it('returns explicit worktree when provided', async () => {
    const result = await resolveWorktreeId('test-repo', 'main', mockClient)
    expect(result.id).toBe(1)
    expect(result.metadata.mode).toBe('explicit')
  })

  it('returns null when explicit null provided', async () => {
    const result = await resolveWorktreeId('test-repo', null, mockClient)
    expect(result.id).toBe(null)
    expect(result.metadata.mode).toBe('all')
  })

  it('auto-detects when worktree is undefined', async () => {
    vi.spyOn(git, 'getCurrentBranch').mockResolvedValue('feature-auth')
    const result = await resolveWorktreeId('test-repo', undefined, mockClient)
    expect(result.id).toBe(42)
    expect(result.metadata.mode).toBe('auto')
  })

  it('falls back through tiers on failure', async () => {
    // Mock: Auto-detect fails, main succeeds
    vi.spyOn(git, 'getCurrentBranch').mockResolvedValue('missing-branch')
    const result = await resolveWorktreeId('test-repo', undefined, mockClient)
    expect(result.metadata.mode).toBe('fallback')
    expect(result.metadata.worktree).toBe('main')
  })
})

describe('getCurrentBranch() caching', () => {
  it('caches results for TTL period', async () => {
    const spy = vi.spyOn(git, 'execGit')
    await getCurrentBranch('/workspace')
    await getCurrentBranch('/workspace')
    expect(spy).toHaveBeenCalledTimes(1) // Cached
  })

  it('separate cache entries for different directories', async () => {
    await getCurrentBranch('/workspace')
    await getCurrentBranch('/other-repo')
    expect(branchCache.size).toBe(2)
  })
})

describe('lookupWorktreeId() caching', () => {
  it('caches database lookups', async () => {
    const spy = vi.spyOn(mockClient, 'query')
    await lookupWorktreeId(mockClient, 'test-repo', 'main')
    await lookupWorktreeId(mockClient, 'test-repo', 'main')
    expect(spy).toHaveBeenCalledTimes(1) // Cached
  })

  it('throws helpful error when worktree not found', async () => {
    await expect(
      lookupWorktreeId(mockClient, 'test-repo', 'nonexistent')
    ).rejects.toThrow(/not found/)
  })
})
```

### Manual Testing Checklist

**Pre-Release Manual Validation (test on Linux + macOS minimum):**

- [ ] **Happy path:** Search in main branch, verify results from main only
- [ ] **Feature branch:** Switch to feature branch, verify auto-detection works
- [ ] **Explicit override:** Pass `worktree: "main"` while in feature branch, verify override
- [ ] **Search all:** Pass `worktree: null`, verify results from all worktrees
- [ ] **New branch:** Create new branch, verify fallback to main with helpful message
- [ ] **Detached HEAD:** Checkout specific commit, verify graceful handling
- [ ] **Not in git repo:** Run from non-git directory, verify error handling
- [ ] **Cache expiry:** Wait 60+ seconds, verify cache refreshes
- [ ] **Performance:** Run multiple searches, verify second search is faster
- [ ] **Error messages:** Trigger each error scenario, verify messages are clear

## Test Data Setup

**Note:** Test fixtures (database SQL and git repository setup) will be created during Phase 4 implementation.

### Database Fixtures

```sql
-- Test repositories
INSERT INTO maproom.repos (id, name, description)
VALUES (1, 'test-repo', 'Test repository for integration tests');

-- Test worktrees
INSERT INTO maproom.worktrees (id, repo_id, name, abs_path, head_commit)
VALUES
  (1, 1, 'main', '/test/main', 'abc123'),
  (42, 1, 'feature-auth', '/test/feature-auth', 'def456'),
  (43, 1, 'feature-jwt', '/test/feature-jwt', 'ghi789');

-- Test chunks (distributed across worktrees)
INSERT INTO maproom.chunks (id, worktree_id, file_id, symbol_name, content_preview)
VALUES
  (101, 1, 1, 'authenticate', 'function authenticate() { ... }'),
  (102, 42, 2, 'authenticate', 'function authenticate() { ... }'),
  (103, 43, 3, 'authenticate', 'function authenticate() { ... }'),
  (201, 1, 4, 'validateUser', 'function validateUser() { ... }'),
  (202, 42, 5, 'validateUser', 'function validateUser() { ... }');
```

### Git Repository Setup

```bash
#!/bin/bash
# Setup test git repository with multiple worktrees

mkdir -p /tmp/test-repo
cd /tmp/test-repo
git init

# Create main branch
git checkout -b main
echo "main content" > file.txt
git add file.txt
git commit -m "Initial commit"

# Create feature branches
git checkout -b feature-auth
echo "auth content" >> file.txt
git commit -am "Add auth"

git checkout -b feature-jwt
echo "jwt content" >> file.txt
git commit -am "Add JWT"

git checkout main
```

## Risk Mitigation Through Testing

### Risk: Cache Staleness

**Test Coverage:**
- [ ] Verify cache expires after TTL
- [ ] Verify cache is keyed by directory path
- [ ] Verify manual branch switch is detected within TTL window

**Acceptance:** Cache staleness is acceptable for 60s window (low impact)

### Risk: Database Lookup Failures

**Test Coverage:**
- [ ] Test worktree not found scenario
- [ ] Test database connection failure
- [ ] Test graceful fallback to main
- [ ] Test fallback to all when main missing

**Acceptance:** All failure modes degrade gracefully with clear messages

### Risk: Git Command Failures

**Test Coverage:**
- [ ] Test not in git repository
- [ ] Test detached HEAD state
- [ ] Test git command timeout
- [ ] Test malformed git output

**Acceptance:** All git failures fall back to safe default (main or all)

### Risk: Breaking Existing Code

**Test Coverage:**
- [ ] Run existing test suite (should all pass)
- [ ] Test explicit worktree parameter still works
- [ ] Test `worktree: null` still searches all
- [ ] Test backward compatibility scenarios

**Acceptance:** Zero breaking changes to existing behavior

## Success Criteria

### Functional Coverage

- [ ] All 4 critical paths have passing integration tests
- [ ] All error scenarios have unit tests
- [ ] All fallback tiers are exercised
- [ ] Cache behavior is verified

### Performance Coverage

- [ ] Search latency measured (cold and warm cache)
- [ ] Cache hit rate measured (>95% expected)
- [ ] Memory overhead measured (<100 KB)

### Quality Gates

**Minimum to Ship:**
1. All integration tests pass
2. All unit tests pass
3. Manual testing checklist complete
4. No breaking changes to existing tests
5. Error messages are clear and actionable

**Nice to Have (Not Blockers):**
1. Load testing with 1000+ searches
2. Multi-user concurrency testing
3. Cache invalidation stress testing
4. Memory leak detection

## Test Execution Plan

### Phase 1: Unit Tests (Day 1)

**Goal:** Verify core logic works in isolation

**Tasks:**
1. Implement unit tests for `resolveWorktreeId()`
2. Implement unit tests for caching logic
3. Implement unit tests for error handling
4. Run tests, iterate until green

**Exit Criteria:** All unit tests passing

### Phase 2: Integration Tests (Day 2-3)

**Goal:** Verify end-to-end behavior with real components

**Tasks:**
1. Setup test database with fixtures
2. Setup test git repository
3. Implement happy path integration test
4. Implement fallback integration tests
5. Implement performance integration tests
6. Run tests, iterate until green

**Exit Criteria:** All integration tests passing

### Phase 3: Manual Testing (Day 4)

**Goal:** Validate UX and edge cases

**Tasks:**
1. Run manual testing checklist
2. Test in real development environment
3. Test with real repositories
4. Verify error messages are helpful
5. Verify performance is acceptable

**Exit Criteria:** Manual checklist complete, no UX issues found

### Phase 4: Regression Testing (Day 4)

**Goal:** Ensure no breaking changes

**Tasks:**
1. Run full existing test suite
2. Verify all existing tests still pass
3. Test existing code that uses search
4. Verify backward compatibility

**Exit Criteria:** Zero existing tests fail

## Monitoring and Metrics (Post-Launch)

### Metrics to Track

**Usage Metrics:**
- % of searches using auto-detection
- % of searches using explicit worktree
- % of searches using null (all worktrees)

**Performance Metrics:**
- Branch cache hit rate
- Worktree ID cache hit rate
- Average search latency (cold vs warm)

**Error Metrics:**
- % of searches that fall back to main
- % of searches that fall back to all
- Git detection failure rate

### Alerts

**Warning:**
- Cache hit rate <80% (indicates TTL might be too short)
- Fallback rate >10% (indicates indexing coverage issues)

**Critical:**
- Git detection failure rate >5% (indicates environment issues)
- Search latency >500ms (indicates performance degradation)

## Conclusion

This testing strategy ensures:

1. **Correctness:** Integration tests prove end-to-end behavior
2. **Reliability:** Unit tests verify complex logic and error handling
3. **Performance:** Benchmarks confirm caching is effective
4. **Compatibility:** Regression tests prevent breaking changes
5. **UX Quality:** Manual testing validates error messages and flow

**MVP Focus:** Tests target critical paths and common errors, not exhaustive edge cases. This provides confidence to ship while avoiding test bloat.

**Risk Acceptance:** Some edge cases (e.g., 60s cache staleness) are accepted as low-impact trade-offs for simplicity.
