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

/**
 * Parameters for the Upsert tool
 */
export interface UpsertParams {
  /** Array of file or directory paths to re-index */
  paths: string[]

  /** Git commit hash for context */
  commit: string

  /** Repository name */
  repo: string

  /** Worktree identifier for isolation */
  worktree: string

  /** Root directory path of the repository */
  root: string
}

/**
 * Return type for the Upsert tool
 */
export interface UpsertResult {
  /** Number of files updated in index */
  updated_files: number

  /** Number of chunks updated in index */
  updated_chunks: number

  /** Duration of indexing operation in milliseconds */
  duration_ms: number
}

/**
 * Configuration options for the Upsert tool
 */
export interface UpsertToolConfig {
  /** Timeout for indexing operation in milliseconds (default 120000 = 2 minutes) */
  timeout?: number

  /** Environment variables to pass to indexer process */
  env?: Record<string, string>
}
