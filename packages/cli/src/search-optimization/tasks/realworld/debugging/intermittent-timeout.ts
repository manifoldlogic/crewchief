/**
 * Task: Debug Intermittent Request Timeout
 *
 * Real-world scenario: Users report API requests failing intermittently with timeouts.
 * Need to find all code that handles timeouts, retries, or request cancellation
 * in the API client to diagnose the issue.
 *
 * Common debugging task: Intermittent failures require tracing timeout handling
 * and retry logic to understand why requests sometimes fail.
 *
 * Tool-agnostic: Task describes the debugging need without prescribing search method.
 * Both grep and semantic search can find timeout-related code.
 */

import type { SearchTask } from '../../../types.js'
import { createTaskValidator } from '../../../validators.js'

export const TASK_INTERMITTENT_TIMEOUT: SearchTask = {
  id: 'tier3-debugging-timeout',
  name: 'Debug Intermittent Timeout: Find Timeout Handling',
  category: 'debugging',
  difficulty: 'medium',
  tier: 'tier3-realworld',

  description:
    'Users are reporting that API requests fail intermittently with timeout errors. ' +
    'To debug this issue, you need to find all code in the API client that handles timeouts, ' +
    'sets timeout values, implements retry logic, or manages request cancellation. ' +
    'Identify where timeouts are configured, how retries work, and what happens when requests time out.',

  realWorldScenario:
    'Based on typical debugging scenario: intermittent failures in production. ' +
    'Common pattern: Users report timeout errors, engineer traces timeout/retry logic to find root cause. ' +
    'Frequency: weekly in production systems with external API dependencies.',

  searchTarget: {
    type: 'pattern',
    // Looking for timeout and retry handling
    pattern:
      /(timeout|setTimeout|setRequestTimeout|retry|retries|backoff|cancel.*request|abort.*request|requestTimeout)/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'List all locations where the API client handles timeouts or retries. ' +
      'For each location, identify: ' +
      '1) The file and function name, ' +
      '2) The timeout duration or retry configuration, ' +
      '3) What happens when a timeout occurs (retry, error, cancel). ' +
      'Focus on actual timeout handling and retry logic, not test code.',

    validator: {
      type: 'explanation',
      mentionsPattern: /(timeout|retry).*(handling|configuration|logic|strategy)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  // Tier 3: Real debugging scenario, both tools work
  basedOnRealScenario: true,
  frequency: 'weekly',

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern:
        /(timeout|setTimeout|setRequestTimeout|retry|retries|backoff|cancel.*request|abort.*request|requestTimeout)/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsPattern: /(timeout|retry).*(handling|configuration|logic|strategy)/i,
      },
    },
  }),
}
