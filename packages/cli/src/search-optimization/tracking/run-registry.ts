/**
 * Run Registry System
 *
 * Tracks all optimization runs with metadata, learnings, and results.
 * Enables comparison between runs and extraction of insights.
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync, renameSync } from 'fs'
import { join } from 'path'
import type { MutationType } from '../../../../maproom-mcp/test/tool-description-optimization/types.js'
import type { IterationConfig, IterationHistory } from '../genetic-iterator.js'

/**
 * Status of an optimization run
 */
export type RunStatus = 'running' | 'completed' | 'failed'

/**
 * Learnings captured from a run
 */
export interface RunLearnings {
  bestMutationTypes: Array<{
    type: MutationType
    avgImprovement: number
    successRate: number
  }>
  convergencePattern: {
    generationsToConverge: number | null
    plateauDetected: boolean
    finalImprovement: number
  }
  taskCoverageTrends: {
    startingPassRate: number
    finalPassRate: number
    improvement: number
  }
  scoreVelocity: {
    avgImprovementPerGeneration: number
    bestGenerationImprovement: number
    worstGenerationImprovement: number
  }
  successfulParameters: {
    populationSize: number
    mutationRate: number
    convergenceThreshold: number
  }
  insights: string[] // Human-readable insights
}

/**
 * Optimization run metadata
 */
export interface OptimizationRun {
  runId: string
  startedAt: Date
  completedAt: Date | null
  status: RunStatus
  convergenceReached: boolean
  bestVariant: {
    id: string
    name: string
    score: number
    generation: number
  }
  config: IterationConfig
  learnings: RunLearnings | null
  generations: number // Total generations run
  finalAvgScore: number
  multiTierEnabled: boolean
}

/**
 * Run registry structure
 */
export interface RunRegistry {
  schemaVersion: number
  runs: OptimizationRun[]
  lastUpdated: Date
}

/**
 * Get run registry directory
 */
export function getRunRegistryDir(baseDir = '.crewchief'): string {
  return join(baseDir, 'optimization-runs')
}

/**
 * Get run registry path
 */
export function getRunRegistryPath(baseDir = '.crewchief'): string {
  return join(getRunRegistryDir(baseDir), 'index.json')
}

/**
 * Load run registry
 */
export function loadRunRegistry(baseDir = '.crewchief'): RunRegistry {
  const path = getRunRegistryPath(baseDir)

  if (!existsSync(path)) {
    return {
      schemaVersion: 1,
      runs: [],
      lastUpdated: new Date(),
    }
  }

  const content = readFileSync(path, 'utf-8')
  const data = JSON.parse(content)

  // Convert date strings to Date objects
  return {
    ...data,
    runs: data.runs.map((run: OptimizationRun) => ({
      ...run,
      startedAt: new Date(run.startedAt),
      completedAt: run.completedAt ? new Date(run.completedAt) : null,
    })),
    lastUpdated: new Date(data.lastUpdated),
  }
}

/**
 * Save run registry using atomic write
 */
export function saveRunRegistry(registry: RunRegistry, baseDir = '.crewchief'): void {
  const path = getRunRegistryPath(baseDir)
  const dir = getRunRegistryDir(baseDir)

  mkdirSync(dir, { recursive: true })

  const tmpPath = `${path}.tmp`
  writeFileSync(tmpPath, JSON.stringify(registry, null, 2))
  renameSync(tmpPath, path)
}

/**
 * Register a new optimization run at start
 */
export function registerRun(runId: string, config: IterationConfig, baseDir = '.crewchief'): OptimizationRun {
  const registry = loadRunRegistry(baseDir)

  const run: OptimizationRun = {
    runId,
    startedAt: new Date(),
    completedAt: null,
    status: 'running',
    convergenceReached: false,
    bestVariant: {
      id: '',
      name: '',
      score: 0,
      generation: 0,
    },
    config,
    learnings: null,
    generations: 0,
    finalAvgScore: 0,
    multiTierEnabled: config.multiTier?.enabled || false,
  }

  registry.runs.push(run)
  registry.lastUpdated = new Date()

  saveRunRegistry(registry, baseDir)

  console.log(`✓ Registered optimization run: ${runId}`)

  return run
}

/**
 * Update run status (completion or failure)
 */
export function updateRunStatus(
  runId: string,
  status: RunStatus,
  history?: IterationHistory,
  baseDir = '.crewchief',
): OptimizationRun {
  const registry = loadRunRegistry(baseDir)

  const run = registry.runs.find((r) => r.runId === runId)

  if (!run) {
    throw new Error(`Run ${runId} not found in registry`)
  }

  run.status = status
  run.completedAt = new Date()

  if (history) {
    run.convergenceReached = history.convergenceReached
    run.generations = history.totalIterations

    const lastGen = history.generations[history.generations.length - 1]
    run.finalAvgScore = lastGen?.avgScore || 0

    run.bestVariant = {
      id: history.bestOverall.id,
      name: history.bestOverall.name,
      score: lastGen?.bestScore || 0,
      generation: history.bestOverall.generation,
    }

    // Extract learnings
    run.learnings = extractLearnings(history)
  }

  registry.lastUpdated = new Date()
  saveRunRegistry(registry, baseDir)

  console.log(`✓ Updated run ${runId} status: ${status}`)

  return run
}

/**
 * Extract learnings from iteration history
 */
export function extractLearnings(history: IterationHistory): RunLearnings {
  const insights: string[] = []

  // Track mutation type performance
  const mutationPerformance = new Map<MutationType, { improvements: number[]; total: number }>()

  for (let i = 1; i < history.generations.length; i++) {
    const gen = history.generations[i]

    for (const variant of gen.variants) {
      if (variant.mutation_type && variant.generation === gen.number) {
        if (!mutationPerformance.has(variant.mutation_type)) {
          mutationPerformance.set(variant.mutation_type, { improvements: [], total: 0 })
        }

        const perf = mutationPerformance.get(variant.mutation_type)!
        perf.total++

        if (gen.improvement > 0) {
          perf.improvements.push(gen.improvement)
        }
      }
    }
  }

  const bestMutationTypes = Array.from(mutationPerformance.entries())
    .map(([type, perf]) => ({
      type,
      avgImprovement: perf.improvements.reduce((sum, i) => sum + i, 0) / perf.improvements.length || 0,
      successRate: perf.improvements.length / perf.total,
    }))
    .sort((a, b) => b.avgImprovement - a.avgImprovement)

  // Convergence pattern
  let generationsToConverge: number | null = null
  let plateauDetected = false

  if (history.convergenceReached) {
    generationsToConverge = history.totalIterations
    insights.push(`Convergence reached in ${generationsToConverge} generations`)
  } else {
    insights.push('Max iterations reached without convergence')
  }

  // Check for plateau (3+ generations with minimal improvement)
  let plateauCount = 0
  for (const gen of history.generations) {
    if (Math.abs(gen.improvement) < 0.01) {
      plateauCount++
    } else {
      plateauCount = 0
    }
    if (plateauCount >= 3) {
      plateauDetected = true
      break
    }
  }

  if (plateauDetected) {
    insights.push('Plateau detected: Consider adjusting mutation strategy')
  }

  const lastGen = history.generations[history.generations.length - 1]
  const finalImprovement = lastGen?.improvement || 0

  // Task coverage trends
  const firstGen = history.generations[0]
  const startingPassRate = firstGen.multiTierScores
    ? Array.from(firstGen.multiTierScores.values())
        .map((s) => s.taskCoverage.passed / s.taskCoverage.total)
        .reduce((sum, r) => sum + r, 0) / firstGen.multiTierScores.size || 0
    : 0

  const finalPassRate = lastGen.multiTierScores
    ? Array.from(lastGen.multiTierScores.values())
        .map((s) => s.taskCoverage.passed / s.taskCoverage.total)
        .reduce((sum, r) => sum + r, 0) / lastGen.multiTierScores.size || 0
    : 0

  const coverageImprovement = finalPassRate - startingPassRate

  if (coverageImprovement > 0.1) {
    insights.push(`Task completion rate improved by ${(coverageImprovement * 100).toFixed(1)}%`)
  }

  // Score velocity
  const improvements = history.generations.map((g) => g.improvement)
  const avgImprovementPerGeneration = improvements.reduce((sum, i) => sum + i, 0) / improvements.length
  const bestGenerationImprovement = Math.max(...improvements)
  const worstGenerationImprovement = Math.min(...improvements)

  if (bestMutationTypes.length > 0) {
    insights.push(
      `Best mutation type: ${bestMutationTypes[0].type} (${(bestMutationTypes[0].avgImprovement * 100).toFixed(2)}% avg improvement)`,
    )
  }

  if (avgImprovementPerGeneration > 0.05) {
    insights.push('Strong optimization velocity - good parameter choices')
  } else if (avgImprovementPerGeneration < 0.01) {
    insights.push('Slow optimization velocity - consider increasing mutation rate')
  }

  // Get config from first generation
  const config = history.generations[0]?.variants[0] ? firstGen : null

  return {
    bestMutationTypes,
    convergencePattern: {
      generationsToConverge,
      plateauDetected,
      finalImprovement,
    },
    taskCoverageTrends: {
      startingPassRate,
      finalPassRate,
      improvement: coverageImprovement,
    },
    scoreVelocity: {
      avgImprovementPerGeneration,
      bestGenerationImprovement,
      worstGenerationImprovement,
    },
    successfulParameters: {
      populationSize: config?.variants.length || 0,
      mutationRate: 0, // Would need to track this separately
      convergenceThreshold: 0.01, // From config
    },
    insights,
  }
}

/**
 * Compare two optimization runs
 */
export function compareRunResults(runId1: string, runId2: string, baseDir = '.crewchief'): string {
  const registry = loadRunRegistry(baseDir)

  const run1 = registry.runs.find((r) => r.runId === runId1)
  const run2 = registry.runs.find((r) => r.runId === runId2)

  if (!run1 || !run2) {
    throw new Error(`Run(s) not found: ${!run1 ? runId1 : ''} ${!run2 ? runId2 : ''}`)
  }

  const lines: string[] = []

  lines.push('RUN COMPARISON')
  lines.push('='.repeat(80))
  lines.push('')

  // Basic info
  lines.push(`Run 1: ${run1.runId}`)
  lines.push(`  Started: ${run1.startedAt.toLocaleString()}`)
  lines.push(`  Status: ${run1.status}`)
  lines.push(`  Generations: ${run1.generations}`)
  lines.push(`  Converged: ${run1.convergenceReached ? 'Yes' : 'No'}`)
  lines.push('')

  lines.push(`Run 2: ${run2.runId}`)
  lines.push(`  Started: ${run2.startedAt.toLocaleString()}`)
  lines.push(`  Status: ${run2.status}`)
  lines.push(`  Generations: ${run2.generations}`)
  lines.push(`  Converged: ${run2.convergenceReached ? 'Yes' : 'No'}`)
  lines.push('')

  // Performance comparison
  lines.push('PERFORMANCE COMPARISON')
  lines.push('-'.repeat(80))
  lines.push('Best Variant Score:')
  lines.push(`  Run 1: ${(run1.bestVariant.score * 100).toFixed(1)}%`)
  lines.push(`  Run 2: ${(run2.bestVariant.score * 100).toFixed(1)}%`)
  lines.push(
    `  Difference: ${run1.bestVariant.score > run2.bestVariant.score ? 'Run 1 wins' : 'Run 2 wins'} by ${(Math.abs(run1.bestVariant.score - run2.bestVariant.score) * 100).toFixed(1)}%`,
  )
  lines.push('')

  lines.push('Final Average Score:')
  lines.push(`  Run 1: ${(run1.finalAvgScore * 100).toFixed(1)}%`)
  lines.push(`  Run 2: ${(run2.finalAvgScore * 100).toFixed(1)}%`)
  lines.push('')

  // Learnings comparison
  if (run1.learnings && run2.learnings) {
    lines.push('LEARNINGS COMPARISON')
    lines.push('-'.repeat(80))

    lines.push('Best Mutation Types:')
    lines.push(`  Run 1: ${run1.learnings.bestMutationTypes[0]?.type || 'None'}`)
    lines.push(`  Run 2: ${run2.learnings.bestMutationTypes[0]?.type || 'None'}`)
    lines.push('')

    lines.push('Convergence:')
    lines.push(`  Run 1: ${run1.learnings.convergencePattern.generationsToConverge || 'Did not converge'} generations`)
    lines.push(`  Run 2: ${run2.learnings.convergencePattern.generationsToConverge || 'Did not converge'} generations`)
    lines.push('')

    lines.push('Task Coverage Improvement:')
    lines.push(`  Run 1: +${(run1.learnings.taskCoverageTrends.improvement * 100).toFixed(1)}%`)
    lines.push(`  Run 2: +${(run2.learnings.taskCoverageTrends.improvement * 100).toFixed(1)}%`)
    lines.push('')
  }

  // Config comparison
  lines.push('CONFIGURATION')
  lines.push('-'.repeat(80))
  lines.push('Population Size:')
  lines.push(`  Run 1: ${run1.config.populationSize}`)
  lines.push(`  Run 2: ${run2.config.populationSize}`)
  lines.push('')

  lines.push('Mutation Rate:')
  lines.push(`  Run 1: ${(run1.config.mutationRate * 100).toFixed(0)}%`)
  lines.push(`  Run 2: ${(run2.config.mutationRate * 100).toFixed(0)}%`)
  lines.push('')

  lines.push('Multi-Tier:')
  lines.push(`  Run 1: ${run1.multiTierEnabled ? 'Yes' : 'No'}`)
  lines.push(`  Run 2: ${run2.multiTierEnabled ? 'Yes' : 'No'}`)
  lines.push('')

  return lines.join('\n')
}

/**
 * Export learnings from a completed run
 */
export function exportLearnings(runId: string, baseDir = '.crewchief'): string {
  const registry = loadRunRegistry(baseDir)
  const run = registry.runs.find((r) => r.runId === runId)

  if (!run) {
    throw new Error(`Run ${runId} not found in registry`)
  }

  if (!run.learnings) {
    throw new Error(`Run ${runId} has no learnings (may still be running or failed)`)
  }

  const lines: string[] = []

  lines.push(`LEARNINGS FROM RUN: ${runId}`)
  lines.push('='.repeat(80))
  lines.push('')

  lines.push(`Run Date: ${run.startedAt.toLocaleString()}`)
  lines.push(`Status: ${run.status}`)
  lines.push(`Converged: ${run.convergenceReached ? 'Yes' : 'No'}`)
  lines.push(`Best Variant: ${run.bestVariant.name} (${(run.bestVariant.score * 100).toFixed(1)}%)`)
  lines.push('')

  lines.push('KEY INSIGHTS')
  lines.push('-'.repeat(80))
  run.learnings.insights.forEach((insight) => {
    lines.push(`• ${insight}`)
  })
  lines.push('')

  lines.push('MUTATION TYPE PERFORMANCE')
  lines.push('-'.repeat(80))
  run.learnings.bestMutationTypes.forEach((mt, i) => {
    lines.push(
      `${i + 1}. ${mt.type}: ${(mt.avgImprovement * 100).toFixed(2)}% avg improvement, ${(mt.successRate * 100).toFixed(0)}% success rate`,
    )
  })
  lines.push('')

  lines.push('CONVERGENCE PATTERN')
  lines.push('-'.repeat(80))
  lines.push(`Generations: ${run.learnings.convergencePattern.generationsToConverge || 'Did not converge'}`)
  lines.push(`Plateau Detected: ${run.learnings.convergencePattern.plateauDetected ? 'Yes' : 'No'}`)
  lines.push(`Final Improvement: ${(run.learnings.convergencePattern.finalImprovement * 100).toFixed(2)}%`)
  lines.push('')

  lines.push('TASK COVERAGE TRENDS')
  lines.push('-'.repeat(80))
  lines.push(`Starting Pass Rate: ${(run.learnings.taskCoverageTrends.startingPassRate * 100).toFixed(1)}%`)
  lines.push(`Final Pass Rate: ${(run.learnings.taskCoverageTrends.finalPassRate * 100).toFixed(1)}%`)
  lines.push(`Improvement: +${(run.learnings.taskCoverageTrends.improvement * 100).toFixed(1)}%`)
  lines.push('')

  lines.push('OPTIMIZATION VELOCITY')
  lines.push('-'.repeat(80))
  lines.push(
    `Average Improvement per Generation: ${(run.learnings.scoreVelocity.avgImprovementPerGeneration * 100).toFixed(2)}%`,
  )
  lines.push(
    `Best Generation Improvement: ${(run.learnings.scoreVelocity.bestGenerationImprovement * 100).toFixed(2)}%`,
  )
  lines.push(
    `Worst Generation Improvement: ${(run.learnings.scoreVelocity.worstGenerationImprovement * 100).toFixed(2)}%`,
  )
  lines.push('')

  lines.push('SUCCESSFUL PARAMETERS')
  lines.push('-'.repeat(80))
  lines.push(`Population Size: ${run.learnings.successfulParameters.populationSize}`)
  lines.push(`Mutation Rate: ${(run.learnings.successfulParameters.mutationRate * 100).toFixed(0)}%`)
  lines.push(`Convergence Threshold: ${(run.learnings.successfulParameters.convergenceThreshold * 100).toFixed(1)}%`)
  lines.push('')

  return lines.join('\n')
}

/**
 * Get run by ID
 */
export function getRun(runId: string, baseDir = '.crewchief'): OptimizationRun | null {
  const registry = loadRunRegistry(baseDir)
  return registry.runs.find((r) => r.runId === runId) || null
}

/**
 * List all runs
 */
export function listRuns(baseDir = '.crewchief'): OptimizationRun[] {
  const registry = loadRunRegistry(baseDir)
  return registry.runs
}

/**
 * Generate run registry report
 */
export function generateRunRegistryReport(baseDir = '.crewchief'): string {
  const registry = loadRunRegistry(baseDir)
  const lines: string[] = []

  lines.push('OPTIMIZATION RUN REGISTRY')
  lines.push('='.repeat(80))
  lines.push('')
  lines.push(`Total Runs: ${registry.runs.length}`)
  lines.push(`Last Updated: ${registry.lastUpdated.toLocaleString()}`)
  lines.push('')

  if (registry.runs.length === 0) {
    lines.push('No optimization runs recorded yet')
  } else {
    // Sort by start date (most recent first)
    const sorted = [...registry.runs].sort((a, b) => b.startedAt.getTime() - a.startedAt.getTime())

    lines.push('RECENT RUNS')
    lines.push('-'.repeat(80))

    sorted.slice(0, 10).forEach((run) => {
      lines.push(`${run.runId}`)
      lines.push(`  Started: ${run.startedAt.toLocaleString()}`)
      lines.push(`  Status: ${run.status}`)
      lines.push(`  Best: ${run.bestVariant.name} (${(run.bestVariant.score * 100).toFixed(1)}%)`)
      lines.push(`  Generations: ${run.generations} | Converged: ${run.convergenceReached ? 'Yes' : 'No'}`)
      lines.push(`  Multi-Tier: ${run.multiTierEnabled ? 'Yes' : 'No'}`)
      lines.push('')
    })
  }

  return lines.join('\n')
}
