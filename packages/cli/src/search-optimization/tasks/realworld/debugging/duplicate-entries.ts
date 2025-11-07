/**
 * Task: Debug Duplicate Entry Creation
 *
 * Real-world scenario: Users report seeing duplicate entries in the database.
 * Need to find all code that creates records without checking for existing entries
 * to identify where duplicates are being created.
 *
 * Common debugging task: Data integrity issues require finding creation logic
 * to understand where duplicate prevention is missing.
 *
 * Tool-agnostic: Task describes the debugging goal without prescribing tools.
 * Both grep and semantic search can find record creation code.
 */

import type { SearchTask } from '../../../types.js'
import { createTaskValidator } from '../../../validators.js'

export const TASK_DUPLICATE_ENTRIES: SearchTask = {
  id: 'tier3-debugging-duplicates',
  name: 'Debug Duplicates: Find Unchecked Record Creation',
  category: 'debugging',
  difficulty: 'medium',
  tier: 'tier3-realworld',

  description:
    'Users are reporting duplicate entries appearing in their records. ' +
    'To debug this issue, you need to find all code that creates new records, ' +
    'particularly places where records are inserted without first checking if they already exist. ' +
    'Identify creation operations, uniqueness checks, and any race condition vulnerabilities.',

  realWorldScenario:
    'Based on typical debugging scenario: data integrity issues in production. ' +
    'Common pattern: Duplicate records appear, engineer traces creation logic to find missing uniqueness checks. ' +
    'Frequency: monthly in systems with concurrent write operations.',

  searchTarget: {
    type: 'pattern',
    // Looking for record creation without checks
    pattern: /(\.create\(|\.insert\(|\.save\(|INSERT INTO|new.*\(.*\)\.save|findOrCreate|upsert|getOrCreate)/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'List all locations where records are created or inserted into the database. ' +
      'For each location, identify: ' +
      '1) The file and function name, ' +
      '2) What type of record is being created, ' +
      '3) Whether there is a check for existing records before creation (findOrCreate, unique constraint, etc.), ' +
      '4) Any potential race conditions. ' +
      'Focus on actual record creation code, prioritize places without existence checks.',

    validator: {
      type: 'explanation',
      mentionsPattern: /(create|insert|save).*(record|entry|row)|duplicate.*check|unique.*constraint|findOrCreate/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  // Tier 3: Real debugging scenario, both approaches work
  basedOnRealScenario: true,
  frequency: 'monthly',

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /(\.create\(|\.insert\(|\.save\(|INSERT INTO|new.*\(.*\)\.save|findOrCreate|upsert|getOrCreate)/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsPattern: /(create|insert|save).*(record|entry|row)|duplicate.*check|unique.*constraint|findOrCreate/i,
      },
    },
  }),
}
