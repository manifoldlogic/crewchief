import type { SearchHit } from './filterable-result'

/**
 * Criteria for filtering search results.
 *
 * All fields are optional and combined with AND logic. At least one
 * criterion should be specified.
 *
 * @example
 * ```typescript
 * // Filter TypeScript functions
 * {kind: "function", file_type: "ts"}
 *
 * // Filter by path and score
 * {path: "src/", min_score: 0.5}
 *
 * // Custom filter
 * {custom: hit => hit.symbol_name?.includes("auth")}
 * ```
 */
export interface FilterCriteria {
  /**
   * Exact match on symbol kind (function, class, interface, etc.)
   *
   * Case-sensitive exact match. Common values:
   * - "function"
   * - "class"
   * - "interface"
   * - "type"
   * - "enum"
   * - "const"
   *
   * @example "function"
   */
  kind?: string

  /**
   * File extension (with or without leading dot).
   *
   * Matched against the file extension. The leading dot is optional
   * and will be normalized internally.
   *
   * @example "ts"
   * @example ".tsx"
   */
  file_type?: string

  /**
   * Path substring matching.
   *
   * Uses string.includes() for simple substring match against the
   * full file path. For more complex path matching, use the custom
   * filter function.
   *
   * For prefix matching, use custom filter:
   * ```typescript
   * {custom: h => h.file_path.startsWith("src/")}
   * ```
   *
   * For suffix matching, use custom filter:
   * ```typescript
   * {custom: h => h.file_path.endsWith(".config.ts")}
   * ```
   *
   * @example "src/"
   * @example "test"
   * @example ".config"
   */
  path?: string

  /**
   * Minimum relevance score (0.0-1.0).
   *
   * Filters out results with scores below this threshold.
   * Use this to focus on more relevant matches.
   *
   * @example 0.5
   */
  min_score?: number

  /**
   * Maximum relevance score (0.0-1.0).
   *
   * Filters out results with scores above this threshold.
   * Useful for finding lower-quality matches that might need improvement.
   *
   * @example 0.8
   */
  max_score?: number

  /**
   * Custom filter function for advanced filtering.
   *
   * Receives each SearchHit and returns true to keep, false to filter out.
   * Use this for complex filtering logic that can't be expressed with
   * the built-in criteria.
   *
   * @param hit - The search hit to evaluate
   * @returns true to keep the hit, false to filter it out
   *
   * @example
   * ```typescript
   * // Filter by symbol name
   * {custom: hit => hit.symbol_name?.includes("auth")}
   *
   * // Filter by line number
   * {custom: hit => hit.start_line > 100}
   *
   * // Complex logic
   * {custom: hit => hit.kind === "function" && hit.score > 0.5}
   * ```
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
