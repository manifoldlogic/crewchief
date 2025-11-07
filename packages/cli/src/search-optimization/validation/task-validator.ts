/**
 * Task Validator - Validates tasks across 5 quality dimensions
 *
 * This is VALIDATION INFRASTRUCTURE for task design quality.
 * It does NOT execute LLM agents or make API calls (too expensive).
 *
 * MOCK MODE (default):
 * - Uses task.expectedGrepSuccess and task.expectedSearchSuccess
 * - Mocks statistical significance tests
 * - No actual LLM execution or API calls
 * - Fast, deterministic, suitable for CI/CD
 *
 * REAL MODE (manual only):
 * - Would require running baseline-runner (expensive)
 * - Would execute actual LLM agents
 * - Would make real API calls
 * - Not implemented in this phase
 *
 * The 5 validation dimensions:
 * 1. Construct Validity - Does grep fail as expected? (baseline)
 * 2. Discriminant Validity - Does search succeed with advantage? (improvement)
 * 3. Ecological Validity - Is this a realistic developer task?
 * 4. Test-Retest Reliability - Are results consistent? (low variance)
 * 5. Statistical Power - Is sample size adequate? (n >= 5)
 */

import type { BenchmarkSuite } from '../benchmarks/tier1-impossible.js'
import type { SearchTask } from '../types.js'
import { validateEcologicalValidity as validateEcologicalValidityFull } from './ecological.js'

/**
 * Configuration for task validation
 */
export interface ValidationConfig {
  /** Task to validate */
  task: SearchTask

  /** Tier determines difficulty thresholds */
  tier: 'tier1-impossible' | 'tier2-hard' | 'tier3-realworld'

  /** Number of iterations for reliability testing */
  iterations?: number // Default: 5

  /** Custom validation thresholds */
  thresholds?: ValidationThresholds

  /**
   * Mock mode (default): Use expected metrics from task definition
   * Real mode: Would execute actual benchmarks (expensive, not implemented)
   */
  useMockData?: boolean // Default: true
}

/**
 * Validation thresholds for different tiers
 *
 * Tier 1 (Impossible): Grep should fail (<30%), search should excel (>70%)
 * Tier 2 (Hard): Grep might succeed sometimes (<60%), search should still win (>70%)
 * Tier 3 (Real-world): More relaxed, focus on realistic scenarios
 */
export interface ValidationThresholds {
  tier1: TierThresholds
  tier2: TierThresholds
  tier3: TierThresholds
}

/**
 * Thresholds for a single tier
 */
export interface TierThresholds {
  /** Maximum allowed grep success rate */
  grepMaxSuccess: number

  /** Minimum required search success rate */
  searchMinSuccess: number

  /** Minimum improvement (search - grep) */
  minAdvantage: number

  /** Maximum allowed variance (coefficient of variation) */
  maxVariance: number

  /** Minimum p-value for statistical significance */
  minPValue: number
}

/**
 * Default validation thresholds
 */
export const DEFAULT_THRESHOLDS: ValidationThresholds = {
  tier1: {
    grepMaxSuccess: 0.3, // 30% - grep should mostly fail
    searchMinSuccess: 0.7, // 70% - search should mostly succeed
    minAdvantage: 0.3, // 30pp improvement minimum
    maxVariance: 0.1, // 10% CV max
    minPValue: 0.05, // p < 0.05 for significance
  },
  tier2: {
    grepMaxSuccess: 0.6, // 60% - grep can succeed sometimes
    searchMinSuccess: 0.7, // 70% - search should still be better
    minAdvantage: 0.2, // 20pp improvement minimum
    maxVariance: 0.1, // 10% CV max
    minPValue: 0.05, // p < 0.05 for significance
  },
  tier3: {
    grepMaxSuccess: 0.8, // 80% - more about realism than difficulty
    searchMinSuccess: 0.6, // 60% - focus on real-world scenarios
    minAdvantage: 0.1, // 10pp improvement minimum
    maxVariance: 0.15, // 15% CV max (more variability expected)
    minPValue: 0.05, // p < 0.05 for significance
  },
}

/**
 * Result of validating a single dimension
 */
export interface DimensionResult {
  /** Dimension name */
  dimension: string

  /** Did this dimension pass validation? */
  passed: boolean

  /** Actual value observed/mocked */
  actual: number | string

  /** Expected threshold or criterion */
  expected: number | string

  /** Detailed explanation */
  details: string
}

/**
 * Result of validating a task across all dimensions
 */
export interface ValidationResult {
  /** Task that was validated */
  task: SearchTask

  /** Overall pass/fail */
  passed: boolean

  /** Which tier was validated */
  tier: string

  /** Results for each dimension */
  dimensions: {
    constructValidity: DimensionResult
    discriminantValidity: DimensionResult
    ecologicalValidity: DimensionResult
    reliability: DimensionResult
    statisticalPower: DimensionResult
  }

  /** Recommendations for improving task quality */
  recommendations: string[]

  /** When validation was performed */
  timestamp: Date
}

/**
 * Result of validating an entire benchmark suite
 */
export interface SuiteValidationResult {
  /** Suite that was validated */
  suite: BenchmarkSuite

  /** Did the entire suite pass? */
  passed: boolean

  /** Total tasks in suite */
  totalTasks: number

  /** Number of tasks that passed validation */
  passedTasks: number

  /** Number of tasks that failed validation */
  failedTasks: number

  /** Individual task validation results */
  taskResults: ValidationResult[]

  /** Summary of suite quality */
  summary: string
}

/**
 * Validate Dimension 1: Construct Validity (Grep Baseline)
 *
 * Ensures the task is appropriately difficult for grep-based search.
 * In mock mode: Uses task.expectedGrepSuccess
 * In real mode: Would run baseline-runner (expensive, not implemented)
 *
 * @param task - The task to validate
 * @param thresholds - Tier-specific thresholds
 * @param useMockData - Use mock data (default: true)
 */
function validateConstructValidity(task: SearchTask, thresholds: TierThresholds, useMockData = true): DimensionResult {
  // In mock mode, use expected success rate from task definition
  const grepSuccess = useMockData ? ((task as SearchTask & Record<string, unknown>).expectedGrepSuccess ?? 0.5) : 0.5 // Real mode not implemented

  const passed = grepSuccess <= thresholds.grepMaxSuccess

  return {
    dimension: 'Construct Validity (Grep Baseline)',
    passed,
    actual: grepSuccess,
    expected: `≤ ${thresholds.grepMaxSuccess}`,
    details: passed
      ? `Grep success rate ${(grepSuccess * 100).toFixed(0)}% meets threshold ≤ ${(thresholds.grepMaxSuccess * 100).toFixed(0)}%. Task is appropriately difficult for keyword search.`
      : `Grep success rate ${(grepSuccess * 100).toFixed(0)}% exceeds threshold ${(thresholds.grepMaxSuccess * 100).toFixed(0)}%. Task may be too easy for grep. Consider making it more challenging by requiring deeper semantic understanding.`,
  }
}

/**
 * Validate Dimension 2: Discriminant Validity (Search Advantage)
 *
 * Ensures semantic search provides meaningful advantage over grep.
 * Checks both absolute performance and relative improvement.
 *
 * @param task - The task to validate
 * @param thresholds - Tier-specific thresholds
 * @param useMockData - Use mock data (default: true)
 */
function validateDiscriminantValidity(
  task: SearchTask,
  thresholds: TierThresholds,
  useMockData = true,
): DimensionResult {
  // In mock mode, use expected success rates
  const searchSuccess = useMockData
    ? ((task as SearchTask & Record<string, unknown>).expectedSearchSuccess ?? 0.5)
    : 0.5 // Real mode not implemented

  const grepSuccess = useMockData ? ((task as SearchTask & Record<string, unknown>).expectedGrepSuccess ?? 0.5) : 0.5

  const advantage = searchSuccess - grepSuccess

  // Check both absolute performance and improvement
  const searchGoodEnough = searchSuccess >= thresholds.searchMinSuccess
  const advantageLargeEnough = advantage >= thresholds.minAdvantage

  // Mock statistical significance (in real mode, would run t-test)
  const statisticallySignificant = advantage > 0.1 // Simple mock threshold

  const passed = searchGoodEnough && advantageLargeEnough && statisticallySignificant

  let details = ''
  if (passed) {
    details = `Search success ${(searchSuccess * 100).toFixed(0)}% (≥ ${(thresholds.searchMinSuccess * 100).toFixed(0)}%), improvement +${(advantage * 100).toFixed(0)}pp (≥ ${(thresholds.minAdvantage * 100).toFixed(0)}pp). Semantic search provides clear advantage.`
  } else {
    const issues: string[] = []
    if (!searchGoodEnough) {
      issues.push(
        `search success ${(searchSuccess * 100).toFixed(0)}% below threshold ${(thresholds.searchMinSuccess * 100).toFixed(0)}%`,
      )
    }
    if (!advantageLargeEnough) {
      issues.push(
        `improvement +${(advantage * 100).toFixed(0)}pp below threshold ${(thresholds.minAdvantage * 100).toFixed(0)}pp`,
      )
    }
    if (!statisticallySignificant) {
      issues.push('difference not statistically significant')
    }
    details = `Discriminant validity issues: ${issues.join(', ')}. Task does not demonstrate sufficient search advantage.`
  }

  return {
    dimension: 'Discriminant Validity (Search Advantage)',
    passed,
    actual: `${(searchSuccess * 100).toFixed(0)}% (Δ +${(advantage * 100).toFixed(0)}pp)`,
    expected: `≥ ${(thresholds.searchMinSuccess * 100).toFixed(0)}% (Δ ≥ ${(thresholds.minAdvantage * 100).toFixed(0)}pp)`,
    details,
  }
}

/**
 * Validate Dimension 3: Ecological Validity (Realism)
 *
 * Ensures the task reflects realistic developer scenarios.
 * Uses comprehensive ecological validation module.
 * In mock mode: Checks for realism indicators in task definition.
 *
 * @param task - The task to validate
 */
function validateEcologicalValidity(task: SearchTask): DimensionResult {
  // Use comprehensive ecological validation module
  const ecologicalResult = validateEcologicalValidityFull(task)

  return {
    dimension: 'Ecological Validity (Realism)',
    passed: ecologicalResult.passed,
    actual: `Score: ${(ecologicalResult.score * 100).toFixed(0)}%, Freq: ${ecologicalResult.checks.frequency}`,
    expected: 'Score ≥ 60%, realistic scenario',
    details: ecologicalResult.passed
      ? `Task passed ecological validation (${(ecologicalResult.score * 100).toFixed(0)}%). ${ecologicalResult.checks.basedOnRealScenario ? 'Real scenario' : 'Concrete task'}, ${ecologicalResult.checks.frequency} frequency, ${ecologicalResult.checks.objectiveSuccessCriteria ? 'objective criteria' : 'some subjectivity'}.`
      : `Task failed ecological validation (${(ecologicalResult.score * 100).toFixed(0)}%). Issues: ${ecologicalResult.failureReasons?.join(', ')}.`,
  }
}

/**
 * Validate Dimension 4: Test-Retest Reliability (Variance)
 *
 * Ensures the task produces consistent results across runs.
 * In mock mode: Checks for reliability indicators (deterministic validator).
 * In real mode: Would run multiple iterations and calculate variance.
 *
 * @param task - The task to validate
 * @param iterations - Number of test iterations
 * @param useMockData - Use mock data (default: true)
 */
function validateReliability(task: SearchTask, iterations: number, useMockData = true): DimensionResult {
  if (useMockData) {
    // Mock reliability based on validator type
    // Tasks with objective validators (code_change, file_creation) are more reliable
    const validatorType = task.followUpTask.validator.type
    const hasObjectiveValidator = validatorType !== 'explanation'

    // Mock coefficient of variation (CV)
    const mockCV = hasObjectiveValidator ? 0.05 : 0.12 // 5% vs 12%

    const passed = mockCV <= 0.1 // 10% threshold

    return {
      dimension: 'Test-Retest Reliability',
      passed,
      actual: `CV = ${(mockCV * 100).toFixed(1)}%`,
      expected: 'CV ≤ 10%',
      details: passed
        ? `Task has ${hasObjectiveValidator ? 'objective' : 'subjective'} validator. Mocked CV ${(mockCV * 100).toFixed(1)}% indicates reliable results. Actual variance would need ${iterations} iterations to measure.`
        : `Task has subjective validator. Mocked CV ${(mockCV * 100).toFixed(1)}% suggests high variance. Consider making validation criteria more objective.`,
    }
  }

  // Real mode not implemented
  return {
    dimension: 'Test-Retest Reliability',
    passed: false,
    actual: 'Not measured',
    expected: 'CV ≤ 10%',
    details: `Real reliability testing requires ${iterations} actual benchmark runs. Use useMockData: true for infrastructure validation.`,
  }
}

/**
 * Validate Dimension 5: Statistical Power (Sample Size)
 *
 * Ensures adequate sample size for detecting differences.
 * Simple check: n >= 5 for basic statistical power.
 *
 * @param iterations - Number of planned iterations
 */
function validateStatisticalPower(iterations: number): DimensionResult {
  const minimumIterations = 5
  const passed = iterations >= minimumIterations

  return {
    dimension: 'Statistical Power',
    passed,
    actual: `n = ${iterations}`,
    expected: `n ≥ ${minimumIterations}`,
    details: passed
      ? `Sample size n = ${iterations} provides adequate statistical power for detecting meaningful differences.`
      : `Sample size n = ${iterations} is below minimum ${minimumIterations}. Increase iterations to improve statistical power.`,
  }
}

/**
 * Generate actionable recommendations based on validation results
 *
 * @param result - The validation result
 * @returns Array of recommendations for improving task quality
 */
function generateRecommendations(result: ValidationResult): string[] {
  const recommendations: string[] = []

  // Construct validity issues
  if (!result.dimensions.constructValidity.passed) {
    recommendations.push(
      'Task is too easy for grep. Consider: (1) Requiring transitive relationships instead of direct matches, (2) Using conceptual queries without obvious keywords, (3) Adding ambiguity that requires semantic understanding.',
    )
  }

  // Discriminant validity issues
  if (!result.dimensions.discriminantValidity.passed) {
    const searchSuccess = parseFloat(result.dimensions.discriminantValidity.actual.toString())
    if (searchSuccess < 60) {
      recommendations.push(
        'Search success rate too low. Consider: (1) Is the task too hard even for semantic search? (2) Do available tools support this task? (3) Is the success validator too strict?',
      )
    } else {
      recommendations.push(
        'Search advantage too small. Consider: (1) Making task harder for grep (add indirection), (2) Adding semantic complexity that helps search, (3) Clarifying what makes this task valuable.',
      )
    }
  }

  // Ecological validity issues
  if (!result.dimensions.ecologicalValidity.passed) {
    recommendations.push(
      'Task may not reflect real-world usage. Consider: (1) Base task on actual developer scenarios, (2) Add context explaining when this matters, (3) Mark with basedOnRealScenario: true if applicable.',
    )
  }

  // Reliability issues
  if (!result.dimensions.reliability.passed) {
    recommendations.push(
      'Task may produce inconsistent results. Consider: (1) Make validator more objective (prefer code_change over explanation), (2) Tighten success criteria, (3) Reduce ambiguity in task description.',
    )
  }

  // Statistical power issues
  if (!result.dimensions.statisticalPower.passed) {
    recommendations.push('Increase iterations to at least 5 for adequate statistical power.')
  }

  // If everything passed, provide optimization suggestions
  if (result.passed) {
    recommendations.push(
      'Task passed all validation criteria. Consider: (1) Adding to benchmark suite, (2) Testing with actual agents to validate metrics, (3) Creating variants for different difficulty levels.',
    )
  }

  return recommendations
}

/**
 * Validate a search task across all 5 quality dimensions
 *
 * This is the main entry point for task validation.
 *
 * MOCK MODE (default, recommended):
 * - Uses task.expectedGrepSuccess and task.expectedSearchSuccess
 * - Fast, deterministic, suitable for CI/CD
 * - Validates task design quality without expensive execution
 *
 * REAL MODE (manual only):
 * - Requires running actual benchmarks with LLM agents
 * - Expensive (API costs)
 * - Not implemented in this phase
 *
 * @param config - Validation configuration
 * @returns Validation result with pass/fail and recommendations
 */
export async function validateTask(config: ValidationConfig): Promise<ValidationResult> {
  const { task, tier, iterations = 5, thresholds = DEFAULT_THRESHOLDS, useMockData = true } = config

  // Get tier-specific thresholds
  const tierKey = tier.split('-')[0] as 'tier1' | 'tier2' | 'tier3'
  const tierThresholds = thresholds[tierKey]

  // Validate each dimension
  const constructValidity = validateConstructValidity(task, tierThresholds, useMockData)
  const discriminantValidity = validateDiscriminantValidity(task, tierThresholds, useMockData)
  const ecologicalValidity = validateEcologicalValidity(task)
  const reliability = validateReliability(task, iterations, useMockData)
  const statisticalPower = validateStatisticalPower(iterations)

  // Overall pass/fail
  const passed =
    constructValidity.passed &&
    discriminantValidity.passed &&
    ecologicalValidity.passed &&
    reliability.passed &&
    statisticalPower.passed

  const result: ValidationResult = {
    task,
    passed,
    tier,
    dimensions: {
      constructValidity,
      discriminantValidity,
      ecologicalValidity,
      reliability,
      statisticalPower,
    },
    recommendations: [],
    timestamp: new Date(),
  }

  // Generate recommendations
  result.recommendations = generateRecommendations(result)

  return result
}

/**
 * Validate an entire benchmark suite
 *
 * Validates all tasks in the suite and provides aggregate statistics.
 *
 * @param suite - The benchmark suite to validate
 * @param config - Optional configuration (iterations, mock mode)
 * @returns Suite validation result with aggregate statistics
 */
export async function validateSuite(
  suite: BenchmarkSuite,
  config?: { iterations?: number; useMockData?: boolean },
): Promise<SuiteValidationResult> {
  const iterations = config?.iterations ?? 5
  const useMockData = config?.useMockData ?? true

  // Determine tier from suite
  const tierMap: Record<number, ValidationConfig['tier']> = {
    1: 'tier1-impossible',
    2: 'tier2-hard',
    3: 'tier3-realworld',
  }
  const tier = tierMap[suite.tier] ?? 'tier1-impossible'

  // Validate each task
  const taskResults: ValidationResult[] = []
  for (const task of suite.tasks) {
    const result = await validateTask({
      task,
      tier,
      iterations,
      useMockData,
    })
    taskResults.push(result)
  }

  // Calculate aggregate statistics
  const totalTasks = taskResults.length
  const passedTasks = taskResults.filter((r) => r.passed).length
  const failedTasks = totalTasks - passedTasks
  const passed = failedTasks === 0

  // Generate summary
  const passRate = (passedTasks / totalTasks) * 100
  let summary = `Suite validation: ${passedTasks}/${totalTasks} tasks passed (${passRate.toFixed(0)}%). `

  if (passed) {
    summary += 'All tasks meet quality criteria. Suite is ready for benchmarking.'
  } else {
    // Categorize failures
    const failuresByDimension = new Map<string, number>()
    for (const result of taskResults) {
      if (!result.passed) {
        for (const [dimName, dimResult] of Object.entries(result.dimensions)) {
          if (!dimResult.passed) {
            failuresByDimension.set(dimName, (failuresByDimension.get(dimName) ?? 0) + 1)
          }
        }
      }
    }

    const topFailures = Array.from(failuresByDimension.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 3)
      .map(([dim, count]) => `${dim} (${count})`)

    summary += `Common issues: ${topFailures.join(', ')}. Review recommendations for failed tasks.`
  }

  return {
    suite,
    passed,
    totalTasks,
    passedTasks,
    failedTasks,
    taskResults,
    summary,
  }
}

/**
 * Format a validation result as a human-readable report
 *
 * @param result - The validation result to format
 * @returns Formatted report string
 */
export function formatValidationReport(result: ValidationResult): string {
  const lines: string[] = []

  // Header
  lines.push('='.repeat(80))
  lines.push(`Task Validation Report: ${result.task.name}`)
  lines.push('='.repeat(80))
  lines.push('')

  // Overall result
  lines.push(`Status: ${result.passed ? '✓ PASSED' : '✗ FAILED'}`)
  lines.push(`Task ID: ${result.task.id}`)
  lines.push(`Category: ${result.task.category}`)
  lines.push(`Difficulty: ${result.task.difficulty}`)
  lines.push(`Tier: ${result.tier}`)
  lines.push(`Timestamp: ${result.timestamp.toISOString()}`)
  lines.push('')

  // Dimensions
  lines.push('Validation Dimensions:')
  lines.push('-'.repeat(80))

  for (const [_name, dimension] of Object.entries(result.dimensions)) {
    const status = dimension.passed ? '✓' : '✗'
    lines.push(`\n[${status}] ${dimension.dimension}`)
    lines.push(`    Actual:   ${dimension.actual}`)
    lines.push(`    Expected: ${dimension.expected}`)
    lines.push(`    Details:  ${dimension.details}`)
  }

  // Recommendations
  if (result.recommendations.length > 0) {
    lines.push('')
    lines.push('Recommendations:')
    lines.push('-'.repeat(80))
    result.recommendations.forEach((rec, i) => {
      lines.push(`${i + 1}. ${rec}`)
    })
  }

  lines.push('')
  lines.push('='.repeat(80))

  return lines.join('\n')
}

/**
 * Format a suite validation result as a human-readable report
 *
 * @param result - The suite validation result to format
 * @returns Formatted report string
 */
export function formatSuiteValidationReport(result: SuiteValidationResult): string {
  const lines: string[] = []

  // Header
  lines.push('='.repeat(80))
  lines.push(`Suite Validation Report: ${result.suite.name}`)
  lines.push('='.repeat(80))
  lines.push('')

  // Overall result
  lines.push(`Status: ${result.passed ? '✓ ALL PASSED' : '✗ SOME FAILED'}`)
  lines.push(`Total Tasks: ${result.totalTasks}`)
  lines.push(`Passed: ${result.passedTasks}`)
  lines.push(`Failed: ${result.failedTasks}`)
  lines.push(`Pass Rate: ${((result.passedTasks / result.totalTasks) * 100).toFixed(1)}%`)
  lines.push('')
  lines.push(`Summary: ${result.summary}`)
  lines.push('')

  // Task-by-task results
  lines.push('Task Results:')
  lines.push('-'.repeat(80))

  for (const taskResult of result.taskResults) {
    const status = taskResult.passed ? '✓' : '✗'
    lines.push(`${status} ${taskResult.task.id}: ${taskResult.task.name}`)

    if (!taskResult.passed) {
      // Show failed dimensions
      const failedDims = Object.values(taskResult.dimensions)
        .filter((d) => !d.passed)
        .map((d) => d.dimension)
      lines.push(`   Failed: ${failedDims.join(', ')}`)
    }
  }

  lines.push('')

  // Failed task details
  const failedTasks = result.taskResults.filter((r) => !r.passed)
  if (failedTasks.length > 0) {
    lines.push('')
    lines.push('Failed Task Details:')
    lines.push('-'.repeat(80))

    for (const taskResult of failedTasks) {
      lines.push('')
      lines.push(`Task: ${taskResult.task.name} (${taskResult.task.id})`)

      for (const dimension of Object.values(taskResult.dimensions)) {
        if (!dimension.passed) {
          lines.push(`  ✗ ${dimension.dimension}`)
          lines.push(`    ${dimension.details}`)
        }
      }

      if (taskResult.recommendations.length > 0) {
        lines.push('  Recommendations:')
        taskResult.recommendations.forEach((rec) => {
          lines.push(`    • ${rec}`)
        })
      }
    }
  }

  lines.push('')
  lines.push('='.repeat(80))

  return lines.join('\n')
}
