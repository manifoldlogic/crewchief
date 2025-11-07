/**
 * Task: Find Transaction Management
 *
 * Find all transaction handling code, where "transaction" is ambiguous.
 * This includes: database transactions (BEGIN/COMMIT/ROLLBACK), ORM transactions,
 * manual transaction management, and transactional middleware.
 *
 * Why grep struggles (30-60% success):
 * - "transaction" appears in many contexts: financial, database, business logic
 * - Must distinguish actual transaction code from comments/documentation
 * - Different implementations: SQL, ORM, custom
 * - Pattern recognition needed: begin/commit/rollback sequences
 *
 * Why semantic search succeeds (>70% success):
 * - Understands "transaction" in database context
 * - Recognizes transactional patterns (atomicity, rollback)
 * - Distinguishes implementation code from discussion
 * - Connects related concepts: BEGIN, COMMIT, ROLLBACK, atomic
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_TRANSACTION_MANAGEMENT: SearchTask = {
  id: 'tier2-ambiguity-transaction',
  name: 'Find Transaction Management',
  category: 'ambiguity-resolution',
  difficulty: 'medium',

  description:
    'Find all database transaction handling code in the codebase. ' +
    'This includes: transaction start/begin, commit operations, rollback logic, ' +
    'transaction middleware, and atomic operations. ' +
    'Identify where transactions are used and how they ensure data consistency.',

  internalNotes:
    'Grep struggles with "transaction" ambiguity: ' +
    '- Database transactions: `BEGIN TRANSACTION`, `COMMIT`, `ROLLBACK` ' +
    '- ORM transactions: `db.transaction(async (tx) => { ... })` ' +
    '- Mentions in comments: "// This needs a transaction" (not implementation) ' +
    '- Business logic: "process transaction" (financial, not database) ' +
    'Semantic search understands the database transaction context.',

  searchTarget: {
    type: 'pattern',
    // Looking for files with transaction handling
    pattern: /transaction|BEGIN.*TRANSACTION|COMMIT|ROLLBACK|tx\.|atomic|transactional/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe the transaction management patterns found in the codebase. ' +
      'For each pattern, identify: 1) The transaction mechanism (raw SQL, ORM, custom), ' +
      '2) Where it is implemented (files and functions), ' +
      '3) What operations are protected by transactions, ' +
      '4) How rollback is handled on errors. ' +
      'Focus on actual transaction implementation code.',
    validator: {
      type: 'explanation',
      // Must mention files with transactions (based on real codebase)
      mentionsFiles: ['merge.ts'],
      // Must discuss transaction concepts
      mentionsPattern:
        /(transaction|transactions|transactional|commit|rollback|atomic).*(?:management|handling|pattern|implementation)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.45, // 45% - grep finds keyword but gets false positives
  expectedSearchSuccess: 0.8, // 80% - search disambiguates context

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /transaction|BEGIN.*TRANSACTION|COMMIT|ROLLBACK|tx\.|atomic|transactional/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['merge.ts'],
        mentionsPattern:
          /(transaction|transactions|transactional|commit|rollback|atomic).*(?:management|handling|pattern|implementation)/i,
      },
    },
  }),
}
