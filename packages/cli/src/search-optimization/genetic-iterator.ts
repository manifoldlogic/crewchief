/**
 * Genetic Iteration Framework
 *
 * Implements continuous optimization through genetic algorithm:
 * 1. Run competition on current generation
 * 2. Select best performers
 * 3. Generate next generation (elitism, crossover, mutation)
 * 4. Repeat until convergence
 */

import { readFileSync, writeFileSync, mkdirSync } from 'fs'
import { join } from 'path'
import type { BenchmarkSuite } from './benchmarks/types.js'
import type { CompetitionResult } from './competition-runner.js'
import { runCompetition } from './competition-runner.js'
import {
  type MultiTierScore,
  type TierSuiteResult,
  type TierWeights,
  aggregateMultiTierScores,
  checkMultiTierConvergence,
  DEFAULT_TIER_WEIGHTS,
} from './multi-tier-scoring.js'
import type { SearchTask } from './types.js'
import { mutate, generateCrossover } from '../../../maproom-mcp/test/tool-description-optimization/mutator.js'
import type { Variant, MutationType } from '../../../maproom-mcp/test/tool-description-optimization/types.js'

/**
 * Configuration for genetic iterations
 */
export interface IterationConfig {
  initialVariants: string[] // Variant IDs to start with
  tasks: SearchTask[] // Tasks to run each iteration (single-tier mode)
  maxIterations: number // Stop after N iterations
  convergenceThreshold: number // Stop if improvement < threshold (e.g., 0.01 = 1%)
  mutationRate: number // Probability of mutation (0-1)
  populationSize: number // Variants per generation
  baseDir?: string // Base directory for iteration runs

  // Multi-tier evaluation mode
  multiTier?: {
    enabled: boolean
    tier1Suite: BenchmarkSuite // Grep-impossible tasks
    tier2Suite: BenchmarkSuite // Grep-hard tasks
    tier3Suite: BenchmarkSuite // Real-world tasks
    weights?: TierWeights // Tier weights (defaults to 40/40/20)
  }
}

/**
 * Result of complete genetic iteration run
 */
export interface IterationHistory {
  generations: Generation[]
  bestOverall: Variant
  convergenceReached: boolean
  totalIterations: number
}

/**
 * Single generation in iteration history
 */
export interface Generation {
  number: number
  variants: Variant[]
  taskResults: Map<string, CompetitionResult> // task.id -> result
  avgScore: number
  bestVariant: Variant
  bestScore: number
  improvement: number // vs previous generation

  // Multi-tier specific
  multiTierScores?: Map<string, MultiTierScore> // variantId -> multi-tier score
  tier1Results?: TierSuiteResult
  tier2Results?: TierSuiteResult
  tier3Results?: TierSuiteResult
}

/**
 * Run a benchmark suite and return tier results
 */
async function runBenchmarkSuite(
  suite: BenchmarkSuite,
  variants: Variant[],
  tier: 1 | 2 | 3,
  baseDir: string,
): Promise<TierSuiteResult> {
  const competitionResults = new Map<string, CompetitionResult>()
  const scores: number[] = []
  let searchUsedCount = 0
  let appropriateUsageCount = 0
  let completedCount = 0

  console.log(`\nRunning ${suite.name} (${suite.tasks.length} tasks)...`)

  for (const task of suite.tasks) {
    const result = await runCompetition({
      task,
      variants,
      parallelExecution: true,
      timeout: task.maxTimeSeconds || 300,
      baseDir: join(baseDir, `tier${tier}`, `task-${task.id}`),
    })

    competitionResults.set(task.id, result)

    // Track metrics
    for (const participant of result.participants) {
      scores.push(participant.score)

      const usedSearch = participant.toolsUsed?.includes('search') || false
      if (usedSearch) {
        searchUsedCount++
      }

      // For Tier 1 and 2, search is appropriate
      // For Tier 3, both tools are appropriate
      if (tier === 3 || usedSearch) {
        appropriateUsageCount++
      }

      if (participant.score >= 0.6) {
        completedCount++
      }
    }
  }

  const totalParticipants = scores.length || 1

  return {
    tier,
    tasks: suite.tasks,
    competitionResults,
    avgScore: scores.reduce((sum, s) => sum + s, 0) / scores.length || 0,
    searchUsageRate: searchUsedCount / totalParticipants,
    appropriateUsage: appropriateUsageCount / totalParticipants,
    taskCompletionRate: completedCount / totalParticipants,
  }
}

/**
 * Run genetic iterations to optimize tool descriptions
 *
 * @param config - Iteration configuration
 * @returns Complete iteration history with best variant
 */
export async function runGeneticIterations(config: IterationConfig): Promise<IterationHistory> {
  console.log('\nStarting genetic iterations...')
  console.log(`Population: ${config.populationSize}`)
  console.log(`Max Iterations: ${config.maxIterations}`)
  console.log(`Convergence Threshold: ${(config.convergenceThreshold * 100).toFixed(1)}%`)

  // Multi-tier mode info
  if (config.multiTier?.enabled) {
    console.log('\n[MULTI-TIER MODE ENABLED]')
    const weights = config.multiTier.weights || DEFAULT_TIER_WEIGHTS
    console.log(`Tier 1 Weight: ${(weights.tier1 * 100).toFixed(0)}%`)
    console.log(`Tier 2 Weight: ${(weights.tier2 * 100).toFixed(0)}%`)
    console.log(`Tier 3 Weight: ${(weights.tier3 * 100).toFixed(0)}%`)
    console.log(
      `Total Tasks: ${config.multiTier.tier1Suite.tasks.length + config.multiTier.tier2Suite.tasks.length + config.multiTier.tier3Suite.tasks.length}`,
    )
  }

  const history: Generation[] = []
  let currentVariants = await Promise.all(config.initialVariants.map((id) => loadVariant(id)))

  const baseDir = config.baseDir || join('.crewchief', 'genetic-iterations', `run-${Date.now()}`)
  mkdirSync(baseDir, { recursive: true })

  for (let i = 0; i < config.maxIterations; i++) {
    console.log(`\n${'='.repeat(60)}`)
    console.log(`GENERATION ${i + 1}`)
    console.log('='.repeat(60))

    const taskResults = new Map<string, CompetitionResult>()
    const variantAvgScores = new Map<string, number>()
    let multiTierScores: Map<string, MultiTierScore> | undefined
    let tier1Results: TierSuiteResult | undefined
    let tier2Results: TierSuiteResult | undefined
    let tier3Results: TierSuiteResult | undefined

    if (config.multiTier?.enabled) {
      // MULTI-TIER MODE: Run all 3 benchmark suites
      const genDir = join(baseDir, `gen-${i + 1}`)

      console.log('\n[RUNNING MULTI-TIER EVALUATION]')

      // Run Tier 1 (Grep-Impossible)
      tier1Results = await runBenchmarkSuite(config.multiTier.tier1Suite, currentVariants, 1, genDir)

      // Run Tier 2 (Grep-Hard)
      tier2Results = await runBenchmarkSuite(config.multiTier.tier2Suite, currentVariants, 2, genDir)

      // Run Tier 3 (Real-World)
      tier3Results = await runBenchmarkSuite(config.multiTier.tier3Suite, currentVariants, 3, genDir)

      // Combine all task results for recording
      for (const [taskId, result] of tier1Results.competitionResults) {
        taskResults.set(taskId, result)
      }
      for (const [taskId, result] of tier2Results.competitionResults) {
        taskResults.set(taskId, result)
      }
      for (const [taskId, result] of tier3Results.competitionResults) {
        taskResults.set(taskId, result)
      }

      // Calculate multi-tier scores
      const variantIds = currentVariants.map((v) => v.id)
      const weights = config.multiTier.weights || DEFAULT_TIER_WEIGHTS
      multiTierScores = aggregateMultiTierScores(variantIds, tier1Results, tier2Results, tier3Results, weights)

      // Extract composite scores for selection
      for (const [variantId, score] of multiTierScores) {
        variantAvgScores.set(variantId, score.composite)
      }

      console.log('\n[MULTI-TIER RESULTS]')
      console.log(`Tier 1 Avg: ${(tier1Results.avgScore * 100).toFixed(1)}%`)
      console.log(`Tier 2 Avg: ${(tier2Results.avgScore * 100).toFixed(1)}%`)
      console.log(`Tier 3 Avg: ${(tier3Results.avgScore * 100).toFixed(1)}%`)
    } else {
      // SINGLE-TIER MODE: Run tasks from config.tasks
      const variantScores = new Map<string, number[]>()

      for (const task of config.tasks) {
        console.log(`\nRunning task: ${task.name}`)

        const result = await runCompetition({
          task,
          variants: currentVariants,
          parallelExecution: true,
          timeout: task.maxTimeSeconds || 300,
          baseDir: join(baseDir, `gen-${i + 1}`, `task-${task.id}`),
        })

        taskResults.set(task.id, result)

        // Aggregate scores for each variant
        for (const participant of result.participants) {
          if (!variantScores.has(participant.variantId)) {
            variantScores.set(participant.variantId, [])
          }
          variantScores.get(participant.variantId)!.push(participant.score)
        }
      }

      // Calculate average score per variant across all tasks
      for (const [variantId, scores] of variantScores) {
        const avg = scores.reduce((sum, s) => sum + s, 0) / scores.length
        variantAvgScores.set(variantId, avg)
      }
    }

    // Find best variant of this generation
    let bestVariant = currentVariants[0]
    let bestScore = variantAvgScores.get(bestVariant.id) || 0

    for (const variant of currentVariants) {
      const score = variantAvgScores.get(variant.id) || 0
      if (score > bestScore) {
        bestScore = score
        bestVariant = variant
      }
    }

    // Calculate improvement vs previous generation
    const prevBest = history[history.length - 1]?.bestScore || 0
    const improvement = bestScore - prevBest

    // Calculate average score across all variants
    const avgScore = Array.from(variantAvgScores.values()).reduce((sum, s) => sum + s, 0) / variantAvgScores.size

    // Record generation
    const generation: Generation = {
      number: i + 1,
      variants: [...currentVariants],
      taskResults,
      avgScore,
      bestVariant,
      bestScore,
      improvement,
      multiTierScores,
      tier1Results,
      tier2Results,
      tier3Results,
    }

    history.push(generation)

    console.log(`\nGeneration ${i + 1} Summary:`)
    console.log(`Best: ${bestVariant.name} (${(bestScore * 100).toFixed(1)}%)`)
    console.log(`Avg: ${(avgScore * 100).toFixed(1)}%`)
    console.log(`Improvement: ${improvement > 0 ? '+' : ''}${(improvement * 100).toFixed(2)}%`)

    // Show multi-tier breakdown if available
    if (multiTierScores && bestVariant) {
      const bestMultiTierScore = multiTierScores.get(bestVariant.id)
      if (bestMultiTierScore) {
        console.log('\nMulti-Tier Breakdown (Best Variant):')
        console.log(`  Tier 1 (40%): ${(bestMultiTierScore.tierMetrics.tier1.avgScore * 100).toFixed(1)}%`)
        console.log(`  Tier 2 (40%): ${(bestMultiTierScore.tierMetrics.tier2.avgScore * 100).toFixed(1)}%`)
        console.log(`  Tier 3 (20%): ${(bestMultiTierScore.tierMetrics.tier3.avgScore * 100).toFixed(1)}%`)
        console.log(`Tool Selection Accuracy: ${(bestMultiTierScore.toolSelection.overallAccuracy * 100).toFixed(0)}%`)
      }
    }

    // Save generation report
    const genDir = join(baseDir, `gen-${i + 1}`)
    mkdirSync(genDir, { recursive: true })
    const genReport = generateGenerationReport(generation, variantAvgScores)
    writeFileSync(join(genDir, 'report.txt'), genReport)

    // Check convergence (multiple criteria)
    const noImprovementCount = history
      .slice(-3)
      .filter((g) => Math.abs(g.improvement) < config.convergenceThreshold).length

    // For multi-tier, also check tier stability
    let multiTierConvergence: ReturnType<typeof checkMultiTierConvergence> | undefined
    if (config.multiTier?.enabled && multiTierScores && history.length >= 2) {
      const prevGen = history[history.length - 2]
      const prevBestScore = prevGen.multiTierScores?.get(prevGen.bestVariant.id)
      const currentBestScore = multiTierScores.get(bestVariant.id)

      if (prevBestScore && currentBestScore) {
        multiTierConvergence = checkMultiTierConvergence(currentBestScore, prevBestScore, config.convergenceThreshold)
      }
    }

    const hasConverged =
      Math.abs(improvement) < config.convergenceThreshold &&
      noImprovementCount >= 3 &&
      (!multiTierConvergence || multiTierConvergence.converged)

    if (hasConverged) {
      console.log(`\n${'='.repeat(60)}`)
      console.log('CONVERGENCE REACHED')
      console.log('='.repeat(60))
      console.log(
        `Improvement: ${(improvement * 100).toFixed(2)}% (threshold: ${(config.convergenceThreshold * 100).toFixed(1)}%)`,
      )
      console.log(`Consecutive no-improvement generations: ${noImprovementCount}/3`)

      // Show multi-tier convergence details
      if (multiTierConvergence) {
        console.log('\nMulti-Tier Convergence:')
        console.log(
          `  Tier 1: ${multiTierConvergence.tier1Stable ? '✓' : '✗'} (${(multiTierConvergence.tier1Improvement * 100).toFixed(2)}%)`,
        )
        console.log(
          `  Tier 2: ${multiTierConvergence.tier2Stable ? '✓' : '✗'} (${(multiTierConvergence.tier2Improvement * 100).toFixed(2)}%)`,
        )
        console.log(
          `  Tier 3: ${multiTierConvergence.tier3Stable ? '✓' : '✗'} (${(multiTierConvergence.tier3Improvement * 100).toFixed(2)}%)`,
        )
      }

      const finalHistory: IterationHistory = {
        generations: history,
        bestOverall: bestVariant,
        convergenceReached: true,
        totalIterations: i + 1,
      }

      // Save final report
      const finalReport = generateIterationReport(finalHistory)
      writeFileSync(join(baseDir, 'final-report.txt'), finalReport)

      return finalHistory
    }

    // Generate next generation
    if (i < config.maxIterations - 1) {
      console.log('\nGenerating next generation...')
      currentVariants = await generateNextGeneration(currentVariants, variantAvgScores, config)

      // Save new variants
      for (const variant of currentVariants) {
        await saveVariant(variant, baseDir)
      }
    }
  }

  // Max iterations reached
  const bestOverall = history.reduce((best, gen) => (gen.bestScore > best.bestScore ? gen : best)).bestVariant

  const finalHistory: IterationHistory = {
    generations: history,
    bestOverall,
    convergenceReached: false,
    totalIterations: config.maxIterations,
  }

  // Save final report
  const finalReport = generateIterationReport(finalHistory)
  writeFileSync(join(baseDir, 'final-report.txt'), finalReport)

  console.log(`\n${'='.repeat(60)}`)
  console.log('MAX ITERATIONS REACHED')
  console.log('='.repeat(60))

  return finalHistory
}

/**
 * Generate next generation from current variants
 *
 * Strategy:
 * 1. Elitism: Keep best variant
 * 2. Crossover: Combine top 2 variants
 * 3. Mutation: Mutate best variant with variety
 * 4. Diversity: Occasional random mutations
 */
export async function generateNextGeneration(
  currentVariants: Variant[],
  scores: Map<string, number>,
  config: IterationConfig,
): Promise<Variant[]> {
  // Sort by score (best first)
  const sorted = [...currentVariants].sort((a, b) => (scores.get(b.id) || 0) - (scores.get(a.id) || 0))

  const nextGen: Variant[] = []

  // 1. Keep best variant (elitism)
  console.log(`  Keeping best variant: ${sorted[0].name}`)
  nextGen.push(sorted[0])

  // 2. Crossover top 2 variants
  if (sorted.length >= 2) {
    console.log(`  Crossing ${sorted[0].name} + ${sorted[1].name}`)
    const crossedResult = generateCrossover(sorted[0], sorted[1])
    if (crossedResult.success && crossedResult.variant) {
      nextGen.push(crossedResult.variant)
    }
  }

  // 3. Mutate best variant (with diversity mechanism)
  const mutationTypes: MutationType[] = ['amplification', 'reduction', 'reframing', 'specialization']

  for (let i = 0; i < config.populationSize - nextGen.length; i++) {
    // Add random variant every 5th slot to maintain diversity
    if (i === 0 && Math.random() < 0.2) {
      // Random mutation from random parent (not just best)
      const randomParent = sorted[Math.floor(Math.random() * Math.min(3, sorted.length))]
      const randomMutationType = mutationTypes[Math.floor(Math.random() * mutationTypes.length)]

      console.log(`  Random ${randomMutationType} mutation from ${randomParent.name}`)

      const randomResult = mutate({
        type: randomMutationType,
        parents: [randomParent],
      })

      if (randomResult.success && randomResult.variant) {
        nextGen.push(randomResult.variant)
      }
    } else {
      // Regular mutation from best
      const mutationType = mutationTypes[i % mutationTypes.length]

      console.log(`  ${mutationType} mutation from ${sorted[0].name}`)

      const mutated = mutate({
        type: mutationType,
        parents: [sorted[0]],
      })

      if (mutated.success && mutated.variant) {
        nextGen.push(mutated.variant)
      }
    }
  }

  return nextGen
}

/**
 * Load variant from JSON file
 */
export async function loadVariant(idOrPath: string): Promise<Variant> {
  // Check if it's a full path or just an ID
  let variantPath: string

  if (idOrPath.endsWith('.json')) {
    variantPath = idOrPath
  } else {
    // Look in standard variant directory
    // Navigate from packages/cli to workspace root, then to maproom-mcp
    const workspaceRoot = join(process.cwd(), '..', '..')
    variantPath = join(
      workspaceRoot,
      'packages',
      'maproom-mcp',
      'test',
      'tool-description-optimization',
      'variants',
      `${idOrPath}.json`,
    )
  }

  const content = readFileSync(variantPath, 'utf-8')
  const data = JSON.parse(content)

  // Convert date strings back to Date objects
  return {
    ...data,
    created_at: new Date(data.created_at),
  }
}

/**
 * Save variant to JSON file
 */
export async function saveVariant(variant: Variant, baseDir?: string): Promise<void> {
  let variantDir: string

  if (baseDir) {
    variantDir = join(baseDir, 'variants')
  } else {
    // Navigate from packages/cli to workspace root, then to maproom-mcp
    const workspaceRoot = join(process.cwd(), '..', '..')
    variantDir = join(workspaceRoot, 'packages', 'maproom-mcp', 'test', 'tool-description-optimization', 'variants')
  }

  mkdirSync(variantDir, { recursive: true })

  const variantPath = join(variantDir, `${variant.id}.json`)
  writeFileSync(variantPath, JSON.stringify(variant, null, 2))
}

/**
 * Generate generation-specific report
 */
function generateGenerationReport(generation: Generation, scores: Map<string, number>): string {
  const lines: string[] = []

  lines.push(`GENERATION ${generation.number} REPORT`)
  lines.push('='.repeat(60))
  lines.push('')

  lines.push('RESULTS')
  lines.push('-'.repeat(60))

  // Sort variants by score
  const sortedVariants = [...generation.variants].sort((a, b) => (scores.get(b.id) || 0) - (scores.get(a.id) || 0))

  sortedVariants.forEach((variant, i) => {
    const score = scores.get(variant.id) || 0
    lines.push(`${i + 1}. ${variant.name}`)
    lines.push(`   ID: ${variant.id}`)
    lines.push(`   Score: ${(score * 100).toFixed(1)}%`)
    lines.push(`   Generation: ${variant.generation}`)
    if (variant.mutation_type) {
      lines.push(`   Mutation: ${variant.mutation_type}`)
    }

    // Show multi-tier breakdown if available
    if (generation.multiTierScores) {
      const multiTierScore = generation.multiTierScores.get(variant.id)
      if (multiTierScore) {
        lines.push(`   Tier 1: ${(multiTierScore.tierMetrics.tier1.avgScore * 100).toFixed(1)}%`)
        lines.push(`   Tier 2: ${(multiTierScore.tierMetrics.tier2.avgScore * 100).toFixed(1)}%`)
        lines.push(`   Tier 3: ${(multiTierScore.tierMetrics.tier3.avgScore * 100).toFixed(1)}%`)
        lines.push(`   Tool Accuracy: ${(multiTierScore.toolSelection.overallAccuracy * 100).toFixed(0)}%`)
      }
    }

    lines.push('')
  })

  lines.push('SUMMARY')
  lines.push('-'.repeat(60))
  lines.push(`Best: ${generation.bestVariant.name} - ${(generation.bestScore * 100).toFixed(1)}%`)
  lines.push(`Average: ${(generation.avgScore * 100).toFixed(1)}%`)
  lines.push(`Improvement: ${generation.improvement > 0 ? '+' : ''}${(generation.improvement * 100).toFixed(2)}%`)

  // Show multi-tier summary if available
  if (generation.multiTierScores && generation.tier1Results && generation.tier2Results && generation.tier3Results) {
    lines.push('')
    lines.push('MULTI-TIER BREAKDOWN')
    lines.push('-'.repeat(60))
    lines.push(`Tier 1 (Grep-Impossible): ${(generation.tier1Results.avgScore * 100).toFixed(1)}%`)
    lines.push(`  Tasks: ${generation.tier1Results.tasks.length}`)
    lines.push(`  Search Usage: ${(generation.tier1Results.searchUsageRate * 100).toFixed(0)}%`)
    lines.push(`  Completion Rate: ${(generation.tier1Results.taskCompletionRate * 100).toFixed(0)}%`)
    lines.push('')
    lines.push(`Tier 2 (Grep-Hard): ${(generation.tier2Results.avgScore * 100).toFixed(1)}%`)
    lines.push(`  Tasks: ${generation.tier2Results.tasks.length}`)
    lines.push(`  Search Usage: ${(generation.tier2Results.searchUsageRate * 100).toFixed(0)}%`)
    lines.push(`  Completion Rate: ${(generation.tier2Results.taskCompletionRate * 100).toFixed(0)}%`)
    lines.push('')
    lines.push(`Tier 3 (Real-World): ${(generation.tier3Results.avgScore * 100).toFixed(1)}%`)
    lines.push(`  Tasks: ${generation.tier3Results.tasks.length}`)
    lines.push(`  Voluntary Search Adoption: ${(generation.tier3Results.searchUsageRate * 100).toFixed(0)}%`)
    lines.push(`  Completion Rate: ${(generation.tier3Results.taskCompletionRate * 100).toFixed(0)}%`)
  }

  lines.push('')

  return lines.join('\n')
}

/**
 * Generate iteration summary report
 */
export function generateIterationReport(history: IterationHistory): string {
  const lines: string[] = []

  lines.push('GENETIC ITERATION REPORT')
  lines.push('='.repeat(60))
  lines.push('')
  lines.push(`Total Iterations: ${history.totalIterations}`)
  lines.push(`Convergence: ${history.convergenceReached ? 'YES' : 'NO (max iterations reached)'}`)
  lines.push('')

  lines.push('PROGRESS OVER TIME')
  lines.push('-'.repeat(60))
  history.generations.forEach((gen) => {
    lines.push(`Generation ${gen.number}:`)
    lines.push(`  Best: ${gen.bestVariant.name} - ${(gen.bestScore * 100).toFixed(1)}%`)
    lines.push(`  Avg:  ${(gen.avgScore * 100).toFixed(1)}%`)
    lines.push(`  Δ:    ${gen.improvement > 0 ? '+' : ''}${(gen.improvement * 100).toFixed(2)}%`)
  })
  lines.push('')

  lines.push('OVERALL BEST')
  lines.push('-'.repeat(60))
  lines.push(`${history.bestOverall.name}`)

  const bestGen = history.generations.find((g) => g.bestVariant.id === history.bestOverall.id)
  if (bestGen) {
    lines.push(`Score: ${(bestGen.bestScore * 100).toFixed(1)}%`)
  }

  lines.push('')
  lines.push('Description preview:')
  lines.push(history.bestOverall.description.substring(0, 200) + '...')
  lines.push('')

  lines.push('RECOMMENDATION')
  lines.push('-'.repeat(60))
  if (history.convergenceReached) {
    lines.push('✓ Convergence reached - deploy this variant to production')
    lines.push('✓ Use this variant as baseline for future iterations')
  } else {
    lines.push('⚠ Max iterations reached without convergence')
    lines.push('Consider:')
    lines.push('- Running more iterations')
    lines.push('- Adjusting mutation strategy')
    lines.push('- Reviewing task diversity')
  }
  lines.push('')

  return lines.join('\n')
}
