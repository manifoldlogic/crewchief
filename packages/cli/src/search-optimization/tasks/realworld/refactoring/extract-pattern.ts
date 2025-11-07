/**
 * Task: Extract Common Pattern into Shared Module
 *
 * Real-world scenario: Noticed the same validation pattern duplicated across
 * multiple files. Need to find all similar implementations to extract them
 * into a shared module.
 *
 * Common refactoring task: When identifying code duplication, developers find
 * similar implementations to extract common patterns and reduce duplication.
 *
 * Tool-agnostic: Task describes the refactoring goal without prescribing tools.
 * Both grep and semantic search can find similar patterns.
 */

import type { SearchTask } from '../../../types.js'
import { createTaskValidator } from '../../../validators.js'

export const TASK_EXTRACT_PATTERN: SearchTask = {
  id: 'tier3-refactoring-extract',
  name: 'Extract Pattern: Find Similar Implementations',
  category: 'refactoring',
  difficulty: 'medium',
  tier: 'tier3-realworld',

  description:
    'You noticed that email validation logic is duplicated across several form handlers. ' +
    'To reduce duplication, you want to extract this pattern into a shared validation module. ' +
    'Find all places where email validation is implemented, understand the validation rules used in each, ' +
    'and identify which implementations are similar enough to be unified into a single shared function.',

  realWorldScenario:
    'Based on typical refactoring scenario: extracting duplicated patterns into shared modules. ' +
    'Common pattern: Notice code duplication, find all instances, create shared abstraction. ' +
    'Frequency: monthly in codebases undergoing active refactoring.',

  searchTarget: {
    type: 'pattern',
    // Looking for email validation patterns
    pattern: /(email.*valid|valid.*email|@.*\.|test.*email|email.*regex|email.*pattern|emailRegex|validateEmail)/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'List all locations where email validation is implemented. ' +
      'For each implementation, identify: ' +
      '1) The file and function name, ' +
      '2) The validation rules or regex pattern used, ' +
      '3) Any additional validation logic (length checks, domain restrictions), ' +
      '4) How similar the implementation is to others (potential for unification). ' +
      'Focus on actual validation logic, identify patterns suitable for extraction.',

    validator: {
      type: 'explanation',
      mentionsPattern: /(email.*validat|validat.*email|validation.*pattern|duplicate.*code|extract|unify)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  // Tier 3: Real refactoring scenario, both tools can succeed
  basedOnRealScenario: true,
  frequency: 'monthly',

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /(email.*valid|valid.*email|@.*\.|test.*email|email.*regex|email.*pattern|emailRegex|validateEmail)/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsPattern: /(email.*validat|validat.*email|validation.*pattern|duplicate.*code|extract|unify)/i,
      },
    },
  }),
}
