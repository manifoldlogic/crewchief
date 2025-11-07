/**
 * Type 1: Finding Feature Implementation tasks
 */

import type { SearchTask } from '../types.js'
import { createTaskValidator } from '../validators.js'

/**
 * Find worktree creation implementation
 */
export const TASK_FIND_WORKTREE_CREATION: SearchTask = {
  id: 'impl-worktree-001',
  name: 'Find Worktree Creation Implementation',
  description: 'Find the code that creates git worktrees in the crewchief CLI and explain how it works',

  searchTarget: {
    type: 'file',
    path: 'packages/cli/src/git/worktree.ts',
    alternatives: ['packages/cli/src/git/worktrees.ts', 'src/git/worktree'],
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Explain how worktree creation works in this codebase',
    validator: {
      type: 'explanation',
      mentionsFiles: ['packages/cli/src/git/worktree'],
      mentionsPattern: /worktree|git worktree add|branch/i,
    },
  },

  difficulty: 'easy',
  category: 'finding-implementation',
  maxSearchAttempts: 5,
  maxTimeSeconds: 180,
  successValidator: createTaskValidator({
    searchTarget: {
      type: 'file',
      path: 'packages/cli/src/git/worktree.ts',
      alternatives: ['packages/cli/src/git/worktrees.ts', 'src/git/worktree'],
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['packages/cli/src/git/worktree'],
        mentionsPattern: /worktree|git worktree add|branch/i,
      },
    },
  }),
}

/**
 * Find agent spawning implementation
 */
export const TASK_FIND_AGENT_SPAWNING: SearchTask = {
  id: 'impl-agent-001',
  name: 'Find Agent Spawning Implementation',
  description: 'Find where agents are spawned using the SDK and explain the process',

  searchTarget: {
    type: 'file',
    path: 'packages/cli/src/sdk/spawner.ts',
    alternatives: ['src/sdk/spawner', 'spawner.ts'],
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Explain how agents are spawned in this codebase',
    validator: {
      type: 'explanation',
      mentionsFiles: ['spawner'],
      mentionsPattern: /spawn.*agent|SDK|query/i,
    },
  },

  difficulty: 'easy',
  category: 'finding-implementation',
  maxSearchAttempts: 5,
  maxTimeSeconds: 180,
  successValidator: createTaskValidator({
    searchTarget: {
      type: 'file',
      path: 'packages/cli/src/sdk/spawner.ts',
      alternatives: ['src/sdk/spawner', 'spawner.ts'],
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['spawner'],
        mentionsPattern: /spawn.*agent|SDK|query/i,
      },
    },
  }),
}

/**
 * Find variant injection implementation
 */
export const TASK_FIND_VARIANT_INJECTION: SearchTask = {
  id: 'impl-variant-001',
  name: 'Find Variant Injection Implementation',
  description: 'Find the code that modifies tool descriptions in worktrees for variant testing',

  searchTarget: {
    type: 'file',
    path: 'packages/cli/src/sdk/variant-injection.ts',
    alternatives: ['variant-injection', 'variant'],
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Explain how variant injection modifies tool descriptions',
    validator: {
      type: 'explanation',
      mentionsFiles: ['variant-injection'],
      mentionsPattern: /variant|worktree|description|modify/i,
    },
  },

  difficulty: 'medium',
  category: 'finding-implementation',
  maxSearchAttempts: 5,
  maxTimeSeconds: 180,
  successValidator: createTaskValidator({
    searchTarget: {
      type: 'file',
      path: 'packages/cli/src/sdk/variant-injection.ts',
      alternatives: ['variant-injection', 'variant'],
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['variant-injection'],
        mentionsPattern: /variant|worktree|description|modify/i,
      },
    },
  }),
}
