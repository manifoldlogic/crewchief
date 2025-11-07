/**
 * Tests for Multi-Tier Scoring Module
 *
 * Validates weighted scoring across 3 benchmark tiers with proper
 * tier-specific metrics and tool selection tracking.
 */

import { describe, it, expect } from 'vitest'
import type { CompetitionResult } from '../competition-runner.js'
import {
  calculateMultiTierScore,
  aggregateMultiTierScores,
  formatMultiTierScore,
  checkMultiTierConvergence,
  DEFAULT_TIER_WEIGHTS,
  type TierSuiteResult,
  type MultiTierScore,
} from '../multi-tier-scoring.js'
import type { SearchTask } from '../types.js'

// Helper to create mock tier suite results
function createMockTierResult(
  tier: 1 | 2 | 3,
  variantScores: Record<string, number>,
  options: {
    searchUsageRate?: number
    appropriateUsage?: number
    taskCompletionRate?: number
  } = {},
): TierSuiteResult {
  const competitionResults = new Map<string, CompetitionResult>()

  // Create mock competition results
  Object.entries(variantScores).forEach(([variantId, score], taskIndex) => {
    const taskId = `tier${tier}-task-${taskIndex}`
    const task: SearchTask = {
      id: taskId,
      name: `Task ${taskIndex}`,
      description: 'Mock task',
      searchTarget: { type: 'pattern', pattern: /test/ },
      followUpTask: {
        type: 'explanation',
        prompt: 'Explain',
        validator: { type: 'explanation', mentionsPattern: /test/ },
      },
      difficulty: 'medium',
      category: 'test',
      successValidator: () => ({ searchQuality: 0, taskCompletion: 0, efficiency: 0, total: 0, details: '' }),
    }

    const result: CompetitionResult = {
      task,
      participants: [
        {
          variantId,
          score,
          searchResults: [],
          workResult: { success: score >= 0.6 },
          toolsUsed: score >= 0.5 ? ['search', 'Read'] : ['Grep', 'Read'],
          searchCount: 1,
          toolCallCount: 2,
          durationSeconds: 10,
        },
      ],
      winner: { variantId, score },
      timestamp: new Date(),
    }

    competitionResults.set(taskId, result)
  })

  const scores = Object.values(variantScores)
  return {
    tier,
    tasks: [],
    competitionResults,
    avgScore: scores.reduce((sum, s) => sum + s, 0) / scores.length,
    searchUsageRate: options.searchUsageRate ?? 0.5,
    appropriateUsage: options.appropriateUsage ?? 0.5,
    taskCompletionRate: options.taskCompletionRate ?? 0.7,
  }
}

describe('Multi-Tier Scoring', () => {
  describe('calculateMultiTierScore', () => {
    it('calculates weighted composite score correctly (40/40/20)', () => {
      const tier1 = createMockTierResult(1, { 'variant-1': 0.9 })
      const tier2 = createMockTierResult(2, { 'variant-1': 0.8 })
      const tier3 = createMockTierResult(3, { 'variant-1': 0.7 })

      const score = calculateMultiTierScore('variant-1', tier1, tier2, tier3, DEFAULT_TIER_WEIGHTS)

      // Expected: 0.9*0.4 + 0.8*0.4 + 0.7*0.2 = 0.36 + 0.32 + 0.14 = 0.82
      expect(score.composite).toBeCloseTo(0.82, 2)
      expect(score.breakdown.tier1Contribution).toBeCloseTo(0.36, 2)
      expect(score.breakdown.tier2Contribution).toBeCloseTo(0.32, 2)
      expect(score.breakdown.tier3Contribution).toBeCloseTo(0.14, 2)
    })

    it('applies custom tier weights correctly', () => {
      const tier1 = createMockTierResult(1, { 'variant-1': 0.9 })
      const tier2 = createMockTierResult(2, { 'variant-1': 0.8 })
      const tier3 = createMockTierResult(3, { 'variant-1': 0.7 })

      // Custom weights: 50/30/20
      const customWeights = { tier1: 0.5, tier2: 0.3, tier3: 0.2 }
      const score = calculateMultiTierScore('variant-1', tier1, tier2, tier3, customWeights)

      // Expected: 0.9*0.5 + 0.8*0.3 + 0.7*0.2 = 0.45 + 0.24 + 0.14 = 0.83
      expect(score.composite).toBeCloseTo(0.83, 2)
    })

    it('handles tier1 metrics correctly (capability)', () => {
      const tier1 = createMockTierResult(1, { 'variant-1': 0.85 }, { searchUsageRate: 0.9 })
      const tier2 = createMockTierResult(2, { 'variant-1': 0.7 })
      const tier3 = createMockTierResult(3, { 'variant-1': 0.6 })

      const score = calculateMultiTierScore('variant-1', tier1, tier2, tier3)

      expect(score.tierMetrics.tier1.avgScore).toBeCloseTo(0.85, 2)
      expect(score.tierMetrics.tier1.searchUsageRate).toBeGreaterThan(0)
      expect(score.tierMetrics.tier1.appropriateUsage).toBeDefined()
      expect(score.tierMetrics.tier1.completeness).toBeDefined()
    })

    it('handles tier2 metrics correctly (efficiency)', () => {
      const tier1 = createMockTierResult(1, { 'variant-1': 0.8 })
      const tier2 = createMockTierResult(2, { 'variant-1': 0.75 })
      const tier3 = createMockTierResult(3, { 'variant-1': 0.6 })

      const score = calculateMultiTierScore('variant-1', tier1, tier2, tier3)

      expect(score.tierMetrics.tier2.avgScore).toBeCloseTo(0.75, 2)
      expect(score.tierMetrics.tier2.searchUsageRate).toBeDefined()
      expect(score.tierMetrics.tier2.efficiencyGain).toBeDefined()
      expect(score.tierMetrics.tier2.precision).toBeDefined()
    })

    it('handles tier3 metrics correctly (utility)', () => {
      const tier1 = createMockTierResult(1, { 'variant-1': 0.8 })
      const tier2 = createMockTierResult(2, { 'variant-1': 0.7 })
      const tier3 = createMockTierResult(3, { 'variant-1': 0.65 })

      const score = calculateMultiTierScore('variant-1', tier1, tier2, tier3)

      expect(score.tierMetrics.tier3.avgScore).toBeCloseTo(0.65, 2)
      expect(score.tierMetrics.tier3.voluntaryAdoptionRate).toBeDefined()
      expect(score.tierMetrics.tier3.naturalBehavior).toBeDefined()
      expect(score.tierMetrics.tier3.taskCompletionRate).toBeDefined()
    })

    it('tracks tool selection across all tiers', () => {
      const tier1 = createMockTierResult(1, { 'variant-1': 0.8 })
      const tier2 = createMockTierResult(2, { 'variant-1': 0.7 })
      const tier3 = createMockTierResult(3, { 'variant-1': 0.6 })

      const score = calculateMultiTierScore('variant-1', tier1, tier2, tier3)

      expect(score.toolSelection.correctSearchUse).toBeGreaterThanOrEqual(0)
      expect(score.toolSelection.correctSearchUse).toBeLessThanOrEqual(1)
      expect(score.toolSelection.correctGrepUse).toBeGreaterThanOrEqual(0)
      expect(score.toolSelection.correctGrepUse).toBeLessThanOrEqual(1)
      expect(score.toolSelection.overallAccuracy).toBeGreaterThanOrEqual(0)
      expect(score.toolSelection.overallAccuracy).toBeLessThanOrEqual(1)
      expect(score.toolSelection.searchUsageRate).toBeGreaterThanOrEqual(0)
      expect(score.toolSelection.searchUsageRate).toBeLessThanOrEqual(1)
    })

    it('handles missing variant gracefully', () => {
      const tier1 = createMockTierResult(1, { 'variant-1': 0.8 })
      const tier2 = createMockTierResult(2, { 'variant-1': 0.7 })
      const tier3 = createMockTierResult(3, { 'variant-1': 0.6 })

      const score = calculateMultiTierScore('nonexistent-variant', tier1, tier2, tier3)

      expect(score.composite).toBe(0)
      expect(score.tierMetrics.tier1.avgScore).toBe(0)
      expect(score.tierMetrics.tier2.avgScore).toBe(0)
      expect(score.tierMetrics.tier3.avgScore).toBe(0)
    })
  })

  describe('aggregateMultiTierScores', () => {
    it('aggregates scores for multiple variants', () => {
      const tier1 = createMockTierResult(1, { 'variant-1': 0.9, 'variant-2': 0.7 })
      const tier2 = createMockTierResult(2, { 'variant-1': 0.8, 'variant-2': 0.6 })
      const tier3 = createMockTierResult(3, { 'variant-1': 0.7, 'variant-2': 0.5 })

      const scores = aggregateMultiTierScores(['variant-1', 'variant-2'], tier1, tier2, tier3)

      expect(scores.size).toBe(2)
      expect(scores.has('variant-1')).toBe(true)
      expect(scores.has('variant-2')).toBe(true)

      const v1Score = scores.get('variant-1')!
      const v2Score = scores.get('variant-2')!

      // Variant 1 should have higher composite score
      expect(v1Score.composite).toBeGreaterThan(v2Score.composite)
    })

    it('handles empty variant list', () => {
      const tier1 = createMockTierResult(1, {})
      const tier2 = createMockTierResult(2, {})
      const tier3 = createMockTierResult(3, {})

      const scores = aggregateMultiTierScores([], tier1, tier2, tier3)

      expect(scores.size).toBe(0)
    })
  })

  describe('formatMultiTierScore', () => {
    it('formats multi-tier score as readable text', () => {
      const tier1 = createMockTierResult(1, { 'variant-1': 0.9 })
      const tier2 = createMockTierResult(2, { 'variant-1': 0.8 })
      const tier3 = createMockTierResult(3, { 'variant-1': 0.7 })

      const score = calculateMultiTierScore('variant-1', tier1, tier2, tier3)
      const formatted = formatMultiTierScore(score)

      expect(formatted).toContain('Composite:')
      expect(formatted).toContain('Tier 1')
      expect(formatted).toContain('Tier 2')
      expect(formatted).toContain('Tier 3')
      expect(formatted).toContain('Tool Selection:')
      expect(formatted).toContain('search use')
      expect(formatted).toContain('grep use')
      expect(formatted).toContain('accuracy')
    })
  })

  describe('checkMultiTierConvergence', () => {
    it('detects convergence when all tiers are stable', () => {
      const current: MultiTierScore = {
        composite: 0.82,
        tierMetrics: {
          tier1: { avgScore: 0.85, searchUsageRate: 0.9, appropriateUsage: 0.9, completeness: 0.8 },
          tier2: { avgScore: 0.8, searchUsageRate: 0.7, efficiencyGain: 0.5, precision: 0.75 },
          tier3: { avgScore: 0.75, voluntaryAdoptionRate: 0.5, naturalBehavior: true, taskCompletionRate: 0.7 },
        },
        toolSelection: { correctSearchUse: 0.8, correctGrepUse: 0.7, overallAccuracy: 0.75, searchUsageRate: 0.7 },
        breakdown: { tier1Contribution: 0.34, tier2Contribution: 0.32, tier3Contribution: 0.15 },
      }

      const previous: MultiTierScore = {
        composite: 0.815,
        tierMetrics: {
          tier1: { avgScore: 0.845, searchUsageRate: 0.88, appropriateUsage: 0.88, completeness: 0.78 },
          tier2: { avgScore: 0.795, searchUsageRate: 0.68, efficiencyGain: 0.48, precision: 0.73 },
          tier3: { avgScore: 0.745, voluntaryAdoptionRate: 0.48, naturalBehavior: true, taskCompletionRate: 0.68 },
        },
        toolSelection: { correctSearchUse: 0.78, correctGrepUse: 0.68, overallAccuracy: 0.73, searchUsageRate: 0.68 },
        breakdown: { tier1Contribution: 0.338, tier2Contribution: 0.318, tier3Contribution: 0.149 },
      }

      const result = checkMultiTierConvergence(current, previous, 0.01)

      expect(result.converged).toBe(true)
      expect(result.tier1Stable).toBe(true)
      expect(result.tier2Stable).toBe(true)
      expect(result.tier3Stable).toBe(true)
    })

    it('detects non-convergence when improvement is large', () => {
      const current: MultiTierScore = {
        composite: 0.85,
        tierMetrics: {
          tier1: { avgScore: 0.9, searchUsageRate: 0.9, appropriateUsage: 0.9, completeness: 0.85 },
          tier2: { avgScore: 0.85, searchUsageRate: 0.75, efficiencyGain: 0.6, precision: 0.8 },
          tier3: { avgScore: 0.75, voluntaryAdoptionRate: 0.5, naturalBehavior: true, taskCompletionRate: 0.7 },
        },
        toolSelection: { correctSearchUse: 0.85, correctGrepUse: 0.75, overallAccuracy: 0.8, searchUsageRate: 0.75 },
        breakdown: { tier1Contribution: 0.36, tier2Contribution: 0.34, tier3Contribution: 0.15 },
      }

      const previous: MultiTierScore = {
        composite: 0.75,
        tierMetrics: {
          tier1: { avgScore: 0.8, searchUsageRate: 0.8, appropriateUsage: 0.8, completeness: 0.75 },
          tier2: { avgScore: 0.75, searchUsageRate: 0.65, efficiencyGain: 0.5, precision: 0.7 },
          tier3: { avgScore: 0.65, voluntaryAdoptionRate: 0.45, naturalBehavior: false, taskCompletionRate: 0.6 },
        },
        toolSelection: { correctSearchUse: 0.75, correctGrepUse: 0.65, overallAccuracy: 0.7, searchUsageRate: 0.65 },
        breakdown: { tier1Contribution: 0.32, tier2Contribution: 0.3, tier3Contribution: 0.13 },
      }

      const result = checkMultiTierConvergence(current, previous, 0.01)

      expect(result.converged).toBe(false)
      expect(result.tier1Improvement).toBeCloseTo(0.1, 2)
      expect(result.tier2Improvement).toBeCloseTo(0.1, 2)
    })

    it('detects degradation in specific tiers', () => {
      const current: MultiTierScore = {
        composite: 0.8,
        tierMetrics: {
          tier1: { avgScore: 0.85, searchUsageRate: 0.9, appropriateUsage: 0.9, completeness: 0.8 },
          tier2: { avgScore: 0.7, searchUsageRate: 0.6, efficiencyGain: 0.4, precision: 0.65 }, // Degraded
          tier3: { avgScore: 0.75, voluntaryAdoptionRate: 0.5, naturalBehavior: true, taskCompletionRate: 0.7 },
        },
        toolSelection: { correctSearchUse: 0.8, correctGrepUse: 0.7, overallAccuracy: 0.75, searchUsageRate: 0.7 },
        breakdown: { tier1Contribution: 0.34, tier2Contribution: 0.28, tier3Contribution: 0.15 },
      }

      const previous: MultiTierScore = {
        composite: 0.82,
        tierMetrics: {
          tier1: { avgScore: 0.85, searchUsageRate: 0.88, appropriateUsage: 0.88, completeness: 0.78 },
          tier2: { avgScore: 0.8, searchUsageRate: 0.7, efficiencyGain: 0.5, precision: 0.75 }, // Was better
          tier3: { avgScore: 0.745, voluntaryAdoptionRate: 0.48, naturalBehavior: true, taskCompletionRate: 0.68 },
        },
        toolSelection: { correctSearchUse: 0.78, correctGrepUse: 0.68, overallAccuracy: 0.73, searchUsageRate: 0.68 },
        breakdown: { tier1Contribution: 0.34, tier2Contribution: 0.32, tier3Contribution: 0.149 },
      }

      const result = checkMultiTierConvergence(current, previous, 0.01)

      // Tier 2 degraded by 0.1, which is significant
      expect(result.tier2Stable).toBe(false)
      expect(result.tier2Improvement).toBeLessThan(0)
      expect(result.converged).toBe(false) // Overall not converged due to degradation
    })

    it('uses custom threshold correctly', () => {
      const current: MultiTierScore = {
        composite: 0.83,
        tierMetrics: {
          tier1: { avgScore: 0.86, searchUsageRate: 0.9, appropriateUsage: 0.9, completeness: 0.8 },
          tier2: { avgScore: 0.81, searchUsageRate: 0.7, efficiencyGain: 0.5, precision: 0.75 },
          tier3: { avgScore: 0.76, voluntaryAdoptionRate: 0.5, naturalBehavior: true, taskCompletionRate: 0.7 },
        },
        toolSelection: { correctSearchUse: 0.8, correctGrepUse: 0.7, overallAccuracy: 0.75, searchUsageRate: 0.7 },
        breakdown: { tier1Contribution: 0.344, tier2Contribution: 0.324, tier3Contribution: 0.152 },
      }

      const previous: MultiTierScore = {
        composite: 0.82,
        tierMetrics: {
          tier1: { avgScore: 0.85, searchUsageRate: 0.88, appropriateUsage: 0.88, completeness: 0.78 },
          tier2: { avgScore: 0.8, searchUsageRate: 0.68, efficiencyGain: 0.48, precision: 0.73 },
          tier3: { avgScore: 0.75, voluntaryAdoptionRate: 0.48, naturalBehavior: true, taskCompletionRate: 0.68 },
        },
        toolSelection: { correctSearchUse: 0.78, correctGrepUse: 0.68, overallAccuracy: 0.73, searchUsageRate: 0.68 },
        breakdown: { tier1Contribution: 0.34, tier2Contribution: 0.32, tier3Contribution: 0.15 },
      }

      // With loose threshold (5%), should converge
      const looseResult = checkMultiTierConvergence(current, previous, 0.05)
      expect(looseResult.converged).toBe(true)

      // With tight threshold (0.1%), should not converge
      const tightResult = checkMultiTierConvergence(current, previous, 0.001)
      expect(tightResult.converged).toBe(false)
    })
  })

  describe('DEFAULT_TIER_WEIGHTS', () => {
    it('has correct default weights (40/40/20)', () => {
      expect(DEFAULT_TIER_WEIGHTS.tier1).toBe(0.4)
      expect(DEFAULT_TIER_WEIGHTS.tier2).toBe(0.4)
      expect(DEFAULT_TIER_WEIGHTS.tier3).toBe(0.2)
    })

    it('weights sum to 1.0', () => {
      const sum = DEFAULT_TIER_WEIGHTS.tier1 + DEFAULT_TIER_WEIGHTS.tier2 + DEFAULT_TIER_WEIGHTS.tier3
      expect(sum).toBeCloseTo(1.0, 10)
    })
  })
})
