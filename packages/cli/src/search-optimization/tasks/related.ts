/**
 * Type 4: Finding Related Code tasks
 */

import type { SearchTask } from '../types.js'
import { createTaskValidator } from '../validators.js'

/**
 * Find related test files
 */
export const TASK_FIND_RELATED_TESTS: SearchTask = {
  id: 'related-tests-001',
  name: 'Find Related Test Files',
  description: 'Find the test files for the variant injection implementation',

  searchTarget: {
    type: 'file',
    path: 'packages/cli/tests/sdk/variant-injection.test.ts',
    alternatives: ['variant-injection.test', 'variant-injection.spec'],
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Explain what the variant injection tests cover',
    validator: {
      type: 'explanation',
      mentionsFiles: ['variant-injection.test'],
      mentionsPattern: /test|expect|describe/i,
    },
  },

  difficulty: 'easy',
  category: 'finding-related',
  maxSearchAttempts: 5,
  maxTimeSeconds: 180,
  successValidator: createTaskValidator({
    searchTarget: {
      type: 'file',
      path: 'packages/cli/tests/sdk/variant-injection.test.ts',
      alternatives: ['variant-injection.test', 'variant-injection.spec'],
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['variant-injection.test'],
        mentionsPattern: /test|expect|describe/i,
      },
    },
  }),
}

/**
 * Find related types
 */
export const TASK_FIND_RELATED_TYPES: SearchTask = {
  id: 'related-types-001',
  name: 'Find Related Type Definitions',
  description: 'Find the type definitions related to agent spawning',

  searchTarget: {
    type: 'file',
    path: 'packages/cli/src/sdk/types.ts',
    alternatives: ['sdk/types', 'agent.*types'],
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Explain the key types used in agent spawning',
    validator: {
      type: 'explanation',
      mentionsFiles: ['types'],
      mentionsPattern: /interface|type|Variant|AgentResult/i,
    },
  },

  difficulty: 'medium',
  category: 'finding-related',
  maxSearchAttempts: 5,
  maxTimeSeconds: 180,
  successValidator: createTaskValidator({
    searchTarget: {
      type: 'file',
      path: 'packages/cli/src/sdk/types.ts',
      alternatives: ['sdk/types', 'agent.*types'],
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['types'],
        mentionsPattern: /interface|type|Variant|AgentResult/i,
      },
    },
  }),
}
