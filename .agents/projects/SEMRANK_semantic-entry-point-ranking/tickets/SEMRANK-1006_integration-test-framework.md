# Ticket: SEMRANK-1006: Integration Test Framework Setup

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
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Set up test harness for search quality tests, database seeding/teardown for tests, benchmark infrastructure for latency tracking.

## Background
Phase 3 will implement semantic ranking features that must be validated through automated integration tests. This ticket creates the test infrastructure needed to:
1. Run search quality tests against the test corpus
2. Verify implementations rank above tests/docs
3. Compare performance against baseline metrics

The test framework will use the test corpus (SEMRANK-1004) and baseline metrics (SEMRANK-1005) to provide automated validation throughout Phase 3 implementation.

This ticket completes the Phase 1 test infrastructure setup.

## Acceptance Criteria
- [ ] Test harness created for search quality tests (Vitest or similar)
- [ ] Database seeding/teardown implemented: Load test corpus before tests, clean up after
- [ ] Test helper functions created: `expectImplementationFirst()`, `expectRankOrder()`, `measureLatency()`
- [ ] Benchmark infrastructure integrated: Can run and compare against baseline CSV
- [ ] Test framework runs successfully: `pnpm test:integration` executes without errors
- [ ] Sample test passing: Verifies search tool returns results for test corpus

## Technical Requirements
- **Testing Framework**: Use Vitest (existing maproom test framework)
- **Test Location**: `/packages/maproom-mcp/tests/integration/search-quality.test.ts`
- **Database Setup**:
  - `beforeAll()`: Ensure test corpus indexed
  - `afterAll()`: Optional cleanup (or leave for manual inspection)
- **Helper Functions**:
  ```typescript
  async function expectImplementationFirst(query: string) {
    const results = await search({ query, repo: 'test-corpus' });
    expect(results[0].kind).toBe('func');
    expect(results[0].relpath).not.toMatch(/test/);
  }

  async function expectRankOrder(query: string, expectedKinds: string[]) {
    const results = await search({ query, repo: 'test-corpus' });
    const actualKinds = results.slice(0, expectedKinds.length).map(r => r.kind);
    expect(actualKinds).toEqual(expectedKinds);
  }

  async function measureLatency(query: string, runs: number = 100) {
    // Returns { p50, p95, p99 }
  }
  ```
- **Benchmark Integration**: Load baseline CSV, compare current latencies

## Implementation Notes
- Follow existing test patterns in `packages/maproom-mcp/tests/`
- Use test corpus indexed in SEMRANK-1004
- Create reusable test utilities for Phase 3 integration tests
- Document test setup in README or test file comments
- Ensure tests can run independently and in parallel
- Use same database connection as MCP server for consistency

## Dependencies
- SEMRANK-1004 (test corpus must be indexed)
- SEMRANK-1005 (baseline metrics needed for comparison)

## Risk Assessment
- **Risk**: Test framework configuration issues
  - **Mitigation**: Follow existing Vitest patterns closely
- **Risk**: Database connection issues in tests
  - **Mitigation**: Use same connection string as MCP server

## Files/Packages Affected
- `/packages/maproom-mcp/tests/integration/search-quality.test.ts` (new)
- `/packages/maproom-mcp/tests/helpers/search-test-utils.ts` (new)
- `/packages/maproom-mcp/package.json` (add test:integration script if needed)
