/**
 * Real-world Search Tasks - Tier 3
 *
 * These 9 tasks represent actual developer scenarios across 3 categories:
 * - Code Review (3): Security checks, migration safety, consistency verification
 * - Debugging (3): Timeout issues, duplicate data, cache problems
 * - Refactoring (3): API deprecation, pattern extraction, impact analysis
 *
 * Tier 3 characteristics:
 * - Tool-agnostic descriptions (no hints to use search vs grep)
 * - Based on real development scenarios
 * - Objective success criteria
 * - Both grep and semantic search CAN complete these tasks
 * - Measures voluntary tool adoption, not coerced capability testing
 *
 * These tasks prove utility through natural adoption, not forced capability tests.
 */

import type { SearchTask } from '../../types.js'
import {
  TASK_AUTH_PERMISSION_CHECK,
  TASK_DATABASE_MIGRATION_SAFETY,
  TASK_ERROR_HANDLING_CONSISTENCY,
} from './code-review/index.js'
import { TASK_CACHE_INVALIDATION, TASK_DUPLICATE_ENTRIES, TASK_INTERMITTENT_TIMEOUT } from './debugging/index.js'
import { TASK_API_IMPACT_ANALYSIS, TASK_DEPRECATE_FUNCTION, TASK_EXTRACT_PATTERN } from './refactoring/index.js'

/**
 * All Tier 3 real-world tasks (9 tasks)
 */
export const TIER3_REALWORLD_TASKS: SearchTask[] = [
  // Code Review (3 tasks)
  TASK_AUTH_PERMISSION_CHECK,
  TASK_DATABASE_MIGRATION_SAFETY,
  TASK_ERROR_HANDLING_CONSISTENCY,

  // Debugging (3 tasks)
  TASK_INTERMITTENT_TIMEOUT,
  TASK_DUPLICATE_ENTRIES,
  TASK_CACHE_INVALIDATION,

  // Refactoring (3 tasks)
  TASK_DEPRECATE_FUNCTION,
  TASK_EXTRACT_PATTERN,
  TASK_API_IMPACT_ANALYSIS,
]

/**
 * Get tasks by category
 */
export function getRealWorldTasksByCategory(category: 'code-review' | 'debugging' | 'refactoring'): SearchTask[] {
  return TIER3_REALWORLD_TASKS.filter((task) => task.category === category)
}

/**
 * Get tasks by difficulty
 */
export function getRealWorldTasksByDifficulty(difficulty: 'easy' | 'medium'): SearchTask[] {
  return TIER3_REALWORLD_TASKS.filter((task) => task.difficulty === difficulty)
}

/**
 * Get task by ID
 */
export function getRealWorldTaskById(id: string): SearchTask | undefined {
  return TIER3_REALWORLD_TASKS.find((task) => task.id === id)
}

// Re-export category modules (exports all tasks)
export * from './code-review/index.js'
export * from './debugging/index.js'
export * from './refactoring/index.js'
