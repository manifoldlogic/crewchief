/**
 * Tier 2: Grep-Hard Benchmark Suite
 *
 * Aggregates all 11 grep-hard tasks from TESTDES-4001.
 * These tasks challenge grep by requiring:
 * - Understanding conceptual similarity across different terminologies
 * - Disambiguating keywords that have multiple meanings
 * - Aggregating scattered patterns across the codebase
 *
 * This suite provides:
 * - Unified task collection
 * - Category-based organization
 * - Statistical analysis
 * - Validation of suite composition
 *
 * Expected performance characteristics:
 * - Grep: 30-60% success (grep-hard range)
 * - Search: >70% success
 * - Advantage: >30% improvement (search - grep)
 */

import {
  TASK_AUTHENTICATION_CHECKS,
  TASK_CACHE_OPERATIONS,
  TASK_RESOURCE_CLEANUP,
  TASK_TRANSACTION_MANAGEMENT,
} from '../tasks/ambiguity-resolution/index.js'
import {
  TASK_CACHING_STRATEGIES,
  TASK_ERROR_HANDLING_PATTERNS,
  TASK_RATE_LIMITING,
  TASK_RETRY_IMPLEMENTATIONS,
} from '../tasks/conceptual-similarity/index.js'
import {
  TASK_ASYNC_ERROR_HANDLING,
  TASK_INPUT_VALIDATION,
  TASK_SECURITY_LOGGING,
} from '../tasks/cross-cutting/index.js'
import type { SearchTask } from '../types.js'

/**
 * Metadata about the benchmark suite
 */
export interface SuiteMetadata {
  /** Total number of tasks in the suite */
  totalTasks: number

  /** Task categories represented */
  categories: string[]

  /** Average expected grep success rate across all tasks */
  expectedGrepSuccessRate: number

  /** Average expected search success rate across all tasks */
  expectedSearchSuccessRate: number

  /** Expected improvement (search - grep) */
  expectedImprovement: number

  /** Suite description */
  description: string
}

/**
 * A collection of related search tasks
 */
export interface BenchmarkSuite {
  /** Suite name */
  name: string

  /** Suite version for tracking changes */
  version: string

  /** Tier level (2 = grep-hard) */
  tier: number

  /** All tasks in this suite */
  tasks: SearchTask[]

  /** Suite metadata */
  metadata: SuiteMetadata
}

/**
 * Calculate average expected success rate from tasks
 */
function calculateAverageRate(tasks: SearchTask[], field: 'expectedGrepSuccess' | 'expectedSearchSuccess'): number {
  const rates = tasks.map((task) => (task as any)[field] ?? 0)
  const sum = rates.reduce((acc, rate) => acc + rate, 0)
  return sum / tasks.length
}

/**
 * Extract unique categories from tasks
 */
function extractCategories(tasks: SearchTask[]): string[] {
  const categories = new Set(tasks.map((task) => task.category))
  return Array.from(categories).sort()
}

/**
 * Tier 2: Grep-Hard Benchmark Suite
 *
 * Contains 11 tasks where grep struggles (30-60% success) but semantic search
 * provides significant advantage (>30% improvement):
 * - 4 conceptual similarity tasks (retry, error handling, rate limiting, caching)
 * - 4 ambiguity resolution tasks (transactions, auth, cleanup, cache ops)
 * - 3 cross-cutting concerns tasks (async errors, security logging, input validation)
 *
 * Expected performance:
 * - Grep: ~42% average success (grep-hard range)
 * - Search: ~79% average success (strong semantic understanding)
 * - Advantage: ~37% improvement (well above 30% threshold)
 */
export const TIER2_GREP_HARD_SUITE: BenchmarkSuite = (() => {
  const tasks = [
    // Conceptual Similarity (4 tasks)
    TASK_RETRY_IMPLEMENTATIONS,
    TASK_ERROR_HANDLING_PATTERNS,
    TASK_RATE_LIMITING,
    TASK_CACHING_STRATEGIES,

    // Ambiguity Resolution (4 tasks)
    TASK_TRANSACTION_MANAGEMENT,
    TASK_AUTHENTICATION_CHECKS,
    TASK_RESOURCE_CLEANUP,
    TASK_CACHE_OPERATIONS,

    // Cross-Cutting Concerns (3 tasks)
    TASK_ASYNC_ERROR_HANDLING,
    TASK_SECURITY_LOGGING,
    TASK_INPUT_VALIDATION,
  ]

  const grepRate = calculateAverageRate(tasks, 'expectedGrepSuccess')
  const searchRate = calculateAverageRate(tasks, 'expectedSearchSuccess')

  return {
    name: 'Tier 2: Grep-Hard Tasks',
    version: '1.0.0',
    tier: 2,
    tasks,

    metadata: {
      totalTasks: tasks.length,
      categories: extractCategories(tasks),
      expectedGrepSuccessRate: grepRate,
      expectedSearchSuccessRate: searchRate,
      expectedImprovement: searchRate - grepRate,
      description:
        'Tasks where grep struggles (30-60% success) due to conceptual similarity, ' +
        'keyword ambiguity, and scattered patterns. Semantic search provides significant ' +
        'advantage (>30% improvement) through conceptual understanding, context disambiguation, ' +
        'and pattern aggregation.',
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

  /** Average expected grep success rate */
  avgGrepSuccess: number

  /** Average expected search success rate */
  avgSearchSuccess: number

  /** Average improvement (search - grep) */
  avgImprovement: number

  /** Task IDs in this category */
  taskIds: string[]
}

/**
 * Statistics for a difficulty level
 */
export interface DifficultyStatistics {
  /** Difficulty level */
  difficulty: 'easy' | 'medium' | 'hard'

  /** Number of tasks at this difficulty */
  taskCount: number

  /** Average expected grep success rate */
  avgGrepSuccess: number

  /** Average expected search success rate */
  avgSearchSuccess: number

  /** Average improvement (search - grep) */
  avgImprovement: number

  /** Task IDs at this difficulty */
  taskIds: string[]
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

  /** Overall expected grep success rate */
  overallGrepSuccess: number

  /** Overall expected search success rate */
  overallSearchSuccess: number

  /** Expected improvement (search - grep) */
  expectedImprovement: number
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
    const avgGrepSuccess = calculateAverageRate(tasks, 'expectedGrepSuccess')
    const avgSearchSuccess = calculateAverageRate(tasks, 'expectedSearchSuccess')

    byCategory.set(category, {
      category,
      taskCount: tasks.length,
      avgGrepSuccess,
      avgSearchSuccess,
      avgImprovement: avgSearchSuccess - avgGrepSuccess,
      taskIds: tasks.map((t) => t.id),
    })
  }

  // Calculate difficulty statistics
  const tasksByDifficulty = getTasksByDifficulty(suite)
  for (const [difficulty, tasks] of tasksByDifficulty) {
    const avgGrepSuccess = calculateAverageRate(tasks, 'expectedGrepSuccess')
    const avgSearchSuccess = calculateAverageRate(tasks, 'expectedSearchSuccess')

    byDifficulty.set(difficulty, {
      difficulty: difficulty as 'easy' | 'medium' | 'hard',
      taskCount: tasks.length,
      avgGrepSuccess,
      avgSearchSuccess,
      avgImprovement: avgSearchSuccess - avgGrepSuccess,
      taskIds: tasks.map((t) => t.id),
    })
  }

  const overallGrepSuccess = suite.metadata.expectedGrepSuccessRate
  const overallSearchSuccess = suite.metadata.expectedSearchSuccessRate

  return {
    totalTasks: suite.tasks.length,
    byCategory,
    byDifficulty,
    overallGrepSuccess,
    overallSearchSuccess,
    expectedImprovement: overallSearchSuccess - overallGrepSuccess,
  }
}
