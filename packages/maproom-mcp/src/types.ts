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

  /** Embedding provider used (ollama, openai, google) */
  provider?: string

  /** Embedding dimension (768 or 1536) */
  dimension?: number
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

/**
 * Parameters for the Explain tool
 */
export interface ExplainParams {
  /** Chunk ID to explain (from search results) */
  chunk_id: number
}

/**
 * Configuration options for the Explain tool
 */
export interface ExplainToolConfig {
  /** Whether the tool is enabled (default: false - experimental) */
  enabled?: boolean

  /** Cache TTL in milliseconds (default: 5 minutes) */
  cacheTtlMs?: number
}

/**
 * Parameters for the Search tool
 */
export interface SearchParams {
  /** Search query text - use 2-3 keyword concepts for best results */
  query: string

  /** Repository name to search (e.g., "crewchief") */
  repo?: string

  /** Worktree/branch name to search (e.g., "main") */
  worktree?: string

  /** Maximum number of results to return (default: 20, max: 100) */
  limit?: number

  /** Search mode: "fts" for full-text, "vector" for semantic, "hybrid" for combined (default: fts) */
  mode?: 'fts' | 'vector' | 'hybrid'

  /** Content type filter */
  filter?: 'all' | 'code' | 'docs' | 'config' | 'tests'

  /** Advanced filters */
  filters?: {
    /** Comma-separated list of file extensions (e.g., "ts,tsx,js") */
    file_type?: string
    /** Filter by specific worktree ID */
    worktree_id?: number
  }

  /** Include score breakdown and debug information in results */
  debug?: boolean
}

/**
 * Search result item
 */
export interface SearchResult {
  /** Chunk ID for further context retrieval */
  chunk_id: number

  /** Symbol name (function, class, etc.) */
  symbol_name: string | null

  /** Chunk kind (function, class, module, etc.) */
  kind: string

  /** Relative path to file from repository root */
  relpath: string

  /** Start line number of the chunk */
  start_line: number

  /** End line number of the chunk */
  end_line: number

  /** Relevance score */
  score: number

  /** Optional preview text */
  preview?: string

  /** Debug information (only if debug=true) */
  debug?: {
    fts_score: number | null
    vector_score: number | null
    recency_score: number | null
    churn_score: number | null
    final_score: number
  }
}

/**
 * Return type for the Search tool
 */
export interface SearchBundle {
  /** Array of search results */
  hits: SearchResult[]

  /** Total number of hits returned */
  total: number

  /** Echo back the query */
  query: string

  /** Echo back the search mode */
  mode: string

  /** Echo back the repository filter */
  repo?: string

  /** Echo back the worktree filter */
  worktree?: string

  /** Optional hint/warning messages */
  hint?: string

  /** Error message if search failed */
  error?: string

  /** Suggestion for fixing the error */
  suggestion?: string
}
