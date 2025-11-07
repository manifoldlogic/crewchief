/**
 * Full Validation Run Script
 *
 * Executes complete validation suite across all 30+ benchmark tasks,
 * comparing grep-only baseline vs search-available performance.
 *
 * This script is EXPENSIVE ($20-50 in API costs) and requires explicit
 * user confirmation before running.
 *
 * Usage:
 *   pnpm search-optimization:validate-full
 */

import { mkdirSync, writeFileSync } from 'fs'
import { join } from 'path'
import { performance } from 'perf_hooks'
import type { BenchmarkSuite } from '../benchmarks/index.js'
import { TIER1_GREP_IMPOSSIBLE_SUITE, TIER2_GREP_HARD_SUITE, TIER3_REALWORLD_SUITE } from '../benchmarks/index.js'
import type { CompetitionResult } from '../competition-runner.js'
import { runCompetition } from '../competition-runner.js'
import { performStatisticalAnalysis, type StatisticalAnalysis } from '../reporting/statistics.js'
import { generateValidationReport, saveValidationReport } from '../reporting/validation-report.js'

/**
 * Configuration for validation run
 */
export interface ValidationOptions {
  outputDir?: string
  skipConfirmation?: boolean
  verbose?: boolean
  dryRun?: boolean
}

/**
 * Results from running a single condition (grep-only or search-available)
 */
export interface ConditionResults {
  condition: 'grep-only' | 'search-available'
  tier1Results: Map<string, CompetitionResult>
  tier2Results: Map<string, CompetitionResult>
  tier3Results: Map<string, CompetitionResult>
  overallScores: {
    tier1Avg: number
    tier2Avg: number
    tier3Avg: number
    compositeAvg: number
  }
  toolUsageStats: {
    searchUsageRate: number
    grepUsageRate: number
    appropriateUsage: number
  }
  durationSeconds: number
}

/**
 * Complete validation results
 */
export interface ValidationResults {
  timestamp: Date
  grepResults: ConditionResults
  searchResults: ConditionResults
  statisticalAnalysis: StatisticalAnalysis
  summary: {
    totalTasks: number
    grepSuccessRate: number
    searchSuccessRate: number
    improvement: number
    statisticallySignificant: boolean
  }
  perTierSummary: {
    tier1: TierSummary
    tier2: TierSummary
    tier3: TierSummary
  }
  perCategorySummary: Map<string, CategorySummary>
}

export interface TierSummary {
  taskCount: number
  grepSuccess: number
  searchSuccess: number
  improvement: number
  pValue: number
}

export interface CategorySummary {
  taskCount: number
  grepSuccess: number
  searchSuccess: number
  improvement: number
}

/**
 * Estimate cost of running validation
 */
export function estimateCost(suites: BenchmarkSuite[]): { min: number; max: number; estimate: number } {
  const totalTasks = suites.reduce((sum, suite) => sum + suite.tasks.length, 0)

  // Conservative estimates:
  // - 10-15 tool calls per task
  // - $0.01-0.02 per call
  // - 2 runs (grep + search)
  const minCallsPerTask = 10
  const maxCallsPerTask = 15
  const minCostPerCall = 0.01
  const maxCostPerCall = 0.02
  const runs = 2

  const minCost = totalTasks * runs * minCallsPerTask * minCostPerCall
  const maxCost = totalTasks * runs * maxCallsPerTask * maxCostPerCall
  const estimate = (minCost + maxCost) / 2

  return { min: minCost, max: maxCost, estimate }
}

/**
 * Prompt user for confirmation
 */
export async function promptConfirmation(cost: { min: number; max: number; estimate: number }): Promise<boolean> {
  console.log('\n⚠️  WARNING: EXPENSIVE OPERATION')
  console.log('='.repeat(60))
  console.log(`Estimated cost: $${cost.min.toFixed(2)} - $${cost.max.toFixed(2)} USD`)
  console.log(`Best estimate: $${cost.estimate.toFixed(2)} USD`)
  console.log('Execution time: 30-60 minutes')
  console.log('='.repeat(60))

  // In a real implementation, use readline or inquirer
  // For now, return false to prevent accidental runs
  console.log('\n❌ Auto-declined: Use --skip-confirmation flag to override')
  return false
}

/**
 * Run benchmark suite with specific condition
 */
async function runSuiteCondition(
  suites: BenchmarkSuite[],
  condition: 'grep-only' | 'search-available',
  baseDir: string,
): Promise<ConditionResults> {
  const startTime = performance.now()

  console.log(`\n${'='.repeat(60)}`)
  console.log(`Running: ${condition.toUpperCase()}`)
  console.log('='.repeat(60))

  // Create mock variant for the condition
  const variant = {
    id: condition,
    name: condition === 'grep-only' ? 'Grep Baseline' : 'Search Available',
    description: `Benchmark with ${condition} tools`,
    created_at: new Date(),
    generation: 0,
  }

  const tier1Results = new Map<string, CompetitionResult>()
  const tier2Results = new Map<string, CompetitionResult>()
  const tier3Results = new Map<string, CompetitionResult>()

  // Run Tier 1
  console.log(`\n[TIER 1: Grep-Impossible (${suites[0].tasks.length} tasks)]`)
  for (const task of suites[0].tasks) {
    const result = await runCompetition({
      task,
      variants: [variant],
      parallelExecution: false,
      timeout: task.maxTimeSeconds || 300,
      baseDir: join(baseDir, condition, 'tier1', task.id),
    })
    tier1Results.set(task.id, result)
  }

  // Run Tier 2
  console.log(`\n[TIER 2: Grep-Hard (${suites[1].tasks.length} tasks)]`)
  for (const task of suites[1].tasks) {
    const result = await runCompetition({
      task,
      variants: [variant],
      parallelExecution: false,
      timeout: task.maxTimeSeconds || 300,
      baseDir: join(baseDir, condition, 'tier2', task.id),
    })
    tier2Results.set(task.id, result)
  }

  // Run Tier 3
  console.log(`\n[TIER 3: Real-World (${suites[2].tasks.length} tasks)]`)
  for (const task of suites[2].tasks) {
    const result = await runCompetition({
      task,
      variants: [variant],
      parallelExecution: false,
      timeout: task.maxTimeSeconds || 300,
      baseDir: join(baseDir, condition, 'tier3', task.id),
    })
    tier3Results.set(task.id, result)
  }

  // Calculate statistics
  const tier1Scores: number[] = []
  const tier2Scores: number[] = []
  const tier3Scores: number[] = []
  let searchUsageCount = 0
  let grepUsageCount = 0
  let totalTools = 0

  for (const result of tier1Results.values()) {
    for (const p of result.participants) {
      tier1Scores.push(p.score)
      if (p.toolsUsed?.includes('search')) searchUsageCount++
      if (p.toolsUsed?.includes('grep') || p.toolsUsed?.includes('Grep')) grepUsageCount++
      totalTools++
    }
  }

  for (const result of tier2Results.values()) {
    for (const p of result.participants) {
      tier2Scores.push(p.score)
      if (p.toolsUsed?.includes('search')) searchUsageCount++
      if (p.toolsUsed?.includes('grep') || p.toolsUsed?.includes('Grep')) grepUsageCount++
      totalTools++
    }
  }

  for (const result of tier3Results.values()) {
    for (const p of result.participants) {
      tier3Scores.push(p.score)
      if (p.toolsUsed?.includes('search')) searchUsageCount++
      if (p.toolsUsed?.includes('grep') || p.toolsUsed?.includes('Grep')) grepUsageCount++
      totalTools++
    }
  }

  const tier1Avg = tier1Scores.reduce((sum, s) => sum + s, 0) / tier1Scores.length || 0
  const tier2Avg = tier2Scores.reduce((sum, s) => sum + s, 0) / tier2Scores.length || 0
  const tier3Avg = tier3Scores.reduce((sum, s) => sum + s, 0) / tier3Scores.length || 0

  const compositeAvg = tier1Avg * 0.4 + tier2Avg * 0.4 + tier3Avg * 0.2

  const durationSeconds = (performance.now() - startTime) / 1000

  return {
    condition,
    tier1Results,
    tier2Results,
    tier3Results,
    overallScores: {
      tier1Avg,
      tier2Avg,
      tier3Avg,
      compositeAvg,
    },
    toolUsageStats: {
      searchUsageRate: totalTools > 0 ? searchUsageCount / totalTools : 0,
      grepUsageRate: totalTools > 0 ? grepUsageCount / totalTools : 0,
      appropriateUsage: 0, // Calculated in analysis
    },
    durationSeconds,
  }
}

/**
 * Run full validation across all tiers
 */
export async function runFullValidation(options: ValidationOptions = {}): Promise<ValidationResults> {
  const timestamp = new Date()
  const timestampStr = timestamp.toISOString().replace(/[:.]/g, '-').slice(0, 19)
  const baseDir = options.outputDir || join('.crewchief', 'validation-results', timestampStr)

  // Create output directory
  mkdirSync(baseDir, { recursive: true })

  // Load benchmark suites
  const suites = [TIER1_GREP_IMPOSSIBLE_SUITE, TIER2_GREP_HARD_SUITE, TIER3_REALWORLD_SUITE]

  console.log('\n' + '='.repeat(60))
  console.log('FULL VALIDATION RUN')
  console.log('='.repeat(60))
  console.log(`Timestamp: ${timestamp.toISOString()}`)
  console.log(`Output Directory: ${baseDir}`)
  console.log(`\nTier 1: ${suites[0].tasks.length} tasks (grep-impossible)`)
  console.log(`Tier 2: ${suites[1].tasks.length} tasks (grep-hard)`)
  console.log(`Tier 3: ${suites[2].tasks.length} tasks (real-world)`)
  console.log(`Total: ${suites.reduce((sum, s) => sum + s.tasks.length, 0)} tasks`)

  // Cost estimation and confirmation
  if (!options.dryRun) {
    const cost = estimateCost(suites)
    if (!options.skipConfirmation) {
      const confirmed = await promptConfirmation(cost)
      if (!confirmed) {
        throw new Error('User cancelled validation run')
      }
    }
  }

  // Run grep-only baseline
  const grepResults = await runSuiteCondition(suites, 'grep-only', baseDir)

  // Run search-available condition
  const searchResults = await runSuiteCondition(suites, 'search-available', baseDir)

  // Perform statistical analysis
  const statisticalAnalysis = performStatisticalAnalysis(grepResults, searchResults)

  // Calculate per-tier summaries
  const perTierSummary = {
    tier1: {
      taskCount: suites[0].tasks.length,
      grepSuccess: grepResults.overallScores.tier1Avg,
      searchSuccess: searchResults.overallScores.tier1Avg,
      improvement: searchResults.overallScores.tier1Avg - grepResults.overallScores.tier1Avg,
      pValue: statisticalAnalysis.tier1PValue || 0,
    },
    tier2: {
      taskCount: suites[1].tasks.length,
      grepSuccess: grepResults.overallScores.tier2Avg,
      searchSuccess: searchResults.overallScores.tier2Avg,
      improvement: searchResults.overallScores.tier2Avg - grepResults.overallScores.tier2Avg,
      pValue: statisticalAnalysis.tier2PValue || 0,
    },
    tier3: {
      taskCount: suites[2].tasks.length,
      grepSuccess: grepResults.overallScores.tier3Avg,
      searchSuccess: searchResults.overallScores.tier3Avg,
      improvement: searchResults.overallScores.tier3Avg - grepResults.overallScores.tier3Avg,
      pValue: statisticalAnalysis.tier3PValue || 0,
    },
  }

  // Calculate per-category summaries
  const perCategorySummary = new Map<string, CategorySummary>()

  // Collect all tasks by category
  const tasksByCategory = new Map<string, { grepScores: number[]; searchScores: number[] }>()

  for (const suite of suites) {
    for (const task of suite.tasks) {
      const category = task.category
      if (!tasksByCategory.has(category)) {
        tasksByCategory.set(category, { grepScores: [], searchScores: [] })
      }

      // Find results for this task in both conditions
      const grepResult =
        grepResults.tier1Results.get(task.id) ||
        grepResults.tier2Results.get(task.id) ||
        grepResults.tier3Results.get(task.id)
      const searchResult =
        searchResults.tier1Results.get(task.id) ||
        searchResults.tier2Results.get(task.id) ||
        searchResults.tier3Results.get(task.id)

      if (grepResult && searchResult) {
        const categoryData = tasksByCategory.get(category)!
        categoryData.grepScores.push(grepResult.participants[0].score)
        categoryData.searchScores.push(searchResult.participants[0].score)
      }
    }
  }

  // Calculate summary for each category
  for (const [category, data] of tasksByCategory) {
    const taskCount = data.grepScores.length
    const grepSuccess = data.grepScores.reduce((sum, s) => sum + s, 0) / taskCount
    const searchSuccess = data.searchScores.reduce((sum, s) => sum + s, 0) / taskCount
    const improvement = searchSuccess - grepSuccess

    perCategorySummary.set(category, {
      taskCount,
      grepSuccess,
      searchSuccess,
      improvement,
    })
  }

  const results: ValidationResults = {
    timestamp,
    grepResults,
    searchResults,
    statisticalAnalysis,
    summary: {
      totalTasks: suites.reduce((sum, s) => sum + s.tasks.length, 0),
      grepSuccessRate: grepResults.overallScores.compositeAvg,
      searchSuccessRate: searchResults.overallScores.compositeAvg,
      improvement: searchResults.overallScores.compositeAvg - grepResults.overallScores.compositeAvg,
      statisticallySignificant: (statisticalAnalysis.pValue || 1) < 0.05,
    },
    perTierSummary,
    perCategorySummary,
  }

  // Generate and save report
  const report = generateValidationReport(results)
  await saveValidationReport(report, baseDir)

  // Save raw results as JSON
  writeFileSync(join(baseDir, 'results.json'), JSON.stringify(results, null, 2))

  console.log(`\n${'='.repeat(60)}`)
  console.log('VALIDATION COMPLETE')
  console.log('='.repeat(60))
  console.log(`Results saved to: ${baseDir}`)
  console.log(`Report: ${join(baseDir, 'report.md')}`)

  return results
}

// CLI execution
if (import.meta.url === `file://${process.argv[1]}`) {
  const options: ValidationOptions = {
    skipConfirmation: process.argv.includes('--skip-confirmation'),
    verbose: process.argv.includes('--verbose'),
    dryRun: process.argv.includes('--dry-run'),
  }

  runFullValidation(options)
    .then((results) => {
      console.log('\n✅ Validation successful')
      console.log(`Improvement: +${(results.summary.improvement * 100).toFixed(1)}%`)
      console.log(`Statistical significance: ${results.summary.statisticallySignificant ? 'YES' : 'NO'}`)
      process.exit(0)
    })
    .catch((error) => {
      console.error('\n❌ Validation failed:', error.message)
      process.exit(1)
    })
}
