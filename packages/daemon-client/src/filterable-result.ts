import type { SearchResult } from './client'

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
}
