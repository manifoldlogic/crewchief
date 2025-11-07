/**
 * Example: Running a benchmark suite
 *
 * This example demonstrates how to use the suite-runner to execute
 * a benchmark suite and analyze the results.
 *
 * IMPORTANT: This uses mock data. For real execution:
 * 1. Run tasks manually with baseline-runner.ts (expensive LLM calls)
 * 2. Collect actual success metrics
 * 3. Use those results for validation
 */

import {
  TIER1_GREP_IMPOSSIBLE_SUITE,
  runBenchmarkSuite,
  formatSuiteSummary,
  calculateAggregateMetrics,
  validateSuiteResults,
  type TaskResult,
} from '../index.js'

/**
 * Example 1: Run suite with mock data (for testing)
 */
async function exampleMockExecution() {
  console.log('Example 1: Running suite with mock data\n')

  // Run the suite (uses mock data based on expected metrics)
  const result = await runBenchmarkSuite(TIER1_GREP_IMPOSSIBLE_SUITE, {
    parallel: false,
    iterations: 1,
  })

  // Print summary
  console.log(formatSuiteSummary(result))
  console.log('\n' + '='.repeat(60) + '\n')
}

/**
 * Example 2: Process real execution results
 *
 * When you have actual results from running baseline-runner.ts,
 * you can process them like this:
 */
async function exampleRealResults() {
  console.log('Example 2: Processing real execution results\n')

  // Simulate real results (in practice, these come from baseline-runner)
  const realResults: TaskResult[] = [
    {
      task: TIER1_GREP_IMPOSSIBLE_SUITE.tasks[0],
      grepSuccess: 0.15, // Grep struggled
      searchSuccess: 0.85, // Search succeeded
      improvement: 0.7,
    },
    {
      task: TIER1_GREP_IMPOSSIBLE_SUITE.tasks[1],
      grepSuccess: 0.25,
      searchSuccess: 0.75,
      improvement: 0.5,
    },
    // ... more results
  ]

  // Calculate aggregate metrics
  const aggregate = calculateAggregateMetrics(realResults)
  console.log('Aggregate Metrics:')
  console.log(`  Grep avg:   ${(aggregate.grepAvgSuccess * 100).toFixed(1)}%`)
  console.log(`  Search avg: ${(aggregate.searchAvgSuccess * 100).toFixed(1)}%`)
  console.log(`  Improvement: +${(aggregate.avgImprovement * 100).toFixed(1)}%`)
  console.log(`  Tasks defeating grep: ${aggregate.tasksDefeatingGrep}/${realResults.length}`)

  // Validate results
  const validation = validateSuiteResults(realResults, TIER1_GREP_IMPOSSIBLE_SUITE)
  console.log('\nValidation:')
  validation.details.forEach((detail) => {
    console.log(`  ${detail}`)
  })

  console.log('\n' + '='.repeat(60) + '\n')
}

/**
 * Example 3: Sequential vs Parallel execution
 */
async function exampleExecutionModes() {
  console.log('Example 3: Comparing execution modes\n')

  // Sequential execution (safer, preserves order)
  const resultSeq = await runBenchmarkSuite(TIER1_GREP_IMPOSSIBLE_SUITE, {
    parallel: false,
  })
  const seqTime = resultSeq.executionTime

  // Parallel execution (faster, unordered)
  const resultPar = await runBenchmarkSuite(TIER1_GREP_IMPOSSIBLE_SUITE, {
    parallel: true,
  })
  const parTime = resultPar.executionTime

  console.log('Execution Times:')
  console.log(`  Sequential: ${seqTime}ms`)
  console.log(`  Parallel:   ${parTime}ms`)
  console.log(`  Speedup:    ${(seqTime / parTime).toFixed(2)}x`)

  console.log('\n' + '='.repeat(60) + '\n')
}

/**
 * Example 4: Individual task analysis
 */
async function exampleTaskAnalysis() {
  console.log('Example 4: Analyzing individual tasks\n')

  const result = await runBenchmarkSuite(TIER1_GREP_IMPOSSIBLE_SUITE)

  // Find best performing task
  const bestTask = result.taskResults.reduce((best, current) =>
    current.improvement > best.improvement ? current : best,
  )

  // Find worst performing task
  const worstTask = result.taskResults.reduce((worst, current) =>
    current.improvement < worst.improvement ? current : worst,
  )

  console.log('Best Performing Task:')
  console.log(`  ID: ${bestTask.task.id}`)
  console.log(`  Name: ${bestTask.task.name}`)
  console.log(`  Improvement: +${(bestTask.improvement * 100).toFixed(1)}%`)
  console.log(`  Grep: ${(bestTask.grepSuccess * 100).toFixed(1)}%`)
  console.log(`  Search: ${(bestTask.searchSuccess * 100).toFixed(1)}%`)

  console.log('\nWorst Performing Task:')
  console.log(`  ID: ${worstTask.task.id}`)
  console.log(`  Name: ${worstTask.task.name}`)
  console.log(`  Improvement: +${(worstTask.improvement * 100).toFixed(1)}%`)
  console.log(`  Grep: ${(worstTask.grepSuccess * 100).toFixed(1)}%`)
  console.log(`  Search: ${(worstTask.searchSuccess * 100).toFixed(1)}%`)

  // Analyze by category
  const byCategory = new Map<string, TaskResult[]>()
  result.taskResults.forEach((tr) => {
    const cat = tr.task.category
    if (!byCategory.has(cat)) {
      byCategory.set(cat, [])
    }
    byCategory.get(cat)!.push(tr)
  })

  console.log('\nPerformance by Category:')
  byCategory.forEach((tasks, category) => {
    const avg = tasks.reduce((sum, t) => sum + t.improvement, 0) / tasks.length
    console.log(`  ${category}: +${(avg * 100).toFixed(1)}% (${tasks.length} tasks)`)
  })

  console.log('\n' + '='.repeat(60) + '\n')
}

/**
 * Run all examples
 */
async function main() {
  console.log('Benchmark Suite Runner Examples')
  console.log('='.repeat(60) + '\n')

  await exampleMockExecution()
  await exampleRealResults()
  await exampleExecutionModes()
  await exampleTaskAnalysis()

  console.log('Examples complete!')
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error)
}
