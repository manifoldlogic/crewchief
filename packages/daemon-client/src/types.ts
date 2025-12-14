/**
 * Type definitions for daemon client
 */

import type { ChildProcess } from 'node:child_process'
import type { Readable, Writable } from 'node:stream'

/**
 * Configuration for daemon process
 */
export interface DaemonConfig {
  /** Path to the daemon binary */
  binaryPath: string

  /** Environment variables for the daemon */
  env?: NodeJS.ProcessEnv

  /** Timeout for daemon startup (ms) */
  startTimeout?: number

  /** Timeout for graceful shutdown (ms) */
  shutdownTimeout?: number

  /** Maximum number of restart attempts */
  maxRestartAttempts?: number

  /** Initial backoff delay for restarts (ms) */
  restartBackoffMs?: number

  /** Request timeout (ms) */
  timeout?: number

  /** Enable auto-restart on crash */
  autoRestart?: boolean
}

/**
 * Daemon process handle with streams
 */
export interface DaemonProcessDef {
  process: ChildProcess
  stdin: Writable
  stdout: Readable
  stderr: Readable
}

/**
 * Pending request waiting for response
 */
export interface PendingRequest<T = unknown> {
  /** Promise that resolves with the response */
  promise: Promise<T>
  /** Resolve callback */
  resolve: (value: T) => void
  /** Reject callback */
  reject: (error: Error) => void
  /** Timestamp when request was created (milliseconds since epoch) */
  timestamp: number
  /** Timeout timer */
  timer: NodeJS.Timeout
}

/**
 * High-level error type categories for search errors.
 *
 * Maps 13+ observed error scenarios to 6 actionable error types.
 *
 * Sync with: crates/maproom/src/search/errors.rs::ErrorType
 */
export type ErrorType =
  | 'embedding_provider'
  | 'database'
  | 'validation'
  | 'timeout'
  | 'not_found'
  | 'unknown'

/**
 * Pipeline stage where error occurred.
 *
 * Helps identify which part of the search pipeline failed.
 *
 * Sync with: crates/maproom/src/search/errors.rs::PipelineStage
 */
export type PipelineStage =
  | 'query_processing'
  | 'search_execution'
  | 'score_fusion'
  | 'result_assembly'

/**
 * Structured error details with actionable suggestions.
 *
 * This interface provides a structured representation of search errors that can be
 * deserialized from JSON responses and used for display or logging.
 *
 * Sync with: crates/maproom/src/search/errors.rs::SearchErrorDetails
 */
export interface SearchErrorDetails {
  /** High-level error type category */
  error_type: ErrorType
  /** Pipeline stage where the error occurred */
  stage: PipelineStage
  /** Whitelisted context information extracted from the error */
  context: Record<string, string>
  /** 1-2 actionable suggestions for resolving the error */
  suggestions: string[]
}

/**
 * Query understanding metadata for successful searches (Phase 2).
 *
 * Provides transparency about how the query was interpreted, what filters
 * were applied, and timing breakdown.
 *
 * Sync with: crates/maproom/src/search/results.rs::QueryUnderstanding
 */
export interface QueryUnderstanding {
  /** Detected search mode */
  mode: 'code' | 'text' | 'auto'
  /** Tokenized query terms */
  tokens: string[]
  /** Expanded query terms (synonyms, variations) */
  expanded_terms: string[]
  /** Applied filters */
  filters: QueryFilters
  /** Fusion strategy name (e.g., "reciprocal_rank_fusion", "basic_weighted") */
  fusion_strategy: string
  /** Timing breakdown for search stages */
  timing: TimingBreakdown
}

/**
 * Filters applied to the search query.
 *
 * Sync with: crates/maproom/src/search/results.rs::QueryFilters
 */
export interface QueryFilters {
  /** Repository ID being searched */
  repo_id: number
  /** Optional worktree ID filter */
  worktree_id: number | null
  /** File type filters (e.g., ["ts", "tsx", "js"]) */
  file_types: string[]
  /** Recency threshold filter (e.g., "7 days", "1 month") */
  recency_threshold: string | null
}

/**
 * Timing breakdown for search execution stages.
 *
 * Sync with: crates/maproom/src/search/results.rs::TimingBreakdown
 */
export interface TimingBreakdown {
  /** Time spent processing the query (ms) */
  query_processing_ms: number
  /** Time spent executing searches (ms) */
  search_execution_ms: number
  /** Time spent fusing scores (ms) */
  score_fusion_ms: number
  /** Time spent assembling final results (ms) */
  result_assembly_ms: number
  /** Total time across all stages (ms) */
  total_ms: number
}

/**
 * Confidence signals for assessing search result quality.
 *
 * Sync with: crates/maproom/src/search/results.rs::ConfidenceSignals
 */
export interface ConfidenceSignals {
  /** Number of search sources that returned this chunk (1-4) */
  source_count: number
  /** Score difference between this result and next result */
  score_gap: number
  /** Whether query exactly matched symbol name */
  is_exact_match: boolean
}

/**
 * Metadata about search execution and results.
 *
 * Extended in Phase 2 with optional query understanding metadata.
 *
 * Sync with: crates/maproom/src/search/results.rs::SearchMetadata
 */
export interface SearchMetadata {
  /** Query understanding metadata (optional, added in Phase 2) */
  understanding?: QueryUnderstanding
}
