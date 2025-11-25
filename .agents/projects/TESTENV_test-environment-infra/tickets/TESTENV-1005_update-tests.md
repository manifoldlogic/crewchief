# Ticket: TESTENV-1005: Update tests to use fixtures

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: Run `pnpm test` - all 397 tests should pass after this ticket.

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update existing tests to use the pre-loaded test fixtures instead of attempting to spawn the daemon for indexing. Remove or refactor daemon-dependent logic from fixture-compatible tests.

## Background
Currently, 5 tests fail because they try to use `ensureTestCorpusIndexed()` which attempts to spawn the Rust daemon. With fixtures in place, these tests can query the pre-indexed data directly. This ticket removes the daemon dependency from fixture-compatible tests and marks true E2E tests for Phase 2.

Reference: [plan.md](../planning/plan.md) - Phase 1, Deliverable 5: "Test Updates"

## Acceptance Criteria
- [ ] Remove/refactor `ensureTestCorpusIndexed()` calls from fixture-compatible tests
- [ ] Tests that query indexed data use fixture data instead of spawning daemon
- [ ] True E2E tests (real indexing) are marked with `describe.skipIf(!isDaemonAvailable())`
- [ ] All 392+ currently passing tests continue to pass
- [ ] 5 previously failing tests now pass with fixtures
- [ ] Test file organization clearly separates fixture tests from E2E tests

## Technical Requirements

### Tests to Update

Based on current failure patterns, these tests need attention:

1. **search-quality.test.ts** - Uses `ensureTestCorpusIndexed()`
   - Remove daemon spawning logic
   - Tests should query fixture data directly
   - Add `isDaemonAvailable()` skip for real-indexing tests if any

2. **Any test using `ensureTestCorpusIndexed()`**
   - Replace with fixture assumption (data already loaded by globalSetup)
   - Or convert to `describe.skipIf(!isDaemonAvailable())` if truly needs daemon

### Code Pattern Changes

**Before (daemon-dependent):**
```typescript
beforeAll(async () => {
  await ensureTestCorpusIndexed()
})

it('searches for authenticate', async () => {
  const results = await search(client, 'authenticate')
  expect(results.length).toBeGreaterThan(0)
})
```

**After (fixture-based):**
```typescript
// No beforeAll needed - fixtures loaded by globalSetup

it('searches for authenticate', async () => {
  // Fixture data is already present
  const results = await search(client, 'authenticate')
  expect(results.length).toBeGreaterThan(0)
  expect(results[0].symbol_name).toBe('AuthService') // Deterministic!
})
```

### Test Organization

```
packages/maproom-mcp/tests/
├── integration/
│   ├── search-quality.test.ts     # Uses fixtures - PASSES
│   ├── jsonb-queries.test.ts      # Uses fixtures - PASSES
│   └── ...
├── e2e/
│   └── real-indexing.test.ts      # Requires daemon - SKIPPED unless daemon available
└── helpers/
    └── daemon.ts                   # isDaemonAvailable(), etc.
```

### Test Classification

| Test File | Type | Daemon Required? | Action |
|-----------|------|------------------|--------|
| `search-quality.test.ts` | Integration | No (use fixtures) | Remove daemon logic |
| `jsonb-queries.test.ts` | Integration | No (use fixtures) | No change needed |
| `schema-validation.test.ts` | Schema | No | No change needed |
| Real indexing tests | E2E | Yes | Add skipIf condition |

## Implementation Notes

1. **Don't delete daemon logic entirely** - Move it to E2E tests that truly need it

2. **Make tests deterministic** - With fixtures, we know exact expected results. Update assertions:
   ```typescript
   // Before: vague assertion
   expect(results.length).toBeGreaterThan(0)

   // After: deterministic assertion
   expect(results[0].symbol_name).toBe('AuthService')
   ```

3. **Check for `ensureTestCorpusIndexed`** usage:
   ```bash
   grep -r "ensureTestCorpusIndexed" packages/maproom-mcp/tests/
   ```

4. **Preserve E2E capability** - Don't remove daemon-related helpers; they're needed for Phase 2

5. **Update test descriptions** - Make it clear which tests use fixtures:
   ```typescript
   describe('Search Quality (fixtures)', () => {
     // ...
   })
   ```

## Dependencies
- TESTENV-1001 (test corpus)
- TESTENV-1002 (fixture script)
- TESTENV-1003 (generated fixtures)
- TESTENV-1004 (fixture integration)

## Risk Assessment
- **Risk**: Breaking currently passing tests
  - **Mitigation**: Run test suite before and after each change
- **Risk**: Removing logic needed for E2E tests
  - **Mitigation**: Move to E2E test file, don't delete
- **Risk**: Tests become less comprehensive
  - **Mitigation**: Fixtures provide deterministic coverage; E2E tests remain for real validation

## Files/Packages Affected
- `packages/maproom-mcp/tests/integration/search-quality.test.ts` (MODIFY)
- `packages/maproom-mcp/tests/helpers/daemon.ts` (MODIFY or CREATE)
- Other test files using `ensureTestCorpusIndexed()` (MODIFY)
