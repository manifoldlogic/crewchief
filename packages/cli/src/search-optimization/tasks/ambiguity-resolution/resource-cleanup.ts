/**
 * Task: Find Resource Cleanup Code
 *
 * Find all resource cleanup implementations, where "close" is ambiguous.
 * This includes: file handle closing, database connection closing, stream closing,
 * socket cleanup, timer cleanup, and event listener removal.
 *
 * Why grep struggles (30-60% success):
 * - "close" has many meanings: close file, close connection, close dialog, close position
 * - Different cleanup methods: close(), dispose(), cleanup(), destroy(), unsubscribe()
 * - Implicit cleanup: try/finally blocks, using statements
 * - False positives: "close to", "closed", "closing" in comments
 *
 * Why semantic search succeeds (>70% success):
 * - Recognizes cleanup patterns: close, dispose, cleanup sequences
 * - Understands resource lifecycle: open/acquire → use → close/release
 * - Identifies cleanup in finally blocks and destructors
 * - Connects related cleanup concepts across different resource types
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_RESOURCE_CLEANUP: SearchTask = {
  id: 'tier2-ambiguity-cleanup',
  name: 'Find Resource Cleanup Code',
  category: 'ambiguity-resolution',
  difficulty: 'medium',

  description:
    'Find all resource cleanup and disposal code in the codebase. ' +
    'This includes: closing file handles, closing database connections, ' +
    'closing streams or sockets, clearing timers, removing event listeners, ' +
    'and any try/finally blocks that ensure cleanup. ' +
    'Identify what resources are being cleaned up and how.',

  internalNotes:
    'Grep struggles with "close" and cleanup ambiguity: ' +
    '- File cleanup: `fs.close(fd)`, `stream.close()` ' +
    '- Connection cleanup: `connection.close()`, `db.disconnect()` ' +
    '- Timer cleanup: `clearTimeout(timer)`, `clearInterval(interval)` ' +
    '- Listener cleanup: `emitter.off("event", handler)` ' +
    '- Generic: `finally { cleanup() }` ' +
    'Semantic search recognizes the cleanup pattern.',

  searchTarget: {
    type: 'pattern',
    // Looking for files with cleanup code
    pattern:
      /\.close\(\)|\.dispose\(\)|\.cleanup\(\)|\.destroy\(\)|clearTimeout|clearInterval|finally\s*\{|unsubscribe|off\(/,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe the resource cleanup patterns found in the codebase. ' +
      'For each pattern, identify: 1) The resource being cleaned up (files, connections, timers, etc.), ' +
      '2) Where cleanup happens (files and functions), ' +
      '3) The cleanup mechanism (explicit close, finally block, etc.), ' +
      '4) Whether cleanup is guaranteed (try/finally, error handling). ' +
      'Focus on actual cleanup implementation.',
    validator: {
      type: 'explanation',
      // Must mention files with cleanup
      mentionsFiles: ['message.bus.ts', 'merge.ts'],
      // Must discuss cleanup concepts
      mentionsPattern:
        /(cleanup|clean.*up|close|dispose|destroy|clear|remove|release).*(?:resource|pattern|mechanism|implementation)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.5, // 50% - grep finds "close" but many false positives
  expectedSearchSuccess: 0.8, // 80% - search understands cleanup context

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern:
        /\.close\(\)|\.dispose\(\)|\.cleanup\(\)|\.destroy\(\)|clearTimeout|clearInterval|finally\s*\{|unsubscribe|off\(/,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['message.bus.ts', 'merge.ts'],
        mentionsPattern:
          /(cleanup|clean.*up|close|dispose|destroy|clear|remove|release).*(?:resource|pattern|mechanism|implementation)/i,
      },
    },
  }),
}
