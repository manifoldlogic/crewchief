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
