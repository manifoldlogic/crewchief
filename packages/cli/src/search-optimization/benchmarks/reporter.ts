/**
 * Suite Reporter Module
 *
 * Generates comprehensive reports about benchmark suites including:
 * - Executive summary
 * - Task-by-task breakdown
 * - Category analysis
 * - Validation status
 *
 * Reports can be formatted as markdown for documentation or
 * as structured data for programmatic analysis.
 */

import type { BenchmarkSuite, CategoryStatistics } from './tier1-impossible.js'
import { getSuiteStatistics } from './tier1-impossible.js'
import { validateSuiteComposition, type ValidationResult } from './validation.js'

/**
 * Summary information about the suite
 */
export interface SuiteSummary {
  /** Total number of tasks */
  totalTasks: number

  /** Unique categories represented */
  categories: string[]

  /** Average expected grep success rate */
  expectedGrepSuccess: number

  /** Average expected search success rate */
  expectedSearchSuccess: number

  /** Expected improvement (search - grep) */
  expectedImprovement: number

  /** Percentage of tasks that defeat grep (<30% grep success) */
  grepFailureRate: number
}

/**
 * Information about a single task
 */
export interface TaskInfo {
  /** Task ID */
  id: string

  /** Task name */
  name: string

  /** Task category */
  category: string

  /** Task difficulty */
  difficulty: string

  /** Expected grep success rate */
  expectedGrepSuccess: number

  /** Expected search success rate */
  expectedSearchSuccess: number

  /** Improvement (search - grep) */
  improvement: number
}

/**
 * Complete suite report
 */
export interface SuiteReport {
  /** Suite name */
  suiteName: string

  /** Suite version */
  suiteVersion: string

  /** Summary statistics */
  summary: SuiteSummary

  /** Task-by-task breakdown */
  taskBreakdown: TaskInfo[]

  /** Category-level statistics */
  categoryBreakdown: Map<string, CategoryStatistics>

  /** Validation results */
  validation: ValidationResult
}

/**
 * Generate a comprehensive report for a benchmark suite
 *
 * @param suite - The benchmark suite to analyze
 * @returns Complete report with summary, tasks, categories, and validation
 */
export function generateSuiteReport(suite: BenchmarkSuite): SuiteReport {
  const statistics = getSuiteStatistics(suite)
  const validation = validateSuiteComposition(suite)

  // Generate task breakdown
  const taskBreakdown: TaskInfo[] = suite.tasks.map((task) => {
    const grepSuccess = (task as any).expectedGrepSuccess ?? 0
    const searchSuccess = (task as any).expectedSearchSuccess ?? 0

    return {
      id: task.id,
      name: task.name,
      category: task.category,
      difficulty: task.difficulty,
      expectedGrepSuccess: grepSuccess,
      expectedSearchSuccess: searchSuccess,
      improvement: searchSuccess - grepSuccess,
    }
  })

  // Sort tasks by improvement (highest first) for better presentation
  taskBreakdown.sort((a, b) => b.improvement - a.improvement)

  return {
    suiteName: suite.name,
    suiteVersion: suite.version,
    summary: {
      totalTasks: suite.tasks.length,
      categories: Array.from(statistics.byCategory.keys()).sort(),
      expectedGrepSuccess: statistics.overallGrepSuccess,
      expectedSearchSuccess: statistics.overallSearchSuccess,
      expectedImprovement: statistics.expectedImprovement,
      grepFailureRate: validation.grepFailureRate,
    },
    taskBreakdown,
    categoryBreakdown: statistics.byCategory,
    validation,
  }
}

/**
 * Format a suite report as markdown
 *
 * @param report - The suite report to format
 * @returns Markdown-formatted report string
 */
export function formatReportMarkdown(report: SuiteReport): string {
  const lines: string[] = []

  // Header
  lines.push(`# ${report.suiteName}`)
  lines.push('')
  lines.push(`**Version:** ${report.suiteVersion}`)
  lines.push('')

  // Executive Summary
  lines.push('## Executive Summary')
  lines.push('')
  lines.push(`- **Total Tasks:** ${report.summary.totalTasks}`)
  lines.push(`- **Categories:** ${report.summary.categories.join(', ')}`)
  lines.push(`- **Expected Grep Success:** ${(report.summary.expectedGrepSuccess * 100).toFixed(1)}%`)
  lines.push(`- **Expected Search Success:** ${(report.summary.expectedSearchSuccess * 100).toFixed(1)}%`)
  lines.push(`- **Expected Improvement:** ${(report.summary.expectedImprovement * 100).toFixed(1)}%`)
  lines.push(`- **Grep Failure Rate:** ${(report.summary.grepFailureRate * 100).toFixed(0)}%`)
  lines.push('')

  // Validation Status
  lines.push('## Validation Status')
  lines.push('')
  lines.push(`**Status:** ${report.validation.passed ? '✓ PASSED' : '✗ FAILED'}`)
  lines.push('')

  if (report.validation.failingTasks.length > 0) {
    const errors = report.validation.failingTasks.filter((f) => f.severity === 'error')
    const warnings = report.validation.failingTasks.filter((f) => f.severity === 'warning')

    if (errors.length > 0) {
      lines.push(`**Errors:** ${errors.length}`)
      for (const error of errors) {
        lines.push(`- ${error.task.id}: ${error.reason}`)
      }
      lines.push('')
    }

    if (warnings.length > 0) {
      lines.push(`**Warnings:** ${warnings.length}`)
      for (const warning of warnings) {
        lines.push(`- ${warning.task.id}: ${warning.reason}`)
      }
      lines.push('')
    }
  }

  if (report.validation.recommendations.length > 0) {
    lines.push('### Recommendations')
    lines.push('')
    for (const rec of report.validation.recommendations) {
      lines.push(`- ${rec}`)
    }
    lines.push('')
  }

  // Task Breakdown
  lines.push('## Task Breakdown')
  lines.push('')
  lines.push('| Task ID | Name | Category | Difficulty | Grep | Search | Improvement |')
  lines.push('|---------|------|----------|------------|------|--------|-------------|')

  for (const task of report.taskBreakdown) {
    lines.push(
      `| ${task.id} | ${task.name} | ${task.category} | ${task.difficulty} | ` +
        `${(task.expectedGrepSuccess * 100).toFixed(0)}% | ` +
        `${(task.expectedSearchSuccess * 100).toFixed(0)}% | ` +
        `+${(task.improvement * 100).toFixed(0)}% |`,
    )
  }
  lines.push('')

  // Category Breakdown
  lines.push('## Category Breakdown')
  lines.push('')

  for (const [category, stats] of report.categoryBreakdown) {
    lines.push(`### ${category}`)
    lines.push('')
    lines.push(`- **Tasks:** ${stats.taskCount}`)
    lines.push(`- **Avg Grep Success:** ${(stats.avgGrepSuccess * 100).toFixed(1)}%`)
    lines.push(`- **Avg Search Success:** ${(stats.avgSearchSuccess * 100).toFixed(1)}%`)
    lines.push(`- **Avg Improvement:** ${((stats.avgSearchSuccess - stats.avgGrepSuccess) * 100).toFixed(1)}%`)
    lines.push(`- **Task IDs:** ${stats.taskIds.join(', ')}`)
    lines.push('')
  }

  return lines.join('\n')
}

/**
 * Format a suite report as plain text (for console output)
 *
 * @param report - The suite report to format
 * @returns Plain text formatted report string
 */
export function formatReportText(report: SuiteReport): string {
  const lines: string[] = []

  // Header
  lines.push('='.repeat(80))
  lines.push(report.suiteName)
  lines.push(`Version: ${report.suiteVersion}`)
  lines.push('='.repeat(80))
  lines.push('')

  // Summary
  lines.push('SUMMARY')
  lines.push('-'.repeat(80))
  lines.push(`Total Tasks:           ${report.summary.totalTasks}`)
  lines.push(`Categories:            ${report.summary.categories.join(', ')}`)
  lines.push(`Expected Grep Success: ${(report.summary.expectedGrepSuccess * 100).toFixed(1)}%`)
  lines.push(`Expected Search:       ${(report.summary.expectedSearchSuccess * 100).toFixed(1)}%`)
  lines.push(`Expected Improvement:  +${(report.summary.expectedImprovement * 100).toFixed(1)}%`)
  lines.push(`Grep Failure Rate:     ${(report.summary.grepFailureRate * 100).toFixed(0)}%`)
  lines.push('')

  // Validation
  lines.push('VALIDATION')
  lines.push('-'.repeat(80))
  lines.push(`Status: ${report.validation.passed ? 'PASSED' : 'FAILED'}`)

  if (report.validation.failingTasks.length > 0) {
    lines.push(`Issues: ${report.validation.failingTasks.length}`)
  }

  if (report.validation.recommendations.length > 0) {
    lines.push('')
    lines.push('Recommendations:')
    for (const rec of report.validation.recommendations) {
      lines.push(`  - ${rec}`)
    }
  }
  lines.push('')

  // Task breakdown
  lines.push('TASKS')
  lines.push('-'.repeat(80))

  // Calculate column widths for alignment
  const maxIdLen = Math.max(...report.taskBreakdown.map((t) => t.id.length), 'Task ID'.length)

  for (const task of report.taskBreakdown) {
    const improvement =
      task.improvement >= 0 ? `+${(task.improvement * 100).toFixed(0)}%` : `${(task.improvement * 100).toFixed(0)}%`

    lines.push(
      `${task.id.padEnd(maxIdLen)} | ` +
        `${task.category.padEnd(25)} | ` +
        `Grep: ${(task.expectedGrepSuccess * 100).toFixed(0).padStart(3)}% | ` +
        `Search: ${(task.expectedSearchSuccess * 100).toFixed(0).padStart(3)}% | ` +
        `${improvement.padStart(5)}`,
    )
  }
  lines.push('')

  // Category breakdown
  lines.push('CATEGORIES')
  lines.push('-'.repeat(80))

  for (const [category, stats] of report.categoryBreakdown) {
    lines.push(`${category}:`)
    lines.push(`  Tasks: ${stats.taskCount}`)
    lines.push(
      `  Grep: ${(stats.avgGrepSuccess * 100).toFixed(1)}% → Search: ${(stats.avgSearchSuccess * 100).toFixed(1)}% (${((stats.avgSearchSuccess - stats.avgGrepSuccess) * 100).toFixed(1)}% improvement)`,
    )
    lines.push('')
  }

  return lines.join('\n')
}

/**
 * Generate a compact one-line summary
 *
 * @param report - The suite report
 * @returns One-line summary string
 */
export function formatCompactSummary(report: SuiteReport): string {
  return (
    `${report.suiteName} v${report.suiteVersion}: ` +
    `${report.summary.totalTasks} tasks, ` +
    `${(report.summary.expectedGrepSuccess * 100).toFixed(0)}% grep → ${(report.summary.expectedSearchSuccess * 100).toFixed(0)}% search ` +
    `(+${(report.summary.expectedImprovement * 100).toFixed(0)}%), ` +
    `${report.validation.passed ? 'VALID' : 'INVALID'}`
  )
}
