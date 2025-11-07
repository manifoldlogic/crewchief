/**
 * Task: Find All Error Handling Patterns
 *
 * Locate all error handling implementations across different paradigms.
 * This includes: try/catch blocks, .catch() promise handlers, error boundaries,
 * Result types, Either monads, and custom error handling middleware.
 *
 * Why grep struggles (30-60% success):
 * - Multiple keywords: "catch", "error", "Error", "throw", "reject"
 * - Different patterns: try/catch, .catch(), if (error), Result<T, E>
 * - Functional vs imperative styles
 * - False positives: comments, variable names containing "error"
 *
 * Why semantic search succeeds (>70% success):
 * - Recognizes error handling intent across paradigms
 * - Understands relationship between error creation and handling
 * - Identifies error propagation patterns
 * - Distinguishes actual error handling from error definitions
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_ERROR_HANDLING_PATTERNS: SearchTask = {
  id: 'tier2-conceptual-error-handling',
  name: 'Find All Error Handling Patterns',
  category: 'conceptual-similarity',
  difficulty: 'medium',

  description:
    'Find all error handling implementations in the codebase. ' +
    'This includes: try/catch blocks, .catch() promise handlers, error type checking, ' +
    'custom error classes, error propagation, and error transformation. ' +
    'Identify the different error handling strategies used and where they appear.',

  internalNotes:
    'Grep struggles with multiple error handling patterns: ' +
    '- Synchronous: `try { ... } catch (error) { ... }` ' +
    '- Promises: `.catch((err) => handleError(err))` ' +
    '- Type guards: `if (error instanceof Error)` ' +
    '- Result types: `if (!result.success) { handle(result.error) }` ' +
    'Semantic search recognizes these as variations of error handling.',

  searchTarget: {
    type: 'pattern',
    // Looking for files with error handling
    pattern: /catch\s*\(|\.catch\(|throw\s+new\s+Error|if.*error|Error\s*\(|reject\(/,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe the error handling patterns used in the codebase. ' +
      'For each pattern, identify: 1) The error handling approach (try/catch, promise chains, etc.), ' +
      '2) Where it is used (files and functions), ' +
      '3) How errors are propagated or transformed. ' +
      'Include at least 3 different files that demonstrate different error handling styles.',
    validator: {
      type: 'explanation',
      // Must mention files with error handling
      mentionsFiles: ['merge.ts', 'loader.ts'],
      // Must discuss error handling concepts
      mentionsPattern: /(error|errors|exception).*(?:handling|pattern|strategy|propagation|catch|throw)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.5, // 50% - grep finds basic catch blocks but misses functional patterns
  expectedSearchSuccess: 0.85, // 85% - search understands error handling concepts

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /catch\s*\(|\.catch\(|throw\s+new\s+Error|if.*error|Error\s*\(|reject\(/,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['merge.ts', 'loader.ts'],
        mentionsPattern: /(error|errors|exception).*(?:handling|pattern|strategy|propagation|catch|throw)/i,
      },
    },
  }),
}
