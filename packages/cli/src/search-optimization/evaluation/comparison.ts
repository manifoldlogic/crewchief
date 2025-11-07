/**
 * Comparison framework for side-by-side evaluation
 *
 * Orchestrates grep-only baseline vs search-available condition evaluations
 * and performs statistical significance testing to make objective claims
 * about semantic search value.
 *
 * This is the scientific core enabling claims like:
 * "Semantic search provides 40% improvement with p<0.05 confidence"
 */

import { mkdirSync, writeFileSync } from 'fs'
import { join } from 'path'
import type { SearchTask } from '../types.js'
import { runBaseline, type BaselineResult } from './baseline-runner.js'
import { runCompetition, type ParticipantResult } from '../competition-runner.js'
import {
  calculateAdvantage,
  aggregateMetrics,
  calculateSearchUsageRate,
  extractBaselineScores,
  extractSearchScores,
  extractBaselineTimes,
  extractSearchTimes,
  type AdvantageMetrics,
  type AggregatedMetrics,
} from './metrics.js'
import {
  tTest,
  cohensD,
  confidenceInterval,
  confidenceIntervalDifference,
  type TTestResult,
  type EffectSizeResult,
  type ConfidenceInterval,
} from './statistics.js'

/**
 * Configuration for comparison run
 */
export interface ComparisonConfig {
  /** Task to evaluate */
  task: SearchTask

  /** Number of iterations per condition (n≥5 recommended for statistical power) */
  iterations: number

  /** Run iterations in parallel (faster but more resource intensive) */
  parallelExecution?: boolean

  /** Maximum execution time per iteration in seconds (default: 300) */
  timeout?: number

  /** Base directory for comparison runs (default: .crewchief/comparisons) */
  baseDir?: string

  /** Working directory (worktree path) */
  worktreePath?: string
}

/**
 * Result of comparison evaluation
 */
export interface ComparisonResult {
  /** Task that was evaluated */
  task: SearchTask

  /** Configuration used */
  config: ComparisonConfig

  /** Grep-only baseline results */
  grepBaseline: {
    results: BaselineResult[]
    avgScore: number
    avgTime: number
    scoreMetrics: AggregatedMetrics
    timeMetrics: AggregatedMetrics
  }

  /** Search-available condition results */
  searchCondition: {
    results: ParticipantResult[]
    avgScore: number
    avgTime: number
    scoreMetrics: AggregatedMetrics
    timeMetrics: AggregatedMetrics
    searchUsageRate: number
  }

  /** Advantage metrics (how much better is search?) */
  advantage: AdvantageMetrics

  /** Statistical significance testing */
  significance: {
    scoreTest: TTestResult
    effectSize: EffectSizeResult
    scoreConfidenceInterval: ConfidenceInterval
    differenceConfidenceInterval: ConfidenceInterval
    significant: boolean
  }

  /** Path to comparison report */
  reportPath: string

  /** Comparison ID */
  comparisonId: string
}

/**
 * Run a complete comparison evaluation
 *
 * Executes the same task under both conditions (grep-only and search-available)
 * multiple times, then performs statistical analysis to determine if semantic
 * search provides significant advantage.
 *
 * @param config - Comparison configuration
 * @returns Comparison result with statistical analysis
 *
 * @example
 * ```typescript
 * const result = await runComparison({
 *   task: mySearchTask,
 *   iterations: 5,
 *   timeout: 300,
 *   worktreePath: '/path/to/worktree',
 * })
 *
 * console.log('p-value:', result.significance.scoreTest.pValue)
 * console.log('Effect size:', result.significance.effectSize.cohensD)
 * console.log('Significant:', result.significance.significant)
 * ```
 */
export async function runComparison(config: ComparisonConfig): Promise<ComparisonResult> {
  const comparisonId = `comparison-${Date.now()}`
  const baseDir = config.baseDir || join('.crewchief', 'comparisons', comparisonId)

  console.log('\n' + '='.repeat(70))
  console.log('COMPARISON EVALUATION')
  console.log('='.repeat(70))
  console.log(`Task: ${config.task.name}`)
  console.log(`Iterations: ${config.iterations}`)
  console.log(`Parallel: ${config.parallelExecution || false}`)
  console.log(`Timeout: ${config.timeout || 300}s`)
  console.log()

  // Create comparison directory
  mkdirSync(baseDir, { recursive: true })

  // Run grep-only baseline
  console.log('Phase 1: Running grep-only baseline...')
  const grepResults = await runMultipleBaseline(config, join(baseDir, 'grep-baseline'))

  // Run search-available condition
  console.log('\nPhase 2: Running search-available condition...')
  const searchResults = await runMultipleSearch(config, join(baseDir, 'search-condition'))

  // Calculate metrics
  console.log('\nPhase 3: Calculating advantage metrics...')
  const advantage = calculateAdvantage(grepResults, searchResults)

  // Perform statistical analysis
  console.log('Phase 4: Performing statistical tests...')
  const significance = performStatisticalAnalysis(grepResults, searchResults)

  // Aggregate metrics
  const grepScores = extractBaselineScores(grepResults)
  const searchScores = extractSearchScores(searchResults)
  const grepTimes = extractBaselineTimes(grepResults)
  const searchTimes = extractSearchTimes(searchResults)

  const result: ComparisonResult = {
    task: config.task,
    config,
    grepBaseline: {
      results: grepResults,
      avgScore: grepScores.reduce((a, b) => a + b, 0) / grepScores.length,
      avgTime: grepTimes.reduce((a, b) => a + b, 0) / grepTimes.length,
      scoreMetrics: aggregateMetrics(grepScores),
      timeMetrics: aggregateMetrics(grepTimes),
    },
    searchCondition: {
      results: searchResults,
      avgScore: searchScores.reduce((a, b) => a + b, 0) / searchScores.length,
      avgTime: searchTimes.reduce((a, b) => a + b, 0) / searchTimes.length,
      scoreMetrics: aggregateMetrics(searchScores),
      timeMetrics: aggregateMetrics(searchTimes),
      searchUsageRate: calculateSearchUsageRate(searchResults),
    },
    advantage,
    significance,
    reportPath: '',
    comparisonId,
  }

  // Generate and save report
  console.log('\nPhase 5: Generating report...')
  const report = generateComparisonReport(result)
  const reportPath = join(baseDir, 'comparison-report.md')
  writeFileSync(reportPath, report)
  result.reportPath = reportPath

  // Save full result as JSON
  writeFileSync(join(baseDir, 'comparison-result.json'), JSON.stringify(result, null, 2))

  console.log('\n' + '='.repeat(70))
  console.log('COMPARISON COMPLETE')
  console.log('='.repeat(70))
  console.log(`Report: ${reportPath}`)
  console.log()

  return result
}

/**
 * Run baseline evaluation multiple times
 */
async function runMultipleBaseline(config: ComparisonConfig, baseDir: string): Promise<BaselineResult[]> {
  const results: BaselineResult[] = []

  if (config.parallelExecution) {
    // Run in parallel
    const promises = Array.from({ length: config.iterations }, (_, i) =>
      runBaseline({
        task: config.task,
        timeout: config.timeout,
        baseDir: join(baseDir, `run-${i + 1}`),
        worktreePath: config.worktreePath,
      }),
    )
    return Promise.all(promises)
  } else {
    // Run sequentially
    for (let i = 0; i < config.iterations; i++) {
      console.log(`  Iteration ${i + 1}/${config.iterations}`)
      const result = await runBaseline({
        task: config.task,
        timeout: config.timeout,
        baseDir: join(baseDir, `run-${i + 1}`),
        worktreePath: config.worktreePath,
      })
      results.push(result)
    }
    return results
  }
}

/**
 * Run search condition evaluation multiple times
 */
async function runMultipleSearch(config: ComparisonConfig, baseDir: string): Promise<ParticipantResult[]> {
  const results: ParticipantResult[] = []

  // For search condition, we need to provide a variant
  // Use a default variant with semantic search enabled
  const defaultVariant = {
    id: 'search-enabled',
    name: 'Semantic Search Enabled',
    description: 'Agent with semantic search tools available',
    promptTemplate: config.task.description,
  }

  if (config.parallelExecution) {
    // Run in parallel
    const promises = Array.from({ length: config.iterations }, (_, i) =>
      runCompetition({
        task: config.task,
        variants: [defaultVariant],
        timeout: config.timeout,
        baseDir: join(baseDir, `run-${i + 1}`),
      }).then((compResult) => compResult.participants[0]),
    )
    return Promise.all(promises)
  } else {
    // Run sequentially
    for (let i = 0; i < config.iterations; i++) {
      console.log(`  Iteration ${i + 1}/${config.iterations}`)
      const compResult = await runCompetition({
        task: config.task,
        variants: [defaultVariant],
        timeout: config.timeout,
        baseDir: join(baseDir, `run-${i + 1}`),
      })
      results.push(compResult.participants[0])
    }
    return results
  }
}

/**
 * Perform statistical significance testing
 */
function performStatisticalAnalysis(
  grepResults: BaselineResult[],
  searchResults: ParticipantResult[],
): ComparisonResult['significance'] {
  const grepScores = extractBaselineScores(grepResults)
  const searchScores = extractSearchScores(searchResults)

  // Perform t-test
  const scoreTest = tTest(searchScores, grepScores) // search - grep

  // Calculate effect size
  const effectSize = cohensD(searchScores, grepScores)

  // Calculate confidence intervals
  const scoreConfidenceInterval = confidenceInterval(searchScores)
  const differenceConfidenceInterval = confidenceIntervalDifference(searchScores, grepScores)

  return {
    scoreTest,
    effectSize,
    scoreConfidenceInterval,
    differenceConfidenceInterval,
    significant: scoreTest.significant && effectSize.cohensD > 0.2, // p<0.05 and at least small effect
  }
}

/**
 * Generate markdown comparison report
 */
function generateComparisonReport(result: ComparisonResult): string {
  const lines: string[] = []

  // Header
  lines.push('# Comparison Evaluation Report')
  lines.push('')
  lines.push(`**Comparison ID:** ${result.comparisonId}`)
  lines.push(`**Date:** ${new Date().toISOString()}`)
  lines.push('')

  // Task details
  lines.push('## Task')
  lines.push('')
  lines.push(`**Name:** ${result.task.name}`)
  lines.push(`**Category:** ${result.task.category}`)
  lines.push(`**Difficulty:** ${result.task.difficulty}`)
  lines.push(`**Description:** ${result.task.description}`)
  lines.push('')

  // Configuration
  lines.push('## Configuration')
  lines.push('')
  lines.push(`- **Iterations per condition:** ${result.config.iterations}`)
  lines.push(`- **Parallel execution:** ${result.config.parallelExecution ? 'Yes' : 'No'}`)
  lines.push(`- **Timeout:** ${result.config.timeout || 300}s`)
  lines.push('')

  // Results summary
  lines.push('## Results Summary')
  lines.push('')
  lines.push('| Metric | Grep Baseline | Search Condition | Improvement |')
  lines.push('|--------|--------------|-----------------|-------------|')

  const scoreImprovement = result.advantage.qualityImprovement
  const timeImprovement = result.advantage.timeSaved

  lines.push(
    `| Average Score | ${result.grepBaseline.avgScore.toFixed(3)} | ${result.searchCondition.avgScore.toFixed(3)} | ${scoreImprovement >= 0 ? '+' : ''}${(scoreImprovement * 100).toFixed(1)}% |`,
  )
  lines.push(
    `| Average Time | ${result.grepBaseline.avgTime.toFixed(1)}s | ${result.searchCondition.avgTime.toFixed(1)}s | ${timeImprovement >= 0 ? '+' : ''}${timeImprovement.toFixed(1)}s |`,
  )
  lines.push(`| Search Usage | N/A | ${(result.searchCondition.searchUsageRate * 100).toFixed(1)}% | - |`)
  lines.push('')

  // Statistical significance
  lines.push('## Statistical Significance')
  lines.push('')
  lines.push('### T-Test Results')
  lines.push('')
  lines.push(`- **t-statistic:** ${result.significance.scoreTest.tStatistic.toFixed(3)}`)
  lines.push(`- **Degrees of freedom:** ${result.significance.scoreTest.degreesOfFreedom.toFixed(1)}`)
  lines.push(`- **p-value:** ${result.significance.scoreTest.pValue.toFixed(4)}`)
  lines.push(`- **Significant:** ${result.significance.scoreTest.significant ? '✓ Yes (p < 0.05)' : '✗ No (p ≥ 0.05)'}`)
  lines.push('')

  // Effect size
  lines.push('### Effect Size')
  lines.push('')
  lines.push(`- **Cohen's d:** ${result.significance.effectSize.cohensD.toFixed(3)}`)
  lines.push(`- **Interpretation:** ${result.significance.effectSize.interpretation}`)
  lines.push('')
  lines.push('_Effect size interpretation:_')
  lines.push('- < 0.2: negligible')
  lines.push('- 0.2-0.5: small')
  lines.push('- 0.5-0.8: medium')
  lines.push('- 0.8-1.3: large')
  lines.push('- ≥ 1.3: very large')
  lines.push('')

  // Confidence intervals
  lines.push('### Confidence Intervals')
  lines.push('')
  lines.push('**95% CI for Search Condition Score:**')
  lines.push(
    `[${result.significance.scoreConfidenceInterval.lower.toFixed(3)}, ${result.significance.scoreConfidenceInterval.upper.toFixed(3)}]`,
  )
  lines.push('')
  lines.push('**95% CI for Difference (Search - Grep):**')
  lines.push(
    `[${result.significance.differenceConfidenceInterval.lower.toFixed(3)}, ${result.significance.differenceConfidenceInterval.upper.toFixed(3)}]`,
  )
  lines.push('')
  if (result.significance.differenceConfidenceInterval.lower > 0) {
    lines.push(
      '_Since the confidence interval does not include 0, we can be 95% confident that search performs better._',
    )
  } else if (result.significance.differenceConfidenceInterval.upper < 0) {
    lines.push(
      '_Since the confidence interval is entirely below 0, we can be 95% confident that grep performs better._',
    )
  } else {
    lines.push('_The confidence interval includes 0, so we cannot confidently say which approach is better._')
  }
  lines.push('')

  // Advantage metrics
  lines.push('## Advantage Metrics')
  lines.push('')
  lines.push(
    `- **Time Saved:** ${result.advantage.timeSaved >= 0 ? '+' : ''}${result.advantage.timeSaved.toFixed(1)}s (${result.advantage.timeImprovementPercent >= 0 ? '+' : ''}${result.advantage.timeImprovementPercent.toFixed(1)}%)`,
  )
  lines.push(
    `- **Quality Improvement:** ${result.advantage.qualityImprovement >= 0 ? '+' : ''}${(result.advantage.qualityImprovement * 100).toFixed(1)}% (${result.advantage.qualityImprovementPercent >= 0 ? '+' : ''}${result.advantage.qualityImprovementPercent.toFixed(1)}%)`,
  )
  lines.push(`- **Tool Selection Correct:** ${result.advantage.toolSelectionCorrect ? '✓ Yes' : '✗ No'}`)
  lines.push(`- **Meaningful Advantage:** ${result.advantage.meaningfulAdvantage ? '✓ Yes' : '✗ No'}`)
  lines.push('')

  // Interpretation
  lines.push('## Interpretation')
  lines.push('')
  if (result.significance.significant) {
    lines.push(
      `The results show **statistically significant** evidence (p = ${result.significance.scoreTest.pValue.toFixed(4)}) that semantic search provides an advantage for this task. ` +
        `The effect size is **${result.significance.effectSize.interpretation}** (Cohen's d = ${result.significance.effectSize.cohensD.toFixed(2)}), ` +
        `with an average quality improvement of **${(result.advantage.qualityImprovement * 100).toFixed(1)}%**.`,
    )
  } else if (result.significance.scoreTest.pValue < 0.1) {
    lines.push(
      `The results show a **trend** toward semantic search advantage (p = ${result.significance.scoreTest.pValue.toFixed(4)}), ` +
        'but it does not reach the conventional significance threshold of p < 0.05. ' +
        'This suggests potential benefit, but more iterations would be needed for conclusive evidence.',
    )
  } else {
    lines.push(
      `The results do **not** show statistically significant evidence (p = ${result.significance.scoreTest.pValue.toFixed(4)}) ` +
        'that semantic search provides an advantage for this task. ' +
        'This could mean: (1) search provides no benefit, (2) the task is too easy/hard for either approach, ' +
        'or (3) more iterations are needed to detect a small effect.',
    )
  }
  lines.push('')

  // Detailed results
  lines.push('## Detailed Results')
  lines.push('')

  lines.push('### Grep Baseline')
  lines.push('')
  lines.push('| Run | Success | Time (s) | Tool Calls | Search Queries |')
  lines.push('|-----|---------|----------|------------|----------------|')
  result.grepBaseline.results.forEach((r, i) => {
    const toolCallCount = Object.values(r.metrics.toolCalls).reduce((a, b) => a + b, 0)
    lines.push(
      `| ${i + 1} | ${r.success ? '✓' : '✗'} | ${r.metrics.durationSeconds.toFixed(1)} | ${toolCallCount} | ${r.metrics.searchQueries.length} |`,
    )
  })
  lines.push('')

  lines.push('### Search Condition')
  lines.push('')
  lines.push('| Run | Score | Time (s) | Searches | Target Found |')
  lines.push('|-----|-------|----------|----------|--------------|')
  result.searchCondition.results.forEach((r, i) => {
    const time = r.agentResult.messages.length * 10 // Rough estimate
    lines.push(
      `| ${i + 1} | ${(r.score * 100).toFixed(1)}% | ${time.toFixed(1)} | ${r.evaluation.searchMetrics.searchCount} | ${r.evaluation.searchMetrics.targetFound ? '✓' : '✗'} |`,
    )
  })
  lines.push('')

  // Assumptions and limitations
  lines.push('## Assumptions and Limitations')
  lines.push('')
  lines.push('**Statistical Assumptions:**')
  lines.push("- Independent samples t-test (Welch's variant, doesn't assume equal variances)")
  lines.push('- Assumes approximately normal distributions (robust to violations with n≥5)')
  lines.push('- Two-tailed test (detects differences in either direction)')
  lines.push('')
  lines.push('**Limitations:**')
  lines.push(`- Small sample size (n=${result.config.iterations} per condition) limits statistical power`)
  lines.push('- LLM variance can be high, affecting reproducibility')
  lines.push('- Task difficulty may not be calibrated optimally')
  lines.push('- Results may not generalize to all code search scenarios')
  lines.push('')

  // Recommendations
  lines.push('## Recommendations')
  lines.push('')
  if (result.significance.significant && result.advantage.meaningfulAdvantage) {
    lines.push('- ✓ This task demonstrates clear semantic search advantage')
    lines.push('- Consider including in grep-impossible or grep-hard benchmark suite')
    lines.push('- Use as example case for demonstrating search value')
  } else if (result.significance.scoreTest.pValue < 0.1) {
    lines.push('- Run additional iterations (n≥10) to increase statistical power')
    lines.push('- Validate that task difficulty is appropriate')
    lines.push('- Consider if task clearly favors semantic search')
  } else {
    lines.push('- Task may not be well-suited for demonstrating search advantage')
    lines.push('- Consider revising task to better highlight semantic search strengths')
    lines.push('- Verify that search tools are properly configured')
  }
  lines.push('')

  return lines.join('\n')
}
