/**
 * Task: API Breaking Change - Impact Analysis
 *
 * Real-world scenario: Need to change a public API signature and understand
 * what code will break. Must find all consumers and assess migration effort.
 *
 * Common refactoring task: Before making breaking changes, developers perform
 * impact analysis to understand the scope of changes needed.
 *
 * Tool-agnostic: Task describes the impact analysis without prescribing tools.
 * Both grep and semantic search can find API consumers.
 */

import type { SearchTask } from '../../../types.js'
import { createTaskValidator } from '../../../validators.js'

export const TASK_API_IMPACT_ANALYSIS: SearchTask = {
  id: 'tier3-refactoring-api-impact',
  name: 'API Change: Analyze Impact on Consumers',
  category: 'refactoring',
  difficulty: 'medium',
  tier: 'tier3-realworld',

  description:
    'You need to change the `sendNotification()` API to require an additional priority parameter. ' +
    'Before making this breaking change, you must understand what code will break. ' +
    'Find all places that call `sendNotification()`, analyze how they use the API, ' +
    'and assess the migration effort required for each caller. This impact analysis will ' +
    'help plan the migration and communicate with affected teams.',

  realWorldScenario:
    'Based on typical refactoring scenario: assessing impact of API breaking changes. ' +
    'Common pattern: API signature changes, need to find all callers and plan migration. ' +
    'Frequency: weekly in teams with evolving internal APIs.',

  searchTarget: {
    type: 'pattern',
    // Looking for function calls
    pattern: /sendNotification\s*\(/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Analyze the impact of changing `sendNotification()` to require a priority parameter. ' +
      'For each location where it is called, identify: ' +
      '1) The file and calling function, ' +
      '2) Current usage pattern (what arguments are passed), ' +
      '3) What priority value should likely be used in that context, ' +
      '4) Migration complexity (simple parameter addition vs. logic changes needed). ' +
      'Provide an impact summary: how many call sites, overall migration effort estimate.',

    validator: {
      type: 'explanation',
      mentionsPattern: /(sendNotification|impact|migration|breaking.*change|caller|consumer)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  // Tier 3: Real refactoring scenario, both approaches work
  basedOnRealScenario: true,
  frequency: 'weekly',

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /sendNotification\s*\(/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsPattern: /(sendNotification|impact|migration|breaking.*change|caller|consumer)/i,
      },
    },
  }),
}
