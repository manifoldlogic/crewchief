/**
 * Task: Check Database Migration Safety
 *
 * Real-world scenario: Reviewing a database migration PR to ensure it won't
 * break existing code that queries the affected tables.
 *
 * Common code review task: Before approving a schema change, verify all code
 * that interacts with the affected table is compatible with the migration.
 *
 * Tool-agnostic: Task describes the safety review without prescribing search method.
 * Both grep and semantic search can complete this task.
 */

import type { SearchTask } from '../../../types.js'
import { createTaskValidator } from '../../../validators.js'

export const TASK_DATABASE_MIGRATION_SAFETY: SearchTask = {
  id: 'tier3-code-review-db-migration',
  name: 'Review DB Migration: Find Table Queries',
  category: 'code-review',
  difficulty: 'medium',
  tier: 'tier3-realworld',

  description:
    'A database migration is being proposed that adds a NOT NULL constraint to the "users" table. ' +
    'For your code review, you need to find all code that queries or inserts into the users table. ' +
    'Identify where user records are created, updated, or queried to verify the migration is safe. ' +
    'Look for SQL queries, ORM operations, and any code that interacts with the users table.',

  realWorldScenario:
    'Based on typical code review scenario: reviewing database migrations for safety. ' +
    'Common pattern: Schema change PR, reviewer must verify all code accessing affected tables remains compatible. ' +
    'Frequency: weekly in teams with active database development.',

  searchTarget: {
    type: 'pattern',
    // Looking for database operations on users table
    pattern:
      /users.*table|SELECT.*FROM users|INSERT.*INTO users|UPDATE users|users\.(find|create|update|delete)|User\.(find|create|save)/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'List all locations where code interacts with the users table (queries, inserts, updates, deletes). ' +
      'For each location, identify: ' +
      '1) The file and function name, ' +
      '2) The type of operation (SELECT, INSERT, UPDATE, DELETE, ORM method), ' +
      '3) Which fields are accessed or modified. ' +
      'Focus on actual database operations, not comments or schema definitions.',

    validator: {
      type: 'explanation',
      mentionsPattern: /(query|insert|update|delete|create|find|save).*user|user.*(table|model|entity)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  // Tier 3: Both tools can succeed, no coercion
  basedOnRealScenario: true,
  frequency: 'weekly',

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern:
        /users.*table|SELECT.*FROM users|INSERT.*INTO users|UPDATE users|users\.(find|create|update|delete)|User\.(find|create|save)/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsPattern: /(query|insert|update|delete|create|find|save).*user|user.*(table|model|entity)/i,
      },
    },
  }),
}
