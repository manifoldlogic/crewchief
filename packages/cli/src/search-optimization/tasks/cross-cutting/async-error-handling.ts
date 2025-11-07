/**
 * Task: Find Error Handling in Async Operations
 *
 * Find all error handling specifically in async/await code.
 * This is scattered across the codebase and requires aggregation.
 *
 * Why grep struggles (30-60% success):
 * - Async error handling is scattered throughout many files
 * - Must combine two concepts: async AND error handling
 * - Different patterns: try/catch around await, .catch() on promises, error event handlers
 * - Cannot aggregate results meaningfully (just list of matches)
 *
 * Why semantic search succeeds (>70% success):
 * - Understands "async error handling" as combined concept
 * - Recognizes patterns: try/catch with await, Promise.catch chains
 * - Aggregates scattered implementations
 * - Identifies async-specific error handling vs synchronous
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_ASYNC_ERROR_HANDLING: SearchTask = {
  id: 'tier2-cross-cutting-async-errors',
  name: 'Find Error Handling in Async Operations',
  category: 'cross-cutting-concerns',
  difficulty: 'medium',

  description:
    'Find all error handling specifically in asynchronous operations. ' +
    'This includes: try/catch blocks around await statements, .catch() handlers on promises, ' +
    'async error propagation, unhandled promise rejection handling, and async error boundaries. ' +
    'Aggregate the different async error handling patterns used across the codebase.',

  internalNotes:
    'Grep struggles because async error handling is scattered: ' +
    '- Try/catch: `try { await operation() } catch (error) { ... }` ' +
    '- Promise catch: `operation().catch(err => handleError(err))` ' +
    '- Error propagation: `async function() { throw error }` (caught by caller) ' +
    '- Must find AND aggregate many small instances ' +
    'Semantic search aggregates async error patterns.',

  searchTarget: {
    type: 'pattern',
    // Looking for files with async error handling
    pattern: /async.*catch|await.*catch|\.catch\(|Promise.*catch|async.*throw|reject\(/,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe the error handling patterns used in async operations across the codebase. ' +
      'Identify: 1) The most common async error handling patterns (try/catch, .catch(), etc.), ' +
      '2) At least 5 different files that handle async errors, ' +
      '3) How errors are propagated in async chains, ' +
      '4) Whether there are unhandled promise rejections. ' +
      'Focus on async-specific error handling, not general error handling.',
    validator: {
      type: 'explanation',
      // Must mention multiple files with async errors
      mentionsFiles: ['merge.ts', 'loader.ts'],
      // Must discuss async error handling
      mentionsPattern:
        /(async|asynchronous|promise|await).*(?:error|exception).*(?:handling|catch|propagation|rejection)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.5, // 50% - grep finds individual instances but can't aggregate well
  expectedSearchSuccess: 0.85, // 85% - search understands and aggregates async error patterns

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /async.*catch|await.*catch|\.catch\(|Promise.*catch|async.*throw|reject\(/,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['merge.ts', 'loader.ts'],
        mentionsPattern:
          /(async|asynchronous|promise|await).*(?:error|exception).*(?:handling|catch|propagation|rejection)/i,
      },
    },
  }),
}
