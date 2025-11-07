/**
 * Tests for benchmark suite runner
 */

import { describe, it, expect, beforeEach } from 'vitest'
import type { SearchTask } from '../../types.js'
import {
  runBenchmarkSuite,
  calculateAggregateMetrics,
  validateSuiteResults,
  formatSuiteSummary,
  type TaskResult,
  type SuiteResult,
} from '../suite-runner.js'
import type { BenchmarkSuite } from '../tier1-impossible.js'

// Helper to create mock tasks
function createMockTask(id: string, expectedGrepSuccess: number, expectedSearchSuccess: number): SearchTask {
  return {
    id,
    name: `Task ${id}`,
    description: 'Mock task',
    category: 'relationship-discovery',
    difficulty: 'hard',
    searchTarget: { type: 'function', name: 'mockFunction' },
    followUpTask: {
      type: 'explanation',
      prompt: 'Explain',
      validator: { type: 'explanation' },
    },
    expectedGrepSuccess,
    expectedSearchSuccess,
    successValidator: () => ({
      searchQuality: 1,
      taskCompletion: 1,
      efficiency: 1,
      total: 1,
      details: 'Mock validator',
    }),
  } as SearchTask
}

// Helper to create mock suite
function createMockSuite(tasks: SearchTask[]): BenchmarkSuite {
  return {
    name: 'Mock Suite',
    version: '1.0.0',
    tier: 1,
    tasks,
    metadata: {
      totalTasks: tasks.length,
      categories: ['relationship-discovery'],
      expectedGrepSuccessRate: 0.25,
      expectedSearchSuccessRate: 0.75,
      description: 'Mock suite for testing',
    },
  }
}

// Helper to create task results
function createTaskResults(metrics: Array<[number, number]>, tasks?: SearchTask[]): TaskResult[] {
  return metrics.map(([grepSuccess, searchSuccess], i) => ({
    task: tasks?.[i] ?? createMockTask(`task-${i}`, grepSuccess, searchSuccess),
    grepSuccess,
    searchSuccess,
    improvement: searchSuccess - grepSuccess,
  }))
}

describe('suite-runner', () => {
  describe('calculateAggregateMetrics', () => {
    it('should calculate correct averages for single task', () => {
      const results = createTaskResults([[0.2, 0.8]])

      const metrics = calculateAggregateMetrics(results)

      expect(metrics.grepAvgSuccess).toBe(0.2)
      expect(metrics.searchAvgSuccess).toBe(0.8)
      expect(metrics.avgImprovement).toBeCloseTo(0.6, 2)
      expect(metrics.tasksDefeatingGrep).toBe(1) // 60% improvement > 30%
    })

    it('should calculate correct averages for multiple tasks', () => {
      const results = createTaskResults([
        [0.1, 0.7], // +0.6 improvement
        [0.2, 0.8], // +0.6 improvement
        [0.3, 0.9], // +0.6 improvement
      ])

      const metrics = calculateAggregateMetrics(results)

      expect(metrics.grepAvgSuccess).toBeCloseTo(0.2, 2)
      expect(metrics.searchAvgSuccess).toBeCloseTo(0.8, 2)
      expect(metrics.avgImprovement).toBeCloseTo(0.6, 2)
      expect(metrics.tasksDefeatingGrep).toBe(3) // All have >30% improvement
    })

    it('should count tasks defeating grep correctly', () => {
      const results = createTaskResults([
        [0.2, 0.6], // +0.4 improvement (defeats grep)
        [0.3, 0.5], // +0.2 improvement (does not defeat grep)
        [0.1, 0.7], // +0.6 improvement (defeats grep)
      ])

      const metrics = calculateAggregateMetrics(results)

      expect(metrics.tasksDefeatingGrep).toBe(2)
    })

    it('should handle tasks with zero improvement', () => {
      const results = createTaskResults([
        [0.5, 0.5], // No improvement
        [0.6, 0.6], // No improvement
      ])

      const metrics = calculateAggregateMetrics(results)

      expect(metrics.grepAvgSuccess).toBe(0.55)
      expect(metrics.searchAvgSuccess).toBe(0.55)
      expect(metrics.avgImprovement).toBe(0)
      expect(metrics.tasksDefeatingGrep).toBe(0)
    })

    it('should handle negative improvement (search worse than grep)', () => {
      const results = createTaskResults([
        [0.8, 0.6], // -0.2 improvement
      ])

      const metrics = calculateAggregateMetrics(results)

      expect(metrics.avgImprovement).toBeCloseTo(-0.2, 2)
      expect(metrics.tasksDefeatingGrep).toBe(0)
    })

    it('should return zeros for empty results', () => {
      const metrics = calculateAggregateMetrics([])

      expect(metrics.grepAvgSuccess).toBe(0)
      expect(metrics.searchAvgSuccess).toBe(0)
      expect(metrics.avgImprovement).toBe(0)
      expect(metrics.tasksDefeatingGrep).toBe(0)
    })
  })

  describe('validateSuiteResults', () => {
    it('should pass validation when criteria are met', () => {
      const tasks = [createMockTask('task-1', 0.2, 0.8), createMockTask('task-2', 0.3, 0.75)]
      const suite = createMockSuite(tasks)
      const results = createTaskResults(
        [
          [0.2, 0.8],
          [0.3, 0.75],
        ],
        tasks,
      )

      const validation = validateSuiteResults(results, suite)

      expect(validation.meetsGrepFailureCriterion).toBe(true) // 25% < 40%
      expect(validation.meetsSearchSuccessCriterion).toBe(true) // 77.5% > 70%
      expect(validation.allTasksValidated).toBe(true)
      expect(validation.details.some((d) => d.includes('✓ Grep failure criterion met'))).toBe(true)
      expect(validation.details.some((d) => d.includes('✓ Search success criterion met'))).toBe(true)
      expect(validation.details.some((d) => d.includes('✓ All 2 tasks met expected performance'))).toBe(true)
    })

    it('should fail when grep succeeds too much', () => {
      const tasks = [createMockTask('task-1', 0.6, 0.8)]
      const suite = createMockSuite(tasks)
      const results = createTaskResults([[0.6, 0.8]], tasks)

      const validation = validateSuiteResults(results, suite)

      expect(validation.meetsGrepFailureCriterion).toBe(false) // 60% >= 40%
      expect(validation.details.some((d) => d.includes('✗ Grep failure criterion not met'))).toBe(true)
      expect(validation.details.some((d) => d.includes('tasks too easy for grep'))).toBe(true)
    })

    it('should fail when search success is too low', () => {
      const tasks = [createMockTask('task-1', 0.2, 0.6)]
      const suite = createMockSuite(tasks)
      const results = createTaskResults([[0.2, 0.6]], tasks)

      const validation = validateSuiteResults(results, suite)

      expect(validation.meetsSearchSuccessCriterion).toBe(false) // 60% <= 70%
      expect(validation.details.some((d) => d.includes('✗ Search success criterion not met'))).toBe(true)
      expect(validation.details.some((d) => d.includes('search not effective enough'))).toBe(true)
    })

    it('should fail when individual tasks deviate from expected', () => {
      const tasks = [createMockTask('task-1', 0.2, 0.8), createMockTask('task-2', 0.3, 0.75)]
      const suite = createMockSuite(tasks)
      // Task 2 has grep 0.6 instead of expected 0.3 (>10% tolerance)
      const results = createTaskResults(
        [
          [0.2, 0.8],
          [0.6, 0.75],
        ],
        tasks,
      )

      const validation = validateSuiteResults(results, suite)

      expect(validation.allTasksValidated).toBe(false)
      expect(validation.details.some((d) => d.includes('✗ 1/2 tasks outside expected ranges'))).toBe(true)
      expect(validation.details.some((d) => d.includes('task-2'))).toBe(true)
    })

    it('should allow tolerance of ±10% for individual tasks', () => {
      const tasks = [createMockTask('task-1', 0.2, 0.8)]
      const suite = createMockSuite(tasks)
      // Within ±10%: grep 0.25 (expected 0.2), search 0.85 (expected 0.8)
      const results = createTaskResults([[0.25, 0.85]], tasks)

      const validation = validateSuiteResults(results, suite)

      expect(validation.allTasksValidated).toBe(true)
    })

    it('should handle empty results', () => {
      const suite = createMockSuite([])
      const validation = validateSuiteResults([], suite)

      expect(validation.meetsGrepFailureCriterion).toBe(true) // 0% < 40%
      expect(validation.meetsSearchSuccessCriterion).toBe(false) // 0% <= 70%
      expect(validation.allTasksValidated).toBe(true) // No tasks to validate
    })
  })

  describe('runBenchmarkSuite', () => {
    it('should execute suite sequentially by default', async () => {
      const tasks = [createMockTask('task-1', 0.2, 0.8), createMockTask('task-2', 0.3, 0.75)]
      const suite = createMockSuite(tasks)

      const result = await runBenchmarkSuite(suite)

      expect(result.suite).toBe(suite)
      expect(result.taskResults).toHaveLength(2)
      expect(result.executionTime).toBeGreaterThan(0)
      expect(result.aggregate.grepAvgSuccess).toBeCloseTo(0.25, 2)
      expect(result.aggregate.searchAvgSuccess).toBeCloseTo(0.775, 2)
      expect(result.validation.meetsGrepFailureCriterion).toBe(true)
      expect(result.validation.meetsSearchSuccessCriterion).toBe(true)
    })

    it('should execute suite in parallel when configured', async () => {
      const tasks = [createMockTask('task-1', 0.2, 0.8), createMockTask('task-2', 0.3, 0.75)]
      const suite = createMockSuite(tasks)

      const result = await runBenchmarkSuite(suite, { parallel: true })

      expect(result.suite).toBe(suite)
      expect(result.taskResults).toHaveLength(2)
      expect(result.executionTime).toBeGreaterThan(0)
    })

    it('should include aggregate metrics', async () => {
      const tasks = [
        createMockTask('task-1', 0.1, 0.9),
        createMockTask('task-2', 0.2, 0.8),
        createMockTask('task-3', 0.3, 0.7),
      ]
      const suite = createMockSuite(tasks)

      const result = await runBenchmarkSuite(suite)

      expect(result.aggregate.grepAvgSuccess).toBeCloseTo(0.2, 2)
      expect(result.aggregate.searchAvgSuccess).toBeCloseTo(0.8, 2)
      expect(result.aggregate.avgImprovement).toBeCloseTo(0.6, 2)
      expect(result.aggregate.tasksDefeatingGrep).toBe(3) // All have >30% improvement
    })

    it('should include validation results', async () => {
      const tasks = [createMockTask('task-1', 0.2, 0.8)]
      const suite = createMockSuite(tasks)

      const result = await runBenchmarkSuite(suite)

      expect(result.validation.meetsGrepFailureCriterion).toBe(true)
      expect(result.validation.meetsSearchSuccessCriterion).toBe(true)
      expect(result.validation.allTasksValidated).toBe(true)
      expect(result.validation.details.length).toBeGreaterThan(0)
    })

    it('should handle empty suite', async () => {
      const suite = createMockSuite([])

      const result = await runBenchmarkSuite(suite)

      expect(result.taskResults).toHaveLength(0)
      expect(result.aggregate.grepAvgSuccess).toBe(0)
      expect(result.aggregate.searchAvgSuccess).toBe(0)
      expect(result.aggregate.avgImprovement).toBe(0)
      expect(result.aggregate.tasksDefeatingGrep).toBe(0)
    })

    it('should respect configuration options', async () => {
      const tasks = [createMockTask('task-1', 0.2, 0.8)]
      const suite = createMockSuite(tasks)

      const result = await runBenchmarkSuite(suite, {
        parallel: false,
        iterations: 5,
        useMockData: true,
      })

      expect(result.taskResults).toHaveLength(1)
    })
  })

  describe('formatSuiteSummary', () => {
    let mockResult: SuiteResult

    beforeEach(() => {
      const tasks = [createMockTask('task-1', 0.2, 0.8), createMockTask('task-2', 0.3, 0.75)]
      const taskResults = createTaskResults(
        [
          [0.2, 0.8],
          [0.3, 0.75],
        ],
        tasks,
      )

      mockResult = {
        suite: createMockSuite(tasks),
        executionTime: 1234,
        taskResults,
        aggregate: calculateAggregateMetrics(taskResults),
        validation: validateSuiteResults(taskResults, createMockSuite(tasks)),
      }
    })

    it('should include suite metadata', () => {
      const summary = formatSuiteSummary(mockResult)

      expect(summary).toContain('Suite: Mock Suite')
      expect(summary).toContain('v1.0.0')
      expect(summary).toContain('Tasks: 2')
      expect(summary).toContain('Execution time: 1234ms')
    })

    it('should include performance metrics', () => {
      const summary = formatSuiteSummary(mockResult)

      expect(summary).toContain('Performance:')
      expect(summary).toContain('Grep avg:')
      expect(summary).toContain('25.0%')
      expect(summary).toContain('Search avg:')
      expect(summary).toContain('77.5%')
      expect(summary).toContain('Improvement: +52.5%')
      expect(summary).toContain('Tasks defeating grep: 2/2')
    })

    it('should include validation details', () => {
      const summary = formatSuiteSummary(mockResult)

      expect(summary).toContain('Validation:')
      expect(summary).toContain('✓ Grep failure criterion met')
      expect(summary).toContain('✓ Search success criterion met')
      expect(summary).toContain('✓ All 2 tasks met expected performance')
    })

    it('should format validation failures correctly', () => {
      const tasks = [createMockTask('task-1', 0.2, 0.6)]
      const taskResults = createTaskResults([[0.2, 0.6]], tasks)

      const failedResult: SuiteResult = {
        suite: createMockSuite(tasks),
        executionTime: 100,
        taskResults,
        aggregate: calculateAggregateMetrics(taskResults),
        validation: validateSuiteResults(taskResults, createMockSuite(tasks)),
      }

      const summary = formatSuiteSummary(failedResult)

      expect(summary).toContain('✗ Search success criterion not met')
      expect(summary).toContain('60.0% <= 70%')
      expect(summary).toContain('search not effective enough')
    })

    it('should be a complete multi-line string', () => {
      const summary = formatSuiteSummary(mockResult)

      expect(summary).toBeTruthy()
      expect(summary.split('\n').length).toBeGreaterThan(10)
    })
  })

  describe('edge cases', () => {
    it('should handle all tasks failing', async () => {
      const tasks = [createMockTask('task-1', 0.0, 0.0), createMockTask('task-2', 0.0, 0.0)]
      const suite = createMockSuite(tasks)

      const result = await runBenchmarkSuite(suite)

      expect(result.aggregate.grepAvgSuccess).toBe(0)
      expect(result.aggregate.searchAvgSuccess).toBe(0)
      expect(result.aggregate.avgImprovement).toBe(0)
      expect(result.validation.meetsGrepFailureCriterion).toBe(true) // 0% < 40%
      expect(result.validation.meetsSearchSuccessCriterion).toBe(false) // 0% <= 70%
    })

    it('should handle all tasks passing perfectly', async () => {
      const tasks = [createMockTask('task-1', 1.0, 1.0), createMockTask('task-2', 1.0, 1.0)]
      const suite = createMockSuite(tasks)

      const result = await runBenchmarkSuite(suite)

      expect(result.aggregate.grepAvgSuccess).toBe(1)
      expect(result.aggregate.searchAvgSuccess).toBe(1)
      expect(result.aggregate.avgImprovement).toBe(0)
      expect(result.validation.meetsGrepFailureCriterion).toBe(false) // 100% >= 40%
      expect(result.validation.meetsSearchSuccessCriterion).toBe(true) // 100% > 70%
    })

    it('should handle mixed success rates', async () => {
      const tasks = [
        createMockTask('task-1', 0.0, 1.0), // Max improvement
        createMockTask('task-2', 0.5, 0.5), // No improvement
        createMockTask('task-3', 0.2, 0.6), // Moderate improvement
      ]
      const suite = createMockSuite(tasks)

      const result = await runBenchmarkSuite(suite)

      expect(result.aggregate.grepAvgSuccess).toBeCloseTo(0.233, 2)
      expect(result.aggregate.searchAvgSuccess).toBeCloseTo(0.7, 2)
      expect(result.aggregate.tasksDefeatingGrep).toBe(2) // Tasks 1 and 3
    })
  })

  describe('integration with real suite structure', () => {
    it('should work with realistic task structure', async () => {
      const tasks = [
        createMockTask('rel-trans-deps', 0.2, 0.8),
        createMockTask('rel-call-chain', 0.25, 0.75),
        createMockTask('rel-api-impact', 0.3, 0.8),
      ]
      const suite = createMockSuite(tasks)

      const result = await runBenchmarkSuite(suite)

      expect(result.taskResults.length).toBe(3)
      expect(result.aggregate.grepAvgSuccess).toBeCloseTo(0.25, 2)
      expect(result.aggregate.searchAvgSuccess).toBeCloseTo(0.783, 2)
      expect(result.validation.meetsGrepFailureCriterion).toBe(true)
      expect(result.validation.meetsSearchSuccessCriterion).toBe(true)
    })
  })
})
