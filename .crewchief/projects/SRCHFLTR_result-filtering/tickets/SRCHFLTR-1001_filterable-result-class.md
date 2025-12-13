# Ticket: [SRCHFLTR-1001]: Create FilterableSearchResult Class Skeleton

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
- verify-ticket
- commit-ticket

## Summary
Create the foundational FilterableSearchResult class structure in daemon-client package with basic constructor, properties, and type safety.

## Background
This is the first ticket in Phase 1 of the Result Filtering project (SRCHFLTR). We need a TypeScript wrapper class that can enhance SearchResult objects with filtering, sorting, and pagination capabilities. This ticket establishes the skeleton structure that subsequent tickets will build upon.

The FilterableSearchResult class wraps the existing SearchResult interface without modifying it, ensuring complete backward compatibility.

## Acceptance Criteria
- [ ] File `packages/daemon-client/src/filterable-result.ts` created
- [ ] FilterableSearchResult class exists with constructor accepting SearchResult
- [ ] Readonly properties implemented: `raw`, `hits`, `total`, `count`
- [ ] TypeScript compiles without errors
- [ ] Class is immutable (uses readonly modifiers)
- [ ] No breaking changes to existing daemon-client exports

## Technical Requirements

### File Structure
Create new file: `packages/daemon-client/src/filterable-result.ts`

### Class Definition
```typescript
import type { SearchResult, SearchHit } from './client'

/**
 * Search result with client-side filtering/sorting/pagination.
 *
 * Wraps SearchResult from daemon and provides chainable methods
 * for progressive result refinement without re-querying.
 *
 * All operations are immutable - original result is preserved.
 *
 * @example
 * const result = new FilterableSearchResult(daemonResult)
 * const functions = result.filter({kind: "function"})
 * const sorted = functions.sortBy("relpath")
 * const page1 = sorted.slice(0, 10)
 */
export class FilterableSearchResult {
  private readonly result: SearchResult

  constructor(result: SearchResult) {
    this.result = result
  }

  /**
   * Get raw search result (unwrapped)
   */
  get raw(): SearchResult {
    return this.result
  }

  /**
   * Get hits array (readonly)
   */
  get hits(): readonly SearchHit[] {
    return this.result.hits
  }

  /**
   * Get total count (before filtering)
   */
  get total(): number {
    return this.result.total
  }

  /**
   * Get count after filtering
   */
  get count(): number {
    return this.result.hits.length
  }
}
```

### Type Safety
- Use TypeScript `readonly` modifiers to enforce immutability
- Import existing types from `./client` (SearchResult, SearchHit)
- Add comprehensive TSDoc comments for IntelliSense

## Implementation Notes

**Location**: `packages/daemon-client/src/filterable-result.ts` (new file)

**Design Pattern**: Wrapper pattern - preserves original SearchResult, enables chaining

**Dependencies**:
- Zero new npm dependencies
- Uses existing SearchResult and SearchHit types from daemon-client

**Immutability Strategy**:
- Private readonly `result` property
- Readonly return types for properties
- Methods (to be added in later tickets) will return new instances

## Dependencies
- None (foundation work)

## Risk Assessment
- **Risk**: Type conflicts with existing SearchResult interface
  - **Mitigation**: Only imports types, doesn't modify them
  - **Severity**: Low
- **Risk**: Breaking changes to daemon-client API
  - **Mitigation**: Pure additive change, no modifications to existing exports
  - **Severity**: Very Low

## Files/Packages Affected
- `packages/daemon-client/src/filterable-result.ts` (NEW)
- `packages/daemon-client/package.json` (no changes - zero new dependencies)

## Verification Notes
Verify the class skeleton by:
1. Run `pnpm build` in packages/daemon-client - should compile without errors
2. Check TypeScript types are correctly inferred (no `any` types)
3. Verify class can be instantiated with mock SearchResult
4. Confirm readonly properties return expected values
5. Run `pnpm lint` - should pass with no warnings
