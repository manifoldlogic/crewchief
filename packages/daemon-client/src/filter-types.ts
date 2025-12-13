import type { SearchHit } from './filterable-result'

/**
 * Criteria for filtering search results.
 *
 * All fields are optional and combined with AND logic.
 *
 * @example
 * // Filter TypeScript functions
 * {kind: "function", file_type: "ts"}
 *
 * @example
 * // Filter by path and score
 * {path: "src/", min_score: 0.5}
 *
 * @example
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
   *
   * @example
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
