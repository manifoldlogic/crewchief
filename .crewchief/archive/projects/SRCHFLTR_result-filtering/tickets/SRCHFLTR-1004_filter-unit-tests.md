# Ticket: [SRCHFLTR-1004]: Write Unit Tests for Filtering

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
Create comprehensive unit tests for FilterableSearchResult class focusing on filter() method, edge cases, and immutability.

## Background
Unit tests provide confidence that filtering logic works correctly across all criteria types and edge cases. This ticket implements 12 core filter tests from the quality strategy, targeting 80%+ coverage on filtering logic.

Tests must be fast (<5 seconds total) and provide clear failure messages.

## Acceptance Criteria
- [x] Test file `packages/daemon-client/tests/filterable-result.test.ts` created
- [x] All 12 filter tests implemented and passing
- [x] Filter by kind (exact match) tested
- [x] Filter by file_type (with/without dot) tested
- [x] Filter by path substring tested
- [x] Filter by min_score and max_score tested
- [x] Filter by custom function tested
- [x] Multiple criteria (AND logic) tested
- [x] Empty results handling tested
- [x] Null symbol_name handling tested
- [x] Invalid score (NaN, Infinity) handling tested
- [x] Immutability tested (original unchanged)
- [x] Test coverage ≥80% on filterable-result.ts
- [x] All tests pass in <5 seconds
- [x] Clear test descriptions and error messages

## Technical Requirements

### Test File Structure
Create: `packages/daemon-client/tests/filterable-result.test.ts`

### Mock Data
```typescript
import type { SearchResult, SearchHit } from '../src/client'
import { FilterableSearchResult } from '../src/filterable-result'

const mockHits: SearchHit[] = [
  {
    chunk_id: 1,
    file_path: "src/auth.ts",
    symbol_name: "login",
    kind: "function",
    start_line: 10,
    end_line: 20,
    content: "function login() {...}",
    score: 0.95,
  },
  {
    chunk_id: 2,
    file_path: "src/user.tsx",
    symbol_name: "UserProfile",
    kind: "class",
    start_line: 5,
    end_line: 50,
    content: "class UserProfile {...}",
    score: 0.85,
  },
  {
    chunk_id: 3,
    file_path: "test/auth.test.ts",
    symbol_name: "testLogin",
    kind: "function",
    start_line: 15,
    end_line: 25,
    content: "function testLogin() {...}",
    score: 0.75,
  },
  {
    chunk_id: 4,
    file_path: "README.md",
    symbol_name: null,
    kind: "markdown",
    start_line: 1,
    end_line: 100,
    content: "# Documentation",
    score: 0.50,
  },
]

const mockSearchResult: SearchResult = {
  hits: mockHits,
  total: mockHits.length,
}
```

### Test Cases (12 tests)
```typescript
describe('FilterableSearchResult', () => {
  describe('filter()', () => {
    it('filters by kind (exact match)', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({kind: "function"})

      expect(filtered.hits.length).toBe(2)
      expect(filtered.hits.every(h => h.kind === "function")).toBe(true)
    })

    it('filters by file_type with dot', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({file_type: ".ts"})

      expect(filtered.hits.length).toBe(2)
      expect(filtered.hits.every(h => h.file_path.endsWith(".ts"))).toBe(true)
    })

    it('filters by file_type without dot', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({file_type: "tsx"})

      expect(filtered.hits.length).toBe(1)
      expect(filtered.hits[0].file_path).toContain(".tsx")
    })

    it('filters by path substring', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({path: "test/"})

      expect(filtered.hits.length).toBe(1)
      expect(filtered.hits[0].file_path).toContain("test/")
    })

    it('filters by min_score', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({min_score: 0.8})

      expect(filtered.hits.length).toBe(2)
      expect(filtered.hits.every(h => h.score >= 0.8)).toBe(true)
    })

    it('filters by max_score', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({max_score: 0.8})

      expect(filtered.hits.length).toBe(2)
      expect(filtered.hits.every(h => h.score <= 0.8)).toBe(true)
    })

    it('filters by custom function', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({
        custom: hit => hit.symbol_name?.includes("login") ?? false
      })

      expect(filtered.hits.length).toBe(2)
      expect(filtered.hits.every(h => h.symbol_name?.includes("login"))).toBe(true)
    })

    it('combines multiple criteria with AND logic', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({
        kind: "function",
        file_type: "ts",
        min_score: 0.7
      })

      expect(filtered.hits.length).toBe(2)
      expect(filtered.hits.every(h =>
        h.kind === "function" &&
        h.file_path.endsWith(".ts") &&
        h.score >= 0.7
      )).toBe(true)
    })

    it('handles empty results gracefully', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({kind: "nonexistent"})

      expect(filtered.hits.length).toBe(0)
      expect(filtered.count).toBe(0)
      expect(filtered.total).toBe(0)
    })

    it('handles null symbol_name', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({kind: "markdown"})

      expect(filtered.hits.length).toBe(1)
      expect(filtered.hits[0].symbol_name).toBeNull()
    })

    it('handles invalid scores gracefully', () => {
      const invalidResult: SearchResult = {
        hits: [{...mockHits[0], score: NaN}],
        total: 1,
      }
      const result = new FilterableSearchResult(invalidResult)
      const filtered = result.filter({min_score: 0.5})

      // Should filter out NaN scores
      expect(filtered.hits.length).toBe(0)
    })

    it('preserves immutability (original unchanged)', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const originalHits = result.hits.length

      result.filter({kind: "function"})

      expect(result.hits.length).toBe(originalHits)
      expect(result.raw.hits.length).toBe(originalHits)
    })
  })
})
```

## Implementation Notes

**File**: `packages/daemon-client/tests/filterable-result.test.ts` (new file)

**Test Strategy**:
- Use realistic mock data (varied kinds, file types, scores)
- Test each filter criterion independently
- Test multiple criteria combined (AND logic)
- Test edge cases (empty results, null values, NaN)
- Verify immutability across all tests

**Performance Target**:
- All tests complete in <5 seconds
- Individual tests <100ms

**Coverage Target**:
- 80%+ on filterable-result.ts
- Focus on logic paths, not trivial getters

## Dependencies
- SRCHFLTR-1001 (class skeleton)
- SRCHFLTR-1002 (filter method implementation)
- SRCHFLTR-1003 (type definitions)

## Risk Assessment
- **Risk**: Tests don't catch real bugs
  - **Mitigation**: Include edge cases from quality strategy
  - **Severity**: Medium
- **Risk**: Slow tests impact developer workflow
  - **Mitigation**: Performance budget <5 seconds total
  - **Severity**: Low

## Files/Packages Affected
- `packages/daemon-client/tests/filterable-result.test.ts` (NEW)

## Verification Notes
Verify the unit tests by:
1. Run `pnpm test packages/daemon-client` - all tests pass
2. Check coverage report: `pnpm test --coverage` - ≥80% on filterable-result.ts
3. Verify tests complete in <5 seconds
4. Run tests in watch mode - fast feedback on changes
5. Verify clear error messages on failures
6. Check no warnings or deprecation notices
7. Run `pnpm lint` on test file - should pass
