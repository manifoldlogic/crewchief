/**
 * Tier 3: Real-World Benchmark Suite
 *
 * Aggregates all 9 real-world tasks from TESTDES-4002.
 * These tasks test natural tool selection in actual development scenarios:
 * - Code review (3 tasks)
 * - Debugging (3 tasks)
 * - Refactoring (3 tasks)
 *
 * This suite provides:
 * - Unified task collection for real-world scenarios
 * - Category-based organization
 * - Voluntary tool adoption tracking
 * - Completion measurement regardless of tool choice
 *
 * Key difference from Tier 1 & 2:
 * - NO expectedGrepSuccess/expectedSearchSuccess rates
 * - Both tools CAN complete these tasks
 * - Measures natural tool selection, not forced capability tests
 * - Validates through actual developer scenarios
 * - Success = task completion, not specific tool usage
 */

import {
  TASK_API_IMPACT_ANALYSIS,
  TASK_AUTH_PERMISSION_CHECK,
  TASK_CACHE_INVALIDATION,
  TASK_DATABASE_MIGRATION_SAFETY,
  TASK_DEPRECATE_FUNCTION,
  TASK_DUPLICATE_ENTRIES,
  TASK_ERROR_HANDLING_CONSISTENCY,
  TASK_EXTRACT_PATTERN,
  TASK_INTERMITTENT_TIMEOUT,
} from '../tasks/realworld/index.js'
import type { SearchTask } from '../types.js'

/**
 * Metadata about the benchmark suite
 */
export interface SuiteMetadata {
  /** Total number of tasks in the suite */
  totalTasks: number

  /** Task categories represented */
  categories: string[]

  /** Suite description */
  description: string

  /** Task frequency distribution */
  frequencyDistribution: Record<string, number>

  /** Scenario types covered */
  scenarioTypes: string[]
}

/**
 * A collection of related search tasks
 */
export interface BenchmarkSuite {
  /** Suite name */
  name: string

  /** Suite version for tracking changes */
  version: string

  /** Tier level (3 = real-world) */
  tier: number

  /** All tasks in this suite */
  tasks: SearchTask[]

  /** Suite metadata */
  metadata: SuiteMetadata
}

/**
 * Extract unique categories from tasks
 */
function extractCategories(tasks: SearchTask[]): string[] {
  const categories = new Set(tasks.map((task) => task.category))
  return Array.from(categories).sort()
}

/**
 * Extract frequency distribution from tasks
 */
function extractFrequencyDistribution(tasks: SearchTask[]): Record<string, number> {
  const distribution: Record<string, number> = {}

  for (const task of tasks) {
    const freq = (task as SearchTask & Record<string, unknown>).frequency as string | undefined
    if (freq) {
      distribution[freq] = (distribution[freq] ?? 0) + 1
    }
  }

  return distribution
}

/**
 * Extract scenario types from tasks
 */
function extractScenarioTypes(tasks: SearchTask[]): string[] {
  const types = new Set(tasks.map((task) => task.category))
  return Array.from(types).sort()
}

/**
 * Tier 3: Real-World Benchmark Suite
 *
 * Contains 9 tasks representing actual developer scenarios:
 * - 3 code review tasks (auth checks, migration safety, error consistency)
 * - 3 debugging tasks (timeouts, duplicates, cache issues)
 * - 3 refactoring tasks (deprecation, pattern extraction, impact analysis)
 *
 * Key characteristics:
 * - Tool-agnostic task descriptions
 * - Based on real development scenarios
 * - Objective success criteria
 * - Both grep and search can complete tasks
 * - Measures voluntary tool adoption
 */
export const TIER3_REALWORLD_SUITE: BenchmarkSuite = (() => {
  const tasks = [
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

  return {
    name: 'Tier 3: Real-World Tasks',
    version: '1.0.0',
    tier: 3,
    tasks,

    metadata: {
      totalTasks: tasks.length,
      categories: extractCategories(tasks),
      frequencyDistribution: extractFrequencyDistribution(tasks),
      scenarioTypes: extractScenarioTypes(tasks),
      description:
        'Tasks based on actual developer scenarios where both grep and semantic search can succeed. ' +
        'Measures voluntary tool adoption in real-world contexts: code review, debugging, and refactoring. ' +
        'Success is measured by task completion, not by which tool was chosen. ' +
        'These tasks prove utility through natural adoption, not forced capability tests.',
    },
  }
})()

/**
 * Statistics for a category
 */
export interface CategoryStatistics {
  /** Category name */
  category: string

  /** Number of tasks in this category */
  taskCount: number

  /** Task IDs in this category */
  taskIds: string[]

  /** Difficulty distribution */
  difficultyDistribution: Record<string, number>

  /** Frequency distribution */
  frequencyDistribution: Record<string, number>
}

/**
 * Statistics for a difficulty level
 */
export interface DifficultyStatistics {
  /** Difficulty level */
  difficulty: 'easy' | 'medium' | 'hard'

  /** Number of tasks at this difficulty */
  taskCount: number

  /** Task IDs at this difficulty */
  taskIds: string[]

  /** Categories represented */
  categories: string[]
}

/**
 * Overall suite statistics
 */
export interface SuiteStatistics {
  /** Total tasks */
  totalTasks: number

  /** Statistics by category */
  byCategory: Map<string, CategoryStatistics>

  /** Statistics by difficulty */
  byDifficulty: Map<string, DifficultyStatistics>

  /** Overall frequency distribution */
  frequencyDistribution: Record<string, number>

  /** Scenario type coverage */
  scenarioTypes: string[]
}

/**
 * Organize tasks by category
 *
 * @param suite - The benchmark suite
 * @returns Map of category name to tasks in that category
 */
export function getTasksByCategory(suite: BenchmarkSuite): Map<string, SearchTask[]> {
  const byCategory = new Map<string, SearchTask[]>()

  for (const task of suite.tasks) {
    const category = task.category
    if (!byCategory.has(category)) {
      byCategory.set(category, [])
    }
    byCategory.get(category)!.push(task)
  }

  return byCategory
}

/**
 * Organize tasks by difficulty
 *
 * @param suite - The benchmark suite
 * @returns Map of difficulty level to tasks at that difficulty
 */
export function getTasksByDifficulty(suite: BenchmarkSuite): Map<string, SearchTask[]> {
  const byDifficulty = new Map<string, SearchTask[]>()

  for (const task of suite.tasks) {
    const difficulty = task.difficulty
    if (!byDifficulty.has(difficulty)) {
      byDifficulty.set(difficulty, [])
    }
    byDifficulty.get(difficulty)!.push(task)
  }

  return byDifficulty
}

/**
 * Calculate comprehensive statistics for the suite
 *
 * @param suite - The benchmark suite
 * @returns Detailed statistics broken down by category and difficulty
 */
export function getSuiteStatistics(suite: BenchmarkSuite): SuiteStatistics {
  const byCategory = new Map<string, CategoryStatistics>()
  const byDifficulty = new Map<string, DifficultyStatistics>()

  // Calculate category statistics
  const tasksByCategory = getTasksByCategory(suite)
  for (const [category, tasks] of tasksByCategory) {
    // Difficulty distribution for this category
    const difficultyDist: Record<string, number> = {}
    for (const task of tasks) {
      difficultyDist[task.difficulty] = (difficultyDist[task.difficulty] ?? 0) + 1
    }

    // Frequency distribution for this category
    const freqDist: Record<string, number> = {}
    for (const task of tasks) {
      const freq = (task as SearchTask & Record<string, unknown>).frequency as string | undefined
      if (freq) {
        freqDist[freq] = (freqDist[freq] ?? 0) + 1
      }
    }

    byCategory.set(category, {
      category,
      taskCount: tasks.length,
      taskIds: tasks.map((t) => t.id),
      difficultyDistribution: difficultyDist,
      frequencyDistribution: freqDist,
    })
  }

  // Calculate difficulty statistics
  const tasksByDifficulty = getTasksByDifficulty(suite)
  for (const [difficulty, tasks] of tasksByDifficulty) {
    const categories = new Set(tasks.map((t) => t.category))

    byDifficulty.set(difficulty, {
      difficulty: difficulty as 'easy' | 'medium' | 'hard',
      taskCount: tasks.length,
      taskIds: tasks.map((t) => t.id),
      categories: Array.from(categories).sort(),
    })
  }

  return {
    totalTasks: suite.tasks.length,
    byCategory,
    byDifficulty,
    frequencyDistribution: suite.metadata.frequencyDistribution,
    scenarioTypes: suite.metadata.scenarioTypes,
  }
}

/**
 * Organize tasks by frequency
 *
 * @param suite - The benchmark suite
 * @returns Map of frequency to tasks at that frequency
 */
export function getTasksByFrequency(suite: BenchmarkSuite): Map<string, SearchTask[]> {
  const byFrequency = new Map<string, SearchTask[]>()

  for (const task of suite.tasks) {
    const frequency = (task as SearchTask & Record<string, unknown>).frequency as string | undefined
    if (frequency) {
      if (!byFrequency.has(frequency)) {
        byFrequency.set(frequency, [])
      }
      byFrequency.get(frequency)!.push(task)
    }
  }

  return byFrequency
}
