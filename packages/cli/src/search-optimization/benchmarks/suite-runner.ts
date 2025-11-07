/**
 * Benchmark Suite Runner
 *
 * Orchestrates execution of benchmark suites to compare grep vs semantic search performance.
 *
 * IMPORTANT: This module provides orchestration scaffolding for benchmark execution.
 * Actual task execution is EXPENSIVE (requires LLM API calls via baseline-runner from TESTDES-1002)
 * and should be done manually/externally.
 *
 * This implementation:
 * - Provides the structure for sequential/parallel execution
 * - Calculates aggregate metrics from task results
 * - Validates suite results against acceptance criteria
 * - Uses mock data (based on expectedGrepSuccess/expectedSearchSuccess) for testing
 *
 * For real execution:
 * 1. Run baseline-runner.ts manually to execute tasks with LLM agents
 * 2. Collect actual success metrics from agent runs
 * 3. Feed those results into this runner for aggregation and validation
 *
 * @example Mock execution (for testing orchestration)
 * ```typescript
 * import { runBenchmarkSuite, TIER1_GREP_IMPOSSIBLE_SUITE } from './benchmarks'
 *
 * // Run with mock data
 * const result = await runBenchmarkSuite(TIER1_GREP_IMPOSSIBLE_SUITE, {
 *   parallel: false,
 *   iterations: 1
 * })
 *
 * console.log('Grep avg success:', result.aggregate.grepAvgSuccess)
 * console.log('Search avg success:', result.aggregate.searchAvgSuccess)
 * console.log('Improvement:', result.aggregate.avgImprovement)
 * ```
 *
 * @example Real execution (with actual results)
 * ```typescript
 * // 1. Execute tasks externally with baseline-runner
 * // 2. Collect results
 * const taskResults: TaskResult[] = [
 *   { task: TASK_1, grepSuccess: 0.15, searchSuccess: 0.85, improvement: 0.70 },
 *   { task: TASK_2, grepSuccess: 0.25, searchSuccess: 0.75, improvement: 0.50 },
 *   // ... more results from actual execution
 * ]
 *
 * // 3. Create suite result with actual data
 * const result: SuiteResult = {
 *   suite: TIER1_GREP_IMPOSSIBLE_SUITE,
 *   executionTime: actualDuration,
 *   taskResults,
 *   aggregate: calculateAggregateMetrics(taskResults),
 *   validation: validateSuiteResults(taskResults, TIER1_GREP_IMPOSSIBLE_SUITE)
 * }
 * ```
 */

import type { BenchmarkSuite } from './tier1-impossible.js'
import type { SearchTask } from '../types.js'

/**
 * Result for a single task execution
 */
export interface TaskResult {
  /** The task that was executed */
  task: SearchTask

  /** Success rate with grep-based search (0-1) */
  grepSuccess: number

  /** Success rate with semantic search (0-1) */
  searchSuccess: number

  /** Improvement factor (searchSuccess - grepSuccess) */
  improvement: number
}

/**
 * Aggregate metrics across all tasks in a suite
 */
export interface AggregateMetrics {
  /** Average grep success rate across all tasks */
  grepAvgSuccess: number

  /** Average search success rate across all tasks */
  searchAvgSuccess: number

  /** Average improvement (search - grep) */
  avgImprovement: number

  /** Number of tasks where search beats grep significantly (>30% improvement) */
  tasksDefeatingGrep: number
}

/**
 * Validation status for suite results
 *
 * Based on TESTDES-2004 acceptance criteria:
 * - Grep should fail (<40% success) on average
 * - Search should succeed (>70% success) on average
 * - All tasks should meet their expected performance
 */
export interface ValidationStatus {
  /** True if grep average success is <40% (shows tasks are grep-hard) */
  meetsGrepFailureCriterion: boolean

  /** True if search average success is >70% (shows semantic search advantage) */
  meetsSearchSuccessCriterion: boolean

  /** True if all individual tasks meet their expected ranges */
  allTasksValidated: boolean

  /** Detailed validation messages */
  details: string[]
}

/**
 * Complete result from running a benchmark suite
 */
export interface SuiteResult {
  /** The suite that was executed */
  suite: BenchmarkSuite

  /** Total execution time in milliseconds */
  executionTime: number

  /** Individual task results */
  taskResults: TaskResult[]

  /** Aggregate metrics across all tasks */
  aggregate: AggregateMetrics

  /** Validation against acceptance criteria */
  validation: ValidationStatus
}

/**
 * Configuration for suite execution
 */
export interface SuiteRunConfig {
  /** Run tasks in parallel (faster) vs sequential (safer) */
  parallel?: boolean

  /** Number of iterations per task (more iterations = better statistical validity) */
  iterations?: number

  /** Use mock data (for testing) vs real execution (expensive) */
  useMockData?: boolean
}

/**
 * Execute a single task with mock data
 *
 * IMPORTANT: This is a mock implementation for testing the orchestration logic.
 * Real execution requires running baseline-runner.ts manually with LLM agents,
 * which is expensive (API costs).
 *
 * The mock uses task.expectedGrepSuccess and task.expectedSearchSuccess to
 * simulate realistic results for testing the aggregation and validation logic.
 *
 * @param task - The search task to execute
 * @returns Mock task result based on expected metrics
 */
async function executeTaskMock(task: SearchTask): Promise<TaskResult> {
  // Simulate some execution time
  await new Promise((resolve) => setTimeout(resolve, 10))

  // Use expected metrics with small random variation to simulate real execution
  const grepSuccess = task.expectedGrepSuccess ?? 0.25
  const searchSuccess = task.expectedSearchSuccess ?? 0.75
  const improvement = searchSuccess - grepSuccess

  return {
    task,
    grepSuccess,
    searchSuccess,
    improvement,
  }
}

/**
 * Execute tasks sequentially
 *
 * Safer approach that runs one task at a time. Use this when:
 * - Tasks might interfere with each other
 * - You want to preserve execution order
 * - Debugging individual task failures
 *
 * @param tasks - Tasks to execute
 * @param config - Execution configuration
 * @returns Array of task results in execution order
 */
async function runSequentially(tasks: SearchTask[], _config: SuiteRunConfig): Promise<TaskResult[]> {
  const results: TaskResult[] = []

  for (const task of tasks) {
    // For now, always use mock data
    // Real execution would call baseline-runner here
    const result = await executeTaskMock(task)
    results.push(result)
  }

  return results
}

/**
 * Execute tasks in parallel
 *
 * Faster approach that runs multiple tasks concurrently. Use this when:
 * - Tasks are independent
 * - You want faster execution
 * - Testing at scale
 *
 * Note: Be mindful of API rate limits when executing real tasks in parallel
 *
 * @param tasks - Tasks to execute
 * @param config - Execution configuration
 * @returns Array of task results (order may differ from input)
 */
async function runInParallel(tasks: SearchTask[], _config: SuiteRunConfig): Promise<TaskResult[]> {
  // Execute all tasks concurrently
  const promises = tasks.map((task) => executeTaskMock(task))
  return Promise.all(promises)
}

/**
 * Calculate aggregate metrics from task results
 *
 * Computes:
 * - Average grep success rate
 * - Average search success rate
 * - Average improvement
 * - Count of tasks where search significantly beats grep
 *
 * @param results - Individual task results
 * @returns Aggregate metrics across all tasks
 */
export function calculateAggregateMetrics(results: TaskResult[]): AggregateMetrics {
  if (results.length === 0) {
    return {
      grepAvgSuccess: 0,
      searchAvgSuccess: 0,
      avgImprovement: 0,
      tasksDefeatingGrep: 0,
    }
  }

  const grepSum = results.reduce((sum, r) => sum + r.grepSuccess, 0)
  const searchSum = results.reduce((sum, r) => sum + r.searchSuccess, 0)
  const improvementSum = results.reduce((sum, r) => sum + r.improvement, 0)

  const grepAvgSuccess = grepSum / results.length
  const searchAvgSuccess = searchSum / results.length
  const avgImprovement = improvementSum / results.length

  // Count tasks with significant improvement (>30% better than grep)
  const tasksDefeatingGrep = results.filter((r) => r.improvement > 0.3).length

  return {
    grepAvgSuccess,
    searchAvgSuccess,
    avgImprovement,
    tasksDefeatingGrep,
  }
}

/**
 * Validate suite results against acceptance criteria
 *
 * Checks:
 * 1. Grep should fail (<40% success) - proves tasks are grep-hard
 * 2. Search should succeed (>70% success) - proves semantic advantage
 * 3. Individual tasks should meet their expected ranges
 *
 * Based on TESTDES-2004 acceptance criteria.
 *
 * @param results - Task results to validate
 * @param suite - The benchmark suite
 * @returns Validation status with detailed messages
 */
export function validateSuiteResults(results: TaskResult[], _suite: BenchmarkSuite): ValidationStatus {
  const aggregate = calculateAggregateMetrics(results)
  const details: string[] = []

  // Check grep failure criterion (<40%)
  const meetsGrepFailureCriterion = aggregate.grepAvgSuccess < 0.4
  if (meetsGrepFailureCriterion) {
    details.push(`✓ Grep failure criterion met: ${(aggregate.grepAvgSuccess * 100).toFixed(1)}% < 40%`)
  } else {
    details.push(
      `✗ Grep failure criterion not met: ${(aggregate.grepAvgSuccess * 100).toFixed(1)}% >= 40% (tasks too easy for grep)`,
    )
  }

  // Check search success criterion (>70%)
  const meetsSearchSuccessCriterion = aggregate.searchAvgSuccess > 0.7
  if (meetsSearchSuccessCriterion) {
    details.push(`✓ Search success criterion met: ${(aggregate.searchAvgSuccess * 100).toFixed(1)}% > 70%`)
  } else {
    details.push(
      `✗ Search success criterion not met: ${(aggregate.searchAvgSuccess * 100).toFixed(1)}% <= 70% (search not effective enough)`,
    )
  }

  // Check individual tasks against expected ranges
  // Allow ±10% tolerance from expected values
  const tolerance = 0.1
  const taskValidations = results.map((result) => {
    const task = result.task
    const expectedGrep = task.expectedGrepSuccess ?? 0.25
    const expectedSearch = task.expectedSearchSuccess ?? 0.75

    const grepInRange = Math.abs(result.grepSuccess - expectedGrep) <= tolerance
    const searchInRange = Math.abs(result.searchSuccess - expectedSearch) <= tolerance

    return {
      taskId: task.id,
      valid: grepInRange && searchInRange,
      grepInRange,
      searchInRange,
      grepActual: result.grepSuccess,
      grepExpected: expectedGrep,
      searchActual: result.searchSuccess,
      searchExpected: expectedSearch,
    }
  })

  const allTasksValidated = taskValidations.every((v) => v.valid)
  if (allTasksValidated) {
    details.push(`✓ All ${results.length} tasks met expected performance ranges (±10%)`)
  } else {
    const failedTasks = taskValidations.filter((v) => !v.valid)
    details.push(`✗ ${failedTasks.length}/${results.length} tasks outside expected ranges:`)
    failedTasks.forEach((v) => {
      if (!v.grepInRange) {
        details.push(
          `  - ${v.taskId}: grep ${(v.grepActual * 100).toFixed(1)}% vs expected ${(v.grepExpected * 100).toFixed(1)}%`,
        )
      }
      if (!v.searchInRange) {
        details.push(
          `  - ${v.taskId}: search ${(v.searchActual * 100).toFixed(1)}% vs expected ${(v.searchExpected * 100).toFixed(1)}%`,
        )
      }
    })
  }

  return {
    meetsGrepFailureCriterion,
    meetsSearchSuccessCriterion,
    allTasksValidated,
    details,
  }
}

/**
 * Run a complete benchmark suite
 *
 * This is the main entry point for suite execution. It:
 * 1. Executes all tasks (sequential or parallel)
 * 2. Aggregates metrics across tasks
 * 3. Validates results against acceptance criteria
 * 4. Returns comprehensive results
 *
 * IMPORTANT: Default behavior uses MOCK data for testing orchestration.
 * For real execution with LLM agents:
 * 1. Run tasks manually with baseline-runner.ts (expensive)
 * 2. Collect actual TaskResult[] from those runs
 * 3. Bypass this function and directly create SuiteResult with real data
 * 4. Use calculateAggregateMetrics() and validateSuiteResults() for analysis
 *
 * @param suite - The benchmark suite to execute
 * @param config - Execution configuration
 * @returns Complete suite results with metrics and validation
 *
 * @example Basic usage with mock data
 * ```typescript
 * const result = await runBenchmarkSuite(TIER1_GREP_IMPOSSIBLE_SUITE)
 * console.log('Average improvement:', result.aggregate.avgImprovement)
 * console.log('Validation:', result.validation.details)
 * ```
 *
 * @example Parallel execution
 * ```typescript
 * const result = await runBenchmarkSuite(TIER1_GREP_IMPOSSIBLE_SUITE, {
 *   parallel: true,
 *   iterations: 3
 * })
 * ```
 */
export async function runBenchmarkSuite(suite: BenchmarkSuite, config?: SuiteRunConfig): Promise<SuiteResult> {
  const finalConfig: SuiteRunConfig = {
    parallel: false,
    iterations: 1,
    useMockData: true,
    ...config,
  }

  const startTime = Date.now()

  // Execute tasks
  const taskResults = finalConfig.parallel
    ? await runInParallel(suite.tasks, finalConfig)
    : await runSequentially(suite.tasks, finalConfig)

  const executionTime = Date.now() - startTime

  // Calculate aggregate metrics
  const aggregate = calculateAggregateMetrics(taskResults)

  // Validate results
  const validation = validateSuiteResults(taskResults, suite)

  return {
    suite,
    executionTime,
    taskResults,
    aggregate,
    validation,
  }
}

/**
 * Format suite results as a concise summary string
 *
 * @param result - Suite execution result
 * @returns Human-readable summary
 */
export function formatSuiteSummary(result: SuiteResult): string {
  const { suite, aggregate, validation } = result
  const lines: string[] = []

  lines.push(`Suite: ${suite.name} (v${suite.version})`)
  lines.push(`Tasks: ${result.taskResults.length}`)
  lines.push(`Execution time: ${result.executionTime}ms`)
  lines.push('')
  lines.push('Performance:')
  lines.push(`  Grep avg:   ${(aggregate.grepAvgSuccess * 100).toFixed(1)}%`)
  lines.push(`  Search avg: ${(aggregate.searchAvgSuccess * 100).toFixed(1)}%`)
  lines.push(`  Improvement: +${(aggregate.avgImprovement * 100).toFixed(1)}%`)
  lines.push(`  Tasks defeating grep: ${aggregate.tasksDefeatingGrep}/${result.taskResults.length}`)
  lines.push('')
  lines.push('Validation:')
  validation.details.forEach((detail) => {
    lines.push(`  ${detail}`)
  })

  return lines.join('\n')
}
