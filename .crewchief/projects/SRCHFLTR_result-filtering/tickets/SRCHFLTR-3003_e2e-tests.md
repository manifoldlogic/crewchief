# Ticket: [SRCHFLTR-3003]: E2E Integration Tests with Real Daemon

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - all tests passing
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create end-to-end tests that validate FilterableSearchResult works correctly with real search results from the daemon, ensuring no data corruption and proper integration.

## Background
E2E tests validate the complete integration: daemon search → FilterableSearchResult wrapper → filtering/sorting/pagination. These tests catch integration issues that unit tests might miss, such as type mismatches or unexpected data formats.

This is the final validation before declaring Phase 3 complete.

## Acceptance Criteria
- [x] E2E test file created
- [x] All 5 E2E tests implemented and passing
- [x] Real daemon search integration tested
- [x] Filter functions on real data tested
- [x] Filter + sort + slice on real data tested
- [x] Backward compatibility verified (existing code works)
- [x] Performance overhead measured (<5ms)
- [x] No data corruption verified
- [x] All tests pass consistently
- [x] Tests can run against local daemon

## Technical Requirements

### Test File Structure
Create: `packages/daemon-client/tests/filterable-result-e2e.test.ts`

### E2E Test Cases
```typescript
import { DaemonClient } from '../src/client'
import { FilterableSearchResult } from '../src/filterable-result'

describe('FilterableSearchResult - E2E Tests', () => {
  let daemon: DaemonClient

  beforeAll(() => {
    daemon = new DaemonClient()
  })

  // Skip if daemon not running (optional - for CI)
  const skipIfNoDaemon = process.env.CI ? test.skip : test

  skipIfNoDaemon('performs real search and filters functions', async () => {
    // Perform real search against crewchief codebase
    const searchResult = await daemon.search({
      query: "function",
      repo: process.env.TEST_REPO || "crewchief"
    })

    expect(searchResult.hits.length).toBeGreaterThan(0)

    // Wrap and filter
    const result = new FilterableSearchResult(searchResult)
    const functions = result.filter({kind: "function"})

    // Verify filtering worked
    expect(functions.hits.length).toBeGreaterThan(0)
    expect(functions.hits.every(h => h.kind === "function")).toBe(true)
    expect(functions.count).toBe(functions.hits.length)
  })

  skipIfNoDaemon('filters TypeScript files on real data', async () => {
    const searchResult = await daemon.search({
      query: "export",
      repo: process.env.TEST_REPO || "crewchief"
    })

    const result = new FilterableSearchResult(searchResult)
    const tsFiles = result.filter({file_type: "ts"})

    // Verify all results are TypeScript files
    expect(tsFiles.hits.every(h => h.file_path.endsWith(".ts"))).toBe(true)
  })

  skipIfNoDaemon('chains filter + sort + slice on real data', async () => {
    const searchResult = await daemon.search({
      query: "auth",
      repo: process.env.TEST_REPO || "crewchief"
    })

    const result = new FilterableSearchResult(searchResult)
    const filtered = result
      .filter({kind: "function", file_type: "ts"})
      .sortBy("relpath")
      .slice(0, 10)

    // Verify chain worked
    expect(filtered.hits.length).toBeLessThanOrEqual(10)
    expect(filtered.hits.every(h =>
      h.kind === "function" && h.file_path.endsWith(".ts")
    )).toBe(true)

    // Verify sorted
    for (let i = 0; i < filtered.hits.length - 1; i++) {
      expect(filtered.hits[i].file_path <= filtered.hits[i + 1].file_path).toBe(true)
    }
  })

  skipIfNoDaemon('maintains backward compatibility', async () => {
    // Existing code pattern - should work unchanged
    const searchResult = await daemon.search({
      query: "test",
      repo: process.env.TEST_REPO || "crewchief"
    })

    // Access hits directly (existing pattern)
    expect(searchResult.hits).toBeDefined()
    expect(Array.isArray(searchResult.hits)).toBe(true)
    expect(searchResult.total).toBeDefined()

    // Can still use SearchResult without FilterableSearchResult
    const firstHit = searchResult.hits[0]
    expect(firstHit.chunk_id).toBeDefined()
    expect(firstHit.file_path).toBeDefined()
    expect(firstHit.score).toBeDefined()
  })

  skipIfNoDaemon('has minimal performance overhead (<5ms)', async () => {
    const searchResult = await daemon.search({
      query: "function",
      repo: process.env.TEST_REPO || "crewchief"
    })

    // Measure filtering overhead
    const start = performance.now()

    const result = new FilterableSearchResult(searchResult)
    const filtered = result
      .filter({kind: "function"})
      .sortBy("relpath")
      .slice(0, 20)

    const elapsed = performance.now() - start

    // Should be very fast (<5ms for typical result sets)
    expect(elapsed).toBeLessThan(5)

    // Verify results are valid
    expect(filtered.hits.length).toBeGreaterThan(0)
  })
})
```

### Test Configuration
Update `packages/daemon-client/package.json` test scripts if needed:

```json
{
  "scripts": {
    "test": "jest",
    "test:unit": "jest --testPathIgnorePatterns=e2e",
    "test:e2e": "jest e2e",
    "test:all": "jest"
  }
}
```

## Implementation Notes

**File**: `packages/daemon-client/tests/filterable-result-e2e.test.ts` (new file)

**Test Strategy**:
- Use real daemon connection (requires daemon running)
- Skip tests in CI if daemon not available (optional)
- Test against real crewchief codebase
- Verify no data corruption or type mismatches
- Measure actual performance with real data

**Test Data**:
- Use real search queries that return results
- Test against crewchief codebase (known data)
- Verify assumptions about result structure

**CI Considerations**:
- E2E tests may need daemon running in CI
- Use `test.skip` or environment variable to skip if needed
- Document daemon setup requirements

## Dependencies
- SRCHFLTR-1001 through SRCHFLTR-2003 (all implementation complete)
- Daemon running locally or in CI

## Risk Assessment
- **Risk**: Tests fail if daemon not running
  - **Mitigation**: Skip tests gracefully in CI, document setup
  - **Severity**: Low
- **Risk**: Tests brittle to codebase changes
  - **Mitigation**: Use flexible assertions, don't hardcode counts
  - **Severity**: Low
- **Risk**: Performance tests flaky
  - **Mitigation**: Generous time budget (5ms), run multiple times
  - **Severity**: Low

## Files/Packages Affected
- `packages/daemon-client/tests/filterable-result-e2e.test.ts` (NEW)
- `packages/daemon-client/package.json` (potentially modify test scripts)

## Verification Notes
Verify the E2E tests by:
1. Ensure daemon is running: `pnpm maproom daemon start`
2. Run E2E tests: `pnpm test:e2e` (or `pnpm test`)
3. Verify all 5 tests pass consistently
4. Run tests multiple times - consistent results
5. Verify backward compatibility test works
6. Verify performance test passes (<5ms)
7. Check no error logs or warnings
8. Verify tests work against real crewchief codebase
9. Run full test suite: `pnpm test` - all tests pass
