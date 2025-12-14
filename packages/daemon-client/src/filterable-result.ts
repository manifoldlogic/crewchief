import type { SearchResult } from './client'
import type { FilterCriteria, SortField, SortOrder } from './filter-types'

/**
 * Individual search hit from daemon
 *
 * Derived from SearchResult['hits'] type - see client.ts for source of truth.
 */
export type SearchHit = SearchResult['hits'][number]

/**
 * Search result with client-side filtering/sorting/pagination.
 *
 * Wraps SearchResult from daemon and provides chainable methods
 * for progressive result refinement without re-querying.
 *
 * All operations are immutable - original result is preserved.
 *
 * @example
 * ```typescript
 * const result = new FilterableSearchResult(daemonResult)
 *
 * // Filter TypeScript functions
 * const functions = result.filter({kind: "function", file_type: "ts"})
 *
 * // Sort by path
 * const sorted = functions.sortBy("relpath")
 *
 * // Get first page
 * const page1 = sorted.slice(0, 10)
 *
 * // Or chain it all
 * const filtered = result
 *   .filter({kind: "function", file_type: "ts"})
 *   .sortBy("relpath")
 *   .slice(0, 10)
 * ```
 */
export class FilterableSearchResult {
  private readonly result: SearchResult

  /**
   * Create a filterable search result wrapper.
   *
   * @param result - Search result from daemon
   *
   * @example
   * ```typescript
   * const daemon = new DaemonClient()
   * const searchResult = await daemon.search({query: "auth"})
   * const filterable = new FilterableSearchResult(searchResult)
   * ```
   */
  constructor(result: SearchResult) {
    this.result = result
  }

  /**
   * Get raw search result (unwrapped).
   *
   * Useful for accessing original daemon response or passing to APIs
   * that expect SearchResult.
   *
   * @returns Original SearchResult from daemon
   */
  get raw(): SearchResult {
    return this.result
  }

  /**
   * Get hits array (readonly).
   *
   * @returns Array of search hits after filtering/sorting
   */
  get hits(): readonly SearchHit[] {
    return this.result.hits
  }

  /**
   * Get total count before filtering.
   *
   * This is the original total from the daemon, not affected by
   * client-side filtering.
   *
   * @returns Original total count from daemon
   */
  get total(): number {
    return this.result.total
  }

  /**
   * Get count after filtering.
   *
   * This reflects the current number of hits after all filter/sort/slice
   * operations have been applied.
   *
   * @returns Current number of hits in result
   */
  get count(): number {
    return this.result.hits.length
  }

  /**
   * Filter results by criteria.
   *
   * Supports:
   * - kind: Exact match on symbol kind
   * - file_type: File extension (with or without dot)
   * - path: Substring match using string.includes()
   * - min_score: Minimum relevance score
   * - max_score: Maximum relevance score
   * - custom: Custom predicate function
   *
   * All criteria are combined with AND logic.
   *
   * @param criteria - Filter criteria object
   * @returns New FilterableSearchResult instance with filtered hits
   *
   * @example
   * // Filter by kind
   * result.filter({kind: "function"})
   *
   * @example
   * // Filter by file type (with or without dot)
   * result.filter({file_type: "ts"})
   * result.filter({file_type: ".tsx"})
   *
   * @example
   * // Filter by path substring
   * result.filter({path: "src/"})
   *
   * @example
   * // Filter by score range
   * result.filter({min_score: 0.5})
   * result.filter({min_score: 0.3, max_score: 0.8})
   *
   * @example
   * // Custom filter function
   * result.filter({custom: hit => hit.symbol_name?.includes("auth")})
   *
   * @example
   * // Multiple criteria (AND logic)
   * result.filter({kind: "function", file_type: "ts", min_score: 0.5})
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

    // Return new instance with filtered hits (preserves original total)
    return new FilterableSearchResult({
      ...this.result,
      hits: filtered,
    })
  }

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
   * @returns New FilterableSearchResult instance with sorted hits
   *
   * @example
   * ```typescript
   * result.sortBy("score")              // Highest scores first
   * result.sortBy("relpath")            // Alphabetical by path
   * result.sortBy("start_line", "desc") // Later lines first
   * ```
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
      let aVal: number | string
      let bVal: number | string

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

  /**
   * Slice results for pagination.
   *
   * Uses the same API as Array.slice() for familiarity.
   *
   * @param start - Start index (inclusive, 0-based)
   * @param end - End index (exclusive), omit for "to end"
   * @returns New FilterableSearchResult instance with sliced hits
   *
   * @example
   * ```typescript
   * result.slice(0, 10)  // First 10 results (page 1)
   * result.slice(10, 20) // Next 10 results (page 2)
   * result.slice(5)      // Skip first 5, return rest
   * ```
   */
  slice(start: number, end?: number): FilterableSearchResult {
    const sliced = this.hits.slice(start, end)

    return new FilterableSearchResult({
      ...this.result,
      hits: sliced as SearchHit[],
    })
  }
}
