#!/usr/bin/env tsx
/**
 * Estimate cost of genetic optimization run without executing
 *
 * Usage:
 *   pnpm tsx scripts/estimate-optimization-cost.ts
 */

import { TASK_FIND_WORKTREE_CREATION } from '../src/search-optimization/tasks/implementation.js'

interface CostEstimate {
  totalSessions: number
  estimatedInputTokens: number
  estimatedOutputTokens: number
  estimatedCost: number
  breakdown: {
    generation: number
    variants: number
    sessionsPerGeneration: number
    inputTokens: number
    outputTokens: number
    cost: number
  }[]
}

// Claude Sonnet 4.5 pricing (as of 2025)
// https://www.anthropic.com/pricing
const PRICING = {
  inputTokensPer1M: 3.0, // $3 per 1M input tokens
  outputTokensPer1M: 15.0, // $15 per 1M output tokens
}

function estimateCost(config: {
  initialVariants: number
  maxIterations: number
  populationSize: number
  tasksPerIteration: number
  convergenceLikely: boolean
}): CostEstimate {
  const breakdown: CostEstimate['breakdown'] = []
  let totalSessions = 0
  let totalInputTokens = 0
  let totalOutputTokens = 0

  // Estimate tokens per agent session
  // Based on typical search task agent runs
  const TOKENS_PER_SESSION = {
    input: 15000, // Task description + tool descriptions + context + search results
    output: 5000, // Agent responses + tool calls + reasoning
  }

  // Generation 1: Use initial variant count
  const gen1Variants = config.initialVariants
  const gen1Sessions = gen1Variants * config.tasksPerIteration
  const gen1Input = gen1Sessions * TOKENS_PER_SESSION.input
  const gen1Output = gen1Sessions * TOKENS_PER_SESSION.output
  const gen1Cost =
    (gen1Input / 1_000_000) * PRICING.inputTokensPer1M + (gen1Output / 1_000_000) * PRICING.outputTokensPer1M

  breakdown.push({
    generation: 1,
    variants: gen1Variants,
    sessionsPerGeneration: gen1Sessions,
    inputTokens: gen1Input,
    outputTokens: gen1Output,
    cost: gen1Cost,
  })

  totalSessions += gen1Sessions
  totalInputTokens += gen1Input
  totalOutputTokens += gen1Output

  // Subsequent generations: Use population size
  const estimatedGenerations = config.convergenceLikely
    ? Math.min(3, config.maxIterations) // Likely converges by gen 3
    : config.maxIterations

  for (let gen = 2; gen <= estimatedGenerations; gen++) {
    const genSessions = config.populationSize * config.tasksPerIteration
    const genInput = genSessions * TOKENS_PER_SESSION.input
    const genOutput = genSessions * TOKENS_PER_SESSION.output
    const genCost =
      (genInput / 1_000_000) * PRICING.inputTokensPer1M + (genOutput / 1_000_000) * PRICING.outputTokensPer1M

    breakdown.push({
      generation: gen,
      variants: config.populationSize,
      sessionsPerGeneration: genSessions,
      inputTokens: genInput,
      outputTokens: genOutput,
      cost: genCost,
    })

    totalSessions += genSessions
    totalInputTokens += genInput
    totalOutputTokens += genOutput
  }

  const totalCost =
    (totalInputTokens / 1_000_000) * PRICING.inputTokensPer1M +
    (totalOutputTokens / 1_000_000) * PRICING.outputTokensPer1M

  return {
    totalSessions,
    estimatedInputTokens: totalInputTokens,
    estimatedOutputTokens: totalOutputTokens,
    estimatedCost: totalCost,
    breakdown,
  }
}

function formatCost(cost: number): string {
  return `$${cost.toFixed(2)}`
}

function formatTokens(tokens: number): string {
  if (tokens >= 1_000_000) {
    return `${(tokens / 1_000_000).toFixed(2)}M`
  } else if (tokens >= 1_000) {
    return `${(tokens / 1_000).toFixed(1)}K`
  }
  return tokens.toString()
}

async function main() {
  console.log('GENETIC OPTIMIZATION COST ESTIMATOR')
  console.log('='.repeat(70))
  console.log()

  // Configuration from run-genetic-optimizer.ts
  const config = {
    initialVariants: 3, // control, detailed, simple
    maxIterations: 5,
    populationSize: 5,
    tasksPerIteration: 1, // Currently only TASK_FIND_WORKTREE_CREATION
    convergenceLikely: true, // Likely to converge before max iterations
  }

  console.log('Configuration:')
  console.log(`  Initial Variants: ${config.initialVariants}`)
  console.log(`  Max Iterations: ${config.maxIterations}`)
  console.log(`  Population Size: ${config.populationSize}`)
  console.log(`  Tasks per Generation: ${config.tasksPerIteration}`)
  console.log(`  Task: ${TASK_FIND_WORKTREE_CREATION.name}`)
  console.log()

  const estimate = estimateCost(config)

  console.log('Cost Breakdown by Generation:')
  console.log('-'.repeat(70))
  console.log('Gen | Variants | Sessions | Input Tokens | Output Tokens | Cost')
  console.log('-'.repeat(70))

  for (const gen of estimate.breakdown) {
    console.log(
      `${gen.generation.toString().padStart(3)} | ` +
        `${gen.variants.toString().padStart(8)} | ` +
        `${gen.sessionsPerGeneration.toString().padStart(8)} | ` +
        `${formatTokens(gen.inputTokens).padStart(12)} | ` +
        `${formatTokens(gen.outputTokens).padStart(13)} | ` +
        `${formatCost(gen.cost).padStart(8)}`,
    )
  }
  console.log('-'.repeat(70))

  console.log()
  console.log('TOTAL ESTIMATE:')
  console.log(`  Agent Sessions: ${estimate.totalSessions}`)
  console.log(
    `  Input Tokens: ${formatTokens(estimate.estimatedInputTokens)} (~${(estimate.estimatedInputTokens / 1_000_000).toFixed(2)}M)`,
  )
  console.log(
    `  Output Tokens: ${formatTokens(estimate.estimatedOutputTokens)} (~${(estimate.estimatedOutputTokens / 1_000_000).toFixed(2)}M)`,
  )
  console.log(`  Estimated Cost: ${formatCost(estimate.estimatedCost)}`)
  console.log()

  console.log('Notes:')
  console.log('  - Assumes convergence by generation 3 (typical)')
  console.log('  - Actual cost may vary ±30% based on agent behavior')
  console.log('  - Each agent session includes search operations and task completion')
  console.log('  - Uses Claude Sonnet 4.5 pricing ($3/1M input, $15/1M output)')
  console.log()

  console.log('To run the actual optimization:')
  console.log('  pnpm tsx scripts/run-genetic-optimizer.ts')
  console.log()

  // Show cost for max iterations scenario
  const maxConfig = { ...config, convergenceLikely: false }
  const maxEstimate = estimateCost(maxConfig)

  console.log('Maximum Cost Scenario (no early convergence):')
  console.log(`  All ${config.maxIterations} iterations: ${formatCost(maxEstimate.estimatedCost)}`)
  console.log(`  ${maxEstimate.totalSessions} total agent sessions`)
  console.log()
}

main()
