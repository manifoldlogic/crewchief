/**
 * Task: Unprotected File System Operations
 *
 * Find file system operations that lack proper validation or safety checks.
 * This is a "negative space" task - finding operations WITHOUT proper guards.
 *
 * Why grep fails:
 * - Grep can find fs.writeFile, fs.unlink, fs.mkdir patterns
 * - Grep can find individual validation patterns (if existsSync, try-catch, validators)
 * - But grep CANNOT determine which operations LACK validation
 * - Multiple protection methods exist: existence checks, path validation, try-catch, helper functions
 * - Cannot enumerate all possible protection patterns
 * - Manual cross-referencing required for every file operation found
 *
 * Why semantic search succeeds:
 * - Understands concept of "file operation without validation"
 * - Reasons about code safety and risk
 * - Identifies operations by their protection level, not just presence
 * - Semantic understanding of what constitutes "unsafe file operation"
 *
 * Real violations in CrewChief codebase:
 * 1. packages/cli/src/utils/worktree-metadata.ts:17 - fs.writeFile() without path validation
 * 2. packages/cli/src/cli/setup.ts:69 - fs.writeFileSync() without existence check
 * 3. packages/cli/src/cli/setup.ts:79 - fs.writeFileSync() without directory validation
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_UNPROTECTED_FILE_OPERATIONS: SearchTask = {
  id: 'negative-space-unprotected-operations',
  name: 'Find File Operations Without Validation',
  description:
    'Find file system operations (write, delete, mkdir) that lack proper safety checks. ' +
    'Look for operations that could fail or cause security issues without validation: ' +
    'path validation, existence checks, permission verification, or error handling. ' +
    'Focus on production code that writes or deletes files without guards.',

  category: 'negative-space',
  difficulty: 'hard',

  searchTarget: {
    type: 'pattern',
    // Looking for unprotected file operations
    // Pattern matches file operation identifiers in violation files
    pattern: /worktree-metadata|setup|fs\.writeFile|fs\.writeFileSync|fs\.mkdir|fs\.unlink|file.*operation/,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'List specific file operations that lack proper validation or safety checks. ' +
      'For each: 1) What file contains it? 2) What line number? 3) What operation (writeFile/unlink/mkdir)? ' +
      '4) What validation is missing (path check, existence check, permission check)? ' +
      'Focus on production code, not test utilities.',
    validator: {
      type: 'explanation',
      // Must mention at least one of the known violation files
      mentionsFiles: ['worktree-metadata.ts', 'setup.ts'],
      // Must discuss validation, safety, or checks
      mentionsPattern:
        /(without|missing|lack|no).*(validation|check|verify|guard|safe)|unprotected|unsafe|unvalidated/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  internalNotes:
    'Demonstrates semantic search ability to identify risky code patterns by reasoning about ' +
    'what protections are ABSENT. Grep cannot solve this because it would require: 1) Find all file operations, ' +
    '2) Find all validation patterns (existsSync, access, stat, try-catch, helper functions, ' +
    'path validators), 3) Manually verify each operation has appropriate protection. ' +
    'This is exponentially complex because protection patterns are varied and context-dependent. ' +
    'A file operation might be protected by: checks in the same function, wrapper utilities, ' +
    'caller validation, or framework guarantees. Grep cannot enumerate all protection methods. ' +
    'Semantic search can directly query for "file operations that could fail without proper guards".',

  expectedGrepSuccess: 0.25, // 25% - grep can find operations but not protection absence
  expectedSearchSuccess: 0.75, // 75% - semantic search reasons about safety

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /worktree-metadata|setup|fs\.writeFile|fs\.writeFileSync|fs\.mkdir|fs\.unlink|file.*operation/,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['worktree-metadata.ts', 'setup.ts'],
        mentionsPattern:
          /(without|missing|lack|no).*(validation|check|verify|guard|safe)|unprotected|unsafe|unvalidated/i,
      },
    },
  }),
}
