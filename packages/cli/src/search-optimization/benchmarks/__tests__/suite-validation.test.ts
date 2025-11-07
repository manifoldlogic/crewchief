/**
 * Tests for suite validation logic
 */

import { describe, it, expect } from 'vitest'
import type { SearchTask } from '../../types.js'
import { TIER1_GREP_IMPOSSIBLE_SUITE, type BenchmarkSuite } from '../tier1-impossible.js'
import { validateSuiteComposition, formatValidationSummary } from '../validation.js'

describe('validateSuiteComposition', () => {
  describe('Valid Suite (TIER1)', () => {
    const result = validateSuiteComposition(TIER1_GREP_IMPOSSIBLE_SUITE)

    it('should pass validation', () => {
      expect(result.passed).toBe(true)
    })

    it('should have high grep failure rate (>80%)', () => {
      expect(result.grepFailureRate).toBeGreaterThanOrEqual(0.8)
    })

    it('should have 3 categories', () => {
      expect(result.categoryCoverage).toBe(3)
    })

    it('should have tasks defeating grep', () => {
      expect(result.tasksDefeatingGrep.length).toBeGreaterThan(0)
    })

    it('should have no critical errors', () => {
      const errors = result.failingTasks.filter((f) => f.severity === 'error')
      expect(errors.length).toBe(0)
    })

    it('should have minimal or no failing tasks', () => {
      // May have warnings, but not errors
      const errors = result.failingTasks.filter((f) => f.severity === 'error')
      expect(errors.length).toBe(0)
    })

    it('should have no recommendations for a valid suite', () => {
      // A perfectly valid suite should have minimal recommendations
      const criticalRecs = result.recommendations.filter((r) => r.includes('error') || r.includes('Fix'))
      expect(criticalRecs.length).toBe(0)
    })
  })

  describe('Invalid Suite - Missing Fields', () => {
    const invalidTask: SearchTask = {
      id: '',
      name: '',
      description: '',
      category: '',
      difficulty: 'hard',
      searchTarget: { type: 'file' },
      followUpTask: {
        type: 'explanation',
        prompt: '',
        validator: { type: 'explanation' },
      },
      successValidator: () => ({ searchQuality: 0, taskCompletion: 0, efficiency: 0, total: 0, details: '' }),
    }

    const invalidSuite: BenchmarkSuite = {
      name: 'Invalid Suite',
      version: '1.0.0',
      tier: 1,
      tasks: [invalidTask],
      metadata: {
        totalTasks: 1,
        categories: [],
        expectedGrepSuccessRate: 0,
        expectedSearchSuccessRate: 0,
        description: '',
      },
    }

    const result = validateSuiteComposition(invalidSuite)

    it('should fail validation', () => {
      expect(result.passed).toBe(false)
    })

    it('should detect missing fields', () => {
      const errors = result.failingTasks.filter((f) => f.severity === 'error')
      expect(errors.length).toBeGreaterThan(0)
    })

    it('should have error about missing fields', () => {
      const fieldErrors = result.failingTasks.filter((f) => f.reason.includes('Missing required fields'))
      expect(fieldErrors.length).toBeGreaterThan(0)
    })

    it('should have recommendations', () => {
      expect(result.recommendations.length).toBeGreaterThan(0)
    })

    it('should recommend fixing errors', () => {
      const errorRecs = result.recommendations.filter((r) => r.includes('error'))
      expect(errorRecs.length).toBeGreaterThan(0)
    })
  })

  describe('Invalid Suite - Too Easy (grep succeeds)', () => {
    const easyTask: SearchTask = {
      id: 'test-easy',
      name: 'Easy Task',
      description: 'A task that grep can solve',
      category: 'test-category',
      difficulty: 'easy',
      searchTarget: { type: 'file', path: 'test.ts' },
      followUpTask: {
        type: 'explanation',
        prompt: 'Explain',
        validator: { type: 'explanation' },
      },
      successValidator: () => ({ searchQuality: 1, taskCompletion: 1, efficiency: 1, total: 1, details: 'ok' }),
      expectedGrepSuccess: 0.8, // Too high - grep succeeds
      expectedSearchSuccess: 0.85,
    } as any

    const easySuite: BenchmarkSuite = {
      name: 'Easy Suite',
      version: '1.0.0',
      tier: 1,
      tasks: [easyTask, easyTask, easyTask], // 3 easy tasks
      metadata: {
        totalTasks: 3,
        categories: ['test-category'],
        expectedGrepSuccessRate: 0.8,
        expectedSearchSuccessRate: 0.85,
        description: 'Too easy',
      },
    }

    const result = validateSuiteComposition(easySuite)

    it('should fail validation', () => {
      expect(result.passed).toBe(false)
    })

    it('should have low grep failure rate', () => {
      expect(result.grepFailureRate).toBeLessThan(0.8)
    })

    it('should have warnings about grep success being too high', () => {
      const warnings = result.failingTasks.filter((f) => f.reason.includes('too high for grep-impossible'))
      expect(warnings.length).toBeGreaterThan(0)
    })

    it('should recommend more grep-impossible tasks', () => {
      const grepRecs = result.recommendations.filter((r) => r.includes('defeat grep'))
      expect(grepRecs.length).toBeGreaterThan(0)
    })
  })

  describe('Invalid Suite - Insufficient Category Coverage', () => {
    const task1: SearchTask = {
      id: 'test-1',
      name: 'Task 1',
      description: 'Task',
      category: 'single-category',
      difficulty: 'hard',
      searchTarget: { type: 'file' },
      followUpTask: {
        type: 'explanation',
        prompt: 'Explain',
        validator: { type: 'explanation' },
      },
      successValidator: () => ({ searchQuality: 1, taskCompletion: 1, efficiency: 1, total: 1, details: 'ok' }),
      expectedGrepSuccess: 0.2,
      expectedSearchSuccess: 0.8,
    } as any

    const narrowSuite: BenchmarkSuite = {
      name: 'Narrow Suite',
      version: '1.0.0',
      tier: 1,
      tasks: [task1, task1, task1, task1], // All same category
      metadata: {
        totalTasks: 4,
        categories: ['single-category'],
        expectedGrepSuccessRate: 0.2,
        expectedSearchSuccessRate: 0.8,
        description: 'Too narrow',
      },
    }

    const result = validateSuiteComposition(narrowSuite)

    it('should fail validation', () => {
      expect(result.passed).toBe(false)
    })

    it('should have low category coverage', () => {
      expect(result.categoryCoverage).toBeLessThan(3)
    })

    it('should recommend adding more categories', () => {
      const categoryRecs = result.recommendations.filter((r) => r.includes('categories'))
      expect(categoryRecs.length).toBeGreaterThan(0)
    })
  })

  describe('Invalid Suite - Weak Search Advantage', () => {
    const weakTask: SearchTask = {
      id: 'weak-1',
      name: 'Weak Task',
      description: 'Task with weak search advantage',
      category: 'test',
      difficulty: 'hard',
      searchTarget: { type: 'file' },
      followUpTask: {
        type: 'explanation',
        prompt: 'Explain',
        validator: { type: 'explanation' },
      },
      successValidator: () => ({ searchQuality: 1, taskCompletion: 1, efficiency: 1, total: 1, details: 'ok' }),
      expectedGrepSuccess: 0.25,
      expectedSearchSuccess: 0.35, // Only 10% improvement - too weak
    } as any

    const weakSuite: BenchmarkSuite = {
      name: 'Weak Suite',
      version: '1.0.0',
      tier: 1,
      tasks: [weakTask],
      metadata: {
        totalTasks: 1,
        categories: ['test'],
        expectedGrepSuccessRate: 0.25,
        expectedSearchSuccessRate: 0.35,
        description: 'Weak advantage',
      },
    }

    const result = validateSuiteComposition(weakSuite)

    it('should detect weak search advantage', () => {
      const advantageWarnings = result.failingTasks.filter(
        (f) => f.reason.includes('advantage') && f.reason.includes('too small'),
      )
      expect(advantageWarnings.length).toBeGreaterThan(0)
    })

    it('should have warnings (not errors) about search advantage', () => {
      const advantageWarnings = result.failingTasks.filter(
        (f) => f.reason.includes('advantage') && f.severity === 'warning',
      )
      expect(advantageWarnings.length).toBeGreaterThan(0)
    })
  })

  describe('Valid Suite - Borderline', () => {
    // Create a suite that just barely passes validation
    const goodTask: SearchTask = {
      id: 'good-1',
      name: 'Good Task',
      description: 'Good task',
      category: 'category-1',
      difficulty: 'hard',
      searchTarget: { type: 'file' },
      followUpTask: {
        type: 'explanation',
        prompt: 'Explain',
        validator: { type: 'explanation' },
      },
      successValidator: () => ({ searchQuality: 1, taskCompletion: 1, efficiency: 1, total: 1, details: 'ok' }),
      expectedGrepSuccess: 0.25,
      expectedSearchSuccess: 0.75,
    } as any

    const goodTask2 = { ...goodTask, id: 'good-2', category: 'category-2' }
    const goodTask3 = { ...goodTask, id: 'good-3', category: 'category-3' }
    const goodTask4 = { ...goodTask, id: 'good-4', category: 'category-1' }
    const goodTask5 = { ...goodTask, id: 'good-5', category: 'category-2' }

    const borderlineSuite: BenchmarkSuite = {
      name: 'Borderline Suite',
      version: '1.0.0',
      tier: 1,
      tasks: [goodTask, goodTask2, goodTask3, goodTask4, goodTask5], // 5 tasks, 3 categories, 100% grep-impossible
      metadata: {
        totalTasks: 5,
        categories: ['category-1', 'category-2', 'category-3'],
        expectedGrepSuccessRate: 0.25,
        expectedSearchSuccessRate: 0.75,
        description: 'Borderline valid',
      },
    }

    const result = validateSuiteComposition(borderlineSuite)

    it('should pass validation', () => {
      expect(result.passed).toBe(true)
    })

    it('should have 100% grep failure rate', () => {
      expect(result.grepFailureRate).toBe(1.0)
    })

    it('should have exactly 3 categories', () => {
      expect(result.categoryCoverage).toBe(3)
    })

    it('should have no critical errors', () => {
      const errors = result.failingTasks.filter((f) => f.severity === 'error')
      expect(errors.length).toBe(0)
    })
  })
})

describe('formatValidationSummary', () => {
  const result = validateSuiteComposition(TIER1_GREP_IMPOSSIBLE_SUITE)
  const summary = formatValidationSummary(result)

  it('should return a non-empty string', () => {
    expect(summary).toBeTruthy()
    expect(summary.length).toBeGreaterThan(0)
  })

  it('should include suite validation summary header', () => {
    expect(summary).toContain('Suite Validation Summary')
  })

  it('should include status', () => {
    expect(summary).toMatch(/Status:/)
  })

  it('should include grep failure rate', () => {
    expect(summary).toMatch(/Grep Failure Rate:/)
  })

  it('should include category coverage', () => {
    expect(summary).toMatch(/Category Coverage:/)
  })

  it('should include tasks defeating grep count', () => {
    expect(summary).toMatch(/Tasks Defeating Grep:/)
  })

  it('should be well-formatted with sections', () => {
    expect(summary).toContain('===')
    expect(summary.split('\n').length).toBeGreaterThan(5)
  })

  describe('Summary for failing suite', () => {
    const invalidTask: SearchTask = {
      id: '',
      name: '',
      description: '',
      category: '',
      difficulty: 'hard',
      searchTarget: { type: 'file' },
      followUpTask: {
        type: 'explanation',
        prompt: '',
        validator: { type: 'explanation' },
      },
      successValidator: () => ({ searchQuality: 0, taskCompletion: 0, efficiency: 0, total: 0, details: '' }),
    }

    const invalidSuite: BenchmarkSuite = {
      name: 'Invalid',
      version: '1.0.0',
      tier: 1,
      tasks: [invalidTask],
      metadata: {
        totalTasks: 1,
        categories: [],
        expectedGrepSuccessRate: 0,
        expectedSearchSuccessRate: 0,
        description: '',
      },
    }

    const failResult = validateSuiteComposition(invalidSuite)
    const failSummary = formatValidationSummary(failResult)

    it('should show FAILED status', () => {
      expect(failSummary).toContain('FAILED')
    })

    it('should include issues section', () => {
      expect(failSummary).toMatch(/Issues:/)
    })

    it('should include recommendations section', () => {
      expect(failSummary).toMatch(/Recommendations:/)
    })

    it('should list errors', () => {
      expect(failSummary).toMatch(/Errors/)
    })
  })
})
