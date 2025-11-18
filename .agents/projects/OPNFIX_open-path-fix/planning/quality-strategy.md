# Quality Strategy: Why Tests Didn't Catch This Bug

**Date:** 2025-11-18
**Project:** OPNFIX - Open Tool Path Resolution Fix
**Focus:** Understanding test failures and preventing regression

## Executive Summary

The open tool bug went undetected because **end-to-end tests were skipped**. Unit tests passed by mocking the broken behavior, and integration tests never validated the complete workflow from database to filesystem.

This document explains:
1. Why existing tests failed to catch the bug
2. What tests would have caught it
3. How to prevent similar gaps in the future

## The Test Gap: A Detailed Autopsy

### Existing Test Coverage

**File: `packages/maproom-mcp/tests/tools/open.test.ts`**
- **Lines of code:** 329
- **Test cases:** 40+
- **Coverage:** Path validation, range extraction, parameter validation
- **Mocking:** All database interactions are mocked or bypassed

**Critical Gap:** Never tests actual database queries or file reading together.

**File: `packages/maproom-mcp/tests/tools/open.int.test.ts`**
- **Lines of code:** 237
- **Test cases:** 15 (most are git utilities, not open tool)
- **Skipped tests:** Lines 199-207

**The Smoking Gun (lines 199-207):**
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

**Why This Matters:**

These skipped tests would have done **exactly** what users do:
1. Set up real database with indexed files
2. Query worktree and file data
3. Call open tool with real parameters
4. Verify returned content matches actual file

**They were never implemented.** The bug went undetected.

### What Each Test Type Validates

#### Unit Tests (open.test.ts)

**What they test:**
- ✅ Parameter validation (Zod schema)
- ✅ Path normalization (`./../` handling)
- ✅ Range extraction (line slicing)
- ✅ Error message formatting
- ✅ Security validation (path traversal detection)

**What they DON'T test:**
- ❌ Database queries returning correct data
- ❌ Path joining producing valid file paths
- ❌ File system interactions
- ❌ Complete data flow from database → filesystem

**Example Test (lines 88-91):**
```typescript
it('should accept valid relative paths', () => {
  expect(validatePath('src/index.ts')).toBe('src/index.ts')
  expect(validatePath('packages/cli/src/main.ts')).toBe(
    path.join('packages', 'cli', 'src', 'main.ts')
  )
})
```

**What's missing:** No verification that this path actually exists or can be read from database-stored abs_path.

#### Integration Tests (open.int.test.ts)

**What they test:**
- ✅ Git utilities (commit detection, git show)
- ✅ Repository initialization
- ✅ Database connection
- ⚠️ Database schema exists (line 175-196)

**What they DON'T test:**
- ❌ getWorktreePath() function with real data
- ❌ Path construction from database abs_path + relpath
- ❌ File reading after path resolution
- ❌ Error handling when paths are wrong

**Database Test (lines 175-196):**
```typescript
it.skipIf(!testClient)('should query worktree path from database', async () => {
  // Check if test data exists
  const { rows } = await testClient.query(
    'SELECT COUNT(*) as count FROM maproom.worktrees LIMIT 1'
  )

  if (parseInt(rows[0].count) === 0) {
    console.warn('No test data in database, skipping test')
    return  // ← Gives up if no data!
  }

  // Query should work without errors
  const result = await testClient.query(
    'SELECT w.abs_path FROM maproom.worktrees w LIMIT 1'
  )
  expect(result.rows.length).toBeGreaterThanOrEqual(0)
})
```

**The Problem:** This test only checks if the **query works**, not if the **data is correct**. It would pass even with the duplicate path bug!

## Why Didn't Tests Catch This?

### Root Cause 1: Test Pyramid Inversion

**Healthy Test Pyramid:**
```
    /\
   /E2E\    ← Few, validate complete workflows
  /-----\
 / INTEG \  ← Some, validate component integration
/----------\
/   UNIT   \ ← Many, validate individual functions
```

**Actual Test Pyramid:**
```
    /\
   / 0\     ← Zero end-to-end tests
  /-----\
 /  Weak \ ← Integration tests skip critical paths
/----------\
/  Strong \ ← Excellent unit test coverage
```

**Result:** High confidence in parts, zero confidence in the whole.

### Root Cause 2: Mock-Heavy Unit Tests

**Pattern in open.test.ts:**
```typescript
describe('Open Tool - Path Validation', () => {
  it('should normalize paths correctly', () => {
    expect(validatePath('src/./index.ts')).toBe(path.join('src', 'index.ts'))
  })
})
```

**This test:**
- ✅ Validates that `validatePath()` normalizes correctly
- ❌ Doesn't validate that normalized path works with database data
- ❌ Doesn't validate that the path can actually be read

**The Danger of Mocking:**

Mocks create a **false sense of security**. The test passes, but the actual integration is broken.

```typescript
// Unit test passes (mocked)
const mockClient = {
  query: vi.fn().mockResolvedValue({
    rows: [{ abs_path: '/workspace' }]  // ← Returns fake data
  })
}

// Real code fails (actual database returns wrong path)
const realClient = new Client(...)
const { rows } = await realClient.query(...)
// rows[0].abs_path = "/workspace/crates/maproom" ← Polluted!
```

### Root Cause 3: "Test Fixtures Are Hard"

From line 200:
```typescript
// This would require a fully set up test environment with database data
// Marked as skip for now - implement when test fixtures are available
```

**Translation:** "This is hard, skip it for now."

**Why This Happened:**

End-to-end tests require:
1. Real PostgreSQL database
2. Running indexer to create data
3. Known test files in known locations
4. Complex setup and teardown

**The developers chose** to skip these tests rather than invest in test infrastructure.

**Cost of skipping:** Complete tool failure in production.

### Root Cause 4: No Contract Tests

**What are contract tests?** Tests that validate the **contract** between components.

**Missing contract:** Database schema ↔ Open tool expectations

**The implicit contract:**
```
Database promises:
- worktrees.abs_path is absolute path to repository root
- files.relpath is relative path from repository root
- abs_path + relpath produces readable file

Open tool assumes:
- Database contract holds true
- path.join(abs_path, relpath) will work
- No validation needed
```

**When contract breaks:** Silent failure until production use.

## What Tests Would Have Caught This

### Test That Would Have Found It #1: Database-to-Filesystem E2E

```typescript
describe('Open Tool - Database to Filesystem E2E', () => {
  it('should read file using real database and filesystem', async () => {
    // 1. Setup: Index a real file
    const testFile = await createTestFile('test.ts', 'export const x = 1')
    await indexFile(testFile)

    // 2. Query: Search for the file
    const searchResults = await searchTool({ query: 'export const x' })
    const chunk = searchResults[0]

    // 3. Action: Call open tool with chunk data
    const result = await openTool({
      relpath: chunk.relpath,
      worktree: chunk.worktree
    })

    // 4. Verify: Content matches
    expect(result.content).toBe('export const x = 1')
  })
})
```

**Why this catches the bug:**
- Uses real database queries (not mocked)
- Uses real file paths (not hardcoded)
- Validates complete workflow
- Fails immediately if paths don't match

### Test That Would Have Found It #2: Polluted Database Scenario

```typescript
describe('Open Tool - Database Pollution Handling', () => {
  it('should handle multiple worktrees with same file', async () => {
    // 1. Setup: Create pollution by indexing from two different roots
    await indexFromRoot('/workspace', 'main')
    await indexFromRoot('/workspace/crates/maproom', 'main')

    // 2. Verify: Database now has duplicate worktrees
    const worktrees = await query(
      'SELECT abs_path FROM maproom.worktrees WHERE name = $1',
      ['main']
    )
    expect(worktrees.length).toBeGreaterThan(1)

    // 3. Action: Call open tool
    const result = await openTool({
      relpath: 'crates/maproom/src/main.rs',
      worktree: 'main'
    })

    // 4. Verify: Still works (fallback to correct path)
    expect(result.content).toContain('fn main()')
  })
})
```

**Why this catches the bug:**
- Explicitly creates the problematic scenario
- Validates resilience to pollution
- Would fail with current implementation
- Passes with proposed fix

### Test That Would Have Found It #3: Path Validation Contract

```typescript
describe('Open Tool - Path Contract Validation', () => {
  it('should validate abs_path + relpath produces real file', async () => {
    // 1. Setup: Insert worktree with WRONG abs_path
    await query(
      `INSERT INTO maproom.worktrees (repo_id, name, abs_path)
       VALUES (1, 'test', '/wrong/path')`
    )
    await query(
      `INSERT INTO maproom.files (worktree_id, relpath)
       VALUES (1, 'test.ts')`
    )

    // 2. Action: Call open tool
    const promise = openTool({ relpath: 'test.ts', worktree: 'test' })

    // 3. Verify: Should fail with clear error
    await expect(promise).rejects.toThrow('File not accessible')
    await expect(promise).rejects.toThrow(/database pollution/)
  })
})
```

**Why this catches the bug:**
- Tests defensive programming
- Validates error messages
- Ensures graceful failure

## Test Strategy for This Project

### Phase 1: Fix the Critical Gap (Part of Implementation)

**Add end-to-end test suite:**

Location: `packages/maproom-mcp/tests/tools/open.e2e.test.ts`

**Tests to implement:**

1. **Happy Path E2E**
   - Index file → Search → Get chunk → Open → Verify content

2. **Polluted Database E2E**
   - Create duplicate worktrees → Open still works via fallback

3. **All Paths Invalid E2E**
   - Create only wrong paths → Open fails with clear error

4. **Multiple Candidates E2E**
   - Three worktrees, second one works → Validates ordering

5. **Security Validation E2E**
   - Path traversal in database → Open rejects it

**Coverage target:** 100% of getWorktreePath() logic

### Phase 2: Strengthen Integration Tests

**Un-skip existing tests:**

Location: `packages/maproom-mcp/tests/tools/open.int.test.ts:199-207`

**Make them real:**
```typescript
describe('Open Tool - End-to-End Tests', () => {
  it('should handle full workflow: filesystem read', async () => {
    // NO LONGER SKIPPED!

    // Setup test environment with database data
    const testRepo = await setupTestRepository()
    await indexRepository(testRepo.path)

    // Call open tool
    const result = await handleOpenTool({
      relpath: 'test.ts',
      worktree: 'main'
    }, dbClient)

    // Verify
    expect(result.content).toBe(testRepo.files['test.ts'].content)
  })
})
```

### Phase 3: Add Contract Tests

**New test suite:** `packages/maproom-mcp/tests/contracts/database-to-open.test.ts`

**Tests:**
1. Database schema matches open tool expectations
2. All worktrees.abs_path values are accessible directories
3. All files.relpath values work with their worktree abs_path
4. Index health check (find polluted data)

**Run schedule:** Part of CI pipeline

### Phase 4: Property-Based Testing (Advanced)

**Library:** `fast-check` (property-based testing for TypeScript)

**Test:** Generate random valid path combinations
```typescript
import fc from 'fast-check'

it('should handle any valid path combination', () => {
  fc.assert(
    fc.property(
      fc.string().filter(isValidRelPath),
      fc.string().filter(isValidAbsPath),
      async (relpath, absPath) => {
        // Setup: Insert into database
        await insertWorktree(absPath)
        await insertFile(relpath)

        // Create real file
        await createFileAt(path.join(absPath, relpath))

        // Action: Open should work
        const result = await openTool({ relpath, worktree: 'test' })

        // Verify: Content matches
        expect(result).toBeDefined()
      }
    )
  )
})
```

**Benefit:** Finds edge cases we didn't think of.

## Preventing Future Test Gaps

### Rule 1: No Skipped Integration Tests

**Policy:** Skipped tests must have:
- Ticket tracking when they'll be implemented
- Justification for why skipping is acceptable
- Regular review (monthly) to prioritize implementation

**This bug's lesson:** Skipped tests represent **known blind spots**. Don't ignore them.

### Rule 2: Critical Paths Need E2E Tests

**Critical Path Definition:** User-facing workflow that crosses component boundaries.

**Examples:**
- ✅ Search → Context → Read file (E2E needed)
- ✅ Index → Search → Verify results (E2E needed)
- ⚠️ Parse TypeScript file (Unit test sufficient)

**Requirement:** Every critical path has at least one E2E test.

### Rule 3: Test Fixtures Are First-Class Code

**Investment:** Build reusable test infrastructure.

**Components:**
- `TestRepository` class (create test repos easily)
- `TestDatabase` class (setup/teardown database state)
- `TestIndexer` class (index test data)
- `TestAssertion` helpers (common assertions)

**Benefit:** E2E tests become easy to write.

**Example:**
```typescript
// Easy to write because infrastructure exists
it('should work end-to-end', async () => {
  const repo = TestRepository.withFiles({ 'test.ts': 'content' })
  await TestIndexer.indexAll(repo)

  const result = await openTool({ relpath: 'test.ts', worktree: 'main' })

  expect(result.content).toBe('content')
})
```

### Rule 4: Contract Tests Are Required

**For each component interface:**
- Document the contract explicitly
- Write tests that validate the contract
- Run contract tests in CI

**Example contract:**
```typescript
// Database-to-Open Tool Contract
interface DatabaseOpenContract {
  // Promise: All abs_path values are accessible directories
  worktreePathsAreValid(): Promise<boolean>

  // Promise: All abs_path + relpath combinations produce real files
  filePathsAreConsistent(): Promise<boolean>

  // Promise: No duplicate worktrees with conflicting paths
  noDuplicateWorktrees(): Promise<boolean>
}
```

### Rule 5: MVP Still Needs Core Tests

**MVP Principle:** Ship fast, but ship with confidence.

**Minimum test bar:**
- ✅ Unit tests for new functions
- ✅ Integration test for happy path
- ✅ Integration test for error path
- ✅ E2E test for critical workflow

**Don't skip:** The tests that validate the fix actually works.

## Test Metrics and Monitoring

### Coverage Metrics (Current)

**Before this project:**
- **Unit test coverage:** ~90% (excellent)
- **Integration test coverage:** ~40% (poor)
- **E2E test coverage:** 0% (critical gap)

**After this project:**
- **Unit test coverage:** ~92% (added fileExists)
- **Integration test coverage:** ~80% (un-skipped tests)
- **E2E test coverage:** 100% of open tool critical paths

### Quality Metrics

**Track:**
- Number of skipped tests (should decrease over time)
- Number of E2E tests (should increase)
- Test execution time (should stay <5 min total)
- Flaky test rate (should be <1%)

**Alert on:**
- New skipped tests added (requires justification)
- E2E coverage drops below 80%
- Test suite takes >10 minutes

## Lessons Learned

### What Went Wrong

1. **Skipped critical tests** - Assumed "we'll implement later" but never did
2. **Mock-heavy unit tests** - High coverage, low integration validation
3. **No test fixtures** - Made E2E tests "too hard" to write
4. **No contract validation** - Database/code contract was implicit, not tested

### What We'll Do Differently

1. **No skipped tests** without tracking and timeline
2. **E2E tests for critical paths** - Even if they're complex to set up
3. **Invest in test infrastructure** - Test fixtures, helpers, utilities
4. **Explicit contracts** - Document and test component interfaces

### The Core Lesson

> **Tests that mock the happy path provide false confidence. Only end-to-end tests validate that the system actually works.**

This bug existed because we tested individual functions perfectly, but never tested them working together with real data.

**MVP mindset applies to tests too:** Test the critical path that matters to users, not every internal function.

## Conclusion

This bug happened because:
- ❌ Critical E2E tests were skipped
- ❌ Unit tests used mocks instead of real data
- ❌ No contract validation between database and code
- ❌ Test infrastructure was too hard to use

The fix includes:
- ✅ Implement the skipped E2E tests
- ✅ Add database pollution scenarios
- ✅ Validate path contracts
- ✅ Build reusable test fixtures

**Result:** This bug type can never happen again.

**Next:** Security review to ensure path validation is bulletproof.
