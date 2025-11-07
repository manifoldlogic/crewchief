/**
 * Statistical Analysis Module
 *
 * Performs statistical tests to determine if semantic search provides
 * statistically significant improvements over grep-only baseline.
 *
 * Tests performed:
 * - Paired t-test (grep vs search scores)
 * - Confidence intervals (95% CI)
 * - Effect size (Cohen's d)
 * - Success rate comparison
 */

import type { ConditionResults } from '../scripts/run-full-validation.js'

/**
 * Statistical analysis results
 */
export interface StatisticalAnalysis {
  // Overall comparison
  pValue: number
  tStatistic: number
  degreesOfFreedom: number
  cohensD: number
  effectSize: 'small' | 'medium' | 'large' | 'very large'

  // Confidence intervals
  confidenceInterval95: { lower: number; upper: number }
  meanDifference: number
  medianDifference: number
  standardDeviation: number
  standardError: number

  // Success rates
  grepSuccessRate: number
  searchSuccessRate: number
  successRateImprovement: number

  // Per-tier analysis
  tier1PValue?: number
  tier2PValue?: number
  tier3PValue?: number

  // Power analysis
  statisticalPower: number
  sampleSize: number
}

/**
 * Calculate paired t-test for grep vs search scores
 */
export function pairedTTest(
  grepScores: number[],
  searchScores: number[],
): {
  t: number
  df: number
  p: number
} {
  const n = grepScores.length

  if (n !== searchScores.length) {
    throw new Error('Score arrays must have same length')
  }

  if (n < 2) {
    return { t: 0, df: 0, p: 1 }
  }

  // Calculate differences
  const differences = grepScores.map((g, i) => searchScores[i] - g)

  // Mean difference
  const meanDiff = differences.reduce((sum, d) => sum + d, 0) / n

  // Standard deviation of differences
  const variance = differences.reduce((sum, d) => sum + Math.pow(d - meanDiff, 2), 0) / (n - 1)
  const sd = Math.sqrt(variance)

  // Standard error
  const se = sd / Math.sqrt(n)

  // t-statistic (handle division by zero when all values are equal)
  const t = se === 0 ? 0 : meanDiff / se

  // Degrees of freedom
  const df = n - 1

  // p-value (two-tailed)
  // For simplicity, using a rough approximation
  // In production, use a proper t-distribution library
  const p = approximatePValue(Math.abs(t), df)

  return { t, df, p }
}

/**
 * Approximate p-value from t-statistic
 * (Simplified - in production use proper statistical library)
 */
function approximatePValue(t: number, _df: number): number {
  // Very rough approximation for demonstration
  // Real implementation should use a proper t-distribution CDF
  if (t === 0) return 1.0 // No difference = null result
  if (t > 6) return 0.000001
  if (t > 4) return 0.0001
  if (t > 3) return 0.001
  if (t > 2.58) return 0.01
  if (t > 1.96) return 0.05
  if (t > 1.645) return 0.1
  return 0.5
}

/**
 * Calculate Cohen's d effect size
 */
export function cohensD(grepScores: number[], searchScores: number[]): number {
  const n = grepScores.length

  if (n < 2) return 0

  // Calculate means
  const grepMean = grepScores.reduce((sum, s) => sum + s, 0) / n
  const searchMean = searchScores.reduce((sum, s) => sum + s, 0) / n

  // Calculate pooled standard deviation
  const grepVariance = grepScores.reduce((sum, s) => sum + Math.pow(s - grepMean, 2), 0) / (n - 1)
  const searchVariance = searchScores.reduce((sum, s) => sum + Math.pow(s - searchMean, 2), 0) / (n - 1)

  const pooledSD = Math.sqrt((grepVariance + searchVariance) / 2)

  if (pooledSD === 0) return 0

  // Cohen's d
  return (searchMean - grepMean) / pooledSD
}

/**
 * Interpret effect size magnitude
 */
export function interpretEffectSize(d: number): 'small' | 'medium' | 'large' | 'very large' {
  const absD = Math.abs(d)
  if (absD >= 1.2) return 'very large'
  if (absD >= 0.8) return 'large'
  if (absD >= 0.5) return 'medium'
  return 'small'
}

/**
 * Calculate median of a number array
 */
export function median(values: number[]): number {
  if (values.length === 0) return 0

  const sorted = [...values].sort((a, b) => a - b)
  const mid = Math.floor(sorted.length / 2)

  if (sorted.length % 2 === 0) {
    return (sorted[mid - 1] + sorted[mid]) / 2
  }
  return sorted[mid]
}

/**
 * Calculate 95% confidence interval for mean difference
 */
export function confidenceInterval95(differences: number[]): { lower: number; upper: number; mean: number } {
  const n = differences.length

  if (n < 1) {
    return { lower: 0, upper: 0, mean: 0 }
  }

  const mean = differences.reduce((sum, d) => sum + d, 0) / n

  if (n < 2) {
    return { lower: 0, upper: 0, mean }
  }
  const variance = differences.reduce((sum, d) => sum + Math.pow(d - mean, 2), 0) / (n - 1)
  const sd = Math.sqrt(variance)
  const se = sd / Math.sqrt(n)

  // t-critical value for 95% CI (approximation for df > 30: ~1.96)
  // For smaller samples, should use proper t-table
  const tCritical = n > 30 ? 1.96 : 2.0

  const marginOfError = tCritical * se

  return {
    lower: mean - marginOfError,
    upper: mean + marginOfError,
    mean,
  }
}

/**
 * Estimate statistical power
 * (Simplified calculation)
 */
export function estimateStatisticalPower(n: number, effectSize: number, _alpha: number = 0.05): number {
  // Simplified power calculation
  // Real implementation should use proper power analysis library
  if (effectSize < 0.2) return 0.1
  if (effectSize < 0.5 && n < 30) return 0.5
  if (effectSize >= 0.8 && n >= 30) return 0.95
  if (effectSize >= 0.5 && n >= 50) return 0.9
  return 0.8
}

/**
 * Perform complete statistical analysis
 */
export function performStatisticalAnalysis(
  grepResults: ConditionResults,
  searchResults: ConditionResults,
): StatisticalAnalysis {
  // Extract all scores
  const grepScores: number[] = []
  const searchScores: number[] = []

  // Collect Tier 1 scores
  for (const [taskId, grepResult] of grepResults.tier1Results) {
    const searchResult = searchResults.tier1Results.get(taskId)
    if (searchResult) {
      grepScores.push(grepResult.participants[0].score)
      searchScores.push(searchResult.participants[0].score)
    }
  }

  // Collect Tier 2 scores
  for (const [taskId, grepResult] of grepResults.tier2Results) {
    const searchResult = searchResults.tier2Results.get(taskId)
    if (searchResult) {
      grepScores.push(grepResult.participants[0].score)
      searchScores.push(searchResult.participants[0].score)
    }
  }

  // Collect Tier 3 scores
  for (const [taskId, grepResult] of grepResults.tier3Results) {
    const searchResult = searchResults.tier3Results.get(taskId)
    if (searchResult) {
      grepScores.push(grepResult.participants[0].score)
      searchScores.push(searchResult.participants[0].score)
    }
  }

  // Paired t-test
  const tTest = pairedTTest(grepScores, searchScores)

  // Effect size
  const d = cohensD(grepScores, searchScores)
  const effectSize = interpretEffectSize(d)

  // Confidence interval
  const differences = grepScores.map((g, i) => searchScores[i] - g)
  const ci95 = confidenceInterval95(differences)

  // Median and standard deviation
  const medianDiff = median(differences)
  const variance = differences.reduce((sum, d) => sum + Math.pow(d - ci95.mean, 2), 0) / (differences.length - 1)
  const stdDev = Math.sqrt(variance)

  // Success rates
  const grepSuccessRate = grepScores.filter((s) => s >= 0.6).length / grepScores.length
  const searchSuccessRate = searchScores.filter((s) => s >= 0.6).length / searchScores.length

  // Statistical power
  const power = estimateStatisticalPower(grepScores.length, d)

  return {
    pValue: tTest.p,
    tStatistic: tTest.t,
    degreesOfFreedom: tTest.df,
    cohensD: d,
    effectSize,
    confidenceInterval95: {
      lower: ci95.lower,
      upper: ci95.upper,
    },
    meanDifference: ci95.mean,
    medianDifference: medianDiff,
    standardDeviation: stdDev,
    standardError: stdDev / Math.sqrt(differences.length),
    grepSuccessRate,
    searchSuccessRate,
    successRateImprovement: searchSuccessRate - grepSuccessRate,
    statisticalPower: power,
    sampleSize: grepScores.length,
  }
}
