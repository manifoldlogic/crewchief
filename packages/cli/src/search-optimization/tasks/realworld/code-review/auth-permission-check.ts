/**
 * Task: Review Authentication Change - Find Permission Checks
 *
 * Real-world scenario: Code review where a developer changed authentication
 * logic and you need to verify all permission checks are still secure.
 *
 * This is a common code review task where developers need to trace security-critical
 * changes through the codebase. Both grep and semantic search can find these,
 * but semantic search may be faster at understanding the security context.
 *
 * Tool-agnostic: Task describes what needs to be found without prescribing tools.
 * Both approaches work, measuring voluntary tool adoption.
 */

import type { SearchTask } from '../../../types.js'
import { createTaskValidator } from '../../../validators.js'

export const TASK_AUTH_PERMISSION_CHECK: SearchTask = {
  id: 'tier3-code-review-auth-permissions',
  name: 'Review Auth Change: Find Permission Checks',
  category: 'code-review',
  difficulty: 'medium',
  tier: 'tier3-realworld',

  description:
    'A developer just modified the authentication middleware. For your code review, ' +
    'you need to find all places in the codebase that check user permissions or roles. ' +
    'Identify where permission checks happen, what roles are verified, and how authentication ' +
    'state is accessed. This is important to ensure the auth changes did not break security.',

  realWorldScenario:
    'Based on typical code review scenario: reviewing authentication/authorization changes. ' +
    'Common pattern: PR changes auth middleware, reviewer needs to verify all permission checks remain secure. ' +
    'Frequency: weekly in teams with active security development.',

  searchTarget: {
    type: 'pattern',
    // Looking for permission/authorization/role checks
    pattern: /(permissions?|authorize?d?|role|hasRole|canAccess|checkPermission|isAdmin|isAuthorized)/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'List all locations where the code checks user permissions, roles, or authorization. ' +
      'For each location, identify: ' +
      '1) The file and function/method name, ' +
      '2) What permission or role is being checked, ' +
      '3) How the authentication state is accessed. ' +
      'Focus on actual permission checks, not comments or documentation.',

    validator: {
      type: 'explanation',
      // Should find actual permission-related code
      mentionsPattern: /(permission|authorize|role|access).*check|verify|validation/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  // Tier 3 characteristics: Both tools can succeed
  // No expectedGrepSuccess/expectedSearchSuccess - we measure voluntary adoption
  basedOnRealScenario: true,
  frequency: 'weekly',

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /(permissions?|authorize?d?|role|hasRole|canAccess|checkPermission|isAdmin|isAuthorized)/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsPattern: /(permission|authorize|role|access).*check|verify|validation/i,
      },
    },
  }),
}
