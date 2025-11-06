/**
 * Statistical Analysis Framework
 *
 * Provides rigorous statistical validation for variant comparison:
 * - Winner detection with statistical significance
 * - Effect size analysis
 * - Confidence intervals
 * - Mutation recommendations
 */

import type { VariantMetrics } from './metrics.js'
import {
  welchTTest,
  cohensD,
  confidenceInterval,
  bonferroniCorrection,
  requiredSampleSize,
  estimatePower,
  mean,
  stdDev,
  type TTestResult,
  type ConfidenceInterval
} from './statistics.js'

/**
 * Analysis result for an experiment
 */
export interface AnalysisResult {
  experiment_id: string
  winner: string | null
  statistical_significance: boolean
  p_value: number
  effect_size: number
  confidence_interval: ConfidenceInterval
  recommendation: string
  variants: VariantResult[]
  warnings: string[]
}

/**
 * Result for a single variant
 */
export interface VariantResult {
  name: string
  variant_id: string
  success_rate: number
  n_trials: number
  mean_result: number
  std_dev: number
  vs_baseline: {
    delta: number
    p_value: number
    effect_size: number
    significant: boolean
  } | null
}

/**
 * Configuration for analyzer
 */
export interface AnalyzerConfig {
  alpha: number // Significance level (default: 0.05)
  minPracticalDelta: number // Minimum practical improvement (default: 0.05 = 5%)
  minSampleSize: number // Minimum sample size (default: 100)
  desiredPower: number // Desired statistical power (default: 0.8)
}

const DEFAULT_CONFIG: AnalyzerConfig = {
  alpha: 0.05,
  minPracticalDelta: 0.05,
  minSampleSize: 100,
  desiredPower: 0.8
}

/**
 * Analyze experiment results and detect winner
 */
export function analyzeExperiment(
  allMetrics: VariantMetrics[],
  experimentId: string = `exp-${Date.now()}`,
  config: Partial<AnalyzerConfig> = {}
): AnalysisResult {
  const cfg: AnalyzerConfig = { ...DEFAULT_CONFIG, ...config }
  const warnings: string[] = []

  // Validate inputs
  if (allMetrics.length < 2) {
    throw new Error('Need at least 2 variants to compare')
  }

  // Check sample sizes
  for (const metrics of allMetrics) {
    if (metrics.total_queries < cfg.minSampleSize) {
      warnings.push(
        `Variant ${metrics.variant_name} has only ${metrics.total_queries} samples (minimum: ${cfg.minSampleSize})`
      )
    }
  }

  // Sort by success rate (descending)
  const sorted = [...allMetrics].sort((a, b) => b.success_rate - a.success_rate)

  // Use best variant as baseline for comparison
  const baseline = sorted[0]

  // Apply Bonferroni correction for multiple comparisons
  const numComparisons = allMetrics.length - 1 // Compare all others to baseline
  const adjustedAlpha = bonferroniCorrection(cfg.alpha, numComparisons)

  // Compute variant results
  const variantResults: VariantResult[] = allMetrics.map(metrics => {
    const isBaseline = metrics.variant_id === baseline.variant_id

    // Extract success/failure data for statistical tests
    const successData = extractSuccessData(metrics)
    const baselineData = extractSuccessData(baseline)

    let vsBaseline: VariantResult['vs_baseline'] = null

    if (!isBaseline) {
      // Perform Welch's t-test
      const tTest = welchTTest(successData, baselineData)

      // Calculate effect size
      const effectSize = cohensD(successData, baselineData)

      // Determine if statistically significant
      const significant = tTest.p_value < adjustedAlpha

      vsBaseline = {
        delta: metrics.success_rate - baseline.success_rate,
        p_value: tTest.p_value,
        effect_size: effectSize,
        significant
      }
    }

    return {
      name: metrics.variant_name,
      variant_id: metrics.variant_id,
      success_rate: metrics.success_rate,
      n_trials: metrics.total_queries,
      mean_result: metrics.avg_result_count,
      std_dev: stdDev(metrics.query_results.map(r => r.result_count)),
      vs_baseline: vsBaseline
    }
  })

  // Determine winner
  const { winner, winnerResult } = detectWinner(variantResults, baseline.variant_id, cfg, adjustedAlpha)

  // Calculate overall statistics for winner
  const winnerMetrics = allMetrics.find(m => m.variant_id === winner)
  const baselineMetrics = baseline

  let overallPValue = 1.0
  let overallEffectSize = 0.0
  let overallCI: ConfidenceInterval = { lower: 0, upper: 0, confidence: 0.95 }

  if (winner && winner !== baseline.variant_id && winnerMetrics) {
    const winnerData = extractSuccessData(winnerMetrics)
    const baselineData = extractSuccessData(baselineMetrics)

    const tTest = welchTTest(winnerData, baselineData)
    overallPValue = tTest.p_value
    overallEffectSize = cohensD(winnerData, baselineData)

    // Calculate CI for the difference in success rates
    const diffData = winnerData.map((v, i) => v - baselineData[i])
    overallCI = confidenceInterval(diffData, 0.95)
  }

  // Generate recommendation
  const recommendation = generateRecommendation(winner, winnerResult, variantResults, cfg)

  return {
    experiment_id: experimentId,
    winner,
    statistical_significance: !!winner && winner !== baseline.variant_id,
    p_value: overallPValue,
    effect_size: overallEffectSize,
    confidence_interval: overallCI,
    recommendation,
    variants: variantResults,
    warnings
  }
}

/**
 * Extract success/failure data from metrics for statistical tests
 *
 * Converts query results to binary success indicators (1 or 0)
 */
function extractSuccessData(metrics: VariantMetrics): number[] {
  return metrics.query_results.map(r => (r.success ? 1 : 0))
}

/**
 * Detect winner based on statistical and practical significance
 */
function detectWinner(
  variants: VariantResult[],
  baselineId: string,
  config: AnalyzerConfig,
  adjustedAlpha: number
): { winner: string | null; winnerResult: VariantResult | null } {
  // Sort by success rate
  const sorted = [...variants].sort((a, b) => b.success_rate - a.success_rate)
  const best = sorted[0]

  // If the best is the baseline, no clear winner
  if (best.variant_id === baselineId) {
    return { winner: baselineId, winnerResult: best }
  }

  // Check winner criteria:
  // 1. Statistically significant (p < adjusted α)
  // 2. Practically significant (improvement > minPracticalDelta)
  // 3. No degradation in simple queries (checked elsewhere)

  if (!best.vs_baseline) {
    return { winner: baselineId, winnerResult: best }
  }

  const isStatSig = best.vs_baseline.p_value < adjustedAlpha
  const isPractSig = best.vs_baseline.delta > config.minPracticalDelta

  if (isStatSig && isPractSig) {
    return { winner: best.variant_id, winnerResult: best }
  }

  // No clear winner - baseline remains
  return { winner: null, winnerResult: null }
}

/**
 * Generate mutation recommendations based on analysis results
 */
function generateRecommendation(
  winner: string | null,
  winnerResult: VariantResult | null,
  variants: VariantResult[],
  config: AnalyzerConfig
): string {
  const recommendations: string[] = []

  if (winner && winnerResult) {
    // Winner found - explore similar space
    recommendations.push(`✅ Winner detected: ${winnerResult.name}`)
    recommendations.push(
      `   Success rate: ${(winnerResult.success_rate * 100).toFixed(1)}% (${winnerResult.n_trials} trials)`
    )

    if (winnerResult.vs_baseline) {
      recommendations.push(
        `   Improvement: +${(winnerResult.vs_baseline.delta * 100).toFixed(1)}% (p=${winnerResult.vs_baseline.p_value.toFixed(4)}, d=${winnerResult.vs_baseline.effect_size.toFixed(2)})`
      )
    }

    recommendations.push('')
    recommendations.push('📋 Recommended next mutations:')
    recommendations.push('   1. Crossover: Combine winner with 2nd-best variant')
    recommendations.push('   2. Amplification: Add more detail/examples to winner')
    recommendations.push('   3. Specialization: Focus winner on high-performing query types')
  } else if (variants.length >= 2) {
    // No winner - try crossover between top variants
    const sorted = [...variants].sort((a, b) => b.success_rate - a.success_rate)
    recommendations.push('⚠️  No statistically significant winner detected')
    recommendations.push('')
    recommendations.push('📋 Recommended next mutations:')
    recommendations.push(
      `   1. Crossover: Combine top 2 variants (${sorted[0].name} + ${sorted[1].name})`
    )
    recommendations.push('   2. Radical mutation: Try significantly different approach')
    recommendations.push('   3. Increase sample size: Re-run with more queries for higher power')
  } else {
    // All variants performed poorly
    const avgSuccessRate = variants.reduce((sum, v) => sum + v.success_rate, 0) / variants.length

    if (avgSuccessRate < 0.5) {
      recommendations.push('❌ All variants underperformed (<50% success rate)')
      recommendations.push('')
      recommendations.push('📋 Recommended next mutations:')
      recommendations.push('   1. Radical rewrite: Start with completely different approach')
      recommendations.push('   2. Review test queries: Ensure queries are appropriate')
      recommendations.push('   3. Analyze failures: Identify patterns in failed queries')
    } else {
      recommendations.push('⚠️  No clear winner, but reasonable performance')
      recommendations.push('')
      recommendations.push('📋 Recommended next mutations:')
      recommendations.push('   1. Refinement: Small targeted improvements')
      recommendations.push('   2. Increase sample size: More data for clearer signal')
    }
  }

  return recommendations.join('\n')
}

/**
 * Format analysis result as human-readable report
 */
export function generateReport(result: AnalysisResult): string {
  const lines: string[] = []

  lines.push('=' .repeat(70))
  lines.push('STATISTICAL ANALYSIS REPORT')
  lines.push('='.repeat(70))
  lines.push('')

  lines.push(`Experiment ID: ${result.experiment_id}`)
  lines.push('')

  // Warnings
  if (result.warnings.length > 0) {
    lines.push('⚠️  WARNINGS:')
    result.warnings.forEach(w => lines.push(`   ${w}`))
    lines.push('')
  }

  // Overall result
  lines.push('OVERALL RESULT:')
  if (result.winner) {
    lines.push(`   Winner: ${result.variants.find(v => v.variant_id === result.winner)?.name || result.winner}`)
    lines.push(`   Statistical Significance: ${result.statistical_significance ? 'YES' : 'NO'}`)
    lines.push(`   P-value: ${result.p_value.toFixed(4)}`)
    lines.push(`   Effect Size (Cohen's d): ${result.effect_size.toFixed(3)}`)
    lines.push(
      `   95% CI for difference: [${result.confidence_interval.lower.toFixed(3)}, ${result.confidence_interval.upper.toFixed(3)}]`
    )
  } else {
    lines.push('   Winner: None (no statistically significant improvement)')
  }
  lines.push('')

  // Variant details
  lines.push('VARIANT DETAILS:')
  lines.push(
    '   Rank | Variant                    | Success Rate | N    | Mean Results | Std Dev | vs Baseline'
  )
  lines.push('   ' + '-'.repeat(100))

  const sorted = [...result.variants].sort((a, b) => b.success_rate - a.success_rate)

  sorted.forEach((v, idx) => {
    const rank = (idx + 1).toString().padStart(4)
    const name = v.name.substring(0, 25).padEnd(25)
    const successRate = `${(v.success_rate * 100).toFixed(1)}%`.padStart(12)
    const nTrials = v.n_trials.toString().padStart(4)
    const meanResult = v.mean_result.toFixed(2).padStart(12)
    const stdDev = v.std_dev.toFixed(2).padStart(7)

    let vsBaseline = '-'
    if (v.vs_baseline) {
      const delta = `${v.vs_baseline.delta > 0 ? '+' : ''}${(v.vs_baseline.delta * 100).toFixed(1)}%`
      const sig = v.vs_baseline.significant ? '✓' : '✗'
      vsBaseline = `${delta} (p=${v.vs_baseline.p_value.toFixed(3)}) ${sig}`
    }

    lines.push(
      `   ${rank} | ${name} | ${successRate} | ${nTrials} | ${meanResult} | ${stdDev} | ${vsBaseline}`
    )
  })

  lines.push('')

  // Recommendations
  lines.push(result.recommendation)

  lines.push('')
  lines.push('='.repeat(70))

  return lines.join('\n')
}
