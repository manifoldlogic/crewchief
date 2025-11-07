/**
 * Task: Transitive Dependency Discovery
 *
 * Find code that indirectly depends on createWorktree() function.
 * This task requires understanding the dependency graph beyond direct references.
 *
 * Why grep fails:
 * - Grep can find direct calls to createWorktree()
 * - Cannot traverse the call chain to find indirect dependents
 * - Exponentially harder at each level of indirection
 *
 * Why search succeeds:
 * - Semantic understanding of "depends on" relationship
 * - Can query for transitive dependencies
 * - Code graph traversal built-in
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_TRANSITIVE_DEPENDENCIES: SearchTask = {
  id: 'relationship-transitive-deps',
  name: 'Find Transitive Dependencies of createWorktree',
  description:
    'Find code that depends on the createWorktree() function, including indirect dependencies. ' +
    'Identify at least 2 levels: 1) Direct callers of createWorktree(), and 2) Code that depends on those callers. ' +
    'Focus on production code that uses worktree creation transitively.',

  category: 'relationship-discovery',
  difficulty: 'hard',

  searchTarget: {
    type: 'pattern',
    // Looking for files that are in the dependency chain
    pattern: /WorktreeService|createWorktree|worktree.*create|Scheduler|assignSingleAgent/,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'List the dependency chain for createWorktree(): ' +
      '1) What file/class contains createWorktree()? ' +
      '2) What code directly calls it? ' +
      '3) What code uses those direct callers? ' +
      'Provide at least 2 levels of dependencies.',
    validator: {
      type: 'explanation',
      // Must mention the core worktree file and at least one caller at each level
      mentionsFiles: ['worktrees.ts', 'scheduler.ts'],
      // Must discuss dependency relationships
      mentionsPattern:
        /(depend|call|use|invoke).*createWorktree|createWorktree.*(depend|call|use|invoke)|transitive|indirect|chain/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  // Internal notes about why this task is valuable
  internalNotes:
    'Demonstrates that semantic search can answer "what would break if I change this API?" ' +
    'by traversing the dependency graph. Grep requires manual multi-step investigation that ' +
    'easily misses indirect paths.',

  // Expected success rates (validated empirically)
  expectedGrepSuccess: 0.2, // 20% - grep can find direct callers but struggles with transitive
  expectedSearchSuccess: 0.8, // 80% - search understands dependency relationships

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /WorktreeService|createWorktree|worktree.*create|Scheduler|assignSingleAgent/,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['worktrees.ts', 'scheduler.ts'],
        mentionsPattern:
          /(depend|call|use|invoke).*createWorktree|createWorktree.*(depend|call|use|invoke)|transitive|indirect|chain/i,
      },
    },
  }),
}
