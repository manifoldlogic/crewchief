# Ticket: [SRCHFLTR-1002]: Implement filter() Method

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - all tests passing
- [ ] **Verified** - by the verify-ticket agent

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
Implement the filter() method in FilterableSearchResult class with support for kind, file_type, path substring, score range, and custom filtering criteria.

## Background
This ticket implements the core filtering functionality that allows users to progressively refine search results without re-querying the daemon. The filter() method supports multiple criteria (kind, file_type, path substring, score range, custom functions) with AND logic, enabling powerful client-side result manipulation.

This builds on SRCHFLTR-1001 (class skeleton) and is a critical component of Phase 1.

## Acceptance Criteria
- [ ] filter() method implemented with all criteria types
- [ ] Filter by kind (exact match) works correctly
- [ ] Filter by file_type (with or without leading dot) works correctly
- [ ] Filter by path substring (using string.includes) works correctly
- [ ] Filter by min_score and max_score works correctly
- [ ] Custom filter function support works correctly
- [ ] Multiple criteria combine with AND logic
- [ ] Returns new FilterableSearchResult instance (immutable)
- [ ] Original result unchanged after filtering
- [ ] TypeScript compiles without errors
- [ ] Unit tests pass with 80%+ coverage on filter logic

## Technical Requirements

### Method Signature
Add to FilterableSearchResult class:

```typescript
/**
 * Filter results by criteria.
 *
 * Supports:
 * - kind: Exact match on symbol kind
 * - file_type: File extension (with or without dot)
 * - path: Simple path matching (.includes(), .startsWith(), .endsWith())
 * - min_score: Minimum relevance score
 * - max_score: Maximum relevance score
 * - custom: Custom predicate function
 *
 * All criteria are combined with AND logic.
 *
 * @example
 * result.filter({kind: "function"})
 * result.filter({file_type: "ts"})
 * result.filter({path: "src/"})  // Uses string.includes()
 * result.filter({min_score: 0.5})
 * result.filter({custom: hit => hit.symbol_name?.includes("auth")})
 * result.filter({kind: "function", file_type: "ts"}) // AND logic
 */
filter(criteria: FilterCriteria): FilterableSearchResult {
  let filtered = this.hits as SearchHit[]

  // Filter by kind (exact match)
  if (criteria.kind) {
    filtered = filtered.filter(hit => hit.kind === criteria.kind)
  }

  // Filter by file_type (normalize with or without dot)
  if (criteria.file_type) {
    const ext = criteria.file_type.replace(/^\./, '') // Remove leading dot
    filtered = filtered.filter(hit =>
      hit.file_path.endsWith(`.${ext}`)
    )
  }

  // Filter by path substring (simple includes)
  if (criteria.path) {
    filtered = filtered.filter(hit =>
      hit.file_path.includes(criteria.path!)
    )
  }

  // Filter by score range
  if (criteria.min_score !== undefined) {
    filtered = filtered.filter(hit => hit.score >= criteria.min_score!)
  }
  if (criteria.max_score !== undefined) {
    filtered = filtered.filter(hit => hit.score <= criteria.max_score!)
  }

  // Custom filter function
  if (criteria.custom) {
    filtered = filtered.filter(criteria.custom)
  }

  // Return new instance with filtered hits
  return new FilterableSearchResult({
    ...this.result,
    hits: filtered,
    total: filtered.length,
  })
}
```

### Implementation Details
- Use native Array.filter() for all operations (no dependencies)
- Chain multiple filter operations (AND logic)
- Handle edge cases: null values, empty strings, NaN scores
- Preserve immutability by returning new instance

## Implementation Notes

**File**: `packages/daemon-client/src/filterable-result.ts`

**Filter Logic**:
1. Start with current hits array
2. Apply each filter criterion sequentially
3. Each filter reduces the result set (AND logic)
4. Return new FilterableSearchResult with filtered hits

**Edge Cases to Handle**:
- null/undefined symbol_name (markdown chunks)
- Invalid score values (NaN, Infinity) - skip filtering
- Empty result set - return valid FilterableSearchResult with 0 hits
- file_type with or without leading dot (.ts vs ts)

**Performance**:
- Target: <1ms for 100 results
- Uses native Array.filter (optimized by V8)
- No caching in MVP (future enhancement)

## Dependencies
- SRCHFLTR-1001 (class skeleton must exist)
- SRCHFLTR-1003 (filter types must be defined)

## Risk Assessment
- **Risk**: Performance regression on large result sets
  - **Mitigation**: Benchmark with 100+ results, optimize if needed
  - **Severity**: Low (client-side, typical result sets <100)
- **Risk**: Edge cases not handled (null values, NaN scores)
  - **Mitigation**: Comprehensive unit tests covering edge cases
  - **Severity**: Medium

## Files/Packages Affected
- `packages/daemon-client/src/filterable-result.ts` (modify)

## Verification Notes
Verify the filter() method by:
1. Run unit tests: `pnpm test packages/daemon-client`
2. Verify filter by kind returns only matching kinds
3. Verify filter by file_type handles both `.ts` and `ts`
4. Verify filter by path substring works with string.includes()
5. Verify score range filtering works correctly
6. Verify custom filter function executes
7. Verify multiple criteria combine with AND logic
8. Verify original result unchanged (immutability)
9. Verify performance <1ms for 100 results
10. Run `pnpm lint` - should pass
