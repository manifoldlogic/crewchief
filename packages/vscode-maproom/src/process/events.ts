/**
 * Type definitions for NDJSON events emitted by the Rust binary
 *
 * The maproom binary outputs structured NDJSON (newline-delimited JSON)
 * events on stdout for progress tracking, error reporting, and status updates.
 *
 * Format versioning:
 * - All events should include a version field for future compatibility
 * - Current version: 1
 *
 * Event types:
 * - progress: Indexing progress (files processed, total count)
 * - error: Error occurred during indexing
 * - complete: Indexing operation completed
 * - status: Process state change (watching, indexing, idle)
 */

/**
 * Progress event - emitted during indexing to track completion
 *
 * Example:
 * ```json
 * {"type":"progress","files":100,"complete":45,"percent":45.0,"elapsed":1234,"current_file":"src/main.rs"}
 * ```
 */
export interface ProgressEvent {
  type: 'progress'
  /** Total number of files to process */
  files: number
  /** Number of files processed so far */
  complete: number
  /** Completion percentage (0-100) */
  percent?: number
  /** Elapsed time in milliseconds */
  elapsed?: number
  /** Currently processing file path */
  current_file?: string
}

/**
 * Error event - emitted when an error occurs
 *
 * Example:
 * ```json
 * {"type":"error","message":"Failed to parse file","file":"src/main.rs","error_type":"parse"}
 * ```
 */
export interface ErrorEvent {
  type: 'error'
  /** Error message */
  message: string
  /** Optional file path where error occurred */
  file?: string
  /** Category of error for better diagnostics */
  error_type?: 'parse' | 'io' | 'embedding' | 'database'
}

/**
 * Complete event - emitted when indexing completes
 *
 * Example:
 * ```json
 * {"type":"complete","files":100,"duration":2500,"elapsed":2500,"timestamp":"2025-01-16T12:34:56Z"}
 * ```
 */
export interface CompleteEvent {
  type: 'complete'
  /** Total number of files processed */
  files: number
  /** Duration in milliseconds */
  duration: number
  /** Elapsed time in milliseconds (same as duration, for consistency) */
  elapsed?: number
  /** ISO 8601 timestamp when completed */
  timestamp?: string
}

/**
 * Status event - emitted when process state changes
 *
 * Example:
 * ```json
 * {"type":"status","state":"watching","files":100}
 * ```
 */
export interface StatusEvent {
  type: 'status'
  /** Current state of the process */
  state: 'watching' | 'indexing' | 'idle'
  /** Optional file count for context */
  files?: number
}

/**
 * File processed event - emitted when a single file completes processing
 *
 * Example:
 * ```json
 * {"type":"file_processed","file_path":"src/main.rs","elapsed":123}
 * ```
 */
export interface FileProcessedEvent {
  type: 'file_processed'
  /** Path to the file that was processed */
  file_path: string
  /** Time taken to process this file in milliseconds */
  elapsed: number
}

/**
 * Branch switched event - emitted when git branch changes
 *
 * Example:
 * ```json
 * {"type":"branch_switched","timestamp":"2025-01-16T10:30:00Z","repo":"crewchief",
 *  "old_branch":"main","new_branch":"feature-auth","old_worktree_id":1,
 *  "new_worktree_id":42,"worktree_created":false}
 * ```
 */
export interface BranchSwitchedEvent {
  type: 'branch_switched'
  /** ISO 8601 timestamp when branch switched */
  timestamp: string
  /** Repository name */
  repo: string
  /** Previous branch name */
  old_branch: string
  /** New branch name */
  new_branch: string
  /** Worktree ID of the previous branch */
  old_worktree_id: number
  /** Worktree ID of the new branch */
  new_worktree_id: number
  /** True if a new worktree entry was created in SQLite */
  worktree_created: boolean
}

/**
 * Union type of all possible watch events
 */
export type WatchEvent = ProgressEvent | ErrorEvent | CompleteEvent | StatusEvent | FileProcessedEvent | BranchSwitchedEvent

/**
 * Type guard to check if object is a valid WatchEvent
 *
 * @param obj - Object to check
 * @returns true if object is a valid WatchEvent
 */
export function isWatchEvent(obj: unknown): obj is WatchEvent {
  if (typeof obj !== 'object' || obj === null) {
    return false
  }

  const event = obj as Record<string, unknown>

  // All events must have a type field
  if (typeof event.type !== 'string') {
    return false
  }

  // Validate based on event type
  switch (event.type) {
    case 'progress':
      return (
        typeof event.files === 'number' &&
        typeof event.complete === 'number' &&
        event.files >= 0 &&
        event.complete >= 0 &&
        event.complete <= event.files &&
        (event.percent === undefined || (typeof event.percent === 'number' && event.percent >= 0)) &&
        (event.elapsed === undefined || (typeof event.elapsed === 'number' && event.elapsed >= 0)) &&
        (event.current_file === undefined || typeof event.current_file === 'string')
      )

    case 'error':
      return (
        typeof event.message === 'string' &&
        (event.file === undefined || typeof event.file === 'string') &&
        (event.error_type === undefined ||
         ['parse', 'io', 'embedding', 'database'].includes(event.error_type as string))
      )

    case 'complete':
      return (
        typeof event.files === 'number' &&
        typeof event.duration === 'number' &&
        event.files >= 0 &&
        event.duration >= 0 &&
        (event.elapsed === undefined || (typeof event.elapsed === 'number' && event.elapsed >= 0)) &&
        (event.timestamp === undefined || typeof event.timestamp === 'string')
      )

    case 'status':
      return (
        typeof event.state === 'string' &&
        ['watching', 'indexing', 'idle'].includes(event.state) &&
        (event.files === undefined || typeof event.files === 'number')
      )

    case 'file_processed':
      return (
        typeof event.file_path === 'string' &&
        typeof event.elapsed === 'number' &&
        event.elapsed >= 0
      )

    case 'branch_switched':
      return (
        typeof event.timestamp === 'string' &&
        typeof event.repo === 'string' &&
        typeof event.old_branch === 'string' &&
        typeof event.new_branch === 'string' &&
        typeof event.old_worktree_id === 'number' &&
        typeof event.new_worktree_id === 'number' &&
        typeof event.worktree_created === 'boolean'
      )

    default:
      return false
  }
}

/**
 * Validate and cast an unknown object to a WatchEvent
 *
 * @param obj - Object to validate
 * @returns WatchEvent if valid
 * @throws TypeError if object is not a valid WatchEvent
 */
export function validateWatchEvent(obj: unknown): WatchEvent {
  if (!isWatchEvent(obj)) {
    throw new TypeError(
      `Invalid watch event: ${JSON.stringify(obj)}`
    )
  }
  return obj
}
