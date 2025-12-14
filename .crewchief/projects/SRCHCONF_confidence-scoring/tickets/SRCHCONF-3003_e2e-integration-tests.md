# Ticket: [SRCHCONF-3003]: End-to-End Integration Tests

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
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
Create end-to-end integration tests that exercise the full confidence scoring pipeline from MCP tool call through daemon to Rust search and back, validating correctness, backward compatibility, and performance.

## Background
This is the final validation for Phase 3. End-to-end tests prove that confidence scoring works correctly through the entire stack: MCP tool → daemon client → Rust daemon → search pipeline → confidence computation → JSON serialization → TypeScript deserialization → MCP response.

These tests complement unit tests (Phase 1) and integration tests (Phase 2) with full-system validation.

## Acceptance Criteria
- [x] Minimum 4 end-to-end tests created in `packages/maproom-mcp/tests/integration/confidence.test.ts`
- [x] Test 1: Search with include_confidence=true returns confidence signals
- [x] Test 2: Search without include_confidence works (backward compatibility)
- [x] Test 3: Confidence signals have correct values (high confidence scenario)
- [x] Test 4: Confidence signals have correct values (low confidence scenario)
- [x] All tests pass (`pnpm test packages/maproom-mcp`)
- [x] Tests use real daemon (not mocked)
- [x] Tests use test database with indexed sample code
- [x] Performance overhead validated (<5ms from benchmark in SRCHCONF-1003)

## Technical Requirements
**Test Setup**:
- Use actual daemon process (spawn/connect)
- Create test database with indexed sample code
- ~50 chunks across 5 files for representative results
- Include exact match scenarios and partial matches

**Test Cases**:

1. **Confidence Enabled**:
```typescript
it('should return confidence when include_confidence=true', async () => {
  const result = await search({
    query: 'authenticate',
    repo: 'test-repo',
    include_confidence: true
  });

  expect(result.hits.length).toBeGreaterThan(0);
  expect(result.hits[0].confidence).toBeDefined();
  expect(result.hits[0].confidence.source_count).toBeGreaterThan(0);
  expect(result.hits[0].confidence.score_gap).toBeGreaterThanOrEqual(0.0);
  expect(typeof result.hits[0].confidence.is_exact_match).toBe('boolean');
});
```

2. **Backward Compatibility**:
```typescript
it('should work without include_confidence parameter', async () => {
  const result = await search({
    query: 'authenticate',
    repo: 'test-repo'
    // include_confidence omitted
  });

  expect(result.hits.length).toBeGreaterThan(0);
  expect(result.hits[0].confidence).toBeUndefined();
});
```

3. **High Confidence Scenario**:
```typescript
it('should show high confidence for exact match', async () => {
  // Query for known function with exact name
  const result = await search({
    query: 'authenticate_user',  // Exact function name in test data
    repo: 'test-repo',
    include_confidence: true
  });

  const topResult = result.hits[0];
  expect(topResult.confidence.is_exact_match).toBe(true);
  expect(topResult.confidence.source_count).toBeGreaterThanOrEqual(2);  // Multiple sources
});
```

4. **Low Confidence Scenario**:
```typescript
it('should show lower confidence for ambiguous query', async () => {
  // Query with multiple weak matches
  const result = await search({
    query: 'test',  // Common word, many partial matches
    repo: 'test-repo',
    include_confidence: true
  });

  // Results exist but confidence signals show ambiguity
  expect(result.hits.length).toBeGreaterThan(0);
  expect(result.hits[0].confidence.score_gap).toBeLessThan(1.0);  // Close scores
});
```

## Implementation Notes
Follow testing strategy from quality-strategy.md:
- Focus on critical paths (full stack correctness)
- Test backward compatibility explicitly
- Validate confidence signals make sense for test data
- Use deterministic test data for reproducibility

Test data requirements:
- Known exact match case (function with unique name)
- Ambiguous query case (common word like "test")
- Multi-source result case (function that matches FTS, vector, graph)
- Edge cases (single result, empty results)

Setup/teardown:
```typescript
beforeAll(async () => {
  // Start daemon with test database
  await indexTestRepository();
});

afterAll(async () => {
  // Clean up test database
  await cleanupTestData();
});
```

## Dependencies
- **Prerequisite**: SRCHCONF-3001 (MCP tool integration must be complete)
- **Prerequisite**: SRCHCONF-2002 (Rust integration must be complete)
- **Prerequisite**: All Phase 1-2 tickets complete

## Risk Assessment
- **Risk**: Tests may be flaky due to daemon timing issues
  - **Mitigation**: Use proper async/await, add timeout handling, ensure daemon ready before tests run.
- **Risk**: Test data may not trigger expected confidence scenarios
  - **Mitigation**: Craft test data carefully, validate signals match expectations, document test data structure.
- **Risk**: Tests pass but don't validate critical paths
  - **Mitigation**: Minimum 4 tests specified covering key scenarios, verify-ticket checks coverage.

## Files/Packages Affected
- `packages/maproom-mcp/tests/integration/confidence.test.ts` - NEW integration test file
- `packages/maproom-mcp/tests/fixtures/` - May need test data files
- `packages/maproom-mcp/jest.config.js` - Verify integration test configuration

## Verification Notes
The verify-ticket agent should check:
- All 4+ tests pass when run via `pnpm test`
- Test output shows individual test results
- Tests use real daemon (check for spawn/connect calls, not mocks)
- High confidence scenario shows is_exact_match=true
- Low confidence scenario shows smaller score_gap
- Backward compatibility test confirms confidence=undefined
- No test flakiness (run tests 3 times to verify consistency)
- Performance target met (<5ms overhead, validated from Phase 1 benchmark)
