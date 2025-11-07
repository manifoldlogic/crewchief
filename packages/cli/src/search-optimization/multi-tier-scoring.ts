/**
 * Multi-Tier Scoring Module
 *
 * Implements weighted scoring across 3 benchmark tiers:
 * - Tier 1 (40%): Grep-impossible tasks - prove capability advantage
 * - Tier 2 (40%): Grep-hard tasks - prove efficiency advantage
 * - Tier 3 (20%): Real-world tasks - prove natural utility
 *
 * Integrates with genetic-iterator.ts to optimize descriptions across
 * all tiers simultaneously, ensuring balanced performance.
 */

import type { CompetitionResult } from './competition-runner.js'
import type { SearchTask } from './types.js'

/**
 * Configuration for multi-tier evaluation weights
 */
export interface TierWeights {
  tier1: number // Grep-impossible (default: 0.4)
  tier2: number // Grep-hard (default: 0.4)
  tier3: number // Real-world (default: 0.2)
}

/**
 * Results from running a single tier benchmark suite
 */
export interface TierSuiteResult {
  tier: 1 | 2 | 3
  tasks: SearchTask[]
  competitionResults: Map<string, CompetitionResult> // task.id -> result
  avgScore: number // Average score across all tasks in this tier
  searchUsageRate: number // % of tasks where search was used
  appropriateUsage: number // % where search use was appropriate
  taskCompletionRate: number // % of tasks successfully completed
}

/**
 * Tool selection tracking for a variant across tasks
 */
export interface ToolSelectionMetrics {
  correctSearchUse: number // Used search on grep-impossible/hard tasks
  correctGrepUse: number // Used grep on grep-possible tasks
  overallAccuracy: number // % of correct tool choices
  searchUsageRate: number // Overall % where search was used
}

/**
 * Tier-specific metrics with tier-appropriate measurements
 */
export interface TierMetrics {
  tier1: {
    avgScore: number
    searchUsageRate: number // Should be high (grep fails)
    appropriateUsage: number // % where search was correct choice
    completeness: number // Found all required elements
  }
  tier2: {
    avgScore: number
    searchUsageRate: number // Should be moderate (grep works but slower)
    efficiencyGain: number // Time/tool-call reduction vs grep baseline
    precision: number // Fewer false positives
  }
  tier3: {
    avgScore: number
    voluntaryAdoptionRate: number // % where search used without coercion
    naturalBehavior: boolean // Tool selection matches expectations
    taskCompletionRate: number // % of tasks completed regardless of tool
  }
}

/**
 * Complete multi-tier score for a variant
 */
export interface MultiTierScore {
  composite: number // 0-1, weighted average across tiers
  tierMetrics: TierMetrics
  toolSelection: ToolSelectionMetrics
  breakdown: {
    tier1Contribution: number // Tier 1 score * weight
    tier2Contribution: number // Tier 2 score * weight
    tier3Contribution: number // Tier 3 score * weight
  }
}

/**
 * Results from running all 3 tiers for multiple variants
 */
export interface MultiTierEvaluationResult {
  tier1Results: TierSuiteResult
  tier2Results: TierSuiteResult
  tier3Results: TierSuiteResult
  variantScores: Map<string, MultiTierScore> // variantId -> score
  weights: TierWeights
  timestamp: Date
}

/**
 * Default tier weights (40% T1, 40% T2, 20% T3)
 */
export const DEFAULT_TIER_WEIGHTS: TierWeights = {
  tier1: 0.4,
  tier2: 0.4,
  tier3: 0.2,
}

/**
 * Calculate multi-tier score for a variant across all tiers
 *
 * @param variantId - Variant to score
 * @param tier1Results - Tier 1 suite results
 * @param tier2Results - Tier 2 suite results
 * @param tier3Results - Tier 3 suite results
 * @param weights - Tier weights (defaults to 40/40/20)
 * @returns Complete multi-tier score with breakdown
 */
export function calculateMultiTierScore(
  variantId: string,
  tier1Results: TierSuiteResult,
  tier2Results: TierSuiteResult,
  tier3Results: TierSuiteResult,
  weights: TierWeights = DEFAULT_TIER_WEIGHTS,
): MultiTierScore {
  // Extract variant-specific scores from each tier
  const tier1Score = getVariantScore(variantId, tier1Results)
  const tier2Score = getVariantScore(variantId, tier2Results)
  const tier3Score = getVariantScore(variantId, tier3Results)

  // Calculate weighted contributions
  const tier1Contribution = tier1Score * weights.tier1
  const tier2Contribution = tier2Score * weights.tier2
  const tier3Contribution = tier3Score * weights.tier3

  // Composite score
  const composite = tier1Contribution + tier2Contribution + tier3Contribution

  // Calculate tier-specific metrics
  const tier1Metrics = calculateTier1Metrics(variantId, tier1Results)
  const tier2Metrics = calculateTier2Metrics(variantId, tier2Results)
  const tier3Metrics = calculateTier3Metrics(variantId, tier3Results)

  // Calculate tool selection metrics across all tiers
  const toolSelection = calculateToolSelectionMetrics(variantId, tier1Results, tier2Results, tier3Results)

  return {
    composite,
    tierMetrics: {
      tier1: tier1Metrics,
      tier2: tier2Metrics,
      tier3: tier3Metrics,
    },
    toolSelection,
    breakdown: {
      tier1Contribution,
      tier2Contribution,
      tier3Contribution,
    },
  }
}

/**
 * Extract variant's average score from a tier suite result
 */
function getVariantScore(variantId: string, tierResult: TierSuiteResult): number {
  const scores: number[] = []

  for (const result of tierResult.competitionResults.values()) {
    const participant = result.participants.find((p) => p.variantId === variantId)
    if (participant) {
      scores.push(participant.score)
    }
  }

  if (scores.length === 0) {
    return 0
  }

  return scores.reduce((sum, s) => sum + s, 0) / scores.length
}

/**
 * Calculate Tier 1 specific metrics
 * Tier 1 measures capability - can the agent solve grep-impossible tasks?
 */
function calculateTier1Metrics(variantId: string, tier1Results: TierSuiteResult) {
  const scores: number[] = []
  let searchUsedCount = 0
  let appropriateUsageCount = 0
  let completenessSum = 0

  for (const [_taskId, result] of tier1Results.competitionResults) {
    const participant = result.participants.find((p) => p.variantId === variantId)
    if (!participant) continue

    scores.push(participant.score)

    // For Tier 1, search should be used (grep fails on these tasks)
    const usedSearch = participant.toolsUsed?.includes('search') || false
    if (usedSearch) {
      searchUsedCount++
      appropriateUsageCount++ // Using search is appropriate for Tier 1
    }

    // Completeness: did they find all required elements?
    // Score >= 0.8 indicates high completeness
    if (participant.score >= 0.8) {
      completenessSum += 1
    } else if (participant.score >= 0.5) {
      completenessSum += 0.5
    }
  }

  const totalTasks = scores.length || 1

  return {
    avgScore: scores.reduce((sum, s) => sum + s, 0) / scores.length || 0,
    searchUsageRate: searchUsedCount / totalTasks,
    appropriateUsage: appropriateUsageCount / totalTasks,
    completeness: completenessSum / totalTasks,
  }
}

/**
 * Calculate Tier 2 specific metrics
 * Tier 2 measures efficiency - does search provide advantage over grep?
 */
function calculateTier2Metrics(variantId: string, tier2Results: TierSuiteResult) {
  const scores: number[] = []
  let searchUsedCount = 0
  let efficiencyGainSum = 0
  let precisionSum = 0

  for (const [_taskId, result] of tier2Results.competitionResults) {
    const participant = result.participants.find((p) => p.variantId === variantId)
    if (!participant) continue

    scores.push(participant.score)

    const usedSearch = participant.toolsUsed?.includes('search') || false
    if (usedSearch) {
      searchUsedCount++
    }

    // Efficiency gain: higher score with fewer tool calls = more efficient
    // Normalize based on score and tool count
    const toolCount = participant.toolsUsed?.length || 0
    if (toolCount > 0 && participant.score > 0) {
      // Efficiency = score per tool call (higher is better)
      const efficiency = participant.score / toolCount
      efficiencyGainSum += efficiency
    }

    // Precision: high score indicates fewer false positives
    if (participant.score >= 0.7) {
      precisionSum += 1
    } else if (participant.score >= 0.5) {
      precisionSum += 0.5
    }
  }

  const totalTasks = scores.length || 1

  return {
    avgScore: scores.reduce((sum, s) => sum + s, 0) / scores.length || 0,
    searchUsageRate: searchUsedCount / totalTasks,
    efficiencyGain: efficiencyGainSum / totalTasks,
    precision: precisionSum / totalTasks,
  }
}

/**
 * Calculate Tier 3 specific metrics
 * Tier 3 measures utility - do agents naturally choose appropriate tools?
 */
function calculateTier3Metrics(variantId: string, tier3Results: TierSuiteResult) {
  const scores: number[] = []
  let voluntarySearchCount = 0
  let completedTaskCount = 0

  for (const [_taskId, result] of tier3Results.competitionResults) {
    const participant = result.participants.find((p) => p.variantId === variantId)
    if (!participant) continue

    scores.push(participant.score)

    // Voluntary adoption: search used without coercion (Tier 3 allows both tools)
    const usedSearch = participant.toolsUsed?.includes('search') || false
    if (usedSearch) {
      voluntarySearchCount++
    }

    // Task completion: score >= 0.6 indicates successful completion
    if (participant.score >= 0.6) {
      completedTaskCount++
    }
  }

  const totalTasks = scores.length || 1
  const avgScore = scores.reduce((sum, s) => sum + s, 0) / scores.length || 0

  // Natural behavior: voluntary adoption rate should be moderate (30-70%)
  // Too high = forced, too low = not useful
  const voluntaryRate = voluntarySearchCount / totalTasks
  const naturalBehavior = voluntaryRate >= 0.3 && voluntaryRate <= 0.7

  return {
    avgScore,
    voluntaryAdoptionRate: voluntaryRate,
    naturalBehavior,
    taskCompletionRate: completedTaskCount / totalTasks,
  }
}

/**
 * Calculate tool selection metrics across all tiers
 */
function calculateToolSelectionMetrics(
  variantId: string,
  tier1Results: TierSuiteResult,
  tier2Results: TierSuiteResult,
  tier3Results: TierSuiteResult,
): ToolSelectionMetrics {
  let correctSearchUse = 0
  let correctGrepUse = 0
  let totalDecisions = 0
  let totalSearchUse = 0

  // Tier 1: Search should be used (grep fails)
  for (const result of tier1Results.competitionResults.values()) {
    const participant = result.participants.find((p) => p.variantId === variantId)
    if (!participant) continue

    totalDecisions++
    const usedSearch = participant.toolsUsed?.includes('search') || false
    if (usedSearch) {
      correctSearchUse++
      totalSearchUse++
    }
  }

  // Tier 2: Either tool works, but search is more efficient
  // Consider it correct if search was used OR score is high with grep
  for (const result of tier2Results.competitionResults.values()) {
    const participant = result.participants.find((p) => p.variantId === variantId)
    if (!participant) continue

    totalDecisions++
    const usedSearch = participant.toolsUsed?.includes('search') || false
    const usedGrep = participant.toolsUsed?.includes('grep') || participant.toolsUsed?.includes('Grep') || false

    if (usedSearch) {
      correctSearchUse++
      totalSearchUse++
    } else if (usedGrep && participant.score >= 0.6) {
      correctGrepUse++
    }
  }

  // Tier 3: Either tool is acceptable
  // Measure voluntary search usage
  for (const result of tier3Results.competitionResults.values()) {
    const participant = result.participants.find((p) => p.variantId === variantId)
    if (!participant) continue

    totalDecisions++
    const usedSearch = participant.toolsUsed?.includes('search') || false
    const usedGrep = participant.toolsUsed?.includes('grep') || participant.toolsUsed?.includes('Grep') || false

    if (usedSearch) {
      totalSearchUse++
      // Both are correct for Tier 3
      correctSearchUse++
    } else if (usedGrep) {
      correctGrepUse++
    }
  }

  const overallAccuracy = totalDecisions > 0 ? (correctSearchUse + correctGrepUse) / totalDecisions : 0

  return {
    correctSearchUse: totalDecisions > 0 ? correctSearchUse / totalDecisions : 0,
    correctGrepUse: totalDecisions > 0 ? correctGrepUse / totalDecisions : 0,
    overallAccuracy,
    searchUsageRate: totalDecisions > 0 ? totalSearchUse / totalDecisions : 0,
  }
}

/**
 * Aggregate multi-tier scores across all variants
 *
 * @param variantIds - All variant IDs to score
 * @param tier1Results - Tier 1 suite results
 * @param tier2Results - Tier 2 suite results
 * @param tier3Results - Tier 3 suite results
 * @param weights - Tier weights
 * @returns Map of variant ID to multi-tier score
 */
export function aggregateMultiTierScores(
  variantIds: string[],
  tier1Results: TierSuiteResult,
  tier2Results: TierSuiteResult,
  tier3Results: TierSuiteResult,
  weights: TierWeights = DEFAULT_TIER_WEIGHTS,
): Map<string, MultiTierScore> {
  const scores = new Map<string, MultiTierScore>()

  for (const variantId of variantIds) {
    const score = calculateMultiTierScore(variantId, tier1Results, tier2Results, tier3Results, weights)
    scores.set(variantId, score)
  }

  return scores
}

/**
 * Format multi-tier score for display
 */
export function formatMultiTierScore(score: MultiTierScore): string {
  const lines: string[] = []

  lines.push(`Composite: ${(score.composite * 100).toFixed(1)}%`)
  lines.push(
    `  Tier 1 (40%): ${(score.tierMetrics.tier1.avgScore * 100).toFixed(1)}% (search used: ${(score.tierMetrics.tier1.searchUsageRate * 100).toFixed(0)}%)`,
  )
  lines.push(
    `  Tier 2 (40%): ${(score.tierMetrics.tier2.avgScore * 100).toFixed(1)}% (efficiency: ${(score.tierMetrics.tier2.efficiencyGain * 100).toFixed(0)}%)`,
  )
  lines.push(
    `  Tier 3 (20%): ${(score.tierMetrics.tier3.avgScore * 100).toFixed(1)}% (voluntary: ${(score.tierMetrics.tier3.voluntaryAdoptionRate * 100).toFixed(0)}%)`,
  )
  lines.push('')
  lines.push('Tool Selection:')
  lines.push(`  Appropriate search use: ${(score.toolSelection.correctSearchUse * 100).toFixed(0)}%`)
  lines.push(`  Appropriate grep use: ${(score.toolSelection.correctGrepUse * 100).toFixed(0)}%`)
  lines.push(`  Overall accuracy: ${(score.toolSelection.overallAccuracy * 100).toFixed(0)}%`)

  return lines.join('\n')
}

/**
 * Check if all tiers are converging (stable improvement across tiers)
 *
 * @param currentScores - Current generation scores
 * @param previousScores - Previous generation scores
 * @param threshold - Convergence threshold (default: 0.01 = 1%)
 * @returns True if all tiers show improvement < threshold
 */
export function checkMultiTierConvergence(
  currentScores: MultiTierScore,
  previousScores: MultiTierScore,
  threshold: number = 0.01,
): {
  converged: boolean
  tier1Stable: boolean
  tier2Stable: boolean
  tier3Stable: boolean
  tier1Improvement: number
  tier2Improvement: number
  tier3Improvement: number
} {
  const tier1Improvement = currentScores.tierMetrics.tier1.avgScore - previousScores.tierMetrics.tier1.avgScore
  const tier2Improvement = currentScores.tierMetrics.tier2.avgScore - previousScores.tierMetrics.tier2.avgScore
  const tier3Improvement = currentScores.tierMetrics.tier3.avgScore - previousScores.tierMetrics.tier3.avgScore

  const tier1Stable = Math.abs(tier1Improvement) < threshold
  const tier2Stable = Math.abs(tier2Improvement) < threshold
  const tier3Stable = Math.abs(tier3Improvement) < threshold

  // Converged if composite improvement is small AND no tier is degrading significantly
  const compositeImprovement = currentScores.composite - previousScores.composite
  const noDegradation = tier1Improvement > -threshold && tier2Improvement > -threshold && tier3Improvement > -threshold

  const converged = Math.abs(compositeImprovement) < threshold && noDegradation

  return {
    converged,
    tier1Stable,
    tier2Stable,
    tier3Stable,
    tier1Improvement,
    tier2Improvement,
    tier3Improvement,
  }
}
