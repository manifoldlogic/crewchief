/**
 * Search task library - all tasks exported
 */

import type { SearchTask } from '../types.js'
import {
  TASK_DATA_FLOW_WORKTREE_CREATION,
  TASK_INIT_SEQUENCE_ORCHESTRATOR,
  TASK_SYSTEM_INTERACTIONS_MCP_SEARCH,
} from './architectural-understanding/index.js'
import { TASK_UNDERSTAND_COMPETITION, TASK_UNDERSTAND_SDK_INTEGRATION } from './architecture.js'
import { TASK_FIND_CLI_ENTRY, TASK_FIND_SDK_CONFIG } from './config.js'
import { TASK_FIND_CLI_ERROR_HANDLING, TASK_FIND_WORKTREE_ERRORS, TASK_FIND_SDK_ERRORS } from './errors.js'
import { TASK_FIND_WORKTREE_CREATION, TASK_FIND_AGENT_SPAWNING, TASK_FIND_VARIANT_INJECTION } from './implementation.js'
import { TASK_MISSING_ERROR_HANDLING, TASK_UNPROTECTED_FILE_OPERATIONS } from './negative-space/index.js'
import { TASK_FIND_RELATED_TESTS, TASK_FIND_RELATED_TYPES } from './related.js'
import {
  TASK_TRANSITIVE_DEPENDENCIES,
  TASK_CALL_CHAIN_TRACING,
  TASK_API_IMPACT_ANALYSIS,
} from './relationship-discovery/index.js'

/**
 * All available search tasks
 */
export const ALL_TASKS: SearchTask[] = [
  // Type 1: Finding Implementation (3 tasks - easy/medium)
  TASK_FIND_WORKTREE_CREATION,
  TASK_FIND_AGENT_SPAWNING,
  TASK_FIND_VARIANT_INJECTION,

  // Type 2: Understanding Architecture (2 tasks - medium)
  TASK_UNDERSTAND_COMPETITION,
  TASK_UNDERSTAND_SDK_INTEGRATION,

  // Type 3: Locating Errors (3 tasks - medium)
  TASK_FIND_CLI_ERROR_HANDLING,
  TASK_FIND_WORKTREE_ERRORS,
  TASK_FIND_SDK_ERRORS,

  // Type 5: Locating Config (2 tasks - easy)
  TASK_FIND_CLI_ENTRY,
  TASK_FIND_SDK_CONFIG,

  // Type 4: Finding Related (2 tasks - easy/medium)
  TASK_FIND_RELATED_TESTS,
  TASK_FIND_RELATED_TYPES,

  // Type 6: Relationship Discovery (3 tasks - hard/grep-impossible)
  TASK_TRANSITIVE_DEPENDENCIES,
  TASK_CALL_CHAIN_TRACING,
  TASK_API_IMPACT_ANALYSIS,

  // Type 7: Architectural Understanding (3 tasks - hard/grep-impossible)
  TASK_DATA_FLOW_WORKTREE_CREATION,
  TASK_INIT_SEQUENCE_ORCHESTRATOR,
  TASK_SYSTEM_INTERACTIONS_MCP_SEARCH,

  // Type 8: Negative Space (2 tasks - hard/grep-impossible)
  TASK_MISSING_ERROR_HANDLING,
  TASK_UNPROTECTED_FILE_OPERATIONS,
]

/**
 * Get tasks by category
 */
export function getTasksByCategory(category: string): SearchTask[] {
  return ALL_TASKS.filter((task) => task.category === category)
}

/**
 * Get tasks by difficulty
 */
export function getTasksByDifficulty(difficulty: 'easy' | 'medium' | 'hard'): SearchTask[] {
  return ALL_TASKS.filter((task) => task.difficulty === difficulty)
}

/**
 * Get task by ID
 */
export function getTaskById(id: string): SearchTask | undefined {
  return ALL_TASKS.find((task) => task.id === id)
}

/**
 * Get random task
 */
export function getRandomTask(): SearchTask {
  return ALL_TASKS[Math.floor(Math.random() * ALL_TASKS.length)]
}

/**
 * Get random tasks (n tasks)
 */
export function getRandomTasks(count: number): SearchTask[] {
  const shuffled = [...ALL_TASKS].sort(() => Math.random() - 0.5)
  return shuffled.slice(0, Math.min(count, ALL_TASKS.length))
}

// Re-export all individual tasks
export {
  TASK_FIND_WORKTREE_CREATION,
  TASK_FIND_AGENT_SPAWNING,
  TASK_FIND_VARIANT_INJECTION,
  TASK_UNDERSTAND_COMPETITION,
  TASK_UNDERSTAND_SDK_INTEGRATION,
  TASK_FIND_CLI_ERROR_HANDLING,
  TASK_FIND_WORKTREE_ERRORS,
  TASK_FIND_SDK_ERRORS,
  TASK_FIND_CLI_ENTRY,
  TASK_FIND_SDK_CONFIG,
  TASK_FIND_RELATED_TESTS,
  TASK_FIND_RELATED_TYPES,
  TASK_TRANSITIVE_DEPENDENCIES,
  TASK_CALL_CHAIN_TRACING,
  TASK_API_IMPACT_ANALYSIS,
  TASK_DATA_FLOW_WORKTREE_CREATION,
  TASK_INIT_SEQUENCE_ORCHESTRATOR,
  TASK_SYSTEM_INTERACTIONS_MCP_SEARCH,
  TASK_MISSING_ERROR_HANDLING,
  TASK_UNPROTECTED_FILE_OPERATIONS,
}
