import { spawn } from 'node:child_process'
import { AgentMessage, FileChangePayload } from './message.types.js'
import { logger } from '../utils/logger.js'
import { findMaproomBinary } from '../utils/maproom-binary.js'

/**
 * Handle `file-change` bus events by calling `maproom upsert` for each
 * modified file listed in the event payload (Connection D).
 *
 * The upsert call is fire-and-forget: errors are logged but never propagated
 * to the bus system to avoid crashing the event loop. If the maproom binary
 * is not found, the handler becomes a no-op after a single warning.
 *
 * @param worktreePath - Absolute path to the worktree (used as cwd for the
 *   maproom child process so it can auto-detect git context).
 */
export function createFileChangeHandler(worktreePath: string) {
  return (message: AgentMessage): void => {
    if (message.type !== 'file-change') return

    const payload = message.payload as FileChangePayload
    if (!payload?.files?.length) return

    // Resolve binary once per invocation (cheap — cached by the OS).
    const binary = findMaproomBinary()
    if (!binary.path) {
      logger.warn('maproom binary not found — skipping upsert for file-change event')
      return
    }

    // Collect paths of added/modified files (deletions are not upserted).
    const filePaths = payload.files.filter((f) => f.status !== 'deleted').map((f) => f.path)

    if (filePaths.length === 0) return

    // Fire-and-forget: spawn maproom upsert as a detached child process.
    try {
      const child = spawn(binary.path, ['upsert', ...filePaths], {
        cwd: worktreePath,
        stdio: 'ignore',
        detached: true,
      })

      // Prevent unhandled error from crashing the bus loop.
      child.on('error', (err) => {
        logger.warn(`maproom upsert failed: ${err.message}`)
      })

      // Allow the parent to exit without waiting for the child.
      if (typeof child.unref === 'function') {
        child.unref()
      }
    } catch (err) {
      logger.warn(`Failed to spawn maproom upsert: ${err instanceof Error ? err.message : err}`)
    }
  }
}
