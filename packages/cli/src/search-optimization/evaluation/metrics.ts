/**
 * Metrics calculation utilities for comparison framework
 *
 * Calculates advantage metrics that quantify the value provided by
 * semantic search compared to grep-only baseline:
 * - Time saved (in seconds)
 * - Quality improvement (score delta)
 * - Tool selection correctness (was search appropriate?)
 */

import type { BaselineResult } from './baseline-runner.js'
import type { ParticipantResult } from '../competition-runner.js'

/**
 * Advantage metrics comparing search vs grep performance
 */
export interface AdvantageMetrics {
  /** Time saved by using search instead of grep (seconds, negative = slower) */
  timeSaved: number

  /** Quality improvement in score (0-1 range, negative = worse) */
  qualityImprovement: number

  /** Whether search was the correct tool choice for this task */
  toolSelectionCorrect: boolean

  /** Percentage improvement in time (negative = slower) */
  timeImprovementPercent: number

  /** Percentage improvement in quality (negative = worse) */
  qualityImprovementPercent: number

  /** Whether search provided statistically meaningful advantage */
  meaningfulAdvantage: boolean
}

/**
 * Aggregated metrics from multiple runs
 */
export interface AggregatedMetrics {
  /** Average value */
  mean: number

  /** Minimum value observed */
  min: number

  /** Maximum value observed */
  max: number

  /** Standard deviation */
  stdDev: number

  /** Number of observations */
  count: number
}

/**
 * Calculate advantage metrics comparing search condition vs grep baseline
 *
 * @param grepResults - Results from grep-only baseline runs
 * @param searchResults - Results from search-available condition runs
 * @returns Advantage metrics showing search value
 *
 * @example
 * ```typescript
 * const advantage = calculateAdvantage(grepResults, searchResults)
 * console.log(`Time saved: ${advantage.timeSaved}s`)
 * console.log(`Quality improvement: ${(advantage.qualityImprovement * 100).toFixed(1)}%`)
 * console.log(`Tool selection correct: ${advantage.toolSelectionCorrect}`)
 * ```
 */
export function calculateAdvantage(
  grepResults: BaselineResult[],
  searchResults: ParticipantResult[],
): AdvantageMetrics {
  if (grepResults.length === 0) {
    throw new Error('Need at least one grep baseline result')
  }
  if (searchResults.length === 0) {
    throw new Error('Need at least one search condition result')
  }

  // Calculate averages for grep baseline
  const avgGrepTime = grepResults.reduce((sum, r) => sum + r.metrics.durationSeconds, 0) / grepResults.length
  const avgGrepScore = grepResults.reduce((sum, r) => sum + (r.success ? 1.0 : 0.0), 0) / grepResults.length

  // Calculate averages for search condition
  const avgSearchTime =
    searchResults.reduce((sum, r) => sum + r.agentResult.messages.length * 10, 0) / searchResults.length // Rough time estimate
  const avgSearchScore = searchResults.reduce((sum, r) => sum + r.score, 0) / searchResults.length

  // Calculate improvements
  const timeSaved = avgGrepTime - avgSearchTime
  const qualityImprovement = avgSearchScore - avgGrepScore

  // Calculate percentage improvements
  const timeImprovementPercent = avgGrepTime > 0 ? (timeSaved / avgGrepTime) * 100 : 0
  const qualityImprovementPercent = avgGrepScore > 0 ? (qualityImprovement / avgGrepScore) * 100 : 0

  // Tool selection is correct if:
  // 1. Search performed better (quality improvement > 0)
  // 2. Or search achieved similar quality in less time
  const toolSelectionCorrect = qualityImprovement > 0.05 || (Math.abs(qualityImprovement) < 0.05 && timeSaved > 0)

  // Meaningful advantage if:
  // 1. Quality improvement > 10% OR
  // 2. Time saved > 20% with similar quality
  const meaningfulAdvantage =
    qualityImprovement > 0.1 || (Math.abs(qualityImprovement) < 0.05 && timeImprovementPercent > 20)

  return {
    timeSaved,
    qualityImprovement,
    toolSelectionCorrect,
    timeImprovementPercent,
    qualityImprovementPercent,
    meaningfulAdvantage,
  }
}

/**
 * Calculate aggregated metrics from an array of values
 *
 * @param values - Array of numeric values
 * @returns Aggregated metrics (mean, min, max, stdDev)
 *
 * @example
 * ```typescript
 * const scores = [0.7, 0.75, 0.72, 0.78, 0.71]
 * const metrics = aggregateMetrics(scores)
 * console.log(`Mean: ${metrics.mean.toFixed(2)} ± ${metrics.stdDev.toFixed(2)}`)
 * ```
 */
export function aggregateMetrics(values: number[]): AggregatedMetrics {
  if (values.length === 0) {
    throw new Error('Cannot aggregate empty array')
  }

  const mean = values.reduce((sum, val) => sum + val, 0) / values.length
  const min = Math.min(...values)
  const max = Math.max(...values)

  // Calculate standard deviation
  let stdDev = 0
  if (values.length > 1) {
    const squaredDiffs = values.map((val) => Math.pow(val - mean, 2))
    const variance = squaredDiffs.reduce((sum, val) => sum + val, 0) / (values.length - 1)
    stdDev = Math.sqrt(variance)
  }

  return {
    mean,
    min,
    max,
    stdDev,
    count: values.length,
  }
}

/**
 * Calculate search usage rate from search condition results
 *
 * @param searchResults - Results from search-available condition
 * @returns Percentage of tool calls that were semantic search (0-1)
 *
 * @example
 * ```typescript
 * const rate = calculateSearchUsageRate(searchResults)
 * console.log(`Search usage: ${(rate * 100).toFixed(1)}%`)
 * ```
 */
export function calculateSearchUsageRate(searchResults: ParticipantResult[]): number {
  if (searchResults.length === 0) {
    return 0
  }

  // Count search tool calls vs total tool calls
  let totalSearchCalls = 0
  let totalToolCalls = 0

  for (const result of searchResults) {
    const searchCount = result.evaluation.searchMetrics.searchCount
    const totalCalls =
      Object.values(result.evaluation.searchMetrics).reduce((sum, val) => {
        return typeof val === 'number' ? sum + val : sum
      }, 0) || 1

    totalSearchCalls += searchCount
    totalToolCalls += totalCalls
  }

  return totalToolCalls > 0 ? totalSearchCalls / totalToolCalls : 0
}

/**
 * Extract scores from baseline results
 *
 * Converts success boolean to score (1.0 for success, 0.0 for failure)
 * for statistical analysis.
 *
 * @param results - Baseline results
 * @returns Array of scores (0 or 1)
 */
export function extractBaselineScores(results: BaselineResult[]): number[] {
  return results.map((r) => (r.success ? 1.0 : 0.0))
}

/**
 * Extract scores from search condition results
 *
 * @param results - Participant results from competition
 * @returns Array of scores (0-1)
 */
export function extractSearchScores(results: ParticipantResult[]): number[] {
  return results.map((r) => r.score)
}

/**
 * Extract execution times from baseline results
 *
 * @param results - Baseline results
 * @returns Array of execution times in seconds
 */
export function extractBaselineTimes(results: BaselineResult[]): number[] {
  return results.map((r) => r.metrics.durationSeconds)
}

/**
 * Extract execution times from search condition results
 *
 * Estimates time based on message count (rough heuristic: ~10s per turn)
 *
 * @param results - Participant results from competition
 * @returns Array of estimated execution times in seconds
 */
export function extractSearchTimes(results: ParticipantResult[]): number[] {
  return results.map((r) => r.agentResult.messages.length * 10) // Rough estimate
}

/**
 * Format advantage metrics as human-readable text
 *
 * @param metrics - Advantage metrics to format
 * @returns Formatted text description
 */
export function formatAdvantageMetrics(metrics: AdvantageMetrics): string {
  const lines: string[] = []

  lines.push('ADVANTAGE METRICS')
  lines.push('-'.repeat(60))

  // Time savings
  const timeSign = metrics.timeSaved >= 0 ? '+' : ''
  lines.push(
    `Time Saved: ${timeSign}${metrics.timeSaved.toFixed(1)}s (${timeSign}${metrics.timeImprovementPercent.toFixed(1)}%)`,
  )

  // Quality improvement
  const qualitySign = metrics.qualityImprovement >= 0 ? '+' : ''
  lines.push(
    `Quality Improvement: ${qualitySign}${(metrics.qualityImprovement * 100).toFixed(1)}% (${qualitySign}${metrics.qualityImprovementPercent.toFixed(1)}%)`,
  )

  // Tool selection
  lines.push(`Tool Selection: ${metrics.toolSelectionCorrect ? 'CORRECT' : 'INCORRECT'}`)
  lines.push(`Meaningful Advantage: ${metrics.meaningfulAdvantage ? 'YES' : 'NO'}`)

  return lines.join('\n')
}

/**
 * Format aggregated metrics as human-readable text
 *
 * @param label - Label for the metrics
 * @param metrics - Aggregated metrics to format
 * @returns Formatted text description
 */
export function formatAggregatedMetrics(label: string, metrics: AggregatedMetrics): string {
  const lines: string[] = []

  lines.push(`${label}:`)
  lines.push(`  Mean: ${metrics.mean.toFixed(3)} ± ${metrics.stdDev.toFixed(3)}`)
  lines.push(`  Range: [${metrics.min.toFixed(3)}, ${metrics.max.toFixed(3)}]`)
  lines.push(`  N: ${metrics.count}`)

  return lines.join('\n')
}
