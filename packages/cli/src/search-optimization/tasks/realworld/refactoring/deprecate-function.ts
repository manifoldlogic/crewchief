/**
 * Task: Deprecate Utility Function - Find All Usages
 *
 * Real-world scenario: Need to deprecate an old utility function and migrate
 * callers to the new API. Must find all usages and understand their contexts
 * to plan the migration.
 *
 * Common refactoring task: When deprecating APIs, developers need to find
 * all usages to understand migration impact and communicate changes.
 *
 * Tool-agnostic: Task describes the refactoring need without prescribing tools.
 * Both grep and semantic search can find function usages.
 */

import type { SearchTask } from '../../../types.js'
import { createTaskValidator } from '../../../validators.js'

export const TASK_DEPRECATE_FUNCTION: SearchTask = {
  id: 'tier3-refactoring-deprecate',
  name: 'Deprecate Function: Find All Usages and Contexts',
  category: 'refactoring',
  difficulty: 'easy',
  tier: 'tier3-realworld',

  description:
    'You need to deprecate the utility function `formatDate()` and migrate callers to the new `formatDateTime()` API. ' +
    'Find all places where `formatDate()` is called, understand how it is being used in each context, ' +
    'and identify any patterns in how parameters are passed. This will help plan the migration and ' +
    'communicate the deprecation to other teams.',

  realWorldScenario:
    'Based on typical refactoring scenario: deprecating old APIs and migrating to new ones. ' +
    'Common pattern: New utility replaces old one, need to find all callers and migration paths. ' +
    'Frequency: monthly in actively maintained codebases.',

  searchTarget: {
    type: 'pattern',
    // Looking for function calls
    pattern: /formatDate\s*\(/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'List all locations where `formatDate()` is called. ' +
      'For each usage, identify: ' +
      '1) The file and function where it is called, ' +
      '2) How the function is being used (what arguments are passed), ' +
      '3) The context or purpose of the formatting in that location, ' +
      '4) Any patterns in usage that would affect migration strategy. ' +
      'Focus on actual function calls, not definitions or imports.',

    validator: {
      type: 'explanation',
      mentionsPattern: /(formatDate|usage|call|migration|deprecat)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  // Tier 3: Real refactoring scenario, both tools work
  basedOnRealScenario: true,
  frequency: 'monthly',

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /formatDate\s*\(/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsPattern: /(formatDate|usage|call|migration|deprecat)/i,
      },
    },
  }),
}
