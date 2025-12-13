# Ticket: [SRCHFLTR-1003]: Add Filter Type Definitions

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
Create comprehensive type definitions for FilterCriteria, SortField, and SortOrder to enable type-safe filtering, sorting, and pagination operations.

## Background
Type definitions are essential for TypeScript autocomplete, compile-time safety, and developer experience. This ticket creates the type system that supports the filter(), sortBy(), and slice() methods.

The types are intentionally simple (no advanced TypeScript features) to maximize compatibility and discoverability.

## Acceptance Criteria
- [ ] File `packages/daemon-client/src/filter-types.ts` created
- [ ] FilterCriteria interface defined with all filter fields
- [ ] SortField type defined with all sortable fields
- [ ] SortOrder type defined (asc/desc)
- [ ] Comprehensive TSDoc comments for all types
- [ ] TypeScript compiles without errors
- [ ] Types exported from daemon-client index
- [ ] IntelliSense shows helpful documentation

## Technical Requirements

### File Structure
Create new file: `packages/daemon-client/src/filter-types.ts`

### Type Definitions
```typescript
import type { SearchHit } from './client'

/**
 * Criteria for filtering search results.
 *
 * All fields are optional and combined with AND logic.
 *
 * @example
 * // Filter TypeScript functions
 * {kind: "function", file_type: "ts"}
 *
 * // Filter by path and score
 * {path: "src/", min_score: 0.5}
 *
 * // Custom filter
 * {custom: hit => hit.symbol_name?.includes("auth")}
 */
export interface FilterCriteria {
  /**
   * Exact match on symbol kind (function, class, interface, etc.)
   *
   * @example "function", "class", "interface"
   */
  kind?: string

  /**
   * File extension (with or without leading dot)
   *
   * @example "ts", ".ts", "tsx", ".tsx"
   */
  file_type?: string

  /**
   * Simple path matching - uses string.includes() for substring match.
   *
   * For prefix matching, use custom filter:
   * {custom: h => h.file_path.startsWith("src/")}
   *
   * For suffix matching, use custom filter:
   * {custom: h => h.file_path.endsWith(".config.ts")}
   *
   * @example "src/", "test", ".config"
   */
  path?: string

  /**
   * Minimum relevance score (0.0-1.0)
   *
   * @example 0.5 for scores >= 50%
   */
  min_score?: number

  /**
   * Maximum relevance score (0.0-1.0)
   *
   * @example 0.8 for scores <= 80%
   */
  max_score?: number

  /**
   * Custom filter function for advanced filtering.
   *
   * Receives each SearchHit and returns true to keep, false to filter out.
   *
   * @example
   * {custom: hit => hit.symbol_name?.includes("auth")}
   * {custom: hit => hit.start_line > 100}
   */
  custom?: (hit: SearchHit) => boolean
}

/**
 * Field to sort search results by.
 *
 * Each field has a default sort order:
 * - score: descending (highest first)
 * - relpath: ascending (alphabetical)
 * - symbol_name: ascending (alphabetical)
 * - start_line: ascending (earlier lines first)
 * - kind: ascending (alphabetical)
 */
export type SortField =
  | "score"        // Relevance score
  | "relpath"      // File path (relative)
  | "symbol_name"  // Symbol name
  | "start_line"   // Line number
  | "kind"         // Symbol kind

/**
 * Sort order direction.
 *
 * - "asc": Ascending (A-Z, 0-9, low to high)
 * - "desc": Descending (Z-A, 9-0, high to low)
 */
export type SortOrder = "asc" | "desc"
```

### Export from Index
Update `packages/daemon-client/src/index.ts`:
```typescript
export { FilterableSearchResult } from './filterable-result'
export type { FilterCriteria, SortField, SortOrder } from './filter-types'
```

## Implementation Notes

**File**: `packages/daemon-client/src/filter-types.ts` (new file)

**Design Decisions**:
- **Interface for FilterCriteria**: Allows flexible partial application
- **Union types for SortField/SortOrder**: Type-safe string literals
- **Comprehensive TSDoc**: Enables IntelliSense documentation
- **No complex generics**: Keeps types simple and discoverable

**Type Safety Benefits**:
- Autocomplete suggests available filter fields
- Compile-time validation of filter criteria
- IntelliSense shows examples and documentation
- Prevents typos in field names

## Dependencies
- None (pure type definitions)

## Risk Assessment
- **Risk**: Type definitions don't match implementation
  - **Mitigation**: SRCHFLTR-1002 uses these exact types
  - **Severity**: Low
- **Risk**: Missing type exports break consumption
  - **Mitigation**: Export verification in acceptance criteria
  - **Severity**: Low

## Files/Packages Affected
- `packages/daemon-client/src/filter-types.ts` (NEW)
- `packages/daemon-client/src/index.ts` (modify - add exports)

## Verification Notes
Verify the type definitions by:
1. Run `pnpm build` in packages/daemon-client - should compile without errors
2. Check TypeScript IntelliSense shows FilterCriteria fields
3. Verify TSDoc comments appear in autocomplete
4. Verify types can be imported: `import type { FilterCriteria } from '@crewchief/daemon-client'`
5. Confirm no `any` types used
6. Run `pnpm lint` - should pass
7. Test in IDE: type `const criteria: FilterCriteria = {` and verify autocomplete works
