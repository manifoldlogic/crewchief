/**
 * Task: API Impact Analysis
 *
 * Identify all code that would be affected by changing the createWorktree() API signature.
 * This requires finding direct callers, tests, mocks, and wrapper functions.
 *
 * Why grep fails:
 * - Grep can find direct function calls
 * - Misses indirect usages (callbacks, higher-order functions)
 * - Misses mocks and test doubles that depend on signature
 * - Cannot reason about type contracts and breaking changes
 * - Returns many false positives (unrelated worktree code)
 *
 * Why search succeeds:
 * - Understands "impact" conceptually
 * - Can find all forms of dependency (direct calls, tests, mocks, wrappers)
 * - Semantic filtering reduces false positives
 * - Can reason about type relationships
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_API_IMPACT_ANALYSIS: SearchTask = {
  id: 'relationship-impact-analysis',
  name: 'Analyze createWorktree API Impact',
  description:
    'If we changed the signature of the createWorktree() method (e.g., adding/removing parameters), ' +
    'what code would need to be updated? Find: ' +
    '1) Production code that calls createWorktree directly, ' +
    '2) Test files that mock or test createWorktree, ' +
    '3) Wrapper functions or abstractions that expose createWorktree, ' +
    '4) Type definitions or interfaces related to the API. ' +
    'Focus on code that would break with an API signature change.',

  category: 'relationship-discovery',
  difficulty: 'hard',

  searchTarget: {
    type: 'pattern',
    // Looking for various forms of API usage
    pattern:
      /createWorktree|WorktreeService.*create|wt\.createWorktree|worktree.*service.*create|mock.*createWorktree/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'List all code that would be affected by changing createWorktree() signature: ' +
      '1) What production code calls it directly? ' +
      '2) What test files test or mock it? ' +
      '3) Are there wrapper functions that delegate to it? ' +
      '4) What type definitions are involved? ' +
      'Be specific about file names and usage patterns.',
    validator: {
      type: 'explanation',
      // Must identify both production usage and test usage
      mentionsFiles: ['scheduler.ts', 'worktree.ts'],
      // Must discuss impact, changes, tests, or API surface
      mentionsPattern: /(impact|affect|break|change|test|mock|wrapper|type|interface|signature|parameter|call|usage)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  internalNotes:
    'Demonstrates that semantic search can perform impact analysis for refactoring. ' +
    'This is a common developer need: "what breaks if I change this?" Grep struggles ' +
    'because it finds too many false positives and misses indirect dependencies. ' +
    'Search can semantically filter to actual impact.',

  expectedGrepSuccess: 0.3, // 30% - grep finds some callers but misses tests and wrappers
  expectedSearchSuccess: 0.75, // 75% - search understands impact across different contexts

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern:
        /createWorktree|WorktreeService.*create|wt\.createWorktree|worktree.*service.*create|mock.*createWorktree/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['scheduler.ts', 'worktree.ts'],
        mentionsPattern:
          /(impact|affect|break|change|test|mock|wrapper|type|interface|signature|parameter|call|usage)/i,
      },
    },
  }),
}
