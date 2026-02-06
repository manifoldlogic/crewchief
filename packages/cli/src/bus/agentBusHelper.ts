import { CrossProcessBusWriter } from './crossProcessBusWriter'
import { ErrorPayload, FileChangePayload } from './message.types'

/**
 * Create a writer from the CREWCHIEF_BUS_PATH environment variable.
 * Returns undefined if the environment variable is not set.
 */
export function createBusWriter(): CrossProcessBusWriter | undefined {
  const busPath = process.env.CREWCHIEF_BUS_PATH
  if (!busPath) return undefined
  return new CrossProcessBusWriter(busPath)
}

/**
 * Convenience: write a status message to the bus.
 * No-op if CREWCHIEF_BUS_PATH is not set.
 *
 * @throws Error if message exceeds 4096 bytes (POSIX atomicity limit)
 */
export function reportStatus(activity: string, from: string): void {
  const writer = createBusWriter()
  if (!writer) return
  writer.write({
    type: 'status',
    from,
    to: 'orchestrator',
    payload: { activity },
    timestamp: new Date(),
  })
}

/**
 * Convenience: write a completion message to the bus.
 * No-op if CREWCHIEF_BUS_PATH is not set.
 *
 * @throws Error if message exceeds 4096 bytes (POSIX atomicity limit)
 */
export function reportCompletion(summary: string, from: string): void {
  const writer = createBusWriter()
  if (!writer) return
  writer.write({
    type: 'completion',
    from,
    to: 'orchestrator',
    payload: { summary },
    timestamp: new Date(),
  })
}

/**
 * Convenience: write an error message to the bus.
 * No-op if CREWCHIEF_BUS_PATH is not set.
 *
 * If an Error object is provided, extracts code and stack trace.
 *
 * @throws Error if message exceeds 4096 bytes (POSIX atomicity limit)
 */
export function reportError(message: string, from: string, recoverable = false, error?: Error): void {
  const writer = createBusWriter()
  if (!writer) return

  const payload: ErrorPayload = {
    message,
    recoverable,
    code: error?.name,
    stack: error?.stack,
  }

  writer.write({
    type: 'error',
    from,
    to: 'orchestrator',
    payload,
    timestamp: new Date(),
  })
}

/**
 * Convenience: write a file-change message to the bus.
 * No-op if CREWCHIEF_BUS_PATH is not set.
 *
 * @throws Error if message exceeds 4096 bytes (POSIX atomicity limit)
 */
export function reportFileChanges(files: FileChangePayload['files'], from: string): void {
  const writer = createBusWriter()
  if (!writer) return
  writer.write({
    type: 'file-change',
    from,
    to: 'orchestrator',
    payload: { files },
    timestamp: new Date(),
  })
}
