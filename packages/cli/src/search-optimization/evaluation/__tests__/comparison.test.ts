/**
 * Integration tests for comparison framework
 *
 * Tests the full comparison flow with mocked baseline and competition runners.
 */

import { describe, it, expect } from 'vitest'
import type { ParticipantResult } from '../../competition-runner.js'
import type { SearchTask } from '../../types.js'
import type { BaselineResult } from '../baseline-runner.js'
import { calculateAdvantage, aggregateMetrics } from '../metrics.js'
import { tTest, cohensD, confidenceInterval, confidenceIntervalDifference } from '../statistics.js'

describe('Comparison Framework Integration', () => {
  // Mock search task
  const mockTask: SearchTask = {
    name: 'Find authentication flow',
    description: 'Trace how users authenticate in the system',
    category: 'relationship-discovery',
    difficulty: 'grep-impossible',
    successCriteria: {
      mustFind: ['AuthService', 'login method'],
      mustNotFind: [],
      filesExamined: { min: 2, max: 10 },
    },
  }

  // Mock baseline results (grep-only)
  const mockGrepResults: BaselineResult[] = [
    {
      task: mockTask,
      success: false,
      metrics: {
        durationSeconds: 150,
        toolCalls: { Grep: 10, Read: 5 },
        searchQueries: ['auth', 'login', 'authenticate'],
        filesExamined: 5,
        timedOut: false,
      },
      agentResult: { success: false, messages: [], sessionId: 'grep-1', transcriptPath: undefined },
      runDir: '/tmp/grep-1',
    },
    {
      task: mockTask,
      success: false,
      metrics: {
        durationSeconds: 180,
        toolCalls: { Grep: 12, Read: 6 },
        searchQueries: ['auth', 'login', 'user'],
        filesExamined: 6,
        timedOut: false,
      },
      agentResult: { success: false, messages: [], sessionId: 'grep-2', transcriptPath: undefined },
      runDir: '/tmp/grep-2',
    },
    {
      task: mockTask,
      success: true,
      metrics: {
        durationSeconds: 200,
        toolCalls: { Grep: 15, Read: 8 },
        searchQueries: ['auth', 'login', 'session'],
        filesExamined: 8,
        timedOut: false,
      },
      agentResult: { success: true, messages: [], sessionId: 'grep-3', transcriptPath: undefined },
      runDir: '/tmp/grep-3',
    },
    {
      task: mockTask,
      success: false,
      metrics: {
        durationSeconds: 160,
        toolCalls: { Grep: 11, Read: 5 },
        searchQueries: ['auth', 'password'],
        filesExamined: 5,
        timedOut: false,
      },
      agentResult: { success: false, messages: [], sessionId: 'grep-4', transcriptPath: undefined },
      runDir: '/tmp/grep-4',
    },
    {
      task: mockTask,
      success: false,
      metrics: {
        durationSeconds: 170,
        toolCalls: { Grep: 13, Read: 7 },
        searchQueries: ['auth', 'token'],
        filesExamined: 7,
        timedOut: false,
      },
      agentResult: { success: false, messages: [], sessionId: 'grep-5', transcriptPath: undefined },
      runDir: '/tmp/grep-5',
    },
  ]

  // Mock search results (semantic search available)
  const mockSearchResults: ParticipantResult[] = [
    {
      variantId: 'search-1',
      variantName: 'Search Enabled',
      score: 0.85,
      evaluation: {
        compositeScore: 0.85,
        taskScore: {
          taskCompletion: 0.9,
          searchQuality: 0.85,
          efficiency: 0.8,
          details: 'Success',
        },
        searchMetrics: {
          searchCount: 3,
          targetFound: true,
          relevanceScore: 0.9,
        },
      },
      agentResult: { success: true, messages: Array(10), sessionId: 'search-1', transcriptPath: undefined },
    },
    {
      variantId: 'search-2',
      variantName: 'Search Enabled',
      score: 0.8,
      evaluation: {
        compositeScore: 0.8,
        taskScore: {
          taskCompletion: 0.85,
          searchQuality: 0.8,
          efficiency: 0.75,
          details: 'Success',
        },
        searchMetrics: {
          searchCount: 4,
          targetFound: true,
          relevanceScore: 0.85,
        },
      },
      agentResult: { success: true, messages: Array(12), sessionId: 'search-2', transcriptPath: undefined },
    },
    {
      variantId: 'search-3',
      variantName: 'Search Enabled',
      score: 0.9,
      evaluation: {
        compositeScore: 0.9,
        taskScore: {
          taskCompletion: 0.95,
          searchQuality: 0.9,
          efficiency: 0.85,
          details: 'Success',
        },
        searchMetrics: {
          searchCount: 2,
          targetFound: true,
          relevanceScore: 0.95,
        },
      },
      agentResult: { success: true, messages: Array(8), sessionId: 'search-3', transcriptPath: undefined },
    },
    {
      variantId: 'search-4',
      variantName: 'Search Enabled',
      score: 0.75,
      evaluation: {
        compositeScore: 0.75,
        taskScore: {
          taskCompletion: 0.8,
          searchQuality: 0.75,
          efficiency: 0.7,
          details: 'Success',
        },
        searchMetrics: {
          searchCount: 5,
          targetFound: true,
          relevanceScore: 0.8,
        },
      },
      agentResult: { success: true, messages: Array(14), sessionId: 'search-4', transcriptPath: undefined },
    },
    {
      variantId: 'search-5',
      variantName: 'Search Enabled',
      score: 0.82,
      evaluation: {
        compositeScore: 0.82,
        taskScore: {
          taskCompletion: 0.87,
          searchQuality: 0.82,
          efficiency: 0.77,
          details: 'Success',
        },
        searchMetrics: {
          searchCount: 3,
          targetFound: true,
          relevanceScore: 0.87,
        },
      },
      agentResult: { success: true, messages: Array(11), sessionId: 'search-5', transcriptPath: undefined },
    },
  ]

  describe('calculateAdvantage', () => {
    it('calculates advantage metrics correctly', () => {
      const advantage = calculateAdvantage(mockGrepResults, mockSearchResults)

      // Grep avg: 1/5 = 0.2, Search avg: ~0.824
      expect(advantage.qualityImprovement).toBeGreaterThan(0.5)

      // Search should be faster (grep avg: 172s, search avg: ~110s)
      expect(advantage.timeSaved).toBeGreaterThan(0)

      // Tool selection should be correct (search performed better)
      expect(advantage.toolSelectionCorrect).toBe(true)

      // Should have meaningful advantage
      expect(advantage.meaningfulAdvantage).toBe(true)
    })

    it('calculates percentage improvements', () => {
      const advantage = calculateAdvantage(mockGrepResults, mockSearchResults)

      expect(advantage.qualityImprovementPercent).toBeGreaterThan(100) // >100% improvement
      expect(advantage.timeImprovementPercent).toBeGreaterThan(20) // >20% time savings
    })
  })

  describe('aggregateMetrics', () => {
    it('calculates aggregated metrics for grep scores', () => {
      const scores = mockGrepResults.map((r) => (r.success ? 1.0 : 0.0))
      const metrics = aggregateMetrics(scores)

      expect(metrics.mean).toBeCloseTo(0.2, 2) // 1 success out of 5
      expect(metrics.min).toBe(0)
      expect(metrics.max).toBe(1)
      expect(metrics.count).toBe(5)
      expect(metrics.stdDev).toBeGreaterThan(0)
    })

    it('calculates aggregated metrics for search scores', () => {
      const scores = mockSearchResults.map((r) => r.score)
      const metrics = aggregateMetrics(scores)

      expect(metrics.mean).toBeCloseTo(0.824, 2)
      expect(metrics.min).toBe(0.75)
      expect(metrics.max).toBe(0.9)
      expect(metrics.count).toBe(5)
      expect(metrics.stdDev).toBeGreaterThan(0)
    })
  })

  describe('statistical significance', () => {
    it('detects significant difference with t-test', () => {
      const grepScores = mockGrepResults.map((r) => (r.success ? 1.0 : 0.0))
      const searchScores = mockSearchResults.map((r) => r.score)

      const result = tTest(searchScores, grepScores)

      expect(result.significant).toBe(true)
      expect(result.pValue).toBeLessThan(0.05)
      expect(result.mean2).toBeCloseTo(0.2, 2) // Grep mean
      expect(result.mean1).toBeCloseTo(0.824, 2) // Search mean
    })

    it('calculates large effect size', () => {
      const grepScores = mockGrepResults.map((r) => (r.success ? 1.0 : 0.0))
      const searchScores = mockSearchResults.map((r) => r.score)

      const result = cohensD(searchScores, grepScores)

      expect(result.cohensD).toBeGreaterThan(1.0) // Large effect
      expect(['large', 'very large']).toContain(result.interpretation)
    })

    it('calculates confidence intervals', () => {
      const searchScores = mockSearchResults.map((r) => r.score)

      const ci = confidenceInterval(searchScores)

      expect(ci.mean).toBeCloseTo(0.824, 2)
      expect(ci.lower).toBeLessThan(ci.mean)
      expect(ci.upper).toBeGreaterThan(ci.mean)
      expect(ci.confidenceLevel).toBe(0.95)
    })

    it('calculates confidence interval for difference', () => {
      const grepScores = mockGrepResults.map((r) => (r.success ? 1.0 : 0.0))
      const searchScores = mockSearchResults.map((r) => r.score)

      const ci = confidenceIntervalDifference(searchScores, grepScores)

      expect(ci.mean).toBeGreaterThan(0.5) // Large positive difference
      expect(ci.lower).toBeGreaterThan(0) // CI excludes 0 (significant)
      expect(ci.upper).toBeGreaterThan(ci.lower)
    })
  })

  describe('full comparison workflow', () => {
    it('performs complete statistical analysis', () => {
      const grepScores = mockGrepResults.map((r) => (r.success ? 1.0 : 0.0))
      const searchScores = mockSearchResults.map((r) => r.score)

      // Calculate advantage
      const advantage = calculateAdvantage(mockGrepResults, mockSearchResults)

      // Perform statistical tests
      const tTestResult = tTest(searchScores, grepScores)
      const effectSize = cohensD(searchScores, grepScores)
      const ciDiff = confidenceIntervalDifference(searchScores, grepScores)

      // Verify all components work together
      expect(advantage.qualityImprovement).toBeGreaterThan(0)
      expect(advantage.toolSelectionCorrect).toBe(true)
      expect(tTestResult.significant).toBe(true)
      expect(effectSize.cohensD).toBeGreaterThan(1.0)
      expect(ciDiff.lower).toBeGreaterThan(0)

      // All indicators should agree: search provides significant advantage
      const allIndicatorsAgree =
        advantage.meaningfulAdvantage && tTestResult.significant && effectSize.cohensD > 0.5 && ciDiff.lower > 0

      expect(allIndicatorsAgree).toBe(true)
    })

    it('aggregates metrics correctly across conditions', () => {
      const grepScores = mockGrepResults.map((r) => (r.success ? 1.0 : 0.0))
      const searchScores = mockSearchResults.map((r) => r.score)

      const grepMetrics = aggregateMetrics(grepScores)
      const searchMetrics = aggregateMetrics(searchScores)

      // Search should have higher mean
      expect(searchMetrics.mean).toBeGreaterThan(grepMetrics.mean)

      // Search should have higher min
      expect(searchMetrics.min).toBeGreaterThan(grepMetrics.min)

      // Both should have valid ranges
      expect(grepMetrics.max).toBeGreaterThanOrEqual(grepMetrics.min)
      expect(searchMetrics.max).toBeGreaterThanOrEqual(searchMetrics.min)
    })
  })

  describe('edge cases', () => {
    it('handles all grep failures', () => {
      const allFailureGrep = mockGrepResults.map((r) => ({
        ...r,
        success: false,
      }))

      const advantage = calculateAdvantage(allFailureGrep, mockSearchResults)

      expect(advantage.qualityImprovement).toBeGreaterThan(0.7) // Search much better
      expect(advantage.toolSelectionCorrect).toBe(true)
    })

    it('handles all search successes', () => {
      const allSuccessSearch = mockSearchResults.map((r) => ({
        ...r,
        score: 1.0,
      }))

      const advantage = calculateAdvantage(mockGrepResults, allSuccessSearch)

      expect(advantage.qualityImprovement).toBeGreaterThan(0.5)
      expect(advantage.toolSelectionCorrect).toBe(true)
    })

    it('handles equal performance', () => {
      const equalGrep: BaselineResult[] = Array(5)
        .fill(null)
        .map((_, i) => ({
          task: mockTask,
          success: true,
          metrics: {
            durationSeconds: 100,
            toolCalls: { Grep: 5 },
            searchQueries: ['query'],
            filesExamined: 3,
            timedOut: false,
          },
          agentResult: { success: true, messages: [], sessionId: `grep-${i}`, transcriptPath: undefined },
          runDir: `/tmp/grep-${i}`,
        }))

      const equalSearch: ParticipantResult[] = Array(5)
        .fill(null)
        .map((_, i) => ({
          variantId: `search-${i}`,
          variantName: 'Search Enabled',
          score: 1.0,
          evaluation: {
            compositeScore: 1.0,
            taskScore: { taskCompletion: 1.0, searchQuality: 1.0, efficiency: 1.0, details: 'Success' },
            searchMetrics: { searchCount: 2, targetFound: true, relevanceScore: 1.0 },
          },
          agentResult: { success: true, messages: Array(10), sessionId: `search-${i}`, transcriptPath: undefined },
        }))

      const advantage = calculateAdvantage(equalGrep, equalSearch)
      const grepScores = equalGrep.map(() => 1.0)
      const searchScores = equalSearch.map((r) => r.score)
      const tTestResult = tTest(searchScores, grepScores)

      // No significant difference expected
      expect(advantage.qualityImprovement).toBeCloseTo(0, 1)
      expect(tTestResult.significant).toBe(false)
    })

    it('handles minimum iterations (n=2)', () => {
      const minGrep = mockGrepResults.slice(0, 2)
      const minSearch = mockSearchResults.slice(0, 2)

      // Should not throw
      expect(() => calculateAdvantage(minGrep, minSearch)).not.toThrow()

      const grepScores = minGrep.map((r) => (r.success ? 1.0 : 0.0))
      const searchScores = minSearch.map((r) => r.score)

      expect(() => tTest(searchScores, grepScores)).not.toThrow()
      expect(() => cohensD(searchScores, grepScores)).not.toThrow()
    })

    it('rejects insufficient data', () => {
      const singleGrep = mockGrepResults.slice(0, 1)
      const singleSearch = mockSearchResults.slice(0, 1)

      // Should throw on statistical tests
      const grepScores = singleGrep.map((r) => (r.success ? 1.0 : 0.0))
      const searchScores = singleSearch.map((r) => r.score)

      expect(() => tTest(searchScores, grepScores)).toThrow()
      expect(() => cohensD(searchScores, grepScores)).toThrow()
    })
  })

  describe('realistic scenarios', () => {
    it('handles grep-impossible task (search advantage)', () => {
      // Grep fails most of the time
      const hardGrepResults = mockGrepResults.map((r, i) => ({
        ...r,
        success: i === 4, // Only 1/5 success
      }))

      // Search succeeds most of the time
      const goodSearchResults = mockSearchResults.map((r) => ({
        ...r,
        score: 0.8 + Math.random() * 0.15, // 80-95%
      }))

      const advantage = calculateAdvantage(hardGrepResults, goodSearchResults)

      expect(advantage.qualityImprovement).toBeGreaterThan(0.5)
      expect(advantage.toolSelectionCorrect).toBe(true)
      expect(advantage.meaningfulAdvantage).toBe(true)
    })

    it('handles grep-possible task (no clear advantage)', () => {
      // Both succeed similarly
      const easyGrepResults = mockGrepResults.map((r) => ({
        ...r,
        success: true,
      }))

      const easySearchResults = mockSearchResults.map((r) => ({
        ...r,
        score: 0.95 + Math.random() * 0.05, // 95-100%
      }))

      const advantage = calculateAdvantage(easyGrepResults, easySearchResults)

      // Small improvement
      expect(advantage.qualityImprovement).toBeLessThan(0.2)
    })

    it('handles high-variance LLM results', () => {
      // Search has high variance
      const variantSearchResults = [
        { ...mockSearchResults[0], score: 0.9 },
        { ...mockSearchResults[1], score: 0.3 },
        { ...mockSearchResults[2], score: 0.8 },
        { ...mockSearchResults[3], score: 0.4 },
        { ...mockSearchResults[4], score: 0.85 },
      ]

      const advantage = calculateAdvantage(mockGrepResults, variantSearchResults)
      const searchScores = variantSearchResults.map((r) => r.score)
      const searchMetrics = aggregateMetrics(searchScores)

      // High std dev indicates variance
      expect(searchMetrics.stdDev).toBeGreaterThan(0.2)

      // But we can still calculate advantage
      expect(advantage.qualityImprovement).toBeDefined()
    })
  })
})
