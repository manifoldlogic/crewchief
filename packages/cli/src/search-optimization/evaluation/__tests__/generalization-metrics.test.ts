/**
 * Tests for cross-codebase generalization metrics
 *
 * TESTDES-5003: Tests for new metrics functions added to metrics.ts
 */

import { describe, it, expect } from 'vitest'
import {
  calculateCrossCodebaseMetrics,
  calculateAdvantageConsistency,
  formatGeneralizationMetrics,
} from '../metrics.js'

describe('Generalization Metrics', () => {
  describe('calculateCrossCodebaseMetrics', () => {
    it('should calculate metrics from task results map', () => {
      const taskResults = new Map([
        ['task-1', [0.8, 0.75, 0.85]], // Success rates across 3 codebases
        ['task-2', [0.9, 0.88, 0.92]],
      ])

      const metrics = calculateCrossCodebaseMetrics(taskResults)

      expect(metrics.taskSuccessRate.mean).toBeCloseTo(0.85, 2)
      expect(metrics.taskSuccessRate.count).toBe(6) // 2 tasks × 3 codebases
      expect(metrics.advantageConsistency.isConsistent).toBeDefined()
      expect(metrics.transferability).toHaveLength(2)
    })

    it('should calculate transferability scores correctly', () => {
      const taskResults = new Map([
        ['high-task', [0.9, 0.85, 0.88]], // All > 0.7
        ['medium-task', [0.75, 0.5, 0.8]], // 2/3 > 0.7
        ['low-task', [0.3, 0.4, 0.35]], // 0/3 > 0.7
      ])

      const metrics = calculateCrossCodebaseMetrics(taskResults)

      const highTask = metrics.transferability.find((t) => t.taskId === 'high-task')
      const mediumTask = metrics.transferability.find((t) => t.taskId === 'medium-task')
      const lowTask = metrics.transferability.find((t) => t.taskId === 'low-task')

      expect(highTask?.score).toBeCloseTo(1.0, 1) // 3/3 = 1.0
      expect(mediumTask?.score).toBeCloseTo(0.67, 1) // 2/3 ≈ 0.67
      expect(lowTask?.score).toBe(0) // 0/3 = 0
    })

    it('should detect consistency in results', () => {
      const consistentResults = new Map([
        ['task-1', [0.8, 0.82, 0.81]], // Low variance
        ['task-2', [0.75, 0.76, 0.74]],
      ])

      const inconsistentResults = new Map([
        ['task-1', [0.3, 0.9, 0.5]], // High variance
        ['task-2', [0.2, 0.8, 0.4]],
      ])

      const consistentMetrics = calculateCrossCodebaseMetrics(consistentResults)
      const inconsistentMetrics = calculateCrossCodebaseMetrics(inconsistentResults)

      expect(consistentMetrics.advantageConsistency.variance).toBeLessThan(
        inconsistentMetrics.advantageConsistency.variance,
      )
    })

    it('should handle single codebase', () => {
      const taskResults = new Map([
        ['task-1', [0.8]],
        ['task-2', [0.9]],
      ])

      const metrics = calculateCrossCodebaseMetrics(taskResults)

      expect(metrics.taskSuccessRate.mean).toBeCloseTo(0.85, 2)
      // With two tasks, there is some variance
      expect(metrics.taskSuccessRate.stdDev).toBeGreaterThanOrEqual(0)
    })

    it('should throw error for empty results', () => {
      const taskResults = new Map()

      expect(() => calculateCrossCodebaseMetrics(taskResults)).toThrow('Need at least one task')
    })
  })

  describe('calculateAdvantageConsistency', () => {
    it('should calculate search advantage gap across codebases', () => {
      const grepResults = new Map([
        ['codebase1', 0.2],
        ['codebase2', 0.25],
        ['codebase3', 0.22],
      ])

      const searchResults = new Map([
        ['codebase1', 0.8],
        ['codebase2', 0.75],
        ['codebase3', 0.82],
      ])

      const consistency = calculateAdvantageConsistency(grepResults, searchResults)

      expect(consistency.meanGap).toBeCloseTo(0.57, 2) // ~0.6, ~0.5, ~0.6
      expect(consistency.variance).toBeLessThan(0.01) // Very consistent
      expect(consistency.isConsistent).toBe(true)
      expect(consistency.perCodebase).toHaveLength(3)
    })

    it('should detect inconsistent advantage', () => {
      const grepResults = new Map([
        ['codebase1', 0.2],
        ['codebase2', 0.3],
        ['codebase3', 0.25],
      ])

      const searchResults = new Map([
        ['codebase1', 0.9], // Large gap
        ['codebase2', 0.4], // Small gap
        ['codebase3', 0.85], // Large gap
      ])

      const consistency = calculateAdvantageConsistency(grepResults, searchResults)

      expect(consistency.variance).toBeGreaterThan(0.05) // High variance
      expect(consistency.isConsistent).toBe(false)
    })

    it('should handle perfect consistency', () => {
      const grepResults = new Map([
        ['codebase1', 0.2],
        ['codebase2', 0.2],
        ['codebase3', 0.2],
      ])

      const searchResults = new Map([
        ['codebase1', 0.8],
        ['codebase2', 0.8],
        ['codebase3', 0.8],
      ])

      const consistency = calculateAdvantageConsistency(grepResults, searchResults)

      expect(consistency.meanGap).toBeCloseTo(0.6, 5)
      expect(consistency.variance).toBeCloseTo(0, 10) // Perfect consistency
      expect(consistency.isConsistent).toBe(true)
    })

    it('should return per-codebase breakdown', () => {
      const grepResults = new Map([
        ['codebase1', 0.2],
        ['codebase2', 0.3],
      ])

      const searchResults = new Map([
        ['codebase1', 0.8],
        ['codebase2', 0.7],
      ])

      const consistency = calculateAdvantageConsistency(grepResults, searchResults)

      expect(consistency.perCodebase).toHaveLength(2)
      expect(consistency.perCodebase[0].codebase).toBe('codebase1')
      expect(consistency.perCodebase[0].gap).toBeCloseTo(0.6, 5)
      expect(consistency.perCodebase[1].codebase).toBe('codebase2')
      expect(consistency.perCodebase[1].gap).toBeCloseTo(0.4, 5)
    })

    it('should handle missing codebases', () => {
      const grepResults = new Map([
        ['codebase1', 0.2],
        ['codebase2', 0.25],
      ])

      const searchResults = new Map([
        ['codebase1', 0.8],
        // codebase2 missing
        ['codebase3', 0.85], // Extra codebase
      ])

      const consistency = calculateAdvantageConsistency(grepResults, searchResults)

      // Should only calculate for matching codebases
      expect(consistency.perCodebase).toHaveLength(1)
      expect(consistency.perCodebase[0].codebase).toBe('codebase1')
    })

    it('should throw error when no matching codebases', () => {
      const grepResults = new Map([['codebase1', 0.2]])
      const searchResults = new Map([['codebase2', 0.8]])

      expect(() => calculateAdvantageConsistency(grepResults, searchResults)).toThrow('No matching codebase results')
    })
  })

  describe('formatGeneralizationMetrics', () => {
    it('should format metrics as readable text', () => {
      const taskResults = new Map([
        ['universal-task', [0.9, 0.85, 0.88]], // High transferability
        ['partial-task', [0.75, 0.5, 0.8]], // Medium transferability
        ['limited-task', [0.3, 0.4, 0.35]], // Low transferability
      ])

      const metrics = calculateCrossCodebaseMetrics(taskResults)
      const formatted = formatGeneralizationMetrics(metrics)

      expect(formatted).toContain('GENERALIZATION METRICS')
      expect(formatted).toContain('Task Success Rate Across Codebases')
      expect(formatted).toContain('Advantage Consistency')
      expect(formatted).toContain('Task Transferability')
      expect(formatted).toContain('Universal (≥80%)')
      expect(formatted).toContain('Partial (40-80%)')
      expect(formatted).toContain('Limited (<40%)')
    })

    it('should categorize tasks by transferability', () => {
      const taskResults = new Map([
        ['high-1', [0.9, 0.88, 0.92]], // 100% transferability (3/3 > 0.7)
        ['high-2', [0.85, 0.87, 0.86]], // 100% transferability (3/3 > 0.7)
        ['medium-1', [0.72, 0.75, 0.65]], // 67% transferability (2/3 > 0.7) - partial
        ['low-1', [0.3, 0.35, 0.32]], // 0% transferability (0/3 > 0.7)
      ])

      const metrics = calculateCrossCodebaseMetrics(taskResults)
      const formatted = formatGeneralizationMetrics(metrics)

      // Should show count of tasks in each category
      expect(formatted).toMatch(/Universal.*2 tasks/)
      expect(formatted).toMatch(/Partial.*1 task/)
      expect(formatted).toMatch(/Limited.*1 task/)
    })

    it('should include success/total counts', () => {
      const taskResults = new Map([['task-1', [0.9, 0.85, 0.88]]])

      const metrics = calculateCrossCodebaseMetrics(taskResults)
      const formatted = formatGeneralizationMetrics(metrics)

      // Should show success count out of total
      expect(formatted).toMatch(/task-1.*\(3\/3\)/)
    })
  })
})
