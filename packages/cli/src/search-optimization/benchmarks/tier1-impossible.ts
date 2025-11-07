/**
 * Tier 1: Grep-Impossible Benchmark Suite
 *
 * Aggregates all 8 grep-impossible tasks from TESTDES-2001, TESTDES-2002, and TESTDES-2003.
 * These tasks fundamentally defeat grep by requiring:
 * - Transitive relationship traversal
 * - Architectural understanding across components
 * - Negative space detection (finding what's missing)
 *
 * This suite provides:
 * - Unified task collection
 * - Category-based organization
 * - Statistical analysis
 * - Validation of suite composition
 */

import {
  TASK_DATA_FLOW_WORKTREE_CREATION,
  TASK_INIT_SEQUENCE_ORCHESTRATOR,
  TASK_SYSTEM_INTERACTIONS_MCP_SEARCH,
} from '../tasks/architectural-understanding/index.js'
import { TASK_MISSING_ERROR_HANDLING, TASK_UNPROTECTED_FILE_OPERATIONS } from '../tasks/negative-space/index.js'
import {
  TASK_TRANSITIVE_DEPENDENCIES,
  TASK_CALL_CHAIN_TRACING,
  TASK_API_IMPACT_ANALYSIS,
} from '../tasks/relationship-discovery/index.js'
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

  /** Tier level (1 = grep-impossible, higher tiers = grep-hard/possible) */
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
 * Tier 1: Grep-Impossible Benchmark Suite
 *
 * Contains 8 tasks that fundamentally defeat grep-based search:
 * - 3 relationship discovery tasks (transitive deps, call chains, impact analysis)
 * - 3 architectural understanding tasks (data flow, init sequence, system interactions)
 * - 2 negative space tasks (missing error handling, unprotected operations)
 *
 * Expected performance:
 * - Grep: ~25% average success (tasks designed to defeat grep)
 * - Search: ~78% average success (semantic understanding provides clear advantage)
 */
export const TIER1_GREP_IMPOSSIBLE_SUITE: BenchmarkSuite = (() => {
  const tasks = [
    // Relationship Discovery (3 tasks)
    TASK_TRANSITIVE_DEPENDENCIES,
    TASK_CALL_CHAIN_TRACING,
    TASK_API_IMPACT_ANALYSIS,

    // Architectural Understanding (3 tasks)
    TASK_DATA_FLOW_WORKTREE_CREATION,
    TASK_INIT_SEQUENCE_ORCHESTRATOR,
    TASK_SYSTEM_INTERACTIONS_MCP_SEARCH,

    // Negative Space (2 tasks)
    TASK_MISSING_ERROR_HANDLING,
    TASK_UNPROTECTED_FILE_OPERATIONS,
  ]

  return {
    name: 'Tier 1: Grep-Impossible Tasks',
    version: '1.0.0',
    tier: 1,
    tasks,

    metadata: {
      totalTasks: tasks.length,
      categories: extractCategories(tasks),
      expectedGrepSuccessRate: calculateAverageRate(tasks, 'expectedGrepSuccess'),
      expectedSearchSuccessRate: calculateAverageRate(tasks, 'expectedSearchSuccess'),
      description:
        'Tasks that fundamentally defeat grep by requiring code graph traversal, ' +
        'architectural understanding, or negative space detection. These tasks demonstrate ' +
        'the critical advantage of semantic search over keyword-based approaches.',
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
    byCategory.set(category, {
      category,
      taskCount: tasks.length,
      avgGrepSuccess: calculateAverageRate(tasks, 'expectedGrepSuccess'),
      avgSearchSuccess: calculateAverageRate(tasks, 'expectedSearchSuccess'),
      taskIds: tasks.map((t) => t.id),
    })
  }

  // Calculate difficulty statistics
  const tasksByDifficulty = getTasksByDifficulty(suite)
  for (const [difficulty, tasks] of tasksByDifficulty) {
    byDifficulty.set(difficulty, {
      difficulty: difficulty as 'easy' | 'medium' | 'hard',
      taskCount: tasks.length,
      avgGrepSuccess: calculateAverageRate(tasks, 'expectedGrepSuccess'),
      avgSearchSuccess: calculateAverageRate(tasks, 'expectedSearchSuccess'),
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
