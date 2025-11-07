/**
 * Task: Debug Stale Cache Issue
 *
 * Real-world scenario: Users are seeing outdated data, suggesting the cache
 * is not being invalidated properly. Need to trace cache invalidation flow
 * through the system to find where invalidation is missing.
 *
 * Common debugging task: Cache consistency issues require understanding the
 * complete cache invalidation flow to identify gaps.
 *
 * Tool-agnostic: Task describes the debugging need without prescribing tools.
 * Both grep and semantic search can trace cache operations.
 */

import type { SearchTask } from '../../../types.js'
import { createTaskValidator } from '../../../validators.js'

export const TASK_CACHE_INVALIDATION: SearchTask = {
  id: 'tier3-debugging-cache',
  name: 'Debug Stale Cache: Trace Invalidation Flow',
  category: 'debugging',
  difficulty: 'medium',
  tier: 'tier3-realworld',

  description:
    'Users are seeing stale data after updates, indicating the cache is not being invalidated correctly. ' +
    'To debug this issue, you need to trace how cache invalidation flows through the system. ' +
    'Find all code that invalidates cache, clears cache entries, or triggers cache updates. ' +
    'Identify where data is cached, when invalidation should happen, and any missing invalidation calls.',

  realWorldScenario:
    'Based on typical debugging scenario: cache consistency issues in production. ' +
    'Common pattern: Stale data appears after updates, engineer traces cache invalidation to find missing calls. ' +
    'Frequency: weekly in systems with complex caching layers.',

  searchTarget: {
    type: 'pattern',
    // Looking for cache invalidation operations
    pattern:
      /(cache\.delete|cache\.clear|cache\.invalidate|clearCache|invalidateCache|cache\.remove|evict.*cache|flush.*cache)/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe how cache invalidation flows through the system. ' +
      'For each location where cache is invalidated, identify: ' +
      '1) The file and function name, ' +
      '2) What triggers the invalidation (data update, time expiry, manual clear), ' +
      '3) Which cache keys or regions are invalidated, ' +
      '4) Any data update operations that might be missing corresponding invalidation. ' +
      'Focus on actual cache invalidation code and the flow of invalidation events.',

    validator: {
      type: 'explanation',
      mentionsPattern: /(cache.*invalidat|invalidat.*cache|clear.*cache|cache.*flow|cache.*consistency)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  // Tier 3: Real debugging scenario, both tools can succeed
  basedOnRealScenario: true,
  frequency: 'weekly',

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern:
        /(cache\.delete|cache\.clear|cache\.invalidate|clearCache|invalidateCache|cache\.remove|evict.*cache|flush.*cache)/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsPattern: /(cache.*invalidat|invalidat.*cache|clear.*cache|cache.*flow|cache.*consistency)/i,
      },
    },
  }),
}
