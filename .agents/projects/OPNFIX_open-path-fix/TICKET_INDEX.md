# OPNFIX Ticket Index

**Project:** OPNFIX - Open Tool Path Resolution Fix
**Total Tickets:** 15
**Status:** Ready for Implementation

---

## Phase 1: Core Fix Implementation (4-6 hours)

### OPNFIX-1001: Update getWorktreePath Function for Multi-Candidate Fallback
- **File:** `packages/maproom-mcp/src/tools/open.ts`
- **Agent:** general-purpose or vscode-extension-specialist
- **Time:** 2-3 hours
- **Summary:** Modify SQL query to fetch all candidate worktrees, implement filesystem validation loop, and return first valid path.

### OPNFIX-1002: Add fileExists Helper Function to Validation Utils
- **File:** `packages/maproom-mcp/src/utils/validation.ts`
- **Agent:** general-purpose
- **Time:** 1 hour
- **Summary:** Create async helper that uses fs.access() to check file existence and readability.

### OPNFIX-1003: Enhance Error Messages for Path Resolution Failures
- **File:** `packages/maproom-mcp/src/tools/open.ts`
- **Agent:** general-purpose
- **Time:** 1-2 hours
- **Summary:** Update error messages to include candidate count, suggest cleanup commands, and add debug logging.

---

## Phase 2: Security Enhancements (3-4 hours)

### OPNFIX-2001: Add Symlink Validation to File Reading
- **File:** `packages/maproom-mcp/src/tools/open.ts`
- **Agent:** general-purpose
- **Time:** 2 hours
- **Summary:** Detect symlinks using fs.lstat(), resolve targets with fs.realpath(), and validate targets stay within repository.

### OPNFIX-2002: Add Optional Root Validation to getWorktreePath
- **File:** `packages/maproom-mcp/src/tools/open.ts`
- **Agent:** general-purpose
- **Time:** 1-2 hours
- **Summary:** Add optional expectedRoot parameter to skip suspicious abs_path values from database.

---

## Phase 3: Test Suite Implementation (4-6 hours)

### OPNFIX-3001: Create End-to-End Test Suite
- **File:** `packages/maproom-mcp/tests/tools/open.e2e.test.ts` (NEW)
- **Agent:** integration-tester
- **Time:** 3-4 hours
- **Summary:** Implement 5 E2E tests: happy path, polluted DB fallback, all invalid paths, multi-candidate ordering, and security validation.

### OPNFIX-3002: Implement Security Test Suite
- **File:** `packages/maproom-mcp/tests/tools/open.security.test.ts` (NEW)
- **Agent:** integration-tester
- **Time:** 2 hours
- **Summary:** Implement 5 security tests: path traversal in relpath, path traversal in DB, symlink escape, absolute paths, null byte injection.

### OPNFIX-3003: Un-skip Integration Tests
- **File:** `packages/maproom-mcp/tests/tools/open.int.test.ts` (lines 199-207)
- **Agent:** integration-tester
- **Time:** 1-2 hours
- **Summary:** Remove .skip from two integration tests and implement using existing fixtures and database helpers.

### OPNFIX-3004: Add Unit Tests for fileExists Helper
- **File:** `packages/maproom-mcp/tests/utils/validation.test.ts`
- **Agent:** general-purpose
- **Time:** 1 hour
- **Summary:** Create comprehensive unit tests for fileExists() covering all branches and edge cases.

---

## Phase 4: Documentation and Cleanup (2-3 hours)

### OPNFIX-4001: Update Tool Documentation
- **Files:** `packages/maproom-mcp/README.md`, JSDoc in `open.ts`
- **Agent:** general-purpose
- **Time:** Not specified
- **Summary:** Document multi-candidate fallback behavior, error messages, and troubleshooting guide.

### OPNFIX-4002: Add Debug Logging
- **File:** `packages/maproom-mcp/src/tools/open.ts`
- **Agent:** general-purpose
- **Time:** Not specified
- **Summary:** Add debug logs for candidate tries, validation failures, successful resolution, and symlink handling.

### OPNFIX-4003: Update CHANGELOG
- **File:** `packages/maproom-mcp/CHANGELOG.md`
- **Agent:** general-purpose
- **Time:** Not specified
- **Summary:** Document bug fix, new features (symlink validation), and improvements in CHANGELOG.

---

## Phase 5: Verification and Deployment (2-3 hours)

### OPNFIX-5001: Run Full Test Suite
- **Agent:** unit-test-runner
- **Time:** 1 hour
- **Summary:** Execute all unit, integration, and E2E tests. Verify 100% pass rate and coverage >80% for modified files.

### OPNFIX-5002: Manual Verification
- **Agent:** verify-ticket
- **Time:** 1-2 hours
- **Summary:** Manual testing with clean/polluted databases, verify error messages, security validations, and performance <10ms overhead.

### OPNFIX-5003: Build and Package
- **Agent:** general-purpose
- **Time:** 30 minutes - 1 hour
- **Summary:** Run pnpm build, verify no TypeScript/lint errors, create package for deployment.

---

## Execution Order

Tickets must be executed sequentially within phases, but phases can overlap:

```
Phase 1 (Day 1):
  OPNFIX-1001 → OPNFIX-1002 → OPNFIX-1003

Phase 2 (Day 1-2):
  OPNFIX-2001 → OPNFIX-2002

Phase 3 (Day 2):
  OPNFIX-3001 → OPNFIX-3002 → OPNFIX-3003 → OPNFIX-3004

Phase 4 (Day 2-3):
  OPNFIX-4001 → OPNFIX-4002 → OPNFIX-4003

Phase 5 (Day 3):
  OPNFIX-5001 → OPNFIX-5002 → OPNFIX-5003
```

---

## Dependencies

**Phase 1 has no dependencies** - can start immediately

**Phase 2 depends on:**
- OPNFIX-1001 (uses multi-candidate logic)
- OPNFIX-1002 (uses fileExists helper)

**Phase 3 depends on:**
- All Phase 1 tickets (tests validate implementation)
- All Phase 2 tickets (tests validate security features)

**Phase 4 depends on:**
- All Phase 1-3 tickets (documents completed features)

**Phase 5 depends on:**
- All Phase 1-4 tickets (verifies complete solution)

---

## Success Metrics

**Project is complete when:**
- ✅ All 15 tickets verified and committed
- ✅ All tests passing (no skipped tests)
- ✅ Open tool reads files correctly with database data
- ✅ Path resolution handles pollution via fallback
- ✅ Security validations block attacks
- ✅ Error messages are clear and actionable
- ✅ Performance impact <10ms
- ✅ Documentation updated

---

## Quick Reference

**Start Implementation:**
```bash
/single-ticket OPNFIX-1001
```

**Check Progress:**
- View ticket files in `.agents/projects/OPNFIX_open-path-fix/tickets/`
- Check status checkboxes in each ticket
- Review commit history for completed work

**Related Documents:**
- Project Overview: `README.md`
- Implementation Plan: `planning/plan.md`
- Quality Strategy: `planning/quality-strategy.md`
- Security Review: `planning/security-review.md`
- Architecture: `planning/architecture.md`
- Root Cause Analysis: `planning/analysis.md`

---

**Created:** 2025-11-18
**Last Updated:** 2025-11-18
**Total Estimated Time:** 13-18 hours (2-3 days)
