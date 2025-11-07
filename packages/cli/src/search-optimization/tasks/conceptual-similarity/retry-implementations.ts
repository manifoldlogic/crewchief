/**
 * Task: Find All Retry Logic Implementations
 *
 * Find all retry logic across the codebase, regardless of implementation approach.
 * This includes: decorators, exponential backoff, circuit breakers, manual loops,
 * promise retries, and timeout-based retries.
 *
 * Why grep struggles (30-60% success):
 * - Multiple keywords: "retry", "backoff", "attempt", "timeout", "circuit"
 * - Different patterns: decorators (@retry), manual loops (for/while), promise chains
 * - Conceptually similar but syntactically different implementations
 * - Must distinguish actual retry logic from comments mentioning "retry"
 *
 * Why semantic search succeeds (>70% success):
 * - Understands the concept of "retry logic" across implementations
 * - Recognizes patterns: loop with delay, exponential backoff calculation
 * - Connects related code even with different keywords
 * - Identifies intent rather than exact string matches
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_RETRY_IMPLEMENTATIONS: SearchTask = {
  id: 'tier2-conceptual-retry',
  name: 'Find All Retry Logic Implementations',
  category: 'conceptual-similarity',
  difficulty: 'medium',

  description:
    'Find all retry logic implementations in the codebase. ' +
    'This includes any code that automatically retries failed operations, such as: ' +
    'exponential backoff, circuit breakers, manual retry loops, promise retry wrappers, ' +
    'and timeout-based retry mechanisms. ' +
    'List the files containing retry logic and explain the different retry patterns used.',

  internalNotes:
    'Grep struggles because retry logic has many forms: ' +
    '- Manual loops: `for (let i = 0; i < maxRetries; i++)` ' +
    '- Exponential backoff: `delay = baseDelay * Math.pow(2, attempt)` ' +
    '- Promise retries: `.catch(() => retryOperation())` ' +
    '- Timeout handling: `setTimeout(() => retry(), delay)` ' +
    'Semantic search recognizes the retry pattern concept regardless of syntax.',

  searchTarget: {
    type: 'pattern',
    // Looking for files with retry patterns
    pattern: /retry|backoff|attempt|circuit.*break|waitFor.*Response|timeout.*retry/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe all retry logic patterns found in the codebase. ' +
      'For each pattern, identify: 1) What kind of retry mechanism (exponential backoff, fixed delay, etc.), ' +
      '2) Where it is implemented (file and function), ' +
      '3) What operations it retries (network calls, file operations, etc.). ' +
      'Focus on actual retry implementations, not just error handling.',
    validator: {
      type: 'explanation',
      // Must mention key files with retry logic
      mentionsFiles: ['message.bus.ts'],
      // Must discuss retry concepts
      mentionsPattern: /(retry|retries|attempt|backoff|circuit|timeout).*(?:logic|pattern|mechanism|implementation)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.45, // 45% - grep finds some with "retry" keyword but misses variations
  expectedSearchSuccess: 0.8, // 80% - search understands retry concepts

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /retry|backoff|attempt|circuit.*break|waitFor.*Response|timeout.*retry/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['message.bus.ts'],
        mentionsPattern: /(retry|retries|attempt|backoff|circuit|timeout).*(?:logic|pattern|mechanism|implementation)/i,
      },
    },
  }),
}
