/**
 * Task: Missing Error Handling in Async Functions
 *
 * Find async functions that don't properly handle promise rejections.
 * This is a "negative space" task - finding what's MISSING rather than what's present.
 *
 * Why grep fails:
 * - Grep can find async functions with pattern matching
 * - Grep can find error handlers (try-catch, .catch())
 * - But grep CANNOT reason about absence - cannot find functions that DON'T have error handling
 * - Would require finding ALL async functions AND ALL error handlers, then manually diffing
 * - Multiple error handling patterns exist (try-catch, .catch(), wrapper functions)
 * - Exponentially complex as codebase grows
 *
 * Why semantic search succeeds:
 * - Understands the concept of "async function without error handling"
 * - Can reason about code structure and missing patterns
 * - Finds functions by their risk profile, not just text patterns
 * - Semantic understanding of what constitutes "unprotected async code"
 *
 * Real violations in CrewChief codebase:
 * 1. packages/cli/src/git/worktrees.ts:112 - await this.git.raw() without try-catch
 * 2. packages/cli/src/cli/release.ts:205-210 - Multiple await git operations without error handling
 * 3. packages/cli/src/utils/worktree-metadata.ts:17 - await fs.writeFile() without try-catch
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_MISSING_ERROR_HANDLING: SearchTask = {
  id: 'negative-space-missing-error-handling',
  name: 'Find Async Functions Without Error Handling',
  description:
    'Find async functions that perform risky operations (git commands, file I/O) without proper error handling. ' +
    'Look for functions using await on operations that could fail, but without try-catch blocks or .catch() handlers. ' +
    'Focus on production code in packages/cli/src/ that performs git operations or file system operations.',

  category: 'negative-space',
  difficulty: 'hard',

  searchTarget: {
    type: 'pattern',
    // Looking for files with async operations that lack error handling
    // Pattern matches key identifiers in violation files
    pattern: /worktrees|release|worktree-metadata|async.*await|git\.raw|fs\.writeFile|git\.(add|commit|push)/,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'List specific async functions that perform git operations or file I/O without error handling. ' +
      'For each function: 1) What file is it in? 2) What line number? 3) What operation is unprotected? ' +
      'Focus on production code, not test files.',
    validator: {
      type: 'explanation',
      // Must mention at least 2 of the 3 known violation files
      mentionsFiles: ['worktrees.ts', 'release.ts', 'worktree-metadata.ts'],
      // Must discuss error handling or lack thereof
      mentionsPattern:
        /(without|missing|lack|no|unprotected).*(error|try|catch|handle)|async.*without.*(try|catch|error)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  internalNotes:
    'This task demonstrates semantic search ability to reason about ABSENCE. ' +
    'Grep can only find what IS there - it cannot find what is MISSING. ' +
    'To solve with grep would require: 1) Find all async functions, 2) Find all error handlers, ' +
    '3) Manually compare and identify gaps. This becomes exponentially harder as multiple ' +
    'error handling patterns exist (try-catch, .catch(), helper functions, wrapper utilities). ' +
    'Semantic search can directly query for "risky async code without protection".',

  expectedGrepSuccess: 0.2, // 20% - grep can find patterns but cannot reason about absence
  expectedSearchSuccess: 0.8, // 80% - semantic search understands "missing" as a concept

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /worktrees|release|worktree-metadata|async.*await|git\.raw|fs\.writeFile|git\.(add|commit|push)/,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['worktrees.ts', 'release.ts', 'worktree-metadata.ts'],
        mentionsPattern:
          /(without|missing|lack|no|unprotected).*(error|try|catch|handle)|async.*without.*(try|catch|error)/i,
      },
    },
  }),
}
