# Implementation Plan: Open Tool Path Resolution Fix

**Date:** 2025-11-18
**Project:** OPNFIX - Open Tool Path Resolution Fix
**Timeline:** 2-3 days (revised from 3-5 days after identifying existing test infrastructure)
**Scope:** Fix open tool path resolution bug with comprehensive testing

## Project Phases

### Phase 1: Core Fix Implementation (Day 1)

**Goal:** Fix the getWorktreePath bug to handle database pollution gracefully.

#### Ticket 1.1: Update getWorktreePath Function
**File:** `packages/maproom-mcp/src/tools/open.ts`
**Agent:** vscode-extension-specialist or general-purpose

**Tasks:**
1. Modify SQL query to remove `LIMIT 1`
2. Add `ORDER BY w.id DESC` for deterministic ordering
3. Implement loop through all candidates
4. Add filesystem validation for each candidate
5. Return first valid abs_path
6. Enhance error message with candidate count

**Acceptance Criteria:**
- Function returns multiple candidate rows from database
- Validates file existence for each candidate in order
- Returns first valid worktree path
- Throws detailed error if all candidates fail

#### Ticket 1.2: Add fileExists Helper Function
**File:** `packages/maproom-mcp/src/utils/validation.ts`
**Agent:** general-purpose

**Tasks:**
1. Create async `fileExists(path: string): Promise<boolean>`
2. Use `fs.access()` with `R_OK` flag
3. Return true if accessible, false otherwise
4. Add JSDoc documentation
5. Export function

**Acceptance Criteria:**
- Function correctly detects existing files
- Function returns false for non-existent paths
- Function returns false for inaccessible files
- No throwing errors (returns boolean)

#### Ticket 1.3: Enhance Error Messages
**File:** `packages/maproom-mcp/src/tools/open.ts:51-85`
**Agent:** general-purpose

**Tasks:**
1. Update error message to include candidate count
2. Add suggestion to run `maproom db cleanup-stale`
3. Add debug logging for each candidate tried
4. Include available worktrees in error when applicable

**Acceptance Criteria:**
- Error messages are actionable and clear
- Errors mention database pollution when detected
- Debug logs show each path attempted
- No sensitive path information in errors

**Time Estimate:** 4-6 hours

---

### Phase 2: Security Enhancements (Day 2)

**Goal:** Add security validations for symlinks and path boundaries.

#### Ticket 2.1: Add Symlink Validation
**File:** `packages/maproom-mcp/src/tools/open.ts:96-129`
**Agent:** general-purpose

**Tasks:**
1. Use `fs.lstat()` to detect symlinks
2. Use `fs.realpath()` to resolve symlink target
3. Validate target path with `validateWithinRepo()`
4. Add debug logging for symlink following
5. Allow symlinks within repository boundaries

**Acceptance Criteria:**
- Symlinks are detected before reading
- Symlink targets are validated against repository root
- Symlinks outside repository are rejected
- Symlinks within repository are allowed
- Debug logs show symlink resolution

#### Ticket 2.2: Add Optional Root Validation
**File:** `packages/maproom-mcp/src/tools/open.ts:51-85`
**Agent:** general-purpose

**Tasks:**
1. Add optional `expectedRoot` parameter to `getWorktreePath()`
2. Skip candidates with abs_path outside expectedRoot
3. Log warnings when abs_path is suspicious
4. Make parameter optional (backward compatible)

**Acceptance Criteria:**
- Optional parameter doesn't break existing calls
- Validation skips suspicious abs_path values
- Warnings logged for debugging
- Works with and without parameter

**Time Estimate:** 3-4 hours

---

### Phase 3: Test Suite Implementation (Days 3-4)

**Goal:** Implement comprehensive E2E and security tests.

#### Ticket 3.1: Create End-to-End Test Suite
**File:** `packages/maproom-mcp/tests/tools/open.e2e.test.ts` (new file)
**Agent:** integration-tester or general-purpose

**Tasks:**
1. Import existing test helpers from `tests/helpers/database.ts`:
   - `setupTestDatabase()`, `teardownTestDatabase()`
   - `createTestRepo()`, `createTestWorktree()`, `createTestFile()`
   - `indexTestFixtures()`
2. Use existing fixtures from `tests/fixtures/sample-repo/` for test data
3. Implement happy path E2E test (index → search → open)
4. Implement polluted database test (multiple worktrees with same name, different abs_path)
5. Implement all-invalid-paths test (error handling)
6. Implement multi-candidate fallback test (validates ordering and filesystem checks)

**Test Cases:**
- ✅ Happy path: Index file → Open → Verify content matches
- ✅ Polluted DB: Multiple worktrees → Opens correctly via fallback
- ✅ All invalid: Wrong paths → Clear error message
- ✅ Ordering: Three worktrees → Second one works → Returns it
- ✅ Security: Path traversal in DB → Rejected

**Acceptance Criteria:**
- All 5 E2E tests pass
- Tests use real database (not mocked)
- Tests validate actual file content
- Tests cover error cases
- Uses existing database helpers without duplication
- Leverages fixtures from tests/fixtures/sample-repo/
- No new test infrastructure created (reuse only)

#### Ticket 3.2: Implement Security Test Suite
**File:** `packages/maproom-mcp/tests/tools/open.security.test.ts` (new file)
**Agent:** integration-tester or general-purpose

**Tasks:**
1. Test path traversal in relpath parameter
2. Test path traversal in database abs_path
3. Test symlink pointing outside repository
4. Test absolute path in relpath
5. Test null byte injection

**Acceptance Criteria:**
- All 5 security tests pass
- Tests verify rejections happen
- Tests verify error messages are appropriate
- Tests don't leak sensitive information

#### Ticket 3.3: Un-skip Integration Tests
**File:** `packages/maproom-mcp/tests/tools/open.int.test.ts:199-207`
**Agent:** integration-tester or general-purpose

**Tasks:**
1. Remove `.skip` from line 199 (filesystem read test)
2. Remove `.skip` from line 205 (git history read test)
3. Use existing fixtures from `tests/fixtures/sample-repo/` - they ARE available
4. Leverage `indexTestFixtures()` from `tests/helpers/database.ts` to populate test data
5. Implement full workflow tests using existing database helpers
6. Verify tests pass with real database and filesystem

**Acceptance Criteria:**
- Both previously-skipped tests now run
- Tests use real database and filesystem
- Tests validate end-to-end workflow
- No tests are skipped in this file
- Uses existing test fixtures and helpers (no new infrastructure created)

#### Ticket 3.4: Add Unit Tests for New Functions
**File:** `packages/maproom-mcp/tests/utils/validation.test.ts`
**Agent:** unit-test-runner or general-purpose

**Tasks:**
1. Add tests for `fileExists()` function
2. Test existing file returns true
3. Test non-existent file returns false
4. Test inaccessible file returns false
5. Test directory returns false (should be file)

**Acceptance Criteria:**
- Full coverage of `fileExists()` function
- Tests cover happy and error paths
- Tests are fast (no integration dependencies)

**Time Estimate:** 4-6 hours (revised from 8-10 hours due to existing test infrastructure reuse)

---

### Phase 4: Documentation and Cleanup (Day 4-5)

**Goal:** Document changes, update logs, prepare for deployment.

#### Ticket 4.1: Update Tool Documentation
**Files:**
- `packages/maproom-mcp/README.md`
- `packages/maproom-mcp/src/tools/open.ts` (JSDoc)
**Agent:** general-purpose

**Tasks:**
1. Document new behavior (multi-candidate fallback)
2. Document error messages and meanings
3. Add troubleshooting guide for path errors
4. Update JSDoc for modified functions

**Acceptance Criteria:**
- README explains open tool behavior
- JSDoc is complete and accurate
- Troubleshooting guide covers common issues

#### Ticket 4.2: Add Debug Logging
**File:** `packages/maproom-mcp/src/tools/open.ts`
**Agent:** general-purpose

**Tasks:**
1. Add debug log when trying each candidate
2. Add debug log for validation failures
3. Add debug log for successful path resolution
4. Add debug log for symlink resolution
5. Ensure no sensitive data in logs

**Acceptance Criteria:**
- Debug logs help troubleshoot path issues
- Logs show decision-making process
- No sensitive paths in production logs
- Logs are at appropriate levels (debug/info/warn/error)

#### Ticket 4.3: Update CHANGELOG
**File:** `packages/maproom-mcp/CHANGELOG.md`
**Agent:** general-purpose

**Tasks:**
1. Add entry for this fix
2. Describe bug that was fixed
3. List new features (symlink validation)
4. Note breaking changes (none)

**Acceptance Criteria:**
- CHANGELOG entry is clear and complete
- Users understand what changed
- Follows existing CHANGELOG format

**Time Estimate:** 2-3 hours

---

### Phase 5: Verification and Deployment (Day 5)

**Goal:** Verify fix works, run full test suite, deploy.

#### Ticket 5.1: Run Full Test Suite
**Agent:** unit-test-runner

**Tasks:**
1. Run unit tests: `pnpm test`
2. Run integration tests: `pnpm test:integration`
3. Run E2E tests: `pnpm test:e2e`
4. Verify all tests pass
5. Check test coverage metrics

**Acceptance Criteria:**
- All unit tests pass
- All integration tests pass
- All E2E tests pass
- No skipped tests remain
- Coverage meets minimum thresholds

#### Ticket 5.2: Manual Verification
**Agent:** verify-ticket or general-purpose

**Tasks:**
1. Test with clean database (normal workflow)
2. Test with polluted database (fallback works)
3. Test error messages are clear
4. Test security validations work
5. Verify performance impact acceptable

**Acceptance Criteria:**
- Happy path works end-to-end
- Fallback recovers from pollution
- Error messages are helpful
- Security tests block attacks
- Performance degradation <10ms

#### Ticket 5.3: Build and Package
**Agent:** general-purpose

**Tasks:**
1. Run `pnpm build`
2. Verify no TypeScript errors
3. Verify no lint errors
4. Create package if needed
5. Tag version if appropriate

**Acceptance Criteria:**
- Build completes without errors
- Linting passes
- Package is created successfully

**Time Estimate:** 2-3 hours

---

## Timeline Summary

| Phase | Duration | Tickets | Agent |
|-------|----------|---------|-------|
| Phase 1: Core Fix | 4-6 hours | 3 | general-purpose |
| Phase 2: Security | 3-4 hours | 2 | general-purpose |
| Phase 3: Tests | 4-6 hours | 4 | integration-tester |
| Phase 4: Documentation | 2-3 hours | 3 | general-purpose |
| Phase 5: Verification | 2-3 hours | 3 | verify-ticket |
| **Total** | **13-18 hours** | **15 tickets** | **~2-3 days** |

**Note:** Timeline reduced from original 19-26 hours (3-5 days) after identifying existing test infrastructure that can be reused.

## Agent Assignment

**Primary Agent:** general-purpose or vscode-extension-specialist
- Best suited for TypeScript/Node.js code changes
- Has access to all necessary tools
- Can handle both implementation and testing

**Secondary Agent:** integration-tester
- For comprehensive E2E test suite creation
- Specialized in test implementation

**Verification Agent:** verify-ticket
- Final verification against acceptance criteria
- Ensures all requirements met

## Dependencies and Blockers

**External Dependencies:**
- PostgreSQL database (already available)
- Test database setup (already available)
- No new dependencies needed

**Blockers:**
- None identified

**Risks:**
- Test implementation may take longer than estimated
- Database pollution scenarios may be complex to reproduce
- Symlink handling may have edge cases

**Mitigation:**
- Start with core fix (Phase 1) immediately
- Security enhancements (Phase 2) can proceed in parallel
- Tests (Phase 3) validate both phases

## Success Criteria

**This project is complete when:**

- ✅ Open tool successfully reads files with real database data
- ✅ Path resolution handles database pollution automatically
- ✅ All new and existing tests pass
- ✅ No skipped integration tests remain
- ✅ Security validations block traversal attacks
- ✅ Error messages are clear and actionable
- ✅ Performance impact is <10ms per operation
- ✅ Documentation is updated
- ✅ Code is reviewed and approved

## Rollback Plan

**If critical bug found after deployment:**

1. **Immediate:** Revert to previous version
   - Simple git revert of changes
   - Redeploy previous package version

2. **Investigation:** Identify what broke
   - Check logs for errors
   - Review test failures
   - Identify root cause

3. **Fix Forward:** Implement proper fix
   - Create new ticket for bug
   - Implement fix with tests
   - Re-deploy when ready

**Rollback is low-risk** - no database migrations, pure code change.

## Post-Deployment Monitoring

**Metrics to track for 7 days:**

1. **Error Rate:**
   - Track `FILE_NOT_FOUND` errors
   - Track `INVALID_PATH` errors
   - Expected: Decrease in errors

2. **Success Rate:**
   - Track successful open tool calls
   - Expected: Increase to >99%

3. **Performance:**
   - Track open tool latency
   - Expected: Increase <10ms

4. **Fallback Usage:**
   - Track how often multiple candidates are tried
   - High number indicates database pollution
   - Informs priority of Project 3 (Index Cleanup)

**Alert if:**
- Error rate increases by >10%
- Success rate drops below 95%
- Latency increases by >20ms
- Critical security errors occur

## Next Actions

1. **Review and approve this plan**
2. **Create ticket files in `tickets/` directory**
3. **Assign to appropriate agent**
4. **Begin with Phase 1 (Core Fix)**
5. **Run `/single-ticket` for each ticket sequentially**

**Ready to execute!**
