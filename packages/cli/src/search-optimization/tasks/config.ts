/**
 * Type 5: Locating Configuration/Entry Points tasks
 */

import type { SearchTask } from '../types.js'
import { createTaskValidator } from '../validators.js'

/**
 * Find CLI entry point
 */
export const TASK_FIND_CLI_ENTRY: SearchTask = {
  id: 'config-entry-001',
  name: 'Find CLI Entry Point',
  description: 'Find the main CLI entry point that handles commands',

  searchTarget: {
    type: 'file',
    path: 'packages/cli/src/cli/index.ts',
    alternatives: ['src/cli/index', 'cli/index'],
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Explain the CLI command structure',
    validator: {
      type: 'explanation',
      mentionsFiles: ['cli/index'],
      mentionsPattern: /command|Commander|program/i,
    },
  },

  difficulty: 'easy',
  category: 'locating-config',
  maxSearchAttempts: 4,
  maxTimeSeconds: 120,
  successValidator: createTaskValidator({
    searchTarget: {
      type: 'file',
      path: 'packages/cli/src/cli/index.ts',
      alternatives: ['src/cli/index', 'cli/index'],
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['cli/index'],
        mentionsPattern: /command|Commander|program/i,
      },
    },
  }),
}

/**
 * Find SDK configuration
 */
export const TASK_FIND_SDK_CONFIG: SearchTask = {
  id: 'config-sdk-001',
  name: 'Find SDK Configuration',
  description: 'Find where SDK configuration is defined and loaded',

  searchTarget: {
    type: 'file',
    path: 'packages/cli/src/sdk/config.ts',
    alternatives: ['src/sdk/config', 'sdk/config'],
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Explain the SDK configuration options',
    validator: {
      type: 'explanation',
      mentionsFiles: ['sdk/config'],
      mentionsPattern: /config|permission|model/i,
    },
  },

  difficulty: 'easy',
  category: 'locating-config',
  maxSearchAttempts: 4,
  maxTimeSeconds: 120,
  successValidator: createTaskValidator({
    searchTarget: {
      type: 'file',
      path: 'packages/cli/src/sdk/config.ts',
      alternatives: ['src/sdk/config', 'sdk/config'],
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['sdk/config'],
        mentionsPattern: /config|permission|model/i,
      },
    },
  }),
}
