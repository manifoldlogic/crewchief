/**
 * Task: Find Authentication Checks
 *
 * Find all authentication verification code, where "auth" is ambiguous.
 * This includes: middleware authentication, decorator-based auth, manual token checks,
 * JWT verification, session validation, and authorization guards.
 *
 * Why grep struggles (30-60% success):
 * - "auth" appears everywhere: imports, comments, variable names, actual checks
 * - Must distinguish auth checks from auth setup/configuration
 * - Different patterns: middleware, decorators, manual if statements
 * - False positives: `authenticated` variable, `useAuth` hook (not a check)
 *
 * Why semantic search succeeds (>70% success):
 * - Recognizes auth verification patterns
 * - Distinguishes checks from setup/configuration
 * - Understands guard patterns across implementations
 * - Identifies actual authorization logic vs mentions
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_AUTHENTICATION_CHECKS: SearchTask = {
  id: 'tier2-ambiguity-auth',
  name: 'Find Authentication Checks',
  category: 'ambiguity-resolution',
  difficulty: 'medium',

  description:
    'Find all authentication and authorization checks in the codebase. ' +
    'This includes: middleware that verifies authentication, decorators that enforce auth, ' +
    'manual token or session checks, JWT verification, and permission guards. ' +
    'Identify where auth is enforced and what mechanisms are used.',

  internalNotes:
    'Grep struggles with "auth" ambiguity: ' +
    '- Auth checks: `if (!req.user) throw Unauthorized`, `@RequireAuth` ' +
    '- Auth setup: `import { authenticate } from "lib"` (not a check) ' +
    '- Variables: `const authenticated = true` (state, not check) ' +
    '- Comments: "// TODO: add authentication" (not implementation) ' +
    'Semantic search recognizes actual verification logic.',

  searchTarget: {
    type: 'pattern',
    // Looking for files with auth checks
    pattern:
      /authenticate|authorization|require.*auth|verify.*token|check.*permission|isAuthenticated|guard|protected/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe the authentication and authorization checks found in the codebase. ' +
      'For each mechanism, identify: 1) The auth check approach (middleware, decorator, manual), ' +
      '2) Where it is implemented (files and functions), ' +
      '3) What resources or operations it protects, ' +
      '4) How it verifies authentication (tokens, sessions, etc.). ' +
      'Focus on actual verification code, not setup or configuration.',
    validator: {
      type: 'explanation',
      // Must mention files with auth checks
      mentionsFiles: ['loader.ts'],
      // Must discuss auth check concepts
      mentionsPattern:
        /(auth|authentication|authorization|verify|check|guard|protected).*(?:check|verification|enforcement|guard|middleware)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.4, // 40% - grep finds keyword but many false positives
  expectedSearchSuccess: 0.75, // 75% - search identifies actual checks

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern:
        /authenticate|authorization|require.*auth|verify.*token|check.*permission|isAuthenticated|guard|protected/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['loader.ts'],
        mentionsPattern:
          /(auth|authentication|authorization|verify|check|guard|protected).*(?:check|verification|enforcement|guard|middleware)/i,
      },
    },
  }),
}
