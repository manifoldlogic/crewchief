# Ticket: CFGVER-5001: Complete Unit Test Suite

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner
- code-reviewer
- verify-ticket
- commit-ticket

## Summary
Ensure complete unit test coverage for all config-manager.js modules. This ticket consolidates unit testing efforts from previous phases and validates that all core logic meets the 80% coverage target with comprehensive test cases for version detection, file integrity, backups, and edge cases.

## Background
Phase 5 validates all unit tests are complete and coverage targets met before release. Unit tests provide fast, isolated verification of core logic (version comparison, file hashing, backup creation). The quality strategy requires 80%+ code coverage with focus on critical paths (version detection, file integrity, backup/rollback).

Reference: `quality-strategy.md` lines 28-85 for unit test structure and targets.

## Acceptance Criteria
- [ ] All core logic has unit tests (from CFGVER-1901)
- [ ] Code coverage >= 80% for config-manager.js module
- [ ] All tests pass in CI environment
- [ ] Coverage report generated using `npm test -- --coverage`
- [ ] Coverage report reviewed and documented
- [ ] Edge cases documented if not tested (with justification)
- [ ] No high-severity untested code paths remain

## Technical Requirements

**Coverage Targets by Component:**
- Core version logic: 90%+ (critical)
- Backup/rollback: 85%+ (high priority)
- Docker integration: 75%+ (some paths hard to test)
- CLI integration: 70%+ (integration-heavy)

**Test Framework:**
- Use Vitest (already in project)
- Use memfs for isolated file system tests
- Mock Docker commands where appropriate
- All tests must be idempotent (repeatable without side effects)

**Test Categories:**
1. Version Detection Logic (from quality-strategy.md lines 74-88)
   - Missing version file (first run)
   - Version mismatch (old config)
   - Version match (skip update)
   - Corrupted version file (invalid JSON)

2. File Integrity Checking (lines 91-105)
   - Missing files
   - Hash mismatches (corrupted files)
   - All files valid

3. Backup Strategy (lines 108-120)
   - Creates backup with all files
   - Keeps only last 5 backups
   - Cleanup removes oldest backups

4. Update Process
   - Copies all required files
   - Preserves user .env file
   - Updates docker-compose.yml header

**Commands:**
```bash
cd packages/maproom-mcp
npm test -- --coverage
```

**Coverage Report Location:**
`packages/maproom-mcp/coverage/lcov-report/index.html`

## Implementation Notes

**Review Existing Tests:**
First review: `packages/maproom-mcp/tests/config-manager.test.ts`

**Identify Coverage Gaps:**
1. Run coverage report
2. Review uncovered lines in HTML report
3. Prioritize uncovered branches and error paths
4. Add tests for critical uncovered paths
5. Document acceptable coverage exceptions (with justification)

**Testing Strategy:**
- Focus on uncovered branches and error paths
- Use memfs for isolated file system tests (fast, no cleanup needed)
- Mock Docker commands to avoid external dependencies
- Test edge cases: corrupted JSON, permission errors, missing files

**Acceptable Coverage Exceptions:**
Some code paths are hard to test in unit tests:
- OS-specific error handling (document, test manually)
- Timing-dependent failures (test in integration tests)
- Docker daemon communication (test in integration tests)

**Reference Tests from Quality Strategy:**
See `quality-strategy.md` lines 69-121 for detailed test examples.

## Dependencies
- **CFGVER-0001** - Vitest testing infrastructure must be installed first (CRITICAL)
- CFGVER-1901 (initial unit tests created)
- All implementation tickets complete (Phase 1-4)

## Risk Assessment
- **Risk**: Hard-to-test paths force low coverage
  - **Mitigation**: Document untested paths, don't force coverage on unreachable code

- **Risk**: Flaky tests cause CI failures
  - **Mitigation**: Ensure test isolation with memfs, no shared state

- **Risk**: Coverage target too high, blocks release
  - **Mitigation**: 80% is balanced target, focus on critical paths first

## Files/Packages Affected
- **Review**: `packages/maproom-mcp/tests/config-manager.test.ts`
- **Modify**: Add additional test cases as needed
- **Create**: Additional test files if needed for organization
- **Review**: `packages/maproom-mcp/src/config-manager.ts` (coverage analysis)
