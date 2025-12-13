# Ticket: [SRCHFLTR-2001]: Implement sortBy() Method

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
Implement the sortBy() method in FilterableSearchResult class to enable sorting by score, file path, symbol name, line number, or kind with ascending/descending order.

## Background
This ticket implements sorting functionality for Phase 2. Sorting allows users to organize filtered results by different fields (score, path, symbol name, etc.) with control over sort order.

The sortBy() method integrates seamlessly with filter() for chained operations, enabling workflows like: filter by kind → sort by path → slice for pagination.

## Acceptance Criteria
- [ ] sortBy() method implemented with all sort fields
- [ ] Sort by score (default descending) works correctly
- [ ] Sort by relpath (default ascending) works correctly
- [ ] Sort by symbol_name (default ascending) works correctly
- [ ] Sort by start_line (default ascending) works correctly
- [ ] Sort by kind (default ascending) works correctly
- [ ] Explicit sort order (asc/desc) works correctly
- [ ] Returns new FilterableSearchResult instance (immutable)
- [ ] Original result unchanged after sorting
- [ ] Null/undefined values handled gracefully
- [ ] Unit tests pass with 80%+ coverage on sort logic
- [ ] Performance <1ms for 100 results

## Technical Requirements

### Method Signature
Add to FilterableSearchResult class:

```typescript
/**
 * Sort results by field.
 *
 * Supported fields:
 * - "score": Relevance score (descending by default)
 * - "relpath": File path (ascending by default)
 * - "symbol_name": Symbol name (ascending by default)
 * - "start_line": Line number (ascending by default)
 * - "kind": Symbol kind (ascending by default)
 *
 * @param field - Field to sort by
 * @param order - "asc" or "desc" (defaults based on field)
 *
 * @example
 * result.sortBy("score")              // Highest scores first
 * result.sortBy("relpath")            // Alphabetical by path
 * result.sortBy("start_line", "desc") // Later lines first
 */
sortBy(field: SortField, order?: SortOrder): FilterableSearchResult {
  // Determine default order based on field
  const defaultOrder: Record<SortField, SortOrder> = {
    score: "desc",      // Higher scores first
    relpath: "asc",     // Alphabetical
    symbol_name: "asc", // Alphabetical
    start_line: "asc",  // Earlier lines first
    kind: "asc",        // Alphabetical
  }

  const sortOrder = order ?? defaultOrder[field]
  const multiplier = sortOrder === "asc" ? 1 : -1

  const sorted = [...this.hits].sort((a, b) => {
    let aVal: any
    let bVal: any

    switch (field) {
      case "score":
        aVal = a.score
        bVal = b.score
        break
      case "relpath":
        aVal = a.file_path
        bVal = b.file_path
        break
      case "symbol_name":
        aVal = a.symbol_name ?? ""
        bVal = b.symbol_name ?? ""
        break
      case "start_line":
        aVal = a.start_line
        bVal = b.start_line
        break
      case "kind":
        aVal = a.kind
        bVal = b.kind
        break
    }

    if (aVal < bVal) return -1 * multiplier
    if (aVal > bVal) return 1 * multiplier
    return 0
  })

  return new FilterableSearchResult({
    ...this.result,
    hits: sorted,
  })
}
```

### Unit Tests
Add to `packages/daemon-client/tests/filterable-result.test.ts`:

```typescript
describe('sortBy()', () => {
  it('sorts by score descending (default)', () => {
    const result = new FilterableSearchResult(mockSearchResult)
    const sorted = result.sortBy("score")

    expect(sorted.hits[0].score).toBeGreaterThanOrEqual(sorted.hits[1].score)
  })

  it('sorts by relpath ascending', () => {
    const result = new FilterableSearchResult(mockSearchResult)
    const sorted = result.sortBy("relpath")

    expect(sorted.hits[0].file_path <= sorted.hits[1].file_path).toBe(true)
  })

  it('sorts by symbol_name ascending', () => {
    const result = new FilterableSearchResult(mockSearchResult)
    const sorted = result.sortBy("symbol_name")

    const names = sorted.hits.map(h => h.symbol_name ?? "")
    expect(names[0] <= names[1]).toBe(true)
  })

  it('sorts by start_line ascending', () => {
    const result = new FilterableSearchResult(mockSearchResult)
    const sorted = result.sortBy("start_line")

    expect(sorted.hits[0].start_line <= sorted.hits[1].start_line).toBe(true)
  })

  it('sorts by kind ascending', () => {
    const result = new FilterableSearchResult(mockSearchResult)
    const sorted = result.sortBy("kind")

    expect(sorted.hits[0].kind <= sorted.hits[1].kind).toBe(true)
  })

  it('sorts descending when explicitly specified', () => {
    const result = new FilterableSearchResult(mockSearchResult)
    const sorted = result.sortBy("relpath", "desc")

    expect(sorted.hits[0].file_path >= sorted.hits[1].file_path).toBe(true)
  })

  it('preserves immutability (original unchanged)', () => {
    const result = new FilterableSearchResult(mockSearchResult)
    const originalFirst = result.hits[0]

    result.sortBy("relpath")

    expect(result.hits[0]).toBe(originalFirst)
  })
})
```

## Implementation Notes

**File**: `packages/daemon-client/src/filterable-result.ts`

**Sort Strategy**:
- Copy hits array (`[...this.hits]`) to preserve immutability
- Use native Array.sort() with custom comparator
- Handle null/undefined symbol_name (default to empty string)
- Field-specific default order (score desc, others asc)

**Edge Cases**:
- Null symbol_name: Sort as empty string
- Equal values: Maintain stable sort order
- NaN scores: Sort to end (< comparison returns false)

**Performance**:
- Target: <1ms for 100 results
- Uses native Array.sort (optimized by V8)
- Time complexity: O(n log n)

## Dependencies
- SRCHFLTR-1001 (class skeleton)
- SRCHFLTR-1003 (SortField and SortOrder types)

## Risk Assessment
- **Risk**: Performance regression on large result sets
  - **Mitigation**: Benchmark with 100+ results
  - **Severity**: Low
- **Risk**: Sort instability (order changes on equal values)
  - **Mitigation**: Use stable sort algorithm (native Array.sort is stable in modern JS)
  - **Severity**: Low

## Files/Packages Affected
- `packages/daemon-client/src/filterable-result.ts` (modify)
- `packages/daemon-client/tests/filterable-result.test.ts` (modify)

## Verification Notes
Verify the sortBy() method by:
1. Run unit tests: `pnpm test packages/daemon-client`
2. Verify each sort field works correctly
3. Verify default order for each field
4. Verify explicit order (asc/desc) works
5. Verify null symbol_name handled gracefully
6. Verify immutability (original unchanged)
7. Verify performance <1ms for 100 results
8. Run `pnpm lint` - should pass
