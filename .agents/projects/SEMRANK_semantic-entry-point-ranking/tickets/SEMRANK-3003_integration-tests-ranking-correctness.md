# Ticket: SEMRANK-3003: Integration Tests for Ranking Correctness

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
Create comprehensive integration tests that validate search returns correct entry points (implementations over tests/docs) using the test corpus created in SEMRANK-1003.

## Background
The semantic ranking implementation (Phase 2) adds kind-based multipliers and exact match scoring to improve FTS results. This ticket validates that these enhancements work correctly by testing against the 30-50 chunk test corpus across Rust, TypeScript, and Python.

This ticket implements Phase 3 validation from the SEMRANK execution plan (plan.md, lines 183-231). The goal is to ensure that:
- Implementations rank higher than tests with the same symbol name
- Implementations rank higher than documentation mentioning the symbol
- Exact match multipliers (3.0×) are applied correctly
- Kind multipliers are applied correctly (func: 2.5×, test: 0.6×, doc: 0.3×)
- Debug mode score breakdown shows all multipliers

The integration test framework was established in SEMRANK-1006, and this ticket extends it with comprehensive ranking validation tests.

## Acceptance Criteria
- [ ] Test: Search "authenticate" returns implementation as #1 result (not test)
- [ ] Test: Implementation ranks higher than test with same symbol name
- [ ] Test: Implementation ranks higher than documentation mentioning symbol
- [ ] Test: Exact function name match gets 3.0× multiplier (verified in debug mode)
- [ ] Test: Kind multipliers apply correctly (func: 2.5×, test: 0.6×, doc: 0.3×)
- [ ] Test: Score breakdown in debug mode shows all multipliers (base_fts, kind_multiplier, exact_match_multiplier, final)
- [ ] All tests pass with >90% top-1 accuracy for exact symbol searches
- [ ] Test results documented in `/packages/maproom-mcp/tests/results/ranking-correctness.md`

## Technical Requirements
- Extend existing integration test file: `/packages/maproom-mcp/tests/integration/search-quality.test.ts`
- Use test corpus from SEMRANK-1003 (30-50 chunks across Rust, TypeScript, Python)
- Test against golden queries from baseline (SEMRANK-1005)
- Use debug mode to verify multiplier calculations
- Calculate top-1 accuracy metric: (queries where implementation is #1) / (total exact symbol queries)
- Document test results with actual vs expected rankings
- Include sample queries: "authenticate", "validate_provider", "parse_config", etc.

## Implementation Notes

### Test Structure
Each ranking correctness test should:
1. Execute search query with debug mode enabled
2. Verify top result is implementation (kind = 'function' or similar)
3. Check that tests and docs rank lower than implementation
4. Validate score breakdown shows correct multipliers
5. Assert final score calculation: `final = base_fts × kind_mult × exact_mult`

### Test Cases to Cover
1. **Exact Match Tests:**
   - Query matches symbol_name exactly (case-insensitive)
   - Verify exact_match_multiplier = 3.0
   - Verify implementation is #1 result

2. **Kind Ranking Tests:**
   - Same symbol exists in implementation, test, and doc
   - Verify implementation > test > doc in ranking
   - Verify kind_multipliers applied correctly

3. **Combined Multiplier Tests:**
   - Test where both kind and exact match apply
   - Verify multipliers combine multiplicatively

4. **Multi-Language Tests:**
   - Test Rust, TypeScript, Python examples
   - Verify language-agnostic ranking behavior

### Metrics to Track
- **Top-1 Accuracy:** Percentage of exact symbol queries where implementation ranks #1
- **Target:** >90% accuracy
- **Implementation Rank:** Average rank of implementations across all queries
- **Target:** <3 (within top 3 results)

### Documentation Format
Create `/packages/maproom-mcp/tests/results/ranking-correctness.md` with:
```markdown
# Ranking Correctness Test Results

## Summary
- Total queries tested: X
- Top-1 accuracy: Y%
- Average implementation rank: Z

## Test Details
| Query | Top Result Kind | Rank of Implementation | Exact Match Applied | Pass/Fail |
|-------|----------------|----------------------|-------------------|-----------|
| authenticate | function | 1 | Yes | Pass |
| validate_provider | function | 1 | Yes | Pass |
...

## Score Breakdown Examples
[Include 3-5 examples of debug mode output showing multiplier calculations]

## Issues Found
[Document any failing tests or unexpected behavior]
```

## Dependencies
- SEMRANK-1003 (test corpus repository)
- SEMRANK-1004 (test corpus indexed in database)
- SEMRANK-1005 (baseline metrics and golden query set)
- SEMRANK-1006 (integration test framework setup)
- SEMRANK-2003 (kind multiplier implementation)
- SEMRANK-2004a (exact match SQL implementation)
- SEMRANK-2004b (query normalization implementation)
- SEMRANK-2005 (combined multipliers in final score)
- SEMRANK-2006 (debug score breakdown implementation)

## Risk Assessment
- **Risk**: Test corpus may not cover all edge cases
  - **Mitigation**: Use diverse set of 30-50 chunks across 3 languages with varied kinds
- **Risk**: Multiplier values may need tuning to achieve 90% accuracy
  - **Mitigation**: Document actual accuracy achieved; flag if <90% for multiplier adjustment
- **Risk**: Debug mode score breakdown may not match manual calculations
  - **Mitigation**: Include manual verification of at least 5 score calculations
- **Risk**: Integration tests may be flaky due to database state
  - **Mitigation**: Use test framework's database seeding/teardown from SEMRANK-1006

## Files/Packages Affected
- `/packages/maproom-mcp/tests/integration/search-quality.test.ts` (enhance with ranking tests)
- `/packages/maproom-mcp/tests/results/ranking-correctness.md` (new file - test results)
