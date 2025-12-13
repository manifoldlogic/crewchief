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
