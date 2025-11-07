/**
 * Tests for task validator
 *
 * Comprehensive test coverage for all 5 validation dimensions,
 * tier differences, batch validation, and report generation.
 */

import { describe, it, expect } from 'vitest'
import {
  validateTask,
  validateSuite,
  formatValidationReport,
  formatSuiteValidationReport,
  DEFAULT_THRESHOLDS,
  type SearchTask,
  type BenchmarkSuite,
} from '../index.js'

// ============================================================================
// Test Fixtures
// ============================================================================

/**
 * Create a mock task with configurable properties
 */
function createMockTask(
  overrides?: Partial<SearchTask & { expectedGrepSuccess?: number; expectedSearchSuccess?: number }>,
): SearchTask {
  return {
    id: 'test-task-001',
    name: 'Test Task',
    description: 'A realistic test task for validating search capabilities across multiple dimensions',
    category: 'relationship-discovery',
    difficulty: 'hard',
    searchTarget: {
      type: 'pattern',
      pattern: /test/,
    },
    followUpTask: {
      type: 'code_change',
      prompt: 'Make a change',
      validator: {
        type: 'code_change',
        fileChanged: 'test.ts',
      },
    },
    successValidator: () => ({
      searchQuality: 1,
      taskCompletion: 1,
      efficiency: 1,
      total: 1,
      details: 'Success',
    }),
    expectedGrepSuccess: 0.2,
    expectedSearchSuccess: 0.8,
    basedOnRealScenario: true,
    internalNotes: 'Test notes',
    ...overrides,
  } as SearchTask
}

/**
 * Create a mock suite with configurable tasks
 */
function createMockSuite(tasks: SearchTask[], tier = 1): BenchmarkSuite {
  return {
    name: `Tier ${tier} Test Suite`,
    version: '1.0.0',
    tier,
    tasks,
    metadata: {
      totalTasks: tasks.length,
      categories: ['relationship-discovery'],
      expectedGrepSuccessRate: 0.2,
      expectedSearchSuccessRate: 0.8,
      description: 'Test suite',
    },
  }
}

// ============================================================================
// Dimension 1: Construct Validity (Grep Baseline)
// ============================================================================

describe('Construct Validity', () => {
  it('should pass when grep success is below threshold', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.2, // Below 0.3 threshold for tier1
      expectedSearchSuccess: 0.8,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      useMockData: true,
    })

    expect(result.dimensions.constructValidity.passed).toBe(true)
    expect(result.dimensions.constructValidity.actual).toBe(0.2)
    expect(result.dimensions.constructValidity.details).toContain('appropriately difficult')
  })

  it('should fail when grep success exceeds threshold', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.5, // Above 0.3 threshold for tier1
      expectedSearchSuccess: 0.8,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      useMockData: true,
    })

    expect(result.dimensions.constructValidity.passed).toBe(false)
    expect(result.dimensions.constructValidity.actual).toBe(0.5)
    expect(result.dimensions.constructValidity.details).toContain('too easy')
  })

  it('should use tier1 thresholds correctly', async () => {
    const task = createMockTask({ expectedGrepSuccess: 0.25 })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.dimensions.constructValidity.passed).toBe(true)
    expect(result.dimensions.constructValidity.expected).toBe('≤ 0.3')
  })

  it('should use tier2 thresholds correctly', async () => {
    const task = createMockTask({ expectedGrepSuccess: 0.5 })

    const result = await validateTask({
      task,
      tier: 'tier2-hard',
    })

    expect(result.dimensions.constructValidity.passed).toBe(true)
    expect(result.dimensions.constructValidity.expected).toBe('≤ 0.6')
  })

  it('should use tier3 thresholds correctly', async () => {
    const task = createMockTask({ expectedGrepSuccess: 0.75 })

    const result = await validateTask({
      task,
      tier: 'tier3-realworld',
    })

    expect(result.dimensions.constructValidity.passed).toBe(true)
    expect(result.dimensions.constructValidity.expected).toBe('≤ 0.8')
  })
})

// ============================================================================
// Dimension 2: Discriminant Validity (Search Advantage)
// ============================================================================

describe('Discriminant Validity', () => {
  it('should pass when search succeeds and advantage is sufficient', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.2,
      expectedSearchSuccess: 0.8, // 60pp advantage
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.dimensions.discriminantValidity.passed).toBe(true)
    expect(result.dimensions.discriminantValidity.details).toContain('clear advantage')
  })

  it('should fail when search success is too low', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.2,
      expectedSearchSuccess: 0.5, // Below 0.7 threshold
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.dimensions.discriminantValidity.passed).toBe(false)
    expect(result.dimensions.discriminantValidity.details).toContain('search success')
  })

  it('should fail when advantage is too small', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.5,
      expectedSearchSuccess: 0.7, // Only 20pp advantage, below 30pp threshold
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.dimensions.discriminantValidity.passed).toBe(false)
    expect(result.dimensions.discriminantValidity.details).toContain('improvement')
  })

  it('should check statistical significance', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.65,
      expectedSearchSuccess: 0.7, // Only 5pp advantage, not significant
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.dimensions.discriminantValidity.passed).toBe(false)
    expect(result.dimensions.discriminantValidity.details).toContain('statistically significant')
  })

  it('should use different thresholds for tier2', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.5,
      expectedSearchSuccess: 0.75, // 25pp advantage (OK for tier2, not tier1)
    })

    const tier2Result = await validateTask({
      task,
      tier: 'tier2-hard',
    })

    const tier1Result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(tier2Result.dimensions.discriminantValidity.passed).toBe(true)
    expect(tier1Result.dimensions.discriminantValidity.passed).toBe(false)
  })

  it('should format actual/expected correctly', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.2,
      expectedSearchSuccess: 0.8,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.dimensions.discriminantValidity.actual).toContain('80%')
    expect(result.dimensions.discriminantValidity.actual).toContain('+60pp')
    expect(result.dimensions.discriminantValidity.expected).toContain('≥ 70%')
    expect(result.dimensions.discriminantValidity.expected).toContain('≥ 30pp')
  })
})

// ============================================================================
// Dimension 3: Ecological Validity (Realism)
// ============================================================================

describe('Ecological Validity', () => {
  it('should pass for tasks marked as real scenarios', async () => {
    const task = createMockTask({
      basedOnRealScenario: true,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.dimensions.ecologicalValidity.passed).toBe(true)
    expect(result.dimensions.ecologicalValidity.actual).toBe('Real scenario')
  })

  it('should pass for concrete tasks with context', async () => {
    const task = createMockTask({
      description: 'Find all transitive dependencies of the createWorktree function to understand impact',
      internalNotes: 'Based on actual refactoring scenario',
      basedOnRealScenario: undefined,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.dimensions.ecologicalValidity.passed).toBe(true)
    expect(result.dimensions.ecologicalValidity.actual).toBe('Concrete task')
  })

  it('should fail for synthetic-looking tasks', async () => {
    const task = createMockTask({
      description: 'Test task',
      internalNotes: undefined,
      basedOnRealScenario: undefined,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.dimensions.ecologicalValidity.passed).toBe(false)
    expect(result.dimensions.ecologicalValidity.actual).toBe('Synthetic')
  })

  it('should fail for tasks with "synthetic" in description', async () => {
    const task = createMockTask({
      description: 'This is a synthetic test task for evaluation purposes only',
      basedOnRealScenario: undefined,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.dimensions.ecologicalValidity.passed).toBe(false)
  })

  it('should recommend adding realism markers', async () => {
    const task = createMockTask({
      description: 'Short task',
      basedOnRealScenario: undefined,
      internalNotes: undefined,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.dimensions.ecologicalValidity.details).toContain('basedOnRealScenario')
  })
})

// ============================================================================
// Dimension 4: Test-Retest Reliability (Variance)
// ============================================================================

describe('Test-Retest Reliability', () => {
  it('should pass for tasks with objective validators', async () => {
    const task = createMockTask({
      followUpTask: {
        type: 'code_change',
        prompt: 'Change code',
        validator: {
          type: 'code_change',
          fileChanged: 'test.ts',
        },
      },
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      useMockData: true,
    })

    expect(result.dimensions.reliability.passed).toBe(true)
    expect(result.dimensions.reliability.details).toContain('objective')
    expect(result.dimensions.reliability.details).toContain('5.0%')
  })

  it('should fail for tasks with subjective validators', async () => {
    const task = createMockTask({
      followUpTask: {
        type: 'explanation',
        prompt: 'Explain',
        validator: {
          type: 'explanation',
          mentionsPattern: /test/,
        },
      },
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      useMockData: true,
    })

    expect(result.dimensions.reliability.passed).toBe(false)
    expect(result.dimensions.reliability.details).toContain('subjective')
    expect(result.dimensions.reliability.details).toContain('12.0%')
  })

  it('should mention required iterations', async () => {
    const task = createMockTask()

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      iterations: 10,
      useMockData: true,
    })

    expect(result.dimensions.reliability.details).toContain('10 iterations')
  })

  it('should handle real mode (not implemented)', async () => {
    const task = createMockTask()

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      useMockData: false,
    })

    expect(result.dimensions.reliability.passed).toBe(false)
    expect(result.dimensions.reliability.actual).toBe('Not measured')
    expect(result.dimensions.reliability.details).toContain('Real reliability testing')
  })

  it('should format CV correctly', async () => {
    const task = createMockTask()

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      useMockData: true,
    })

    expect(result.dimensions.reliability.actual).toMatch(/CV = \d+\.\d%/)
    expect(result.dimensions.reliability.expected).toBe('CV ≤ 10%')
  })
})

// ============================================================================
// Dimension 5: Statistical Power (Sample Size)
// ============================================================================

describe('Statistical Power', () => {
  it('should pass for adequate sample size', async () => {
    const task = createMockTask()

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      iterations: 5,
    })

    expect(result.dimensions.statisticalPower.passed).toBe(true)
    expect(result.dimensions.statisticalPower.actual).toBe('n = 5')
  })

  it('should fail for inadequate sample size', async () => {
    const task = createMockTask()

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      iterations: 3,
    })

    expect(result.dimensions.statisticalPower.passed).toBe(false)
    expect(result.dimensions.statisticalPower.actual).toBe('n = 3')
  })

  it('should use default iterations of 5', async () => {
    const task = createMockTask()

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      // No iterations specified
    })

    expect(result.dimensions.statisticalPower.passed).toBe(true)
    expect(result.dimensions.statisticalPower.actual).toBe('n = 5')
  })

  it('should pass for larger sample sizes', async () => {
    const task = createMockTask()

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      iterations: 20,
    })

    expect(result.dimensions.statisticalPower.passed).toBe(true)
    expect(result.dimensions.statisticalPower.details).toContain('n = 20')
  })

  it('should have correct threshold', async () => {
    const task = createMockTask()

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      iterations: 5,
    })

    expect(result.dimensions.statisticalPower.expected).toBe('n ≥ 5')
  })
})

// ============================================================================
// Overall Validation
// ============================================================================

describe('Overall Validation', () => {
  it('should pass when all dimensions pass', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.2,
      expectedSearchSuccess: 0.8,
      basedOnRealScenario: true,
      followUpTask: {
        type: 'code_change',
        prompt: 'Change',
        validator: { type: 'code_change', fileChanged: 'test.ts' },
      },
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      iterations: 5,
    })

    expect(result.passed).toBe(true)
  })

  it('should fail when any dimension fails', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.5, // Too high for tier1
      expectedSearchSuccess: 0.8,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.passed).toBe(false)
  })

  it('should include task metadata in result', async () => {
    const task = createMockTask()

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.task.id).toBe('test-task-001')
    expect(result.tier).toBe('tier1-impossible')
    expect(result.timestamp).toBeInstanceOf(Date)
  })

  it('should generate recommendations', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.5, // Too easy
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.recommendations.length).toBeGreaterThan(0)
  })
})

// ============================================================================
// Recommendations
// ============================================================================

describe('Recommendations', () => {
  it('should recommend making task harder for grep failures', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.5, // Too easy for grep
      expectedSearchSuccess: 0.8,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.recommendations.some((r) => r.includes('too easy for grep'))).toBe(true)
  })

  it('should diagnose low search success', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.2,
      expectedSearchSuccess: 0.5, // Too low
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.recommendations.some((r) => r.includes('Search success rate too low'))).toBe(true)
  })

  it('should diagnose small advantage', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.5,
      expectedSearchSuccess: 0.7, // Only 20pp advantage
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.recommendations.some((r) => r.includes('advantage too small'))).toBe(true)
  })

  it('should recommend adding realism', async () => {
    const task = createMockTask({
      description: 'Short',
      basedOnRealScenario: undefined,
      internalNotes: undefined,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.recommendations.some((r) => r.includes('real-world usage'))).toBe(true)
  })

  it('should recommend objective validators', async () => {
    const task = createMockTask({
      followUpTask: {
        type: 'explanation',
        prompt: 'Explain',
        validator: { type: 'explanation', mentionsPattern: /test/ },
      },
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.recommendations.some((r) => r.includes('inconsistent results'))).toBe(true)
  })

  it('should recommend more iterations', async () => {
    const task = createMockTask()

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      iterations: 3,
    })

    expect(result.recommendations.some((r) => r.includes('Increase iterations'))).toBe(true)
  })

  it('should give positive recommendations for passed tasks', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.2,
      expectedSearchSuccess: 0.8,
      basedOnRealScenario: true,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    expect(result.recommendations.some((r) => r.includes('passed all validation'))).toBe(true)
  })
})

// ============================================================================
// Suite Validation
// ============================================================================

describe('Suite Validation', () => {
  it('should validate all tasks in suite', async () => {
    const tasks = [createMockTask({ id: 'task-1' }), createMockTask({ id: 'task-2' }), createMockTask({ id: 'task-3' })]
    const suite = createMockSuite(tasks)

    const result = await validateSuite(suite)

    expect(result.taskResults.length).toBe(3)
    expect(result.taskResults[0].task.id).toBe('task-1')
    expect(result.taskResults[1].task.id).toBe('task-2')
    expect(result.taskResults[2].task.id).toBe('task-3')
  })

  it('should pass when all tasks pass', async () => {
    const tasks = [
      createMockTask({
        id: 'task-1',
        expectedGrepSuccess: 0.2,
        expectedSearchSuccess: 0.8,
      }),
      createMockTask({
        id: 'task-2',
        expectedGrepSuccess: 0.25,
        expectedSearchSuccess: 0.85,
      }),
    ]
    const suite = createMockSuite(tasks)

    const result = await validateSuite(suite)

    expect(result.passed).toBe(true)
    expect(result.passedTasks).toBe(2)
    expect(result.failedTasks).toBe(0)
  })

  it('should fail when any task fails', async () => {
    const tasks = [
      createMockTask({
        id: 'task-1',
        expectedGrepSuccess: 0.2,
        expectedSearchSuccess: 0.8,
      }),
      createMockTask({
        id: 'task-2',
        expectedGrepSuccess: 0.5, // Too easy
        expectedSearchSuccess: 0.8,
      }),
    ]
    const suite = createMockSuite(tasks)

    const result = await validateSuite(suite)

    expect(result.passed).toBe(false)
    expect(result.passedTasks).toBe(1)
    expect(result.failedTasks).toBe(1)
  })

  it('should calculate aggregate statistics', async () => {
    const tasks = [createMockTask({ id: 'task-1' }), createMockTask({ id: 'task-2' }), createMockTask({ id: 'task-3' })]
    const suite = createMockSuite(tasks)

    const result = await validateSuite(suite)

    expect(result.totalTasks).toBe(3)
    expect(result.passedTasks + result.failedTasks).toBe(3)
  })

  it('should generate summary for passed suite', async () => {
    const tasks = [createMockTask()]
    const suite = createMockSuite(tasks)

    const result = await validateSuite(suite)

    expect(result.summary).toContain('1/1')
    expect(result.summary).toContain('100%')
    expect(result.summary).toContain('ready for benchmarking')
  })

  it('should generate summary for failed suite', async () => {
    const tasks = [
      createMockTask({
        expectedGrepSuccess: 0.5, // Fails construct validity
        expectedSearchSuccess: 0.5, // Fails discriminant validity
      }),
    ]
    const suite = createMockSuite(tasks)

    const result = await validateSuite(suite)

    expect(result.summary).toContain('0/1')
    expect(result.summary).toContain('Common issues')
  })

  it('should respect tier from suite', async () => {
    const tasks = [
      createMockTask({
        expectedGrepSuccess: 0.5, // OK for tier2, not tier1
        expectedSearchSuccess: 0.75,
      }),
    ]
    const tier2Suite = createMockSuite(tasks, 2)

    const result = await validateSuite(tier2Suite)

    expect(result.taskResults[0].tier).toBe('tier2-hard')
    expect(result.taskResults[0].passed).toBe(true)
  })

  it('should accept custom iterations', async () => {
    const tasks = [createMockTask()]
    const suite = createMockSuite(tasks)

    const result = await validateSuite(suite, { iterations: 10 })

    expect(result.taskResults[0].dimensions.statisticalPower.actual).toBe('n = 10')
  })

  it('should accept useMockData flag', async () => {
    const tasks = [createMockTask()]
    const suite = createMockSuite(tasks)

    const result = await validateSuite(suite, { useMockData: false })

    expect(result.taskResults[0].dimensions.reliability.passed).toBe(false)
    expect(result.taskResults[0].dimensions.reliability.actual).toBe('Not measured')
  })
})

// ============================================================================
// Report Formatting
// ============================================================================

describe('Report Formatting', () => {
  it('should format validation report', async () => {
    const task = createMockTask()
    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    const report = formatValidationReport(result)

    expect(report).toContain('Task Validation Report')
    expect(report).toContain(task.name)
    expect(report).toContain(task.id)
    expect(report).toContain('✓ PASSED')
    expect(report).toContain('Construct Validity')
    expect(report).toContain('Discriminant Validity')
    expect(report).toContain('Ecological Validity')
    expect(report).toContain('Test-Retest Reliability')
    expect(report).toContain('Statistical Power')
  })

  it('should show failed status in report', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.5, // Fails
    })
    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    const report = formatValidationReport(result)

    expect(report).toContain('✗ FAILED')
    expect(report).toContain('[✗]')
  })

  it('should include recommendations in report', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.5,
    })
    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    const report = formatValidationReport(result)

    expect(report).toContain('Recommendations')
    expect(result.recommendations.length).toBeGreaterThan(0)
    for (const rec of result.recommendations) {
      expect(report).toContain(rec)
    }
  })

  it('should format suite report', async () => {
    const tasks = [createMockTask({ id: 'task-1' }), createMockTask({ id: 'task-2' })]
    const suite = createMockSuite(tasks)
    const result = await validateSuite(suite)

    const report = formatSuiteValidationReport(result)

    expect(report).toContain('Suite Validation Report')
    expect(report).toContain(suite.name)
    expect(report).toContain('Total Tasks: 2')
    expect(report).toContain('task-1')
    expect(report).toContain('task-2')
  })

  it('should show pass rate in suite report', async () => {
    const tasks = [
      createMockTask({ id: 'task-1' }),
      createMockTask({ id: 'task-2', expectedGrepSuccess: 0.5 }), // Fails
    ]
    const suite = createMockSuite(tasks)
    const result = await validateSuite(suite)

    const report = formatSuiteValidationReport(result)

    expect(report).toContain('Pass Rate: 50.0%')
    expect(report).toContain('Passed: 1')
    expect(report).toContain('Failed: 1')
  })

  it('should show failed task details in suite report', async () => {
    const tasks = [
      createMockTask({
        id: 'task-1',
        expectedGrepSuccess: 0.5, // Fails construct validity
      }),
    ]
    const suite = createMockSuite(tasks)
    const result = await validateSuite(suite)

    const report = formatSuiteValidationReport(result)

    expect(report).toContain('Failed Task Details')
    expect(report).toContain('task-1')
    expect(report).toContain('Construct Validity')
  })
})

// ============================================================================
// Edge Cases
// ============================================================================

describe('Edge Cases', () => {
  it('should handle missing expectedGrepSuccess', async () => {
    const task = createMockTask({
      expectedGrepSuccess: undefined,
      expectedSearchSuccess: 0.8,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    // Should default to 0.5
    expect(result.dimensions.constructValidity.actual).toBe(0.5)
  })

  it('should handle missing expectedSearchSuccess', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.2,
      expectedSearchSuccess: undefined,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    // Should default to 0.5
    expect(result.dimensions.discriminantValidity.actual).toContain('50%')
  })

  it('should handle missing basedOnRealScenario', async () => {
    const task = createMockTask({
      basedOnRealScenario: undefined,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    // Should check other indicators
    expect(result.dimensions.ecologicalValidity).toBeDefined()
  })

  it('should handle empty suite', async () => {
    const suite = createMockSuite([])

    const result = await validateSuite(suite)

    expect(result.totalTasks).toBe(0)
    expect(result.passedTasks).toBe(0)
    expect(result.failedTasks).toBe(0)
    expect(result.passed).toBe(true) // Vacuously true
  })

  it('should handle custom thresholds', async () => {
    const task = createMockTask({
      expectedGrepSuccess: 0.4,
      expectedSearchSuccess: 0.8,
    })

    const customThresholds = {
      ...DEFAULT_THRESHOLDS,
      tier1: {
        ...DEFAULT_THRESHOLDS.tier1,
        grepMaxSuccess: 0.5, // More lenient
      },
    }

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      thresholds: customThresholds,
    })

    expect(result.dimensions.constructValidity.passed).toBe(true)
  })

  it('should handle zero iterations gracefully', async () => {
    const task = createMockTask()

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
      iterations: 0,
    })

    expect(result.dimensions.statisticalPower.passed).toBe(false)
  })

  it('should handle negative expected values', async () => {
    const task = createMockTask({
      expectedGrepSuccess: -0.1,
      expectedSearchSuccess: 1.1,
    })

    const result = await validateTask({
      task,
      tier: 'tier1-impossible',
    })

    // Should still validate (garbage in, garbage out)
    expect(result).toBeDefined()
  })
})
