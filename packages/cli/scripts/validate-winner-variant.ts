#!/usr/bin/env tsx
/**
 * Validate Winner Variant - TOOLOPT-2001
 *
 * Runs standalone benchmark comparing variant-control vs variant-a-detailed
 * to validate the +1.9% performance gain from genetic optimization.
 *
 * Success Criteria:
 * - variant-a-detailed scores ≥19.0% (allows 0.6% margin for variance)
 * - variant-control baseline ~17.7%
 * - Performance delta ~+1.9%
 *
 * Usage:
 *   pnpm tsx scripts/validate-winner-variant.ts
 */

import { writeFileSync } from 'fs'
import { join } from 'path'
import { runCompetition } from '../src/search-optimization/competition-runner.js'
import { loadVariant } from '../src/search-optimization/genetic-iterator.js'
import { TASK_FIND_WORKTREE_CREATION } from '../src/search-optimization/tasks/implementation.js'

const ITERATIONS = 5
const MIN_WINNER_SCORE = 0.19 // 19.0%
const EXPECTED_CONTROL_SCORE = 0.177 // 17.7%
const EXPECTED_DELTA = 0.019 // 1.9%

async function main() {
  console.log('='.repeat(80))
  console.log('WINNER VARIANT VALIDATION - TOOLOPT-2001')
  console.log('='.repeat(80))
  console.log(`Task: ${TASK_FIND_WORKTREE_CREATION.name}`)
  console.log(`Iterations: ${ITERATIONS}`)
  console.log(`Expected Control: ~${(EXPECTED_CONTROL_SCORE * 100).toFixed(1)}%`)
  console.log(`Expected Winner: ≥${(MIN_WINNER_SCORE * 100).toFixed(1)}%`)
  console.log(`Expected Delta: ~${(EXPECTED_DELTA * 100).toFixed(1)}%`)
  console.log('')

  // Load variants
  console.log('Loading variants...')
  const variantControl = await loadVariant('variant-control')
  const variantWinner = await loadVariant('variant-a-detailed')

  console.log(`✓ Loaded ${variantControl.name}`)
  console.log(`✓ Loaded ${variantWinner.name}`)
  console.log('')

  const results: {
    iteration: number
    controlScore: number
    winnerScore: number
    delta: number
  }[] = []

  // Run iterations
  for (let i = 1; i <= ITERATIONS; i++) {
    console.log(`\n${'='.repeat(80)}`)
    console.log(`ITERATION ${i}/${ITERATIONS}`)
    console.log('='.repeat(80))

    const baseDir = join('.crewchief', 'validation-benchmark', `iteration-${i}`)

    console.log('\nRunning competition...')
    const result = await runCompetition({
      task: TASK_FIND_WORKTREE_CREATION,
      variants: [variantControl, variantWinner],
      parallelExecution: false, // Sequential for stability
      timeout: 180,
      baseDir,
    })

    // Extract scores
    const controlResult = result.participants.find((p) => p.variantId === 'variant-control')
    const winnerResult = result.participants.find((p) => p.variantId === 'variant-a-detailed')

    if (!controlResult || !winnerResult) {
      throw new Error('Missing participant results')
    }

    const controlScore = controlResult.score
    const winnerScore = winnerResult.score
    const delta = winnerScore - controlScore

    results.push({
      iteration: i,
      controlScore,
      winnerScore,
      delta,
    })

    console.log('\nIteration Results:')
    console.log(`  Control Score: ${(controlScore * 100).toFixed(1)}%`)
    console.log(`  Winner Score:  ${(winnerScore * 100).toFixed(1)}%`)
    console.log(`  Delta:         ${delta >= 0 ? '+' : ''}${(delta * 100).toFixed(1)}%`)
  }

  // Calculate aggregates
  const avgControl = results.reduce((sum, r) => sum + r.controlScore, 0) / results.length
  const avgWinner = results.reduce((sum, r) => sum + r.winnerScore, 0) / results.length
  const avgDelta = avgWinner - avgControl

  console.log('\n' + '='.repeat(80))
  console.log('FINAL RESULTS')
  console.log('='.repeat(80))
  console.log('\nIndividual Iterations:')
  console.log('| Iteration | Control Score | Winner Score | Delta      |')
  console.log('|-----------|---------------|--------------|------------|')
  results.forEach((r) => {
    console.log(
      `| ${r.iteration.toString().padStart(9)} | ${(r.controlScore * 100).toFixed(1).padStart(13)}% | ${(r.winnerScore * 100).toFixed(1).padStart(12)}% | ${(r.delta >= 0 ? '+' : '') + (r.delta * 100).toFixed(1).padStart(9)}% |`,
    )
  })

  console.log('\nAggregated Results:')
  console.log(`  Average Control Score: ${(avgControl * 100).toFixed(1)}%`)
  console.log(`  Average Winner Score:  ${(avgWinner * 100).toFixed(1)}%`)
  console.log(`  Average Delta:         ${avgDelta >= 0 ? '+' : ''}${(avgDelta * 100).toFixed(1)}%`)
  console.log('')

  // Validation
  console.log('Validation Criteria:')
  const passWinnerThreshold = avgWinner >= MIN_WINNER_SCORE
  const passControlBaseline = Math.abs(avgControl - EXPECTED_CONTROL_SCORE) <= 0.05 // 5% tolerance
  const passDelta = Math.abs(avgDelta - EXPECTED_DELTA) <= 0.01 // 1% tolerance

  console.log(
    `  Winner ≥${(MIN_WINNER_SCORE * 100).toFixed(1)}%: ${passWinnerThreshold ? '✓ PASS' : '✗ FAIL'} (${(avgWinner * 100).toFixed(1)}%)`,
  )
  console.log(
    `  Control ~${(EXPECTED_CONTROL_SCORE * 100).toFixed(1)}%: ${passControlBaseline ? '✓ PASS' : '✗ FAIL'} (${(avgControl * 100).toFixed(1)}%)`,
  )
  console.log(
    `  Delta ~${(EXPECTED_DELTA * 100).toFixed(1)}%: ${passDelta ? '✓ PASS' : '✗ FAIL'} (${avgDelta >= 0 ? '+' : ''}${(avgDelta * 100).toFixed(1)}%)`,
  )
  console.log('')

  const allPass = passWinnerThreshold && passControlBaseline && passDelta

  // Save results
  const reportPath = join('.crewchief', 'validation-benchmark', 'validation-report.json')
  const report = {
    timestamp: new Date().toISOString(),
    ticket: 'TOOLOPT-2001',
    iterations: ITERATIONS,
    task: TASK_FIND_WORKTREE_CREATION.id,
    results,
    aggregates: {
      avgControl,
      avgWinner,
      avgDelta,
    },
    validation: {
      passWinnerThreshold,
      passControlBaseline,
      passDelta,
      allPass,
    },
    criteria: {
      minWinnerScore: MIN_WINNER_SCORE,
      expectedControlScore: EXPECTED_CONTROL_SCORE,
      expectedDelta: EXPECTED_DELTA,
    },
  }

  writeFileSync(reportPath, JSON.stringify(report, null, 2))
  console.log(`Report saved: ${reportPath}`)
  console.log('')

  if (allPass) {
    console.log('🎉 VALIDATION PASSED - Winner variant ready for deployment!')
    process.exit(0)
  } else {
    console.log('❌ VALIDATION FAILED - Further investigation needed')
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('Fatal error:', error)
  process.exit(1)
})
