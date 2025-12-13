import type { SearchResult } from './client'
import type { FilterCriteria } from './filter-types'

/**
 * Individual search hit from daemon
 *
 * Sync with: crates/maproom/src/db/mod.rs SearchHit
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

    // Return new instance with filtered hits
    return new FilterableSearchResult({
      ...this.result,
      hits: filtered,
      total: filtered.length,
    })
  }
}
