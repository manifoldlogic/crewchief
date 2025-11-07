#!/usr/bin/env tsx
/**
 * Run genetic optimizer with premium settings for best results
 *
 * This configuration prioritizes quality over cost:
 * - Larger population for better diversity
 * - More iterations for thorough evolution
 * - Stricter convergence for higher quality
 * - Multiple tasks for robust optimization
 *
 * Estimated cost: $15-25 (vs $1.56 for standard)
 *
 * Usage:
 *   pnpm tsx scripts/run-genetic-optimizer-premium.ts
 */

import { join } from 'path'
import { runGeneticIterations } from '../src/search-optimization/genetic-iterator.js'
import { TASK_FIND_WORKTREE_CREATION } from '../src/search-optimization/tasks/implementation.js'

async function main() {
  console.log('🚀 PREMIUM GENETIC OPTIMIZATION')
  console.log('Optimizing for maximum quality results\n')

  const config = {
    // Start with all available baseline variants
    initialVariants: [
      'variant-control',
      'variant-a-detailed',
      'variant-b-simple',
      'variant-c-conversational',
      'variant-d-code-like',
    ],

    // Multiple tasks for robust optimization
    // TODO: Add more tasks as they're created:
    // - TASK_FIND_ERROR_HANDLING
    // - TASK_LOCATE_API_ENDPOINT
    // - TASK_UNDERSTAND_ARCHITECTURE
    // - TASK_DEBUG_ISSUE
    tasks: [
      TASK_FIND_WORKTREE_CREATION,
      // Add more tasks here for multi-objective optimization
    ],

    // Premium iteration parameters
    maxIterations: 10, // More generations for thorough evolution
    convergenceThreshold: 0.005, // Stricter: Stop only if improvement < 0.5% (vs 1%)
    mutationRate: 0.5,
    populationSize: 8, // Larger population for better diversity

    // Output directory
    baseDir: join(process.cwd(), '.crewchief', 'genetic-iterations', `premium-run-${Date.now()}`),
  }

  console.log('Configuration:')
  console.log(`  Initial Variants: ${config.initialVariants.length}`)
  console.log(`  Population Size: ${config.populationSize} (vs 5 standard)`)
  console.log(`  Max Iterations: ${config.maxIterations} (vs 5 standard)`)
  console.log(`  Convergence Threshold: ${(config.convergenceThreshold * 100).toFixed(1)}% (vs 1% standard)`)
  console.log(`  Tasks: ${config.tasks.length}`)
  console.log()

  console.log('Expected Benefits:')
  console.log('  ✓ Better exploration of variant space')
  console.log('  ✓ More thorough convergence validation')
  console.log('  ✓ Higher quality final variant')
  console.log('  ✓ Statistical confidence from larger population')
  console.log()

  console.log('Estimated Cost: $15-25 (depends on convergence)')
  console.log('Estimated Time: 30-60 minutes\n')

  const readline = await import('readline')
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  })

  const proceed = await new Promise<boolean>((resolve) => {
    rl.question('Proceed with premium optimization? (yes/no): ', (answer) => {
      rl.close()
      resolve(answer.toLowerCase() === 'yes' || answer.toLowerCase() === 'y')
    })
  })

  if (!proceed) {
    console.log('\nOptimization cancelled.')
    return
  }

  console.log('\nStarting optimization...\n')

  try {
    const startTime = Date.now()
    const result = await runGeneticIterations(config)
    const durationMinutes = ((Date.now() - startTime) / 1000 / 60).toFixed(1)

    console.log('\n' + '='.repeat(70))
    console.log('PREMIUM OPTIMIZATION COMPLETE')
    console.log('='.repeat(70))
    console.log(`Duration: ${durationMinutes} minutes`)
    console.log(`Total Iterations: ${result.totalIterations}`)
    console.log(`Convergence Reached: ${result.convergenceReached ? 'YES' : 'NO'}`)
    console.log()

    console.log('Best Variant:')
    console.log(`  Name: ${result.bestOverall.name}`)
    console.log(`  ID: ${result.bestOverall.id}`)
    console.log(`  Generation: ${result.bestOverall.generation}`)
    if (result.bestOverall.mutation_type) {
      console.log(`  Mutation Type: ${result.bestOverall.mutation_type}`)
    }

    const bestGen = result.generations.find((g) => g.bestVariant.id === result.bestOverall.id)
    if (bestGen) {
      console.log(`  Final Score: ${(bestGen.bestScore * 100).toFixed(1)}%`)
      console.log(
        `  Improvement from Gen 1: ${((bestGen.bestScore - result.generations[0].bestScore) * 100).toFixed(1)}%`,
      )
    }

    console.log()
    console.log('Evolution Progress:')
    result.generations.forEach((gen, idx) => {
      const arrow = idx === result.generations.length - 1 ? '→' : ' '
      console.log(
        `  ${arrow} Gen ${gen.number}: ${(gen.bestScore * 100).toFixed(1)}% ` +
          `(avg: ${(gen.avgScore * 100).toFixed(1)}%) ` +
          `${gen.improvement > 0 ? `+${(gen.improvement * 100).toFixed(2)}%` : `${(gen.improvement * 100).toFixed(2)}%`}`,
      )
    })

    console.log()
    console.log('Next Steps:')
    console.log(`  1. Review final report: ${config.baseDir}/final-report.txt`)
    console.log(`  2. Review best variant: ${config.baseDir}/variants/${result.bestOverall.id}.json`)
    console.log('  3. Deploy variant to production if scores justify it')
    console.log('  4. Run A/B test against current baseline')
    console.log()
  } catch (error) {
    console.error('\nOptimization failed:', error)
    process.exit(1)
  }
}

main()
