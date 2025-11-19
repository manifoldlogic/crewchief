# Ranking Correctness Test Results

## Summary

This document tracks the results of ranking correctness integration tests for SEMRANK Phase 3. These tests validate that semantic ranking enhancements (kind multipliers and exact match scoring) correctly prioritize implementations over tests and documentation.

**Test Suite:** `tests/integration/search-quality.test.ts` - "Ranking Correctness - SEMRANK Phase 3"

### Metrics Overview

- **Total golden queries tested:** 7
- **Top-1 accuracy target:** >90%
- **Average implementation rank target:** <3
- **Kind multiplier values:**
  - Functions: 2.5×
  - Classes: 2.0×
  - Methods: 2.0×
  - Components: 2.0×
  - Hooks: 1.8×
  - Tests: 0.6×
  - Documentation (headings): 0.3×
- **Exact match multiplier:** 3.0×

## Test Coverage

### 1. Exact Match Tests (4 tests)

Tests that verify exact symbol name matches return implementations as #1 result with 3.0× exact match multiplier applied.

| Test | Query | Expected Behavior | Status |
|------|-------|------------------|--------|
| 1.1 | `authenticate` | Implementation #1, exact_mult=3.0× | Pending execution |
| 1.2 | `validate_provider` | Implementation #1, exact_mult=3.0× | Pending execution |
| 1.3 | `parse_config` | Implementation #1, exact_mult=3.0× | Pending execution |
| 1.4 | `AUTHENTICATE` | Case-insensitive match, exact_mult=3.0× | Pending execution |

### 2. Kind Ranking Tests (4 tests)

Tests that verify implementation chunks rank higher than test/doc chunks for the same symbol.

| Test | Query | Expected Behavior | Status |
|------|-------|------------------|--------|
| 2.1 | `authenticate` | impl_rank < test_rank | Pending execution |
| 2.2 | `authenticate` | impl_rank < doc_rank | Pending execution |
| 2.3 | `connect_database` | func kind has kind_mult=2.5× | Pending execution |
| 2.4 | `authentication` | heading kind has kind_mult≤1.0× | Pending execution |

### 3. Combined Multiplier Tests (2 tests)

Tests that verify multipliers combine multiplicatively: `final = base_fts × kind_mult × exact_mult`

| Test | Query | Expected Behavior | Status |
|------|-------|------------------|--------|
| 3.1 | `authenticate` | func + exact: final = base × 2.5 × 3.0 | Pending execution |
| 3.2 | `validate_provider` | Formula verified for all results | Pending execution |

### 4. Multi-Language Tests (3 tests)

Tests that verify ranking works correctly across Rust, TypeScript, and Python.

| Test | Query | Language | Expected Behavior | Status |
|------|-------|----------|------------------|--------|
| 4.1 | `connect_database` | Rust | Implementation #1 in .rs file | Pending execution |
| 4.2 | `validateToken` | TypeScript | Implementation #1 in .ts/.tsx file | Pending execution |
| 4.3 | `validate_token` | Python | Implementation #1 in .py file | Pending execution |

### 5. Top-1 Accuracy Metrics (2 tests)

Tests that measure overall ranking quality across golden query set.

| Test | Metric | Target | Status |
|------|--------|--------|--------|
| 5.1 | Top-1 accuracy | >90% | Pending execution |
| 5.2 | Avg implementation rank | <3 | Pending execution |

### 6. Score Breakdown Validation (3 tests)

Tests that verify debug mode score breakdown is returned correctly.

| Test | Expected Behavior | Status |
|------|------------------|--------|
| 6.1 | Debug mode returns score_breakdown | Pending execution |
| 6.2 | Breakdown includes all fields (base_fts, kind_mult, exact_mult, final) | Pending execution |
| 6.3 | Debug=false does not return score_breakdown | Pending execution |

## Golden Queries

The following queries are tested for top-1 accuracy:

1. `authenticate` - Common auth function
2. `validate_provider` - Snake_case validation
3. `parse_config` - Config parsing function
4. `connect_database` - Rust database connection
5. `execute_query` - Query execution function
6. `validateToken` - CamelCase validation
7. `validate_token` - Snake_case token validation

## Detailed Test Results

### Run: [Date to be added after test execution]

**Environment:**
- Test corpus: test-corpus (104 chunks indexed)
- Database: PostgreSQL with maproom schema
- FTS implementation: Rust binary with kind and exact match multipliers

**Execution Details:**

#### Exact Match Tests

Query: `authenticate`
- Top result: [To be filled]
  - Kind: [To be filled]
  - Path: [To be filled]
  - Score: [To be filled]
  - Score breakdown:
    - base_fts: [To be filled]
    - kind_multiplier: [To be filled]
    - exact_match_multiplier: [To be filled]
    - final: [To be filled]
  - Verification: final = base_fts × kind_mult × exact_mult ✓/✗

Query: `validate_provider`
- Top result: [To be filled]
  - [Same structure as above]

Query: `parse_config`
- Top result: [To be filled]
  - [Same structure as above]

#### Kind Ranking Tests

Query: `authenticate`
- Implementation rank: [To be filled]
- Test rank: [To be filled]
- Documentation rank: [To be filled]
- Verification: impl < test < doc ✓/✗

#### Combined Multiplier Tests

Query: `authenticate` (func with exact match)
- Base FTS score: [To be filled]
- Kind multiplier: [To be filled] (expected: 2.5)
- Exact match multiplier: [To be filled] (expected: 3.0)
- Final score: [To be filled]
- Expected final: base × 2.5 × 3.0 = [To be filled]
- Match: ✓/✗

#### Multi-Language Tests

Rust (`connect_database`):
- Top result path: [To be filled]
- File extension: .rs ✓/✗
- Kind: func ✓/✗

TypeScript (`validateToken`):
- Top result path: [To be filled]
- File extension: .ts/.tsx ✓/✗
- Kind: func/component/hook ✓/✗

Python (`validate_token`):
- Top result path: [To be filled]
- File extension: .py ✓/✗
- Kind: func ✓/✗

#### Top-1 Accuracy

| Query | Top Result Kind | Implementation? | In Test File? | Correct? |
|-------|----------------|-----------------|---------------|----------|
| authenticate | [TBF] | [TBF] | [TBF] | ✓/✗ |
| validate_provider | [TBF] | [TBF] | [TBF] | ✓/✗ |
| parse_config | [TBF] | [TBF] | [TBF] | ✓/✗ |
| connect_database | [TBF] | [TBF] | [TBF] | ✓/✗ |
| execute_query | [TBF] | [TBF] | [TBF] | ✓/✗ |
| validateToken | [TBF] | [TBF] | [TBF] | ✓/✗ |
| validate_token | [TBF] | [TBF] | [TBF] | ✓/✗ |

**Summary:**
- Correct: [X] / 7
- Top-1 Accuracy: [X]%
- Target: >90% ✓/✗

#### Implementation Rank Distribution

| Query | Implementation Rank |
|-------|-------------------|
| authenticate | [TBF] |
| validate_provider | [TBF] |
| parse_config | [TBF] |
| connect_database | [TBF] |
| execute_query | [TBF] |
| validateToken | [TBF] |
| validate_token | [TBF] |

**Average:** [X.XX]
**Target:** <3 ✓/✗

## Score Breakdown Examples

### Example 1: Function with Exact Match

```
Query: "authenticate"
Result: src/auth/authenticate.ts::authenticate (func)

Score Breakdown:
  base_fts:                  [TBF]
  kind_multiplier:           [TBF] (func = 2.5×)
  exact_match_multiplier:    [TBF] (exact = 3.0×)
  final:                     [TBF]

Calculation:
  expected = [base] × 2.5 × 3.0 = [result]
  actual   = [final]
  match: ✓/✗
```

### Example 2: Class without Exact Match

```
Query: "database connection"
Result: [TBF]

Score Breakdown:
  base_fts:                  [TBF]
  kind_multiplier:           [TBF] (class = 2.0×)
  exact_match_multiplier:    [TBF] (no exact = 1.0×)
  final:                     [TBF]

Calculation:
  expected = [base] × 2.0 × 1.0 = [result]
  actual   = [final]
  match: ✓/✗
```

### Example 3: Documentation Chunk

```
Query: "authentication"
Result: [TBF]

Score Breakdown:
  base_fts:                  [TBF]
  kind_multiplier:           [TBF] (heading = 0.3×)
  exact_match_multiplier:    [TBF]
  final:                     [TBF]

Calculation:
  expected = [base] × 0.3 × [exact] = [result]
  actual   = [final]
  match: ✓/✗
```

### Example 4: Test Chunk (Lower Ranking)

```
Query: "authenticate"
Result: [TBF]

Score Breakdown:
  base_fts:                  [TBF]
  kind_multiplier:           [TBF] (test context = 0.6×)
  exact_match_multiplier:    [TBF]
  final:                     [TBF]

Note: Should rank lower than implementation with same symbol
```

### Example 5: Multi-word Query Normalization

```
Query: "HTTP handler"
Normalized: "http_handler"
Result: [TBF]

Score Breakdown:
  base_fts:                  [TBF]
  kind_multiplier:           [TBF]
  exact_match_multiplier:    [TBF] (normalized exact match)
  final:                     [TBF]
```

## Issues Found

[Document any failing tests or unexpected behavior after test execution]

### Issue Template

**Issue ID:** [e.g., RANK-001]
**Test:** [Test name that failed]
**Query:** [Query that produced unexpected results]
**Expected:** [What should have happened]
**Actual:** [What actually happened]
**Root Cause:** [Analysis of why it happened]
**Severity:** [Critical/High/Medium/Low]
**Status:** [Open/In Progress/Resolved]

---

## Test Execution Instructions

To run these tests and populate the results:

```bash
cd /workspace/packages/maproom-mcp
pnpm test tests/integration/search-quality.test.ts
```

The test output will show:
- Pass/fail status for each test
- Console logs with Top-1 Accuracy and Average Implementation Rank
- Debug score breakdowns for manual verification

To update this document:
1. Run the tests
2. Copy test output to this file
3. Fill in [TBF] placeholders with actual values
4. Mark ✓/✗ based on pass/fail
5. Document any issues found

## References

- **Ticket:** SEMRANK-3003
- **Test Suite:** `/packages/maproom-mcp/tests/integration/search-quality.test.ts`
- **Test Helpers:** `/packages/maproom-mcp/tests/helpers/search-test-utils.ts`
- **Test Corpus:** `/tmp/semrank-test-corpus` (indexed as 'test-corpus')
- **Baseline Metrics:** `/packages/maproom-mcp/benchmarks/baseline-fts.csv`
- **Implementation:**
  - Kind multipliers: `crates/maproom/src/search/fts.rs`
  - Exact match: `crates/maproom/src/search/fts.rs`
  - Query normalization: `packages/maproom-mcp/src/tools/search.ts`
