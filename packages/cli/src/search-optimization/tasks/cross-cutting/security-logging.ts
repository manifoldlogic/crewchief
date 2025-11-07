/**
 * Task: Find Security Event Logging
 *
 * Find all logging of security-relevant events.
 * This is scattered across the codebase wherever security decisions are made.
 *
 * Why grep struggles (30-60% success):
 * - Security logging is scattered across many modules
 * - No single keyword: "log", "audit", "security", "warn", "error"
 * - Must distinguish security logging from general logging
 * - Context needed: is this logging a security event?
 *
 * Why semantic search succeeds (>70% success):
 * - Recognizes security-related logging patterns
 * - Understands audit trail concepts
 * - Identifies security events: auth failures, permission checks, sensitive operations
 * - Aggregates scattered security logging
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_SECURITY_LOGGING: SearchTask = {
  id: 'tier2-cross-cutting-security-logging',
  name: 'Find Security Event Logging',
  category: 'cross-cutting-concerns',
  difficulty: 'medium',

  description:
    'Find all logging of security-related events in the codebase. ' +
    'This includes: authentication attempts (success/failure), authorization checks, ' +
    'access to sensitive resources, permission denied events, and security errors. ' +
    'Identify what security events are logged and where.',

  internalNotes:
    'Grep struggles with scattered security logging: ' +
    '- Auth logging: `logger.warn("Authentication failed for user")` ' +
    '- Permission logging: `logger.error("Permission denied")` ' +
    '- Access logging: `logger.info("Accessed sensitive resource")` ' +
    '- Generic logging: `logger.info("Config loaded")` (not security) ' +
    'Semantic search identifies security-relevant logging.',

  searchTarget: {
    type: 'pattern',
    // Looking for files with security logging
    pattern:
      /logger.*(?:auth|permission|access|security|denied|unauthorized|forbidden)|log.*(?:security|audit|auth|access)/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe the security event logging found in the codebase. ' +
      'Identify: 1) What types of security events are logged (auth, permissions, access), ' +
      '2) At least 3 files that log security events, ' +
      '3) The logging level used for security events (info, warn, error), ' +
      '4) Whether there is consistent security logging or if it is ad-hoc. ' +
      'Focus on logging that creates an audit trail for security events.',
    validator: {
      type: 'explanation',
      // Must mention files with logging
      mentionsFiles: ['loader.ts'],
      // Must discuss security logging
      mentionsPattern: /(security|audit|auth|permission|access).*(?:log|logging|logged|event|trail)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.4, // 40% - grep finds "log" but can't identify security relevance
  expectedSearchSuccess: 0.75, // 75% - search recognizes security logging patterns

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern:
        /logger.*(?:auth|permission|access|security|denied|unauthorized|forbidden)|log.*(?:security|audit|auth|access)/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['loader.ts'],
        mentionsPattern: /(security|audit|auth|permission|access).*(?:log|logging|logged|event|trail)/i,
      },
    },
  }),
}
