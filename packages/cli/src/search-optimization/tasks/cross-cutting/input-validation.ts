/**
 * Task: Find Input Validation
 *
 * Find all input validation code across the codebase.
 * This is scattered across entry points, APIs, and data processing.
 *
 * Why grep struggles (30-60% success):
 * - Validation is scattered across many files
 * - Multiple approaches: Zod schemas, manual checks, type guards, sanitization
 * - No single keyword: "validate", "check", "parse", "sanitize", "guard"
 * - Must distinguish validation from other checks (business logic, conditions)
 *
 * Why semantic search succeeds (>70% success):
 * - Recognizes validation patterns across approaches
 * - Understands data validation concepts: schemas, type guards, sanitization
 * - Identifies input validation vs other conditional logic
 * - Aggregates scattered validation code
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_INPUT_VALIDATION: SearchTask = {
  id: 'tier2-cross-cutting-input-validation',
  name: 'Find Input Validation',
  category: 'cross-cutting-concerns',
  difficulty: 'medium',

  description:
    'Find all input validation code in the codebase. ' +
    'This includes: schema validation (Zod, JSON schema), manual validation checks, ' +
    'type guards, input sanitization, and parameter validation. ' +
    'Identify the different validation approaches and where they are used.',

  internalNotes:
    'Grep struggles with scattered validation patterns: ' +
    '- Schema: `ConfigSchema.safeParse(data)`, `validate(input, schema)` ' +
    '- Manual: `if (!isValid(input)) throw Error`, `if (typeof x !== "string")` ' +
    '- Type guards: `function isUser(obj): obj is User { ... }` ' +
    '- Sanitization: `sanitize(input)`, `input.trim()` ' +
    'Semantic search recognizes these as validation patterns.',

  searchTarget: {
    type: 'pattern',
    // Looking for files with input validation
    pattern:
      /validate|validation|schema.*parse|safeParse|sanitize|type.*guard|is[A-Z][a-z]+.*:\s*.*is\s+|check.*input|verify.*input/,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe the input validation patterns found in the codebase. ' +
      'Identify: 1) The validation approaches used (schemas, manual checks, type guards, sanitization), ' +
      '2) At least 3 files that validate input, ' +
      '3) What inputs are validated (user input, config, API data), ' +
      '4) How validation errors are handled. ' +
      'Focus on code that validates external input before use.',
    validator: {
      type: 'explanation',
      // Must mention files with validation
      mentionsFiles: ['loader.ts', 'schema.ts'],
      // Must discuss validation concepts
      mentionsPattern:
        /(validat|validation|validate|schema|sanitize|type.*guard|input.*check).*(?:pattern|implementation|approach)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.45, // 45% - grep finds some keywords but misses patterns
  expectedSearchSuccess: 0.8, // 80% - search recognizes validation patterns

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern:
        /validate|validation|schema.*parse|safeParse|sanitize|type.*guard|is[A-Z][a-z]+.*:\s*.*is\s+|check.*input|verify.*input/,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['loader.ts', 'schema.ts'],
        mentionsPattern:
          /(validat|validation|validate|schema|sanitize|type.*guard|input.*check).*(?:pattern|implementation|approach)/i,
      },
    },
  }),
}
