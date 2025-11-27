# Ticket: TESTFIX-1005: Fix CLI Package Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Fix all 53 failing tests in the CLI package (`packages/cli`). Issues include binary path expectations, message format assertions, and worktree creation in nested test environments.

## Background
The CLI package has 53 failing tests (with 2634 passing). After TESTFIX-1001 adds the vitest.config.ts and removes stale worktrees, the duplicate test count should drop. The remaining failures are due to assertion mismatches where tests expect specific string formats or binary paths that have changed. This is Phase 4 of the TESTFIX project - TypeScript test fixes.

## Acceptance Criteria
- [ ] `pnpm test` in packages/cli passes with 0 failures
- [ ] ScanOrchestrator tests pass (binary path expectations fixed)
- [ ] PreFlightValidator tests pass (message format assertions updated)
- [ ] Search optimization tests pass (genetic-iterator, competition-runner)
- [ ] Variant injection tests pass (worktree creation fixed)
- [ ] No tests skipped without documented justification

## Technical Requirements

**Pattern 1: Binary Path Expectations**
Tests should be flexible about binary paths:
```typescript
// Before (broken) - expects specific binary name
expect(spawn.args[0]).toBe('crewchief-maproom')

// After (fixed) - checks for crewchief binary (any path)
expect(spawn.args[0]).toContain('crewchief')
// or use regex pattern matching
expect(spawn.args[0]).toMatch(/crewchief/)
```

**Pattern 2: Message Format Assertions**
Update assertions to match current message formats:
```typescript
// Before (broken)
expect(result.message).toContain('0 chunks')

// After (fixed) - match actual message
expect(result.message).toContain('Worktree not in database')
// or use toBeTruthy for presence check
expect(result.message.includes('0 chunks') || result.message.includes('not in database')).toBe(true)
```

**Pattern 3: Test Isolation**
Ensure tests don't pollute each other's state:
```typescript
// Add cleanup hooks
afterEach(async () => {
  // Clean up any created worktrees
  await cleanupTestWorktrees()
})

// Use unique identifiers
const testId = `test-${Date.now()}-${Math.random().toString(36).slice(2)}`
```

## Implementation Notes
1. Run `pnpm test` first to get exact failure list after TESTFIX-1001
2. Group failures by test file
3. Fix in order:
   - `src/search-optimization/scan-orchestrator.test.ts` (~3 failures)
   - `src/search-optimization/validation/pre-flight-validator.test.ts` (~5 failures)
   - `tests/search-optimization/genetic-iterator.test.ts` (~6 failures)
   - `tests/sdk/variant-injection.test.ts` (~6 failures)
4. For each test file, understand the test intent before fixing
5. Update assertions to match current behavior while preserving test intent
6. Re-run tests after each file is fixed

## Dependencies
- TESTFIX-1001 (vitest config must exist to prevent duplicate test discovery)
- TESTFIX-1002 (baseline must be documented)

## Risk Assessment
- **Risk**: Some failures may be due to actual bugs, not test drift
  - **Mitigation**: Review carefully; if implementation seems wrong, document and flag for review

- **Risk**: Fixing assertions may hide real issues
  - **Mitigation**: Understand test intent; ensure new assertions still validate the same behavior

- **Risk**: Tests may be flaky after fixes
  - **Mitigation**: Run tests multiple times; add proper async handling

## Files/Packages Affected
- `packages/cli/src/search-optimization/scan-orchestrator.test.ts`
- `packages/cli/src/search-optimization/validation/pre-flight-validator.test.ts`
- `packages/cli/tests/search-optimization/genetic-iterator.test.ts`
- `packages/cli/tests/search-optimization/competition-runner.test.ts`
- `packages/cli/tests/sdk/variant-injection.test.ts`
- Other failing test files as discovered
