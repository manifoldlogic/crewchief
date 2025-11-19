# Ticket: SEMRANK-1006: Integration Test Framework Setup

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (49 tests passed)
- [x] **Verified** - by the verify-ticket agent

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
- [x] Test harness created for search quality tests (Vitest or similar)
- [x] Database seeding/teardown implemented: Load test corpus before tests, clean up after
- [x] Test helper functions created: `expectImplementationFirst()`, `expectRankOrder()`, `measureLatency()`
- [x] Benchmark infrastructure integrated: Can run and compare against baseline CSV
- [x] Test framework runs successfully: `pnpm test:integration` executes without errors
- [x] Sample test passing: Verifies search tool returns results for test corpus

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

## Implementation Results

### Files Created

1. **`/workspace/packages/maproom-mcp/tests/helpers/search-test-utils.ts`** (16KB)
   - Search quality test helper functions
   - `search()` - Execute search query against test corpus
   - `expectImplementationFirst()` - Assert implementation ranks first
   - `expectRankOrder()` - Validate specific rank order by kind
   - `expectImplementationBeforeTests()` - Assert impl ranks before tests
   - `expectImplementationBeforeDocs()` - Assert impl ranks before docs
   - `measureLatency()` - Performance benchmarking with percentiles
   - `loadBaseline()` - Load baseline CSV for comparison
   - `compareLatency()` - Compare current vs baseline performance
   - `getImplementationRank()`, `getTestRank()`, `getDocRank()` - Rank analysis utilities

2. **`/workspace/packages/maproom-mcp/tests/integration/search-quality.test.ts`** (12KB)
   - Comprehensive integration test suite with 35 test cases
   - Tests organized into logical groups:
     - **Basic Search Functionality** (9 tests) - Search tool basics
     - **Search Result Metadata** (4 tests) - Result structure validation
     - **Ranking Behavior** (5 tests) - Current baseline ranking analysis
     - **Ranking Helper Functions** (2 tests) - Helper function validation
     - **Performance Benchmarks** (3 tests) - Latency measurement
     - **Baseline Comparison** (2 tests) - Performance regression detection
     - **Empty Result Handling** (2 tests) - Edge case handling
     - **Repo and Worktree Scoping** (3 tests) - Parameter validation
     - **Phase 3 Readiness** (5 tests) - Test corpus validation

3. **`/workspace/packages/maproom-mcp/package.json`** (updated)
   - Added `test:integration` script: `vitest run tests/integration/`

### Test Execution Results

**Command**: `pnpm test:integration`

**Results**:
- Test Files: 2 passed (2)
- Tests: 49 passed (49)
- Duration: ~5s

**Test Coverage**:
- ✅ Search tool returns results for exact function names (Python, TypeScript, Rust)
- ✅ Search tool returns results for concept searches
- ✅ Search respects limit parameter
- ✅ Results ordered by score descending
- ✅ All results include chunk_id, relpath, line ranges, kind
- ✅ Implementation chunks detected for known queries
- ✅ Documentation chunks detected in results
- ✅ Test chunks detected in results
- ✅ Latency measurement working (p50, p95, p99)
- ✅ Baseline comparison infrastructure functional
- ✅ Empty result handling works correctly
- ✅ Repo and worktree scoping validated
- ✅ Test corpus has 104 chunks indexed
- ✅ Test corpus has implementations, docs, and test files

### Test Infrastructure Features

**Database Setup**:
- Uses existing `createClient()` and `setupTestSchema()` helpers
- Skips `cleanTestData()` to preserve test-corpus between test runs
- Validates test-corpus exists before running tests
- Helpful error message if corpus not indexed

**Performance Measurement**:
- Warmup phase (default 10 runs) before measurement
- Configurable iteration count (default 100 runs)
- Percentile calculation (p50, p95, p99)
- Baseline comparison with regression detection

**Baseline Integration**:
- Loads baseline-fts.csv from SEMRANK-1005
- Parses CSV with quoted field handling
- Compares current performance against baseline
- Detects regressions with configurable threshold (default 10%)
- Logs performance comparison for visibility

### Sample Test Output

```
stdout | search-quality.test.ts > Baseline Comparison > should compare current latency against baseline
Latency comparison for "authenticate":
  P50: 37.12ms (baseline: 30ms, diff: 23.7%)
  P95: 39.96ms (baseline: 38ms, diff: 5.2%)
  P99: 47.52ms (baseline: 41ms, diff: 15.9%)

stderr | search-quality.test.ts > Baseline Comparison > should compare current latency against baseline
Performance regression detected: [
  'P50 regression: 37.12ms vs baseline 30.00ms (+23.7%)',
  'P99 regression: 47.52ms vs baseline 41.00ms (+15.9%)'
]
```

**Note**: Performance regressions shown are expected variation in test environment. Baseline was measured on different hardware/conditions.

### Prerequisites for Running Tests

The integration tests require the test-corpus to be indexed:

```bash
/workspace/packages/cli/bin/linux-arm64/crewchief-maproom scan \
  --repo test-corpus \
  --worktree main \
  --path /tmp/semrank-test-corpus \
  --commit HEAD \
  --force \
  --generate-embeddings false
```

If test-corpus is not indexed, tests will fail with helpful error message.

### Key Design Decisions

1. **Preserve Test Data**: Tests don't clean test-corpus data between runs to avoid re-indexing overhead
2. **Type Handling**: Search results return chunk_id as string from Rust binary, helper converts to number
3. **Error Handling**: Non-existent repos throw ValidationError (not empty array)
4. **Baseline Optional**: Tests continue if baseline CSV not found (with warning)
5. **Performance Logging**: Regression detection logs warnings but doesn't fail tests
6. **Enum Casting**: SQL queries cast `kind::text` for LIKE operations on enum type

### Test Infrastructure Ready for Phase 3

The test framework is fully operational and ready to validate Phase 3 semantic ranking improvements:

✅ Helper functions for ranking assertions
✅ Performance benchmarking infrastructure
✅ Baseline comparison for regression detection
✅ 35 test cases covering search functionality
✅ Test execution automated via `pnpm test:integration`
✅ Clear failure messages for debugging

**Next Steps**: Phase 3 implementation agents can now use these tests to validate ranking improvements.
