/**
 * Task: Verify Error Handling Consistency
 *
 * Real-world scenario: Reviewing error handling changes to ensure consistency
 * across the API layer. Need to find similar error patterns to verify they all
 * follow the same approach.
 *
 * Common code review task: When error handling patterns change, reviewers verify
 * consistency across the codebase to maintain code quality.
 *
 * Tool-agnostic: Task describes the consistency check without prescribing tools.
 * Both grep and semantic search can identify error patterns.
 */

import type { SearchTask } from '../../../types.js'
import { createTaskValidator } from '../../../validators.js'

export const TASK_ERROR_HANDLING_CONSISTENCY: SearchTask = {
  id: 'tier3-code-review-error-consistency',
  name: 'Review Error Handling: Find API Error Patterns',
  category: 'code-review',
  difficulty: 'easy',
  tier: 'tier3-realworld',

  description:
    'A developer updated error handling in one API endpoint to return structured error responses. ' +
    'For your code review, you need to find all similar error handling patterns in other API endpoints. ' +
    'Identify where errors are caught and returned, what error formats are used, and whether ' +
    'error handling is consistent across the API layer. This ensures a uniform error handling strategy.',

  realWorldScenario:
    'Based on typical code review scenario: reviewing error handling changes for consistency. ' +
    'Common pattern: One endpoint improves error handling, reviewer checks if others should match. ' +
    'Frequency: monthly in teams focused on API quality.',

  searchTarget: {
    type: 'pattern',
    // Looking for error handling in API/route handlers
    pattern: /(catch.*error|try.*catch|throw.*Error|error.*response|handle.*error|\.status\(\d{3}\))/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'List all locations in the API layer where errors are caught and handled. ' +
      'For each location, identify: ' +
      '1) The file and endpoint/route name, ' +
      '2) How errors are caught (try/catch, error middleware, etc.), ' +
      '3) What error response format is used (status code, message structure). ' +
      'Focus on actual error handling code in route handlers or API controllers.',

    validator: {
      type: 'explanation',
      mentionsPattern: /(error|exception).*(handling|catch|response|return|status)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  // Tier 3: Natural tool selection, both approaches work
  basedOnRealScenario: true,
  frequency: 'monthly',

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /(catch.*error|try.*catch|throw.*Error|error.*response|handle.*error|\.status\(\d{3}\))/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsPattern: /(error|exception).*(handling|catch|response|return|status)/i,
      },
    },
  }),
}
