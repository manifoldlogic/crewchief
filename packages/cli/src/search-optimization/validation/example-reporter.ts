/**
 * Example: Using the Validation Report Generator
 *
 * This example demonstrates how to validate a benchmark suite and generate
 * comprehensive reports in various formats.
 *
 * Run with: tsx src/search-optimization/validation/example-reporter.ts
 */

import { ReportGenerator } from './reporter.js'
import { validateSuite } from './task-validator.js'
import { TIER1_SUITE } from '../benchmarks/tier1-impossible.js'

/**
 * Example 1: Generate markdown report for Tier 1 suite
 */
async function example1_BasicMarkdownReport() {
  console.log('\n=== Example 1: Basic Markdown Report ===\n')

  // Validate the suite
  const suiteResult = await validateSuite(TIER1_SUITE)

  // Generate report
  const generator = new ReportGenerator({ format: 'markdown' })
  const report = generator.generate(suiteResult.taskResults, 'tier1-impossible', 'tier1')

  // Print summary to console
  console.log(`Total tasks: ${report.metadata.totalTasks}`)
  console.log(`Pass rate: ${report.summary.passRate.toFixed(1)}%`)
  console.log(`Passed: ${report.summary.passed}, Failed: ${report.summary.failed}`)

  // Save to file
  await generator.save(report)
  console.log('\nMarkdown report saved!')
}

/**
 * Example 2: Generate JSON report with custom output directory
 */
async function example2_JsonReport() {
  console.log('\n=== Example 2: JSON Report ===\n')

  // Validate the suite
  const suiteResult = await validateSuite(TIER1_SUITE)

  // Generate JSON report with custom output directory
  const generator = new ReportGenerator({
    format: 'json',
    outputDir: './validation-reports-json',
  })

  const report = generator.generate(suiteResult.taskResults, 'tier1-impossible', 'tier1')

  // Save with custom filename
  await generator.save(report, 'tier1-validation-results.json')
  console.log('JSON report saved!')
}

/**
 * Example 3: Console report for quick feedback
 */
async function example3_ConsoleReport() {
  console.log('\n=== Example 3: Console Report ===\n')

  // Validate the suite
  const suiteResult = await validateSuite(TIER1_SUITE)

  // Generate and print console report
  const generator = new ReportGenerator({ format: 'console' })
  const report = generator.generate(suiteResult.taskResults, 'tier1-impossible', 'tier1')

  generator.print(report)
}

/**
 * Example 4: Analyze failure patterns programmatically
 */
async function example4_AnalyzePatterns() {
  console.log('\n=== Example 4: Pattern Analysis ===\n')

  // Validate the suite
  const suiteResult = await validateSuite(TIER1_SUITE)

  // Generate report with patterns
  const generator = new ReportGenerator({ includePatterns: true })
  const report = generator.generate(suiteResult.taskResults, 'tier1-impossible', 'tier1')

  // Analyze patterns
  if (report.patterns && report.patterns.length > 0) {
    console.log('Identified failure patterns:')
    for (const pattern of report.patterns) {
      console.log(`\n- ${pattern.description}`)
      console.log(`  Affected tasks: ${pattern.count}`)
      console.log(`  Task IDs: ${pattern.taskIds.join(', ')}`)
      console.log('  Recommended fixes:')
      for (const fix of pattern.fixes) {
        console.log(`    • ${fix}`)
      }
    }
  } else {
    console.log('No failure patterns detected - all tasks passed validation!')
  }
}

/**
 * Example 5: Extract high-priority recommendations
 */
async function example5_HighPriorityRecommendations() {
  console.log('\n=== Example 5: High-Priority Recommendations ===\n')

  // Validate the suite
  const suiteResult = await validateSuite(TIER1_SUITE)

  // Generate report with recommendations
  const generator = new ReportGenerator({ includeRecommendations: true })
  const report = generator.generate(suiteResult.taskResults, 'tier1-impossible', 'tier1')

  // Filter high-priority recommendations
  const highPriority = report.recommendations?.filter((r) => r.priority === 'high')

  if (highPriority && highPriority.length > 0) {
    console.log(`Found ${highPriority.length} high-priority tasks requiring attention:\n`)
    for (const rec of highPriority) {
      console.log(`Task: ${rec.taskId}`)
      console.log('Actions:')
      for (let i = 0; i < rec.actions.length; i++) {
        console.log(`  ${i + 1}. ${rec.actions[i]}`)
      }
      console.log()
    }
  } else {
    console.log('No high-priority issues found!')
  }
}

/**
 * Example 6: Custom report analysis
 */
async function example6_CustomAnalysis() {
  console.log('\n=== Example 6: Custom Analysis ===\n')

  // Validate the suite
  const suiteResult = await validateSuite(TIER1_SUITE)

  // Generate report
  const generator = new ReportGenerator()
  const report = generator.generate(suiteResult.taskResults, 'tier1-impossible', 'tier1')

  // Custom analysis: Which dimension has the most failures?
  const dimensionFailures = report.summary.dimensionBreakdown
  const failureCounts = Object.entries(dimensionFailures)
    .map(([name, counts]) => ({ name, failures: counts.failed }))
    .sort((a, b) => b.failures - a.failures)

  console.log('Dimension Failure Analysis:')
  for (const dim of failureCounts) {
    if (dim.failures > 0) {
      const displayName = dim.name
        .replace(/([A-Z])/g, ' $1')
        .replace(/^./, (str) => str.toUpperCase())
        .trim()
      console.log(`  ${displayName}: ${dim.failures} failures`)
    }
  }

  // Custom analysis: What percentage of tasks need improvement?
  const needsImprovement = (report.summary.failed / report.summary.total) * 100
  console.log(`\n${needsImprovement.toFixed(1)}% of tasks need improvement`)

  // Custom analysis: Most common pattern
  if (report.patterns && report.patterns.length > 0) {
    const mostCommon = report.patterns.reduce((prev, current) => (prev.count > current.count ? prev : current))
    console.log(`\nMost common failure pattern: ${mostCommon.description} (${mostCommon.count} tasks)`)
  }
}

/**
 * Run all examples
 */
async function main() {
  console.log('╔════════════════════════════════════════════════════════════════╗')
  console.log('║       Validation Report Generator Examples                     ║')
  console.log('╚════════════════════════════════════════════════════════════════╝')

  try {
    await example1_BasicMarkdownReport()
    await example2_JsonReport()
    await example3_ConsoleReport()
    await example4_AnalyzePatterns()
    await example5_HighPriorityRecommendations()
    await example6_CustomAnalysis()

    console.log('\n✅ All examples completed successfully!')
  } catch (error) {
    console.error('\n❌ Error running examples:', error)
    process.exit(1)
  }
}

// Run examples if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main()
}

export { main }
