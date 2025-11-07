/**
 * Task: Find Rate Limiting and Flow Control
 *
 * Find all rate limiting and flow control implementations.
 * This includes: throttle functions, debounce functions, queue management,
 * circuit breakers, concurrency limiters, and backpressure handling.
 *
 * Why grep struggles (30-60% success):
 * - Multiple terms: "throttle", "debounce", "queue", "limit", "delay", "wait"
 * - Different implementations: setTimeout-based, promise-based, queue-based
 * - Implicit rate limiting (no explicit "rate limit" keyword)
 * - Pattern recognition needed: delay + loop = rate limiting
 *
 * Why semantic search succeeds (>70% success):
 * - Understands flow control concepts
 * - Recognizes rate limiting patterns without explicit keywords
 * - Connects related concepts: queues, delays, backpressure
 * - Identifies intent from implementation patterns
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_RATE_LIMITING: SearchTask = {
  id: 'tier2-conceptual-rate-limiting',
  name: 'Find Rate Limiting and Flow Control',
  category: 'conceptual-similarity',
  difficulty: 'medium',

  description:
    'Find all rate limiting and flow control mechanisms in the codebase. ' +
    'This includes: throttling, debouncing, queue management, concurrency limiting, ' +
    'delay mechanisms, and backpressure handling. ' +
    'Identify where these patterns are used and what operations they control.',

  internalNotes:
    'Grep struggles because rate limiting appears in many forms: ' +
    '- Throttle: `if (Date.now() - lastCall < minInterval) return` ' +
    '- Debounce: `clearTimeout(timer); timer = setTimeout(...)` ' +
    '- Queue: `queue.push(task); if (!running) processQueue()` ' +
    '- Delay: `await new Promise(resolve => setTimeout(resolve, ms))` ' +
    'Semantic search recognizes these as flow control mechanisms.',

  searchTarget: {
    type: 'pattern',
    // Looking for files with flow control
    pattern: /throttle|debounce|queue|delay|wait|timeout|limit.*concurrent|backpressure/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe the rate limiting and flow control patterns used in the codebase. ' +
      'For each pattern, identify: 1) The type of flow control (throttle, queue, delay, etc.), ' +
      '2) Where it is implemented (files and functions), ' +
      '3) What operations or resources it controls. ' +
      'Explain how these mechanisms prevent overload or manage resource usage.',
    validator: {
      type: 'explanation',
      // Must mention files with flow control
      mentionsFiles: ['message.bus.ts'],
      // Must discuss flow control concepts
      mentionsPattern:
        /(throttle|debounce|queue|rate.*limit|flow.*control|delay|timeout|concurrency).*(?:mechanism|pattern|control)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.4, // 40% - grep finds explicit keywords but misses implicit patterns
  expectedSearchSuccess: 0.75, // 75% - search understands flow control concepts

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /throttle|debounce|queue|delay|wait|timeout|limit.*concurrent|backpressure/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['message.bus.ts'],
        mentionsPattern:
          /(throttle|debounce|queue|rate.*limit|flow.*control|delay|timeout|concurrency).*(?:mechanism|pattern|control)/i,
      },
    },
  }),
}
