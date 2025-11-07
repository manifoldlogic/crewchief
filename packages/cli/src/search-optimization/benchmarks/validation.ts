/**
 * Suite Validation Module
 *
 * Validates that benchmark suites meet quality criteria:
 * - All tasks have required fields
 * - 80%+ tasks are truly grep-impossible (<30% grep success)
 * - Categories are properly represented
 * - Tasks have diverse difficulty levels
 *
 * Validation is static analysis - it checks suite composition,
 * not actual task execution (which is expensive).
 */

import type { BenchmarkSuite } from './tier1-impossible.js'
import type { SearchTask } from '../types.js'

/**
 * Result of suite validation
 */
export interface ValidationResult {
  /** Whether the suite passes all validation checks */
  passed: boolean

  /** Percentage of tasks that defeat grep (expectedGrepSuccess < 0.3) */
  grepFailureRate: number

  /** Number of unique categories represented */
  categoryCoverage: number

  /** Tasks that successfully defeat grep */
  tasksDefeatingGrep: SearchTask[]

  /** Tasks that fail validation criteria */
  failingTasks: ValidationFailure[]

  /** Recommendations for improving the suite */
  recommendations: string[]
}

/**
 * Details about a task that failed validation
 */
export interface ValidationFailure {
  /** The task that failed */
  task: SearchTask

  /** Reason for failure */
  reason: string

  /** Severity of the failure */
  severity: 'error' | 'warning'
}

/**
 * Threshold for what counts as "grep-impossible"
 * Tasks with expectedGrepSuccess < 0.3 are considered grep-impossible
 */
const GREP_IMPOSSIBLE_THRESHOLD = 0.3

/**
 * Minimum percentage of tasks that should defeat grep in a Tier 1 suite
 */
const MIN_GREP_DEFEATING_PERCENTAGE = 0.8

/**
 * Minimum number of categories that should be represented
 */
const MIN_CATEGORY_COVERAGE = 3

/**
 * Validate that a task has all required fields
 */
function validateTaskFields(task: SearchTask): ValidationFailure | null {
  const missingFields: string[] = []

  if (!task.id) missingFields.push('id')
  if (!task.name) missingFields.push('name')
  if (!task.description) missingFields.push('description')
  if (!task.category) missingFields.push('category')
  if (!task.difficulty) missingFields.push('difficulty')
  if (!task.searchTarget) missingFields.push('searchTarget')
  if (!task.followUpTask) missingFields.push('followUpTask')
  if (!task.successValidator) missingFields.push('successValidator')

  // Check for expected success rates
  const hasExpectedRates = 'expectedGrepSuccess' in task && 'expectedSearchSuccess' in task

  if (!hasExpectedRates) {
    missingFields.push('expectedGrepSuccess', 'expectedSearchSuccess')
  }

  if (missingFields.length > 0) {
    return {
      task,
      reason: `Missing required fields: ${missingFields.join(', ')}`,
      severity: 'error',
    }
  }

  return null
}

/**
 * Validate that a task truly defeats grep
 */
function validateGrepDifficulty(task: SearchTask): ValidationFailure | null {
  const grepSuccess = (task as any).expectedGrepSuccess

  if (grepSuccess === undefined) {
    return {
      task,
      reason: 'Missing expectedGrepSuccess field',
      severity: 'error',
    }
  }

  if (grepSuccess >= GREP_IMPOSSIBLE_THRESHOLD) {
    return {
      task,
      reason: `expectedGrepSuccess (${grepSuccess}) is too high for grep-impossible task (should be < ${GREP_IMPOSSIBLE_THRESHOLD})`,
      severity: 'warning',
    }
  }

  return null
}

/**
 * Validate that search provides meaningful improvement
 */
function validateSearchAdvantage(task: SearchTask): ValidationFailure | null {
  const grepSuccess = (task as any).expectedGrepSuccess
  const searchSuccess = (task as any).expectedSearchSuccess

  if (searchSuccess === undefined) {
    return {
      task,
      reason: 'Missing expectedSearchSuccess field',
      severity: 'error',
    }
  }

  const improvement = searchSuccess - grepSuccess

  if (improvement < 0.3) {
    return {
      task,
      reason: `Search advantage (${improvement.toFixed(2)}) is too small (should be >= 0.3)`,
      severity: 'warning',
    }
  }

  return null
}

/**
 * Validate the composition of a benchmark suite
 *
 * Checks that:
 * - All tasks have required fields
 * - 80%+ tasks have expectedGrepSuccess < 0.3
 * - At least 3 categories are represented
 * - Tasks provide meaningful search advantage
 *
 * @param suite - The benchmark suite to validate
 * @returns Validation result with pass/fail status and recommendations
 */
export function validateSuiteComposition(suite: BenchmarkSuite): ValidationResult {
  const failures: ValidationFailure[] = []
  const tasksDefeatingGrep: SearchTask[] = []

  // Validate each task
  for (const task of suite.tasks) {
    // Check required fields
    const fieldFailure = validateTaskFields(task)
    if (fieldFailure) {
      failures.push(fieldFailure)
      continue // Skip other checks if basic fields are missing
    }

    // Check grep difficulty
    const grepFailure = validateGrepDifficulty(task)
    if (grepFailure) {
      failures.push(grepFailure)
    } else {
      tasksDefeatingGrep.push(task)
    }

    // Check search advantage
    const searchFailure = validateSearchAdvantage(task)
    if (searchFailure) {
      failures.push(searchFailure)
    }
  }

  // Calculate metrics
  const grepFailureRate = tasksDefeatingGrep.length / suite.tasks.length
  const categories = new Set(suite.tasks.map((t) => t.category))
  const categoryCoverage = categories.size

  // Generate recommendations
  const recommendations: string[] = []

  if (grepFailureRate < MIN_GREP_DEFEATING_PERCENTAGE) {
    recommendations.push(
      `Only ${(grepFailureRate * 100).toFixed(0)}% of tasks defeat grep. ` +
        `Target: ${MIN_GREP_DEFEATING_PERCENTAGE * 100}%+. ` +
        'Consider adding more grep-impossible tasks or adjusting expectedGrepSuccess values.',
    )
  }

  if (categoryCoverage < MIN_CATEGORY_COVERAGE) {
    recommendations.push(
      `Only ${categoryCoverage} categories represented. ` +
        `Target: ${MIN_CATEGORY_COVERAGE}+. ` +
        'Add tasks from underrepresented categories for better coverage.',
    )
  }

  // Check for diversity in difficulty levels
  const difficulties = new Set(suite.tasks.map((t) => t.difficulty))
  if (difficulties.size < 2) {
    recommendations.push(
      'Tasks have limited difficulty diversity. ' + 'Consider adding tasks at different difficulty levels.',
    )
  }

  // Check for errors vs warnings
  const errors = failures.filter((f) => f.severity === 'error')
  const warnings = failures.filter((f) => f.severity === 'warning')

  if (errors.length > 0) {
    recommendations.push(`Fix ${errors.length} critical error(s) before using this suite.`)
  }

  if (warnings.length > 0 && errors.length === 0) {
    recommendations.push(`Address ${warnings.length} warning(s) to improve suite quality.`)
  }

  // Suite passes if:
  // - No critical errors
  // - At least 80% tasks defeat grep
  // - At least 3 categories represented
  const passed =
    errors.length === 0 && grepFailureRate >= MIN_GREP_DEFEATING_PERCENTAGE && categoryCoverage >= MIN_CATEGORY_COVERAGE

  return {
    passed,
    grepFailureRate,
    categoryCoverage,
    tasksDefeatingGrep,
    failingTasks: failures,
    recommendations,
  }
}

/**
 * Get a human-readable summary of validation results
 *
 * @param result - The validation result
 * @returns Formatted summary string
 */
export function formatValidationSummary(result: ValidationResult): string {
  const lines: string[] = []

  lines.push('Suite Validation Summary')
  lines.push('========================')
  lines.push('')
  lines.push(`Status: ${result.passed ? 'PASSED' : 'FAILED'}`)
  lines.push(
    `Grep Failure Rate: ${(result.grepFailureRate * 100).toFixed(0)}% (target: ${MIN_GREP_DEFEATING_PERCENTAGE * 100}%+)`,
  )
  lines.push(`Category Coverage: ${result.categoryCoverage} (target: ${MIN_CATEGORY_COVERAGE}+)`)
  lines.push(`Tasks Defeating Grep: ${result.tasksDefeatingGrep.length}`)
  lines.push('')

  if (result.failingTasks.length > 0) {
    lines.push('Issues:')
    lines.push('-------')

    const errors = result.failingTasks.filter((f) => f.severity === 'error')
    const warnings = result.failingTasks.filter((f) => f.severity === 'warning')

    if (errors.length > 0) {
      lines.push(`\nErrors (${errors.length}):`)
      for (const error of errors) {
        lines.push(`  - ${error.task.id}: ${error.reason}`)
      }
    }

    if (warnings.length > 0) {
      lines.push(`\nWarnings (${warnings.length}):`)
      for (const warning of warnings) {
        lines.push(`  - ${warning.task.id}: ${warning.reason}`)
      }
    }

    lines.push('')
  }

  if (result.recommendations.length > 0) {
    lines.push('Recommendations:')
    lines.push('----------------')
    for (const rec of result.recommendations) {
      lines.push(`- ${rec}`)
    }
    lines.push('')
  }

  return lines.join('\n')
}
