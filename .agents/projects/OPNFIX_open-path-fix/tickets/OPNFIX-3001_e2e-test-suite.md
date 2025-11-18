# Ticket: OPNFIX-3001: Create End-to-End Test Suite

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
Create comprehensive end-to-end test suite for the open tool that validates the complete workflow from indexing to file reading, including database pollution scenarios and fallback behavior.

## Background
This ticket implements Phase 3.1 of the OPNFIX project plan. The critical bug in the open tool was never caught because E2E tests were skipped with the comment "implement when test fixtures are available". This ticket addresses that gap by implementing real E2E tests using the existing test infrastructure that was already available (`tests/helpers/database.ts`, `tests/fixtures/sample-repo/`).

These tests will validate:
- Complete workflow: index → search → open → verify content
- Database pollution handling via multi-candidate fallback
- Error handling when all candidates fail
- Deterministic ordering with filesystem validation
- Security validation against path traversal

Reference: `.agents/projects/OPNFIX_open-path-fix/planning/plan.md` - Phase 3, Ticket 3.1

## Acceptance Criteria
- [ ] All 5 E2E test cases are implemented and pass
- [ ] Tests use real database (not mocked) via `setupTestDatabase()`
- [ ] Tests validate actual file content matches expectations
- [ ] Tests cover error cases with appropriate error messages
- [ ] Tests use existing database helpers without duplication (`tests/helpers/database.ts`)
- [ ] Tests leverage fixtures from `tests/fixtures/sample-repo/` for test data
- [ ] No new test infrastructure created (reuse existing helpers only)
- [ ] Happy path test validates complete index → open workflow
- [ ] Polluted database test validates multi-candidate fallback
- [ ] All-invalid-paths test validates error handling
- [ ] Multi-candidate ordering test validates filesystem checks
- [ ] Security test validates path traversal rejection

## Technical Requirements
- **File:** `packages/maproom-mcp/tests/tools/open.e2e.test.ts` (NEW FILE)
- **Testing Framework:** Vitest (existing)
- **Database:** Real PostgreSQL database via test helpers
- **Fixtures:** Use `tests/fixtures/sample-repo/` (already available)
- **Test Helpers:** Import from `tests/helpers/database.ts`:
  - `setupTestDatabase()` - Initialize test database
  - `teardownTestDatabase()` - Cleanup after tests
  - `createTestRepo()` - Create test repository
  - `createTestWorktree()` - Create test worktree
  - `createTestFile()` - Create test files
  - `indexTestFixtures()` - Index fixture data into database
- **Test Isolation:** Each test must clean up properly (no pollution between tests)
- **Assertions:** Use Vitest `expect()` for all assertions

## Implementation Notes

### Test Cases to Implement

**1. Happy Path Test (`should handle full E2E workflow: index → search → open`)**
- Index sample repo files using `indexTestFixtures()`
- Search for a chunk using search tool
- Call open tool with chunk's worktree and relpath
- Verify returned content matches actual file content
- Validates: Complete workflow works with clean database

**2. Polluted Database Test (`should handle database pollution via fallback`)**
- Create multiple worktrees with same name but different `abs_path` values
- First abs_path is invalid (simulates pollution)
- Second abs_path is valid (current worktree)
- Call open tool
- Verify it returns content from valid path (automatic fallback)
- Validates: Multi-candidate fallback works correctly

**3. All Invalid Paths Test (`should provide clear error when all candidates fail`)**
- Create worktrees with invalid `abs_path` values
- Call open tool
- Verify it throws appropriate error
- Verify error message mentions candidate count
- Validates: Error handling when filesystem validation fails for all candidates

**4. Multi-Candidate Ordering Test (`should validate candidates in order`)**
- Create three worktrees with same name
- First: invalid path
- Second: valid path (should be returned)
- Third: also valid but shouldn't be checked
- Call open tool
- Verify second candidate is returned
- Validates: Deterministic ordering (DESC by id) and early return on first valid

**5. Security Test (`should reject path traversal in database abs_path`)**
- Create worktree with malicious `abs_path` containing `../`
- Call open tool
- Verify it rejects the path
- Verify error message is appropriate
- Validates: Security validation against path traversal attacks

### Test Structure Template

```typescript
import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import {
  setupTestDatabase,
  teardownTestDatabase,
  createTestRepo,
  createTestWorktree,
  createTestFile,
  indexTestFixtures
} from '../helpers/database.js'
import { openTool } from '../../src/tools/open.js'

describe('Open Tool E2E Tests', () => {
  beforeEach(async () => {
    await setupTestDatabase()
  })

  afterEach(async () => {
    await teardownTestDatabase()
  })

  it('should handle full E2E workflow: index → search → open', async () => {
    // Implementation here
  })

  it('should handle database pollution via fallback', async () => {
    // Implementation here
  })

  it('should provide clear error when all candidates fail', async () => {
    // Implementation here
  })

  it('should validate candidates in order', async () => {
    // Implementation here
  })

  it('should reject path traversal in database abs_path', async () => {
    // Implementation here
  })
})
```

### Key Implementation Details

1. **Use Existing Infrastructure:**
   - Do NOT create new database setup utilities
   - Do NOT create new test fixtures
   - Reuse everything from `tests/helpers/` and `tests/fixtures/`

2. **Database Pollution Simulation:**
   - Use `createTestWorktree()` multiple times with same name
   - Manually set different `abs_path` values in database
   - First one invalid, second one valid

3. **Content Validation:**
   - Read actual file content with `fs.readFile()`
   - Compare with tool output
   - Use exact string matching

4. **Error Message Validation:**
   - Check error is thrown (`expect(() => ...).toThrow()`)
   - Check error message contains expected text
   - Don't check exact wording (may change)

5. **Cleanup:**
   - `teardownTestDatabase()` runs after each test
   - No manual cleanup needed if using helpers correctly

## Dependencies
- **Prerequisite Tickets:**
  - OPNFIX-1001: Update getWorktreePath Function (must be completed)
  - OPNFIX-1002: Add fileExists Helper Function (must be completed)
- **External Dependencies:**
  - PostgreSQL test database (already available)
  - Test helpers in `tests/helpers/database.ts` (already available)
  - Test fixtures in `tests/fixtures/sample-repo/` (already available)
- **No blockers identified**

## Risk Assessment
- **Risk:** Tests may be flaky due to database timing
  - **Mitigation:** Use proper async/await, ensure cleanup runs, use test database isolation

- **Risk:** Database pollution scenarios may be complex to set up
  - **Mitigation:** Use existing `createTestWorktree()` helper, manual DB updates are acceptable for tests

- **Risk:** Fixtures may not contain needed test data
  - **Mitigation:** Inspect `tests/fixtures/sample-repo/` first, add minimal files if needed

- **Risk:** Test execution time may be slow with real database
  - **Mitigation:** Acceptable for E2E tests, keep test count focused (5 tests only)

## Files/Packages Affected
- **NEW:** `packages/maproom-mcp/tests/tools/open.e2e.test.ts` (create this file)
- **READ:** `packages/maproom-mcp/tests/helpers/database.ts` (import helpers)
- **READ:** `packages/maproom-mcp/tests/fixtures/sample-repo/` (use existing fixtures)
- **READ:** `packages/maproom-mcp/src/tools/open.ts` (tool being tested)
