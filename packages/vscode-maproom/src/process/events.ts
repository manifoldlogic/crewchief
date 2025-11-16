/**
 * Type definitions for NDJSON events emitted by the Rust binary
 *
 * The crewchief-maproom binary outputs structured NDJSON (newline-delimited JSON)
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
 * {"type":"progress","files":100,"complete":45}
 * ```
 */
export interface ProgressEvent {
  type: 'progress'
  /** Total number of files to process */
  files: number
  /** Number of files processed so far */
  complete: number
}

/**
 * Error event - emitted when an error occurs
 *
 * Example:
 * ```json
 * {"type":"error","message":"Failed to parse file","file":"src/main.rs"}
 * ```
 */
export interface ErrorEvent {
  type: 'error'
  /** Error message */
  message: string
  /** Optional file path where error occurred */
  file?: string
}

/**
 * Complete event - emitted when indexing completes
 *
 * Example:
 * ```json
 * {"type":"complete","files":100,"duration":2500}
 * ```
 */
export interface CompleteEvent {
  type: 'complete'
  /** Total number of files processed */
  files: number
  /** Duration in milliseconds */
  duration: number
}

/**
 * Status event - emitted when process state changes
 *
 * Example:
 * ```json
 * {"type":"status","state":"watching"}
 * ```
 */
export interface StatusEvent {
  type: 'status'
  /** Current state of the process */
  state: 'watching' | 'indexing' | 'idle'
}

/**
 * Union type of all possible watch events
 */
export type WatchEvent = ProgressEvent | ErrorEvent | CompleteEvent | StatusEvent

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
        event.complete <= event.files
      )

    case 'error':
      return (
        typeof event.message === 'string' &&
        (event.file === undefined || typeof event.file === 'string')
      )

    case 'complete':
      return (
        typeof event.files === 'number' &&
        typeof event.duration === 'number' &&
        event.files >= 0 &&
        event.duration >= 0
      )

    case 'status':
      return (
        typeof event.state === 'string' &&
        ['watching', 'indexing', 'idle'].includes(event.state)
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
