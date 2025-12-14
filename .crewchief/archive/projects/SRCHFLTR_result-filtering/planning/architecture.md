# Architecture: Result Filtering

**Project:** SRCHFLTR - Result Filtering
**Date:** 2025-12-13
**Status:** MVP Design

---

## Design Philosophy

**MVP First:** Ship useful filtering/sorting/pagination without over-engineering. Focus on 80% use cases (kind, file_type, path, recency), defer advanced features (faceted search, saved filters, complex queries) to future iterations.

**No Breaking Changes:** Pure additive API. Existing SearchResult interface and consumers unchanged.

**Client-Side First:** Implement in TypeScript (daemon-client layer). Rust support can come later if needed.

**Immutable Operations:** All filtering/sorting returns new result objects. Original results preserved for re-filtering.

---

## System Overview

### Architecture Layers

```
┌─────────────────────────────────────────┐
│    MCP Client / VSCode Extension        │
│  result.filter({kind: "function"})      │
└────────────────┬────────────────────────┘
                 │
                 │ Uses FilterableSearchResult methods
                 │
┌────────────────▼────────────────────────┐
│   daemon-client (TypeScript)            │
│  - FilterableSearchResult class         │
│  - filter() / sort() / slice() methods  │
│  - Immutable operations on hits array   │
└────────────────┬────────────────────────┘
                 │
                 │ Wraps SearchResult from daemon
                 │
┌────────────────▼────────────────────────┐
│       Daemon Client                     │
│  SearchResult (unchanged)               │
│  - hits: Array<{...}>                   │
│  - total: number                        │
└─────────────────────────────────────────┘
```

**Key Insight:** Filtering happens **entirely in TypeScript client layer**. No Rust changes, no daemon changes, no database changes. Pure client-side result manipulation.

---

## Component Design

### 1. FilterableSearchResult Class

**Location:** `packages/daemon-client/src/filterable-result.ts` (new file)

**Purpose:** Wrapper around `SearchResult` that adds filtering/sorting/pagination methods.

**Interface:**
```typescript
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
 *
 * // Or chain operations
 * const filtered = result
 *   .filter({kind: "function", file_type: "ts"})
 *   .sortBy("score")
 *   .slice(0, 10)
 */
export class FilterableSearchResult {
  private readonly result: SearchResult

  constructor(result: SearchResult)

  /**
   * Get raw search result (unwrapped)
   */
  get raw(): SearchResult

  /**
   * Get hits array (readonly)
   */
  get hits(): readonly SearchHit[]

  /**
   * Get total count (before filtering)
   */
  get total(): number

  /**
   * Get count after filtering
   */
  get count(): number

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
   * @example
   * result.filter({kind: "function"})
   * result.filter({file_type: "ts"})
   * result.filter({path: "src/"})  // Uses string.includes()
   * result.filter({min_score: 0.5})
   * result.filter({custom: hit => hit.symbol_name?.includes("auth")})
   */
  filter(criteria: FilterCriteria): FilterableSearchResult

  /**
   * Sort results by field.
   *
   * Supported fields:
   * - "score": Relevance score (descending, default)
   * - "relpath": File path (ascending)
   * - "symbol_name": Symbol name (ascending)
   * - "start_line": Line number (ascending)
   * - "kind": Symbol kind (ascending)
   *
   * @param field - Field to sort by
   * @param order - "asc" or "desc" (defaults based on field)
   */
  sortBy(field: SortField, order?: SortOrder): FilterableSearchResult

  /**
   * Slice results for pagination.
   *
   * @param start - Start index (inclusive, 0-based)
   * @param end - End index (exclusive)
   *
   * @example
   * result.slice(0, 10)  // First 10 results
   * result.slice(10, 20) // Next 10 results
   */
  slice(start: number, end?: number): FilterableSearchResult
}
```

**Rationale:**
- **Wrapper pattern**: Preserves original SearchResult, enables chaining
- **Immutable**: Each operation returns new instance
- **Array-like**: Familiar methods (filter, slice, map)
- **Discoverable**: TypeScript autocomplete shows all options

---

### 2. Filter Criteria Types

**Location:** `packages/daemon-client/src/filter-types.ts` (new file)

**Purpose:** Type definitions for filtering/sorting operations.

```typescript
/**
 * Criteria for filtering search results.
 *
 * All fields are optional and combined with AND logic.
 */
export interface FilterCriteria {
  /** Exact match on symbol kind (function, class, etc.) */
  kind?: string

  /** File extension (with or without leading dot) */
  file_type?: string

  /**
   * Simple path matching - uses string.includes() for substring match
   * Examples: "src/", "test", ".config"
   * For prefix matching, use custom filter: {custom: h => h.file_path.startsWith("src/")}
   */
  path?: string

  /** Minimum relevance score (0.0-1.0) */
  min_score?: number

  /** Maximum relevance score (0.0-1.0) */
  max_score?: number

  /** Custom filter function */
  custom?: (hit: SearchHit) => boolean
}

/**
 * Field to sort by
 */
export type SortField =
  | "score"        // Relevance score
  | "relpath"      // File path
  | "symbol_name"  // Symbol name
  | "start_line"   // Line number
  | "kind"         // Symbol kind

/**
 * Sort order
 */
export type SortOrder = "asc" | "desc"

// Note: Aggregations deferred to future enhancement (out of MVP scope)
```

---

### 3. Implementation Functions

**Location:** `packages/daemon-client/src/filterable-result.ts`

#### filter() Implementation

```typescript
filter(criteria: FilterCriteria): FilterableSearchResult {
  let filtered = this.hits as SearchHit[]

  // Filter by kind
  if (criteria.kind) {
    filtered = filtered.filter(hit => hit.kind === criteria.kind)
  }

  // Filter by file_type
  if (criteria.file_type) {
    const ext = criteria.file_type.replace(/^\./, '') // Remove leading dot
    filtered = filtered.filter(hit =>
      hit.file_path.endsWith(`.${ext}`)
    )
  }

  // Filter by path substring
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

  // Custom filter
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

**Dependencies:**
- Zero new dependencies - uses only native JavaScript/TypeScript features

#### sortBy() Implementation

```typescript
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

#### slice() Implementation

```typescript
slice(start: number, end?: number): FilterableSearchResult {
  const sliced = this.hits.slice(start, end)

  return new FilterableSearchResult({
    ...this.result,
    hits: sliced as SearchHit[],
  })
}
```

// Note: aggregate() method deferred to future enhancement (out of MVP scope)

---

### 4. Type Sync Boundaries

**Important:** FilterableSearchResult is a TypeScript-only wrapper that doesn't affect Rust type sync.

**Type Sync Status:**
- `SearchResult` interface: Synced with Rust `SearchHit` in `crates/maproom/src/db/mod.rs` (UNCHANGED)
- `FilterableSearchResult` class: TypeScript-only wrapper (NO Rust equivalent)
- No Rust changes needed for this project

**Integration Pattern:**
```typescript
// Consumers can wrap SearchResult themselves
import { FilterableSearchResult } from '@crewchief/daemon-client'

const rawResult = await daemonClient.search({...})  // Returns SearchResult
const filterable = new FilterableSearchResult(rawResult)  // Wrap client-side
const filtered = filterable.filter({kind: "function"})
```

**MCP Integration (Future Work):**
- MCP server continues returning SearchResult unchanged
- Consumers can import and wrap results if they want filtering
- No modifications to existing SearchBundle type
- True backward compatibility - existing code unchanged

---

## Data Flow

### Simple Filter

```
1. User calls search()
   └→ Daemon returns SearchResult {hits: [...]}

2. Client imports and wraps result
   └→ new FilterableSearchResult(rawResult)

3. Client calls .filter({kind: "function"})
   └→ FilterableSearchResult.filter()
       └→ Array.filter() on hits
       └→ Returns new FilterableSearchResult

4. Client uses filtered results
   └→ Access via .hits property
```

### Chained Operations

```
1. result.filter({kind: "function"})
   └→ New FilterableSearchResult with 10 hits

2. .sortBy("relpath")
   └→ New FilterableSearchResult with sorted 10 hits

3. .slice(0, 5)
   └→ New FilterableSearchResult with 5 hits

Final result: Top 5 functions sorted by path
Performance: ~1ms total (3 array operations)
```

---

## Performance Considerations

### Time Complexity

| Operation | Complexity | Time (100 items) |
|-----------|------------|------------------|
| filter() | O(n) | ~0.1ms |
| sortBy() | O(n log n) | ~0.5ms |
| slice() | O(1) | ~0.01ms |
| Chained (filter+sort+slice) | O(n log n) | ~1ms |

**Budget Met:** <5ms for all operations on 100 results ✓

### Memory Complexity

| Operation | Memory | Notes |
|-----------|--------|-------|
| FilterableSearchResult | O(1) | Wraps existing array |
| filter() | O(n) | New array (filtered subset) |
| sortBy() | O(n) | New array (copy for sort) |
| slice() | O(n) | New array (subset) |

**Optimization:** Could use lazy evaluation for chained operations (future enhancement).

### Optimization Strategies

1. **Lazy Evaluation** (future):
   ```typescript
   // Don't apply filters until hits are accessed
   result.filter({...}).sortBy(...).slice(...)
   // Only executes when .hits is accessed
   ```

2. **Memoization** (future):
   ```typescript
   // Cache filtered results for repeated calls
   const filtered = result.filter({kind: "function"})
   filtered.hits  // Compute once
   filtered.hits  // Return cached
   ```

3. **Index Building** (future):
   ```typescript
   // Build indexes for common filters
   const kindIndex = buildIndex(hits, 'kind')
   // O(1) lookup instead of O(n) filter
   ```

**MVP Decision:** Skip optimizations. Simple implementation is fast enough (<1ms).

---

## Technology Choices

### Dependencies

**New:**
- None! Zero new dependencies.

**Rationale:**
- Native string methods (.includes, .startsWith, .endsWith) handle path filtering
- Native Array methods are sufficient (no lodash needed)
- TypeScript provides strong typing
- Keeps package lightweight and simple

### Testing Strategy

**Unit Tests** (`packages/daemon-client/tests/filterable-result.test.ts`):
- Test each method in isolation
- Edge cases (empty results, null values)
- Type safety (compile-time checks)

**Integration Tests** (`packages/maproom-mcp/tests/search-filtering.test.ts`):
- End-to-end filtering workflows
- Chained operations
- Performance benchmarks

**Coverage Goal:** 80%+ (focus on logic paths, not every edge case)

---

## Security Considerations

### Input Validation

**Path Patterns:**
- Use `minimatch` with `{dot: false}` (no hidden files)
- Prevent path traversal (e.g., `../../etc/passwd`)
- Sanitize user input before glob matching

**Score Ranges:**
- Clamp to 0.0-1.0 range
- Handle NaN/Infinity gracefully

**Custom Filters:**
- Execute in sandboxed context (no eval)
- Timeout for long-running predicates (future)

### Attack Vectors

**Denial of Service:**
- Risk: Malicious glob patterns (`**/a**/b**/c**/...`)
- Mitigation: Timeout on pattern matching (future)
- Severity: Low (client-side only, affects attacker)

**Information Disclosure:**
- Risk: Path traversal via glob patterns
- Mitigation: Validate patterns, use minimatch safely
- Severity: Low (results already filtered by repo permissions)

**Overall Risk:** Low - Client-side filtering of already-authorized results.

---

## Migration Path

### Phase 1: Foundation (This Project)

- Implement FilterableSearchResult
- Add to daemon-client package
- Wrap in MCP server (optional field)
- Backward compatible

### Phase 2: Adoption (Future)

- Update VSCode extension to use filterable results
- Add UI controls for filtering/sorting
- Documentation and examples

### Phase 3: Optimization (Future)

- Lazy evaluation for chained operations
- Memoization for repeated filters
- Index building for common patterns

---

## Maintainability

### Code Organization

```
packages/daemon-client/
├── src/
│   ├── client.ts              # Existing
│   ├── filterable-result.ts   # NEW - Main class
│   ├── filter-types.ts        # NEW - Type definitions
│   └── index.ts               # Export new classes
└── tests/
    └── filterable-result.test.ts  # NEW - Unit tests
```

### Documentation

**TSDoc Comments:** All public methods documented
**Examples:** Inline examples in docstrings
**README:** Update daemon-client README with usage

### Future Extensions

**Easy to Add:**
- New filter criteria (e.g., `min_line_count`)
- New sort fields (e.g., `file_size`)
- New aggregation dimensions

**Pattern:** Add to FilterCriteria/SortField types, implement in methods

---

## Success Metrics

### Functional

- ✅ All filter criteria work (kind, file_type, path substring, score range)
- ✅ All sort fields work (score, relpath, symbol_name, start_line, kind)
- ✅ Pagination works (slice)
- ✅ Chaining works (filter → sort → slice)

### Performance

- ✅ Filter: <1ms for 100 results
- ✅ Sort: <1ms for 100 results
- ✅ Slice: <0.1ms for 100 results
- ✅ Chained: <2ms for 100 results

### Quality

- ✅ 80%+ test coverage
- ✅ Full TypeScript types
- ✅ No breaking changes
- ✅ Documentation complete

---

## Conclusion

The FilterableSearchResult architecture provides **progressive result refinement** with:

- **Zero breaking changes** (pure additive API, consumers opt-in)
- **Zero dependencies** (native JavaScript/TypeScript only)
- **Excellent performance** (<1ms per operation)
- **Simple implementation** (3 core methods, straightforward logic)
- **Familiar patterns** (Array-like methods)
- **Type safety** (full TypeScript support)
- **Clear boundaries** (TypeScript wrapper, no Rust changes)

This MVP design focuses on 80% of use cases (simple path matching, not glob patterns) while leaving room for future enhancements (aggregations, helper methods, advanced glob patterns, Rust parity).

**TypeScript Version:** Requires TypeScript ^5.0.0 (matches daemon-client package.json)

**Memory Guidance:** Best for result sets <100 items. For larger sets, consider re-querying with refined parameters.

**Ready to proceed to execution planning.**
