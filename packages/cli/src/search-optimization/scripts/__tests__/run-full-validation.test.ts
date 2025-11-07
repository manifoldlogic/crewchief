/**
 * Tests for Full Validation Run Script
 *
 * These are dry-run tests that validate structure and logic without
 * running expensive API calls.
 */

import { describe, it, expect } from 'vitest'
import type { BenchmarkSuite } from '../../benchmarks/index.js'
import {
  pairedTTest,
  cohensD,
  interpretEffectSize,
  confidenceInterval95,
  performStatisticalAnalysis,
} from '../../reporting/statistics.js'
import { generateValidationReport } from '../../reporting/validation-report.js'
import { estimateCost } from '../run-full-validation.js'
import type { ValidationResults } from '../run-full-validation.js'

// Mock benchmark suite
function createMockSuite(taskCount: number): BenchmarkSuite {
  return {
    name: `Mock Suite (${taskCount} tasks)`,
    version: '1.0.0',
    tier: 1,
    tasks: Array.from({ length: taskCount }, (_, i) => ({
      id: `task-${i}`,
      name: `Task ${i}`,
      description: 'Mock task',
      searchTarget: { type: 'pattern' as const, pattern: /test/ },
      followUpTask: {
        type: 'explanation' as const,
        prompt: 'Explain',
        validator: { type: 'explanation' as const, mentionsPattern: /test/ },
      },
      difficulty: 'medium' as const,
      category: 'test',
      successValidator: () => ({ searchQuality: 0, taskCompletion: 0, efficiency: 0, total: 0, details: '' }),
    })),
    metadata: {
      totalTasks: taskCount,
      categories: ['test'],
      description: 'Mock suite',
    },
  }
}

describe('run-full-validation', () => {
  describe('estimateCost', () => {
    it('estimates cost for 30 tasks', () => {
      const suites = [createMockSuite(10), createMockSuite(10), createMockSuite(10)]
      const cost = estimateCost(suites)

      expect(cost.min).toBeGreaterThan(0)
      expect(cost.max).toBeGreaterThan(cost.min)
      expect(cost.estimate).toBeGreaterThanOrEqual(cost.min)
      expect(cost.estimate).toBeLessThanOrEqual(cost.max)

      // 30 tasks * 2 runs * 10 calls * $0.01 = $6 min
      // 30 tasks * 2 runs * 15 calls * $0.02 = $18 max
      expect(cost.min).toBeCloseTo(6, 0)
      expect(cost.max).toBeCloseTo(18, 0)
      expect(cost.estimate).toBeCloseTo(12, 0)
    })

    it('estimates cost for different suite sizes', () => {
      const smallSuites = [createMockSuite(5), createMockSuite(5), createMockSuite(5)]
      const largeSuites = [createMockSuite(20), createMockSuite(20), createMockSuite(20)]

      const smallCost = estimateCost(smallSuites)
      const largeCost = estimateCost(largeSuites)

      expect(largeCost.estimate).toBeGreaterThan(smallCost.estimate)
      expect(largeCost.estimate / smallCost.estimate).toBeCloseTo(4, 0) // 60 vs 15 tasks
    })
  })
})

describe('statistical-analysis', () => {
  describe('pairedTTest', () => {
    it('calculates t-test for paired samples', () => {
      const grepScores = [0.4, 0.5, 0.3, 0.6, 0.45]
      const searchScores = [0.7, 0.8, 0.65, 0.85, 0.75]

      const result = pairedTTest(grepScores, searchScores)

      expect(result.t).toBeGreaterThan(0) // Search should be better
      expect(result.df).toBe(4) // n - 1
      expect(result.p).toBeGreaterThan(0)
      expect(result.p).toBeLessThanOrEqual(1)
    })

    it('handles equal scores (no difference)', () => {
      const scores = [0.5, 0.6, 0.7, 0.8]

      const result = pairedTTest(scores, scores)

      expect(result.t).toBeCloseTo(0, 1) // Should be near zero
      expect(result.p).toBeGreaterThan(0.5) // Should be non-significant
    })

    it('handles small samples gracefully', () => {
      const grepScores = [0.3]
      const searchScores = [0.8]

      const result = pairedTTest(grepScores, searchScores)

      // Should not crash with n=1
      expect(result.t).toBeDefined()
      expect(result.df).toBe(0)
    })

    it('throws error for mismatched array lengths', () => {
      const grepScores = [0.4, 0.5]
      const searchScores = [0.7, 0.8, 0.9]

      expect(() => pairedTTest(grepScores, searchScores)).toThrow()
    })
  })

  describe('cohensD', () => {
    it('calculates effect size', () => {
      const grepScores = [0.3, 0.4, 0.35, 0.45, 0.38]
      const searchScores = [0.7, 0.8, 0.75, 0.85, 0.78]

      const d = cohensD(grepScores, searchScores)

      expect(d).toBeGreaterThan(0) // Positive effect
      expect(d).toBeGreaterThan(1) // Large effect
    })

    it('returns 0 for identical distributions', () => {
      const scores = [0.5, 0.6, 0.7]

      const d = cohensD(scores, scores)

      expect(d).toBeCloseTo(0, 1)
    })

    it('handles negative effects (search worse than grep)', () => {
      const grepScores = [0.8, 0.9, 0.85]
      const searchScores = [0.4, 0.5, 0.45]

      const d = cohensD(grepScores, searchScores)

      expect(d).toBeLessThan(0) // Negative effect
    })
  })

  describe('interpretEffectSize', () => {
    it('interprets small effect size', () => {
      expect(interpretEffectSize(0.2)).toBe('small')
      expect(interpretEffectSize(0.4)).toBe('small')
    })

    it('interprets medium effect size', () => {
      expect(interpretEffectSize(0.5)).toBe('medium')
      expect(interpretEffectSize(0.7)).toBe('medium')
    })

    it('interprets large effect size', () => {
      expect(interpretEffectSize(0.8)).toBe('large')
      expect(interpretEffectSize(1.1)).toBe('large')
    })

    it('interprets very large effect size', () => {
      expect(interpretEffectSize(1.2)).toBe('very large')
      expect(interpretEffectSize(2.0)).toBe('very large')
    })

    it('handles negative effect sizes', () => {
      expect(interpretEffectSize(-0.9)).toBe('large')
      expect(interpretEffectSize(-1.5)).toBe('very large')
    })
  })

  describe('confidenceInterval95', () => {
    it('calculates 95% confidence interval', () => {
      const differences = [0.3, 0.35, 0.4, 0.3, 0.35]

      const ci = confidenceInterval95(differences)

      expect(ci.mean).toBeCloseTo(0.34, 2)
      expect(ci.lower).toBeLessThan(ci.mean)
      expect(ci.upper).toBeGreaterThan(ci.mean)
      expect(ci.upper - ci.lower).toBeGreaterThan(0) // Non-zero interval
    })

    it('handles zero differences', () => {
      const differences = [0, 0, 0, 0]

      const ci = confidenceInterval95(differences)

      expect(ci.mean).toBe(0)
      expect(ci.lower).toBeLessThanOrEqual(0)
      expect(ci.upper).toBeGreaterThanOrEqual(0)
    })

    it('handles small sample', () => {
      const differences = [0.5]

      const ci = confidenceInterval95(differences)

      // Should not crash with n=1
      expect(ci.mean).toBe(0.5)
      expect(ci.lower).toBe(0)
      expect(ci.upper).toBe(0)
    })
  })

  describe('performStatisticalAnalysis', () => {
    it('performs complete analysis with mock results', () => {
      // Create mock task results with actual data
      const createMockResult = (taskId: string, grepScore: number, searchScore: number) => {
        return {
          grep: {
            task: { id: taskId },
            participants: [{ variantId: 'grep', score: grepScore }],
            winner: { variantId: 'grep', score: grepScore },
          },
          search: {
            task: { id: taskId },
            participants: [{ variantId: 'search', score: searchScore }],
            winner: { variantId: 'search', score: searchScore },
          },
        }
      }

      // Tier 1: Grep struggles, search succeeds
      const tier1Mock = [
        createMockResult('t1-1', 0.3, 0.7),
        createMockResult('t1-2', 0.35, 0.75),
        createMockResult('t1-3', 0.4, 0.8),
      ]

      // Tier 2: Both work, search better
      const tier2Mock = [
        createMockResult('t2-1', 0.5, 0.8),
        createMockResult('t2-2', 0.45, 0.85),
        createMockResult('t2-3', 0.4, 0.75),
      ]

      // Tier 3: Similar performance
      const tier3Mock = [
        createMockResult('t3-1', 0.6, 0.7),
        createMockResult('t3-2', 0.55, 0.72),
        createMockResult('t3-3', 0.5, 0.68),
      ]

      // Create mock condition results
      const grepResults = {
        condition: 'grep-only' as const,
        tier1Results: new Map(tier1Mock.map((m) => [m.grep.task.id, m.grep])),
        tier2Results: new Map(tier2Mock.map((m) => [m.grep.task.id, m.grep])),
        tier3Results: new Map(tier3Mock.map((m) => [m.grep.task.id, m.grep])),
        overallScores: { tier1Avg: 0.35, tier2Avg: 0.45, tier3Avg: 0.55, compositeAvg: 0.45 },
        toolUsageStats: { searchUsageRate: 0, grepUsageRate: 1, appropriateUsage: 0 },
        durationSeconds: 100,
      }

      const searchResults = {
        condition: 'search-available' as const,
        tier1Results: new Map(tier1Mock.map((m) => [m.search.task.id, m.search])),
        tier2Results: new Map(tier2Mock.map((m) => [m.search.task.id, m.search])),
        tier3Results: new Map(tier3Mock.map((m) => [m.search.task.id, m.search])),
        overallScores: { tier1Avg: 0.75, tier2Avg: 0.8, tier3Avg: 0.7, compositeAvg: 0.75 },
        toolUsageStats: { searchUsageRate: 0.7, grepUsageRate: 0.3, appropriateUsage: 0.7 },
        durationSeconds: 95,
      }

      const analysis = performStatisticalAnalysis(grepResults, searchResults)

      expect(analysis.pValue).toBeGreaterThanOrEqual(0)
      expect(analysis.pValue).toBeLessThanOrEqual(1)
      expect(analysis.cohensD).toBeGreaterThan(0) // Search should be better
      expect(analysis.effectSize).toBeDefined()
      expect(analysis.confidenceInterval95.lower).toBeLessThan(analysis.confidenceInterval95.upper)
      expect(analysis.sampleSize).toBeGreaterThanOrEqual(0)
    })
  })
})

describe('validation-report', () => {
  describe('generateValidationReport', () => {
    it('generates markdown report with all sections', () => {
      const mockResults: ValidationResults = {
        timestamp: new Date('2025-01-15T14:30:00Z'),
        grepResults: {
          condition: 'grep-only',
          tier1Results: new Map(),
          tier2Results: new Map(),
          tier3Results: new Map(),
          overallScores: { tier1Avg: 0.25, tier2Avg: 0.45, tier3Avg: 0.6, compositeAvg: 0.43 },
          toolUsageStats: { searchUsageRate: 0, grepUsageRate: 1, appropriateUsage: 0 },
          durationSeconds: 120,
        },
        searchResults: {
          condition: 'search-available',
          tier1Results: new Map(),
          tier2Results: new Map(),
          tier3Results: new Map(),
          overallScores: { tier1Avg: 0.85, tier2Avg: 0.78, tier3Avg: 0.72, compositeAvg: 0.78 },
          toolUsageStats: { searchUsageRate: 0.65, grepUsageRate: 0.35, appropriateUsage: 0.65 },
          durationSeconds: 115,
        },
        statisticalAnalysis: {
          pValue: 0.001,
          tStatistic: 8.5,
          degreesOfFreedom: 29,
          cohensD: 1.45,
          effectSize: 'very large',
          confidenceInterval95: { lower: 0.28, upper: 0.42 },
          meanDifference: 0.35,
          medianDifference: 0.33,
          standardDeviation: 0.12,
          standardError: 0.041,
          grepSuccessRate: 0.43,
          searchSuccessRate: 0.78,
          successRateImprovement: 0.35,
          statisticalPower: 0.98,
          sampleSize: 30,
        },
        summary: {
          totalTasks: 30,
          grepSuccessRate: 0.43,
          searchSuccessRate: 0.78,
          improvement: 0.35,
          statisticallySignificant: true,
        },
        perTierSummary: {
          tier1: { taskCount: 10, grepSuccess: 0.25, searchSuccess: 0.85, improvement: 0.6, pValue: 0.001 },
          tier2: { taskCount: 12, grepSuccess: 0.45, searchSuccess: 0.78, improvement: 0.33, pValue: 0.005 },
          tier3: { taskCount: 8, grepSuccess: 0.6, searchSuccess: 0.72, improvement: 0.12, pValue: 0.08 },
        },
        perCategorySummary: new Map(),
      }

      const report = generateValidationReport(mockResults)

      // Check structure
      expect(report).toContain('# Full Validation Report')
      expect(report).toContain('## Executive Summary')
      expect(report).toContain('## Tier 1: Grep-Impossible Tasks')
      expect(report).toContain('## Tier 2: Grep-Hard Tasks')
      expect(report).toContain('## Tier 3: Real-World Tasks')
      expect(report).toContain('## Statistical Analysis')
      expect(report).toContain('## Tool Selection Analysis')
      expect(report).toContain('## Conclusion')

      // Check metrics
      expect(report).toContain('+35.0%') // Improvement
      expect(report).toContain('p = 0.0010') // p-value
      expect(report).toContain("Cohen's d")
      expect(report).toContain('very large')
      expect(report).toContain('✅ Strong Evidence')
    })

    it('generates warning for non-significant results', () => {
      const mockResults: ValidationResults = {
        timestamp: new Date(),
        grepResults: {
          condition: 'grep-only',
          tier1Results: new Map(),
          tier2Results: new Map(),
          tier3Results: new Map(),
          overallScores: { tier1Avg: 0.5, tier2Avg: 0.5, tier3Avg: 0.5, compositeAvg: 0.5 },
          toolUsageStats: { searchUsageRate: 0, grepUsageRate: 1, appropriateUsage: 0 },
          durationSeconds: 100,
        },
        searchResults: {
          condition: 'search-available',
          tier1Results: new Map(),
          tier2Results: new Map(),
          tier3Results: new Map(),
          overallScores: { tier1Avg: 0.52, tier2Avg: 0.53, tier3Avg: 0.51, compositeAvg: 0.52 },
          toolUsageStats: { searchUsageRate: 0.3, grepUsageRate: 0.7, appropriateUsage: 0.3 },
          durationSeconds: 100,
        },
        statisticalAnalysis: {
          pValue: 0.45, // Not significant
          tStatistic: 0.8,
          degreesOfFreedom: 20,
          cohensD: 0.15,
          effectSize: 'small',
          confidenceInterval95: { lower: -0.05, upper: 0.09 },
          meanDifference: 0.02,
          medianDifference: 0.01,
          standardDeviation: 0.08,
          standardError: 0.025,
          grepSuccessRate: 0.5,
          searchSuccessRate: 0.52,
          successRateImprovement: 0.02,
          statisticalPower: 0.15,
          sampleSize: 21,
        },
        summary: {
          totalTasks: 21,
          grepSuccessRate: 0.5,
          searchSuccessRate: 0.52,
          improvement: 0.02,
          statisticallySignificant: false, // Not significant
        },
        perTierSummary: {
          tier1: { taskCount: 7, grepSuccess: 0.5, searchSuccess: 0.52, improvement: 0.02, pValue: 0.4 },
          tier2: { taskCount: 7, grepSuccess: 0.5, searchSuccess: 0.53, improvement: 0.03, pValue: 0.5 },
          tier3: { taskCount: 7, grepSuccess: 0.5, searchSuccess: 0.51, improvement: 0.01, pValue: 0.8 },
        },
        perCategorySummary: new Map(),
      }

      const report = generateValidationReport(mockResults)

      expect(report).toContain('❌ Insufficient Evidence')
      expect(report).toContain('did not show statistically significant improvements')
    })
  })
})
