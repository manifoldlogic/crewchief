#!/usr/bin/env tsx
/**
 * Run genetic iteration optimizer with standard settings
 *
 * The competition runner now includes three phases:
 * 1. Setup: Create worktrees, scan, validate (~2-3 min for 5 variants)
 * 2. Validation: Check environment readiness (~10-20 sec)
 * 3. Execution: Run agents in parallel (~2-5 min)
 *
 * Total time per generation: ~4-8 minutes
 * Total time for 5 generations: ~20-40 minutes
 *
 * Validation ensures 100% of agents have tool access (vs 0% previously).
 *
 * Usage:
 *   tsx scripts/run-genetic-optimizer.ts
 */

import { join } from 'path'
import { runGeneticIterations } from '../src/search-optimization/genetic-iterator.js'
import { TASK_FIND_WORKTREE_CREATION } from '../src/search-optimization/tasks/implementation.js'

async function main() {
  console.log('Starting genetic optimization...\n')

  const config = {
    // Initial variants (from maproom-mcp package)
    initialVariants: ['variant-control', 'variant-a-detailed', 'variant-b-simple'],

    // Tasks to optimize against
    tasks: [
      TASK_FIND_WORKTREE_CREATION,
      // Add more tasks here as they're created
    ],

    // Iteration parameters
    maxIterations: 5,
    convergenceThreshold: 0.01, // Stop if improvement < 1%
    mutationRate: 0.5,
    populationSize: 5,

    // Output directory
    baseDir: join(process.cwd(), '.crewchief', 'genetic-iterations', `run-${Date.now()}`),
  }

  try {
    const result = await runGeneticIterations(config)

    console.log('\n' + '='.repeat(70))
    console.log('OPTIMIZATION COMPLETE')
    console.log('='.repeat(70))
    console.log(`Total Iterations: ${result.totalIterations}`)
    console.log(`Convergence Reached: ${result.convergenceReached ? 'YES' : 'NO'}`)
    console.log(`\nBest Variant: ${result.bestOverall.name}`)
    console.log(`ID: ${result.bestOverall.id}`)
    console.log(`Generation: ${result.bestOverall.generation}`)

    if (result.bestOverall.mutation_type) {
      console.log(`Mutation Type: ${result.bestOverall.mutation_type}`)
    }

    const bestGen = result.generations.find((g) => g.bestVariant.id === result.bestOverall.id)
    if (bestGen) {
      console.log(`\nFinal Score: ${(bestGen.bestScore * 100).toFixed(1)}%`)
    }

    console.log(`\nResults saved to: ${config.baseDir}`)
    console.log(`Final report: ${config.baseDir}/final-report.txt`)
  } catch (error) {
    console.error('Optimization failed:', error)
    process.exit(1)
  }
}

main()
