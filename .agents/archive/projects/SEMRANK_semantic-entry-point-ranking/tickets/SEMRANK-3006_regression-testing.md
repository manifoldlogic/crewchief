# Ticket: SEMRANK-3006: Regression Testing

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - all 11 regression tests passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create regression tests for known failure cases from analysis.md to ensure they remain fixed after implementation, validating that specific problems identified during analysis are resolved by semantic ranking.

## Background
During the SEMRANK project analysis phase, specific search failures were documented that motivated the semantic ranking implementation. These known failures represent real-world problems users encountered:
- "validate_provider" returns tests first (not implementation)
- "authentication" returns docs before AuthService implementation
- Case variations don't match (Authenticate vs authenticate)
- Multi-word queries fail to find snake_case symbols

This ticket implements regression validation from Phase 3 of the SEMRANK execution plan (plan.md, lines 218-224). The goal is to create tests that:
- **Fail on baseline FTS** (demonstrating the original problem)
- **Pass with semantic ranking** (demonstrating the fix)

This ensures we don't regress back to the original poor ranking behavior and validates that the implementation actually solves the documented problems.

## Acceptance Criteria
- [x] Test: Implementations rank before tests for exact symbol match - PASS (2 tests)
- [x] Test: Implementations rank before documentation for concept queries - PASS (2 tests)
- [x] Test: Case variations match (authenticate, Authenticate, AUTHENTICATE all return same #1 result) - PASS
- [x] Test: Multi-word queries work ("user authentication", "database connection") - PASS (2 tests)
- [x] Test: Acronym normalization works (validateToken, useAuth, etc.) - PASS (2 tests)
- [x] All regression tests pass with semantic ranking enabled - 11/11 PASS
- [x] Document before/after behavior in test comments - included in test code
- [x] Create regression test report - `/packages/maproom-mcp/tests/results/regression-validation.md`

## Technical Requirements
- Create new test file: `/packages/maproom-mcp/tests/integration/regression.test.ts`
- Reference specific failures from project analysis.md
- Include before/after comparisons (baseline vs semantic ranking behavior)
- Use test corpus chunks that reproduce the documented failures
- Test with semantic ranking enabled (current implementation)
- Optionally test against baseline query (if easy to toggle) to demonstrate fix
- Document expected vs actual results for each regression case

## Implementation Notes

### Known Failure Cases to Test

Based on typical FTS ranking issues (reference project analysis.md for actual documented cases):

#### 1. Implementation vs Test Ranking
```typescript
describe('Implementation vs Test ranking', () => {
  it('validate_provider returns implementation first', async () => {
    // Known failure: Test file ranked higher than implementation
    const results = await search({ query: 'validate_provider', limit: 5 });

    // Expected: Implementation is #1 result
    expect(results[0].kind).toBe('function'); // or appropriate kind
    expect(results[0].symbol_name).toBe('validate_provider');

    // Test should rank lower
    const testIndex = results.findIndex(r => r.kind === 'test');
    expect(testIndex).toBeGreaterThan(0);
  });
});
```

#### 2. Implementation vs Documentation Ranking
```typescript
describe('Implementation vs Documentation ranking', () => {
  it('authentication returns AuthService before docs', async () => {
    // Known failure: Documentation mentions ranked higher than actual implementation
    const results = await search({ query: 'authentication', limit: 5 });

    // Expected: AuthService implementation is #1
    const authServiceIndex = results.findIndex(r =>
      r.symbol_name === 'AuthService' && r.kind === 'class'
    );
    expect(authServiceIndex).toBe(0);

    // Docs should rank lower
    const docIndex = results.findIndex(r => r.kind === 'doc');
    expect(docIndex).toBeGreaterThan(authServiceIndex);
  });
});
```

#### 3. Case Sensitivity
```typescript
describe('Case-insensitive exact match', () => {
  it('case variations return same top result', async () => {
    const queries = ['Authenticate', 'authenticate', 'AUTHENTICATE', 'AuThEnTiCaTe'];

    const allResults = await Promise.all(
      queries.map(q => search({ query: q, limit: 1 }))
    );

    // All should return same #1 result
    const topChunks = allResults.map(r => r[0].chunk_id);
    expect(new Set(topChunks).size).toBe(1); // All same chunk

    // All should have exact match multiplier applied
    allResults.forEach(([result]) => {
      expect(result.score).toBeGreaterThan(baselineScore * 2.5); // At least kind+exact
    });
  });
});
```

#### 4. Multi-Word Queries
```typescript
describe('Multi-word query normalization', () => {
  it('HTTP handler finds http_handler', async () => {
    // Known failure: Space-separated query doesn't match snake_case
    const results = await search({ query: 'HTTP handler', limit: 5 });

    // Expected: http_handler symbol ranks high
    const handlerIndex = results.findIndex(r =>
      r.symbol_name?.toLowerCase().includes('http_handler')
    );
    expect(handlerIndex).toBeGreaterThanOrEqual(0);
    expect(handlerIndex).toBeLessThan(3); // Top 3 results
  });

  it('database connection finds database_connection', async () => {
    const results = await search({ query: 'database connection', limit: 5 });

    const dbConnIndex = results.findIndex(r =>
      r.symbol_name?.toLowerCase().includes('database_connection')
    );
    expect(dbConnIndex).toBeGreaterThanOrEqual(0);
    expect(dbConnIndex).toBeLessThan(3);
  });
});
```

#### 5. Acronym Normalization
```typescript
describe('Acronym query handling', () => {
  it('XML parser finds xml_parser', async () => {
    // Known failure: Acronym case doesn't match snake_case
    const results = await search({ query: 'XMLParser', limit: 5 });

    const parserIndex = results.findIndex(r =>
      r.symbol_name === 'xml_parser'
    );
    expect(parserIndex).toBe(0); // Exact match should be #1
  });

  it('HTTP client finds https_handler', async () => {
    const results = await search({ query: 'HTTPSHandler', limit: 5 });

    const handlerIndex = results.findIndex(r =>
      r.symbol_name === 'https_handler'
    );
    expect(handlerIndex).toBe(0);
  });
});
```

### Baseline Comparison (Optional)

If feasible, create a "baseline mode" to demonstrate the fix:

```typescript
describe('Baseline comparison', () => {
  it('validates fix compared to baseline', async () => {
    // Toggle semantic ranking off (if supported)
    const baselineResults = await search({
      query: 'validate_provider',
      useSemanticRanking: false
    });

    // Toggle semantic ranking on
    const semanticResults = await search({
      query: 'validate_provider',
      useSemanticRanking: true
    });

    // Baseline: test ranks first (the bug)
    expect(baselineResults[0].kind).toBe('test');

    // Semantic: implementation ranks first (the fix)
    expect(semanticResults[0].kind).toBe('function');
  });
});
```

**Note:** Only implement if easy to toggle. Otherwise, document baseline behavior in comments.

### Regression Test Report

Create `/packages/maproom-mcp/tests/results/regression-validation.md`:

```markdown
# Regression Test Validation Report

## Summary
- Total known failures tested: X
- Failures fixed by semantic ranking: Y
- Failures remaining: Z
- Test pass rate: 100%

## Known Failure Cases

### 1. validate_provider (Implementation vs Test)
- **Original Problem:** Test file ranked #1, implementation ranked #3
- **Root Cause:** Tests mention function name frequently, boosting FTS score
- **Fix Applied:** Kind multiplier (func: 2.5×, test: 0.6×) + exact match (3.0×)
- **Result:** Implementation now ranks #1
- **Test Status:** ✓ Pass

### 2. authentication (Implementation vs Documentation)
- **Original Problem:** Documentation guide ranked #1, AuthService ranked #4
- **Root Cause:** Docs have many mentions of "authentication" keyword
- **Fix Applied:** Kind multiplier (class: 2.5×, doc: 0.3×)
- **Result:** AuthService now ranks #1
- **Test Status:** ✓ Pass

### 3. Case Sensitivity
- **Original Problem:** "Authenticate" vs "authenticate" returned different results
- **Root Cause:** FTS query case-sensitive, no normalization
- **Fix Applied:** Case-insensitive exact match (LOWER(symbol_name) = LOWER(query))
- **Result:** All case variations return same #1 result
- **Test Status:** ✓ Pass

### 4. Multi-Word Queries
- **Original Problem:** "HTTP handler" didn't match "http_handler" symbol
- **Root Cause:** No query normalization for spaces → underscores
- **Fix Applied:** Query normalization in TypeScript (spaces → underscores)
- **Result:** Multi-word queries now match snake_case symbols
- **Test Status:** ✓ Pass

### 5. Acronyms
- **Original Problem:** "XMLParser" didn't match "xml_parser"
- **Root Cause:** No camelCase → snake_case normalization
- **Fix Applied:** Acronym-aware normalization (XMLParser → xml_parser)
- **Result:** Acronym queries now match snake_case symbols
- **Test Status:** ✓ Pass

## Conclusion
All known failure cases from analysis.md are now resolved. Regression tests pass 100%.
```

## Dependencies
- SEMRANK-1006 (integration test framework)
- SEMRANK-2003 (kind multiplier implementation)
- SEMRANK-2004a (exact match SQL implementation)
- SEMRANK-2004b (query normalization implementation)
- SEMRANK-2005 (combined scoring)
- SEMRANK-3003 (integration test examples and patterns)

## Risk Assessment
- **Risk**: Original failure cases may not be reproducible in test corpus
  - **Mitigation**: Ensure test corpus includes representative examples of each failure mode
- **Risk**: Tests may pass even if semantic ranking is disabled (false positive)
  - **Mitigation**: Include assertions on score multipliers in debug mode
- **Risk**: New edge cases may emerge that aren't covered by known failures
  - **Mitigation**: Regression tests focus on documented issues; edge cases covered in SEMRANK-3004
- **Risk**: Baseline behavior may be hard to replicate without old code
  - **Mitigation**: Document expected baseline behavior in comments; actual baseline testing is optional

## Files/Packages Affected
- `/packages/maproom-mcp/tests/integration/regression.test.ts` (new file)
- `/packages/maproom-mcp/tests/results/regression-validation.md` (new file - test report)
- `/workspace/.agents/projects/SEMRANK_semantic-entry-point-ranking/planning/analysis.md` (reference for known failures)

## Implementation Summary

**Work Completed:**

1. **Created Regression Test Suite** (`tests/integration/regression.test.ts`)
   - 11 comprehensive regression tests covering all known failure categories
   - Tests organized by failure type for clear documentation
   - Each test includes code comments explaining the original problem and fix
   - Uses existing search helper infrastructure from search-quality.test.ts
   - Tests run against test-corpus (104 chunks)

2. **Test Categories Implemented:**
   - **Implementation vs Test Ranking** (2 tests)
     - `should rank implementations before tests for exact symbol match`
     - `should rank create_session implementation before test`
   - **Implementation vs Documentation Ranking** (2 tests)
     - `should rank implementations before documentation for concept queries`
     - `should rank database implementations before docs mentioning "database"`
   - **Case Sensitivity** (1 test)
     - `should return same #1 result for all case variations`
   - **Multi-Word Queries** (2 tests)
     - `should normalize multi-word queries correctly` (user authentication)
     - `should normalize "database connection" to match database_connection`
   - **Acronym Normalization** (2 tests)
     - `should normalize validateToken (camelCase) to match validate_token (snake_case)`
     - `should handle common programming acronyms (HTTP, XML, JSON)`
   - **Regression Summary** (2 tests)
     - `should demonstrate overall semantic ranking quality`
     - `should validate exact match multiplier is applied`

3. **Created Regression Validation Report** (`tests/results/regression-validation.md`)
   - Executive summary: ALL TESTS PASSED (11/11)
   - Detailed breakdown by category
   - Before/after ranking behavior examples
   - Test execution details
   - Conclusion: All known failures resolved

### Test Execution Results

```bash
pnpm exec vitest run tests/integration/regression.test.ts

✓ tests/integration/regression.test.ts  (11 tests) 664ms

Test Files  1 passed (1)
Tests  11 passed (11)
```

**All 11 regression tests PASS**, validating that:
- ✅ Implementations rank before tests (2/2 tests)
- ✅ Implementations rank before documentation (2/2 tests)
- ✅ Case-insensitive matching works (1/1 test)
- ✅ Multi-word query normalization works (2/2 tests)
- ✅ Acronym normalization works (2/2 tests)
- ✅ Overall semantic ranking quality validated (2/2 tests)

### Known Failures Resolved

**Before Semantic Ranking:**
- Query "authenticate" → Top 3: `heading_2, heading_1, heading_2` (docs ranked #1)
- Query "validate_token" → Top 3: `heading_2, heading_2, heading_1` (docs ranked #1)
- Implementation ranks: #8-#9 (buried below documentation)

**After Semantic Ranking:**
- Query "authenticate" → Top 3: `func, func, func` (implementations ranked #1)
- Query "validate_token" → Top 3: `func, func, func` (implementations ranked #1)
- Implementation ranks: #1 (consistently at the top)

### Files Created

1. `/workspace/packages/maproom-mcp/tests/integration/regression.test.ts` (274 lines)
   - Complete regression test suite
   - Documents all known failure cases
   - Validates fixes with comprehensive assertions

2. `/workspace/packages/maproom-mcp/tests/results/regression-validation.md` (3.2 KB)
   - Executive summary with pass/fail verdict
   - Detailed test results by category
   - Before/after behavior comparisons
   - Conclusion: Ready for Phase 4

### Verification Notes

**All acceptance criteria met:**
- ✅ Implementation vs test ranking: 2 tests PASS
- ✅ Implementation vs documentation ranking: 2 tests PASS
- ✅ Case sensitivity: 1 test PASS (all variations return same result)
- ✅ Multi-word queries: 2 tests PASS
- ✅ Acronym normalization: 2 tests PASS
- ✅ All regression tests passing: 11/11 (100%)
- ✅ Before/after documented in code comments
- ✅ Regression validation report created

**Verdict:** All known failure cases from SEMRANK analysis are now resolved and validated through comprehensive regression testing. No regressions detected.
