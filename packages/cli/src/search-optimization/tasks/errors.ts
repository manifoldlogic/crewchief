/**
 * Type 3: Locating Error Handling tasks
 */

import type { SearchTask } from '../types.js'
import { createTaskValidator } from '../validators.js'

/**
 * Find CLI error handling
 */
export const TASK_FIND_CLI_ERROR_HANDLING: SearchTask = {
  id: 'error-cli-001',
  name: 'Find CLI Error Handling',
  description: 'Find how CLI command errors are handled and logged',

  searchTarget: {
    type: 'pattern',
    pattern: /catch.*error|\.catch\(|error handling|handleError/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Explain the error handling strategy in the CLI',
    validator: {
      type: 'explanation',
      mentionsPattern: /catch|error|try.*catch|exception/i,
    },
  },

  difficulty: 'medium',
  category: 'locating-errors',
  maxSearchAttempts: 6,
  maxTimeSeconds: 240,
  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /catch.*error|\.catch\(|error handling|handleError/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsPattern: /catch|error|try.*catch|exception/i,
      },
    },
  }),
}

/**
 * Find worktree creation error handling
 */
export const TASK_FIND_WORKTREE_ERRORS: SearchTask = {
  id: 'error-worktree-001',
  name: 'Find Worktree Error Handling',
  description: 'Find how worktree creation failures are handled',

  searchTarget: {
    type: 'pattern',
    pattern: /worktree.*error|createWorktree.*catch|worktree.*fail/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Explain how worktree errors are caught and handled',
    validator: {
      type: 'explanation',
      mentionsFiles: ['worktree'],
      mentionsPattern: /error|catch|fail|exception/i,
    },
  },

  difficulty: 'medium',
  category: 'locating-errors',
  maxSearchAttempts: 6,
  maxTimeSeconds: 240,
  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /worktree.*error|createWorktree.*catch|worktree.*fail/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['worktree'],
        mentionsPattern: /error|catch|fail|exception/i,
      },
    },
  }),
}

/**
 * Find SDK error handling
 */
export const TASK_FIND_SDK_ERRORS: SearchTask = {
  id: 'error-sdk-001',
  name: 'Find SDK Error Handling',
  description: 'Find how errors from the SDK are caught and handled',

  searchTarget: {
    type: 'pattern',
    pattern: /SDK.*error|spawnAgent.*catch|agent.*fail/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Explain SDK error handling',
    validator: {
      type: 'explanation',
      mentionsFiles: ['spawner', 'sdk'],
      mentionsPattern: /error|catch|fail/i,
    },
  },

  difficulty: 'medium',
  category: 'locating-errors',
  maxSearchAttempts: 6,
  maxTimeSeconds: 240,
  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /SDK.*error|spawnAgent.*catch|agent.*fail/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['spawner', 'sdk'],
        mentionsPattern: /error|catch|fail/i,
      },
    },
  }),
}
