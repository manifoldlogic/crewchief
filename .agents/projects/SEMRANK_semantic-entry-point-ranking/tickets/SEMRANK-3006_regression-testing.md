# Ticket: SEMRANK-3006: Regression Testing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

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
- [ ] Test: "validate_provider" returns implementation first (known failure from analysis.md)
- [ ] Test: "authentication" returns AuthService implementation before documentation
- [ ] Test: Case variations match (Authenticate, authenticate, AUTHENTICATE all return same #1 result)
- [ ] Test: Multi-word queries work (e.g., "HTTP handler", "database connection")
- [ ] Test: Acronym queries work (e.g., "XML parser", "HTTP client")
- [ ] All regression tests pass with semantic ranking enabled
- [ ] Document baseline behavior (tests should fail without semantic ranking)
- [ ] Create regression test report: `/packages/maproom-mcp/tests/results/regression-validation.md`

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
