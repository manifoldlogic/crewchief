/**
 * Type definitions for MCP server tools and data structures
 */

/**
 * Parameters for the Open tool
 */
export interface OpenParams {
  /** Relative path to file from repository root */
  relpath: string

  /** Optional line range to extract */
  range?: {
    /** Start line number (1-indexed, inclusive) */
    start: number
    /** End line number (1-indexed, inclusive) */
    end: number
  }

  /** Optional worktree identifier for multi-worktree support */
  worktree?: string

  /** Optional git commit SHA to retrieve file from */
  commit?: string
}

/**
 * Return type for the Open tool
 */
export interface FileContent {
  /** File contents as string */
  content: string

  /** Echo back the relative path */
  relpath: string

  /** Echo back the range if provided */
  range?: {
    start: number
    end: number
  }
}

/**
 * Configuration options for the Open tool
 */
export interface OpenToolConfig {
  /** Maximum file size in bytes (default 1MB) */
  maxFileSize?: number

  /** Default worktree to use if not specified */
  defaultWorktree?: string
}
