/**
 * Example usage of the task validator
 *
 * This demonstrates how to validate individual tasks and entire suites.
 * Run with: tsx src/search-optimization/validation/example.ts
 */

import { validateTask, validateSuite, formatValidationReport, formatSuiteValidationReport } from './task-validator.js'
import { TIER1_GREP_IMPOSSIBLE_SUITE } from '../benchmarks/tier1-impossible.js'

/**
 * Example 1: Validate a single task
 */
async function exampleSingleTask() {
  console.log('='.repeat(80))
  console.log('Example 1: Validating a Single Task')
  console.log('='.repeat(80))
  console.log()

  // Get first task from Tier 1 suite
  const task = TIER1_GREP_IMPOSSIBLE_SUITE.tasks[0]

  console.log(`Validating task: ${task.name}`)
  console.log(`Category: ${task.category}`)
  console.log(`Difficulty: ${task.difficulty}`)
  console.log()

  // Validate the task (uses mock data by default)
  const result = await validateTask({
    task,
    tier: 'tier1-impossible',
    iterations: 5,
    useMockData: true, // Fast, deterministic validation
  })

  // Print the formatted report
  console.log(formatValidationReport(result))
}

/**
 * Example 2: Validate an entire suite
 */
async function exampleSuiteValidation() {
  console.log()
  console.log('='.repeat(80))
  console.log('Example 2: Validating Entire Suite')
  console.log('='.repeat(80))
  console.log()

  console.log(`Validating suite: ${TIER1_GREP_IMPOSSIBLE_SUITE.name}`)
  console.log(`Total tasks: ${TIER1_GREP_IMPOSSIBLE_SUITE.tasks.length}`)
  console.log()

  // Validate the entire suite
  const result = await validateSuite(TIER1_GREP_IMPOSSIBLE_SUITE, {
    iterations: 5,
    useMockData: true,
  })

  // Print the formatted report
  console.log(formatSuiteValidationReport(result))
}

/**
 * Example 3: Programmatic access to results
 */
async function exampleProgrammaticAccess() {
  console.log()
  console.log('='.repeat(80))
  console.log('Example 3: Programmatic Access to Results')
  console.log('='.repeat(80))
  console.log()

  const task = TIER1_GREP_IMPOSSIBLE_SUITE.tasks[0]
  const result = await validateTask({
    task,
    tier: 'tier1-impossible',
  })

  // Access individual dimension results
  console.log('Dimension Results:')
  console.log(`- Construct Validity: ${result.dimensions.constructValidity.passed ? 'PASS' : 'FAIL'}`)
  console.log(`- Discriminant Validity: ${result.dimensions.discriminantValidity.passed ? 'PASS' : 'FAIL'}`)
  console.log(`- Ecological Validity: ${result.dimensions.ecologicalValidity.passed ? 'PASS' : 'FAIL'}`)
  console.log(`- Reliability: ${result.dimensions.reliability.passed ? 'PASS' : 'FAIL'}`)
  console.log(`- Statistical Power: ${result.dimensions.statisticalPower.passed ? 'PASS' : 'FAIL'}`)
  console.log()

  // Access recommendations
  if (result.recommendations.length > 0) {
    console.log('Recommendations:')
    result.recommendations.forEach((rec, i) => {
      console.log(`${i + 1}. ${rec}`)
    })
  }
  console.log()

  // Overall result
  console.log(`Overall: ${result.passed ? 'PASSED ✓' : 'FAILED ✗'}`)
  console.log()
}

/**
 * Run all examples
 */
async function main() {
  console.log()
  console.log('Task Validator Examples')
  console.log('=======================')
  console.log()
  console.log('These examples demonstrate validation in MOCK MODE.')
  console.log('Mock mode uses task.expectedGrepSuccess and task.expectedSearchSuccess')
  console.log('without running expensive LLM benchmarks.')
  console.log()

  await exampleSingleTask()
  await exampleSuiteValidation()
  await exampleProgrammaticAccess()

  console.log('='.repeat(80))
  console.log('Examples Complete')
  console.log('='.repeat(80))
  console.log()
  console.log('Next steps:')
  console.log('1. Use validateTask() to validate individual tasks during development')
  console.log('2. Use validateSuite() to validate entire benchmark suites')
  console.log('3. Set useMockData: false only when ready for expensive real validation')
  console.log()
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error)
}
