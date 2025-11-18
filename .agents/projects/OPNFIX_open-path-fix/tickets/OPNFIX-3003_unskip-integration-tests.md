# Ticket: OPNFIX-3003: Un-skip Integration Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- integration-tester (primary)
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Remove `.skip` from two integration tests in `open.int.test.ts` (lines 199-207) and implement the full workflow tests using existing test fixtures and database helpers that are already available.

## Background
This ticket implements Phase 3.3 of the OPNFIX project plan. During the initial test suite creation, two critical integration tests were skipped with the comment "implement when test fixtures are available". However, the test fixtures (`tests/fixtures/sample-repo/`) and database helpers (`tests/helpers/database.ts`) have always been available and ready to use.

These two tests validate:
1. **Filesystem read workflow** - Full E2E test reading from indexed files
2. **Git history read workflow** - Full E2E test reading from git history

The tests were incorrectly skipped, and this ticket corrects that oversight by implementing them properly using the existing infrastructure.

Reference: `.agents/projects/OPNFIX_open-path-fix/planning/plan.md` - Phase 3, Ticket 3.3

## Acceptance Criteria
- [ ] `.skip` removed from line 199 (filesystem read test)
- [ ] `.skip` removed from line 205 (git history read test)
- [ ] Both tests are fully implemented (not just stub code)
- [ ] Tests use real database via `setupTestDatabase()` and `teardownTestDatabase()`
- [ ] Tests use existing fixtures from `tests/fixtures/sample-repo/`
- [ ] Tests leverage `indexTestFixtures()` to populate test data
- [ ] Both tests pass when executed
- [ ] Tests validate end-to-end workflow (index → search → open → verify)
- [ ] No tests are skipped in the entire `open.int.test.ts` file
- [ ] No new test infrastructure created (reuse existing helpers only)

## Technical Requirements
- **File:** `packages/maproom-mcp/tests/tools/open.int.test.ts` (lines 199-207)
- **Testing Framework:** Vitest (existing)
- **Database:** Real PostgreSQL database via test helpers
- **Fixtures:** Use `tests/fixtures/sample-repo/` (already available)
- **Test Helpers:** Import from `tests/helpers/database.ts`:
  - `setupTestDatabase()` - Initialize test database
  - `teardownTestDatabase()` - Cleanup after tests
  - `indexTestFixtures()` - Index fixture data into database
- **Test Isolation:** Must integrate with existing `beforeEach`/`afterEach` hooks if present
- **Assertions:** Use Vitest `expect()` for all assertions

## Implementation Notes

### Current State (Lines 199-207)

```typescript
describe('Open Tool - End-to-End Tests', () => {
  it.skip('should handle full workflow: filesystem read', async () => {
    // This would require a fully set up test environment with database data
    // Marked as skip for now - implement when test fixtures are available
  })

  it.skip('should handle full workflow: git history read', async () => {
    // This would require a fully set up test environment with database data
    // Marked as skip for now - implement when test fixtures are available
  })
})
```

### Required Changes

1. **Remove `.skip` from both tests**
2. **Implement filesystem read test:**
   - Call `indexTestFixtures()` to populate database with sample repo data
   - Use open tool to read a file from the indexed repository
   - Verify content matches actual file on disk
   - Validate complete workflow from database query to file read

3. **Implement git history read test:**
   - Index git repository with history
   - Use open tool with git history parameters (if applicable)
   - Verify historical content is retrieved correctly
   - Validate git integration works end-to-end

### Implementation Approach

**Test 1: Filesystem Read Workflow**
```typescript
it('should handle full workflow: filesystem read', async () => {
  // Index the sample repository fixtures
  await indexTestFixtures()

  // Query database to find an indexed file (e.g., from sample-repo/)
  // Use open tool to read the file
  const result = await openTool({
    worktree: 'test-worktree',
    relpath: 'path/to/sample/file.ts',
    start: 1,
    end: 10
  })

  // Read actual file content
  const actualContent = await fs.readFile(
    path.join(fixturesPath, 'sample-repo', 'path/to/sample/file.ts'),
    'utf-8'
  )

  // Validate content matches
  expect(result.content).toContain(expectedSnippet)
  expect(result.content).toBe(actualContent.split('\n').slice(0, 10).join('\n'))
})
```

**Test 2: Git History Read Workflow**
```typescript
it('should handle full workflow: git history read', async () => {
  // Index git repository with history
  await indexTestFixtures()

  // Open file from git history (if git history feature exists)
  // Otherwise, validate that git metadata is properly indexed
  const result = await openTool({
    worktree: 'test-worktree',
    relpath: 'path/to/versioned/file.ts',
    start: 1,
    end: 20
  })

  // Verify git-tracked file can be read
  expect(result.content).toBeDefined()
  expect(result.content.length).toBeGreaterThan(0)

  // Validate file came from git-tracked repository
  expect(result.source).toBe('filesystem') // or 'git' depending on implementation
})
```

### Key Considerations

1. **Use Existing Infrastructure:**
   - Database helpers are in `tests/helpers/database.ts`
   - Fixtures are in `tests/fixtures/sample-repo/`
   - Do NOT create new setup utilities or fixtures

2. **Inspect Fixtures First:**
   - Check what files exist in `tests/fixtures/sample-repo/`
   - Use actual file paths from the fixtures
   - Ensure file content is predictable for assertions

3. **Database Setup:**
   - If `beforeEach` already exists in file, use it
   - If not, add setup/teardown for this describe block only
   - Ensure database is clean before each test

4. **Git History Test:**
   - If open tool doesn't have explicit git history feature, test that git-tracked files can be read
   - Focus on validating the tool works with git repositories
   - Keep test simple and focused

5. **Cleanup:**
   - Ensure `teardownTestDatabase()` runs after tests
   - No manual cleanup needed if using helpers correctly

## Dependencies
- **Prerequisite Tickets:**
  - OPNFIX-1001: Update getWorktreePath Function (must be completed)
  - OPNFIX-1002: Add fileExists Helper Function (must be completed)
  - OPNFIX-3001: E2E Test Suite (helpful context but not blocking)
- **External Dependencies:**
  - PostgreSQL test database (already available)
  - Test helpers in `tests/helpers/database.ts` (already available)
  - Test fixtures in `tests/fixtures/sample-repo/` (already available)
- **No blockers identified**

## Risk Assessment
- **Risk:** Fixtures may not contain expected files
  - **Mitigation:** Inspect `tests/fixtures/sample-repo/` first to determine available files

- **Risk:** Git history feature may not exist yet in open tool
  - **Mitigation:** Adapt test to validate git repository integration instead of historical reads

- **Risk:** Tests may conflict with other tests in same file
  - **Mitigation:** Review existing test setup, use proper isolation with beforeEach/afterEach

- **Risk:** Database state may pollute between tests
  - **Mitigation:** Ensure `teardownTestDatabase()` is called in afterEach hook

## Files/Packages Affected
- **MODIFY:** `packages/maproom-mcp/tests/tools/open.int.test.ts` (lines 199-207)
- **READ:** `packages/maproom-mcp/tests/helpers/database.ts` (import helpers)
- **READ:** `packages/maproom-mcp/tests/fixtures/sample-repo/` (use existing fixtures)
- **READ:** `packages/maproom-mcp/src/tools/open.ts` (tool being tested)
