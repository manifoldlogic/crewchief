/**
 * Evaluation report generation
 */

import type { SearchEvaluationSummary } from './checks.js'

/**
 * Generate a human-readable evaluation report
 *
 * @param summary - The evaluation summary
 * @returns Formatted report string
 */
export function generateEvaluationReport(summary: SearchEvaluationSummary): string {
  const sections: string[] = []

  // Header
  sections.push('Search Task Evaluation Report')
  sections.push('='.repeat(60))
  sections.push('')

  // Task info
  sections.push(`Task: ${summary.task.name}`)
  sections.push(`ID: ${summary.task.id}`)
  sections.push(`Difficulty: ${summary.task.difficulty}`)
  sections.push(`Category: ${summary.task.category}`)
  sections.push('')

  // Composite score
  sections.push(`Composite Score: ${formatPercentage(summary.compositeScore)}`)
  sections.push('')

  // Score breakdown
  sections.push('Score Breakdown:')
  sections.push(`  - Search Quality:   ${formatPercentage(summary.taskScore.searchQuality)} (40% weight)`)
  sections.push(`  - Task Completion:  ${formatPercentage(summary.taskScore.taskCompletion)} (40% weight)`)
  sections.push(`  - Efficiency:       ${formatPercentage(summary.taskScore.efficiency)} (20% weight)`)
  sections.push('')

  // Search metrics
  sections.push('Search Metrics:')
  sections.push(`  - Searches Performed: ${summary.searchMetrics.searchCount}`)
  sections.push(`  - Avg Results/Search: ${summary.searchMetrics.avgResultsPerSearch.toFixed(1)}`)
  sections.push(`  - Target Found: ${summary.searchMetrics.targetFound ? 'YES' : 'NO'}`)
  if (summary.searchMetrics.targetFoundInTop !== null) {
    sections.push(`  - Target Rank: #${summary.searchMetrics.targetFoundInTop}`)
  }
  if (summary.searchMetrics.queriesIssued.length > 0) {
    sections.push(
      `  - Queries: ${summary.searchMetrics.queriesIssued.slice(0, 3).join(', ')}${summary.searchMetrics.queriesIssued.length > 3 ? '...' : ''}`,
    )
  }
  sections.push('')

  // Tool usage
  sections.push('Tool Usage:')
  sections.push(`  - Total Tool Calls: ${summary.toolUsage.totalToolCalls}`)
  sections.push(`  - Search Tool: ${summary.toolUsage.searchToolCalls}`)
  const otherTools = Object.entries(summary.toolUsage.otherToolCalls)
  if (otherTools.length > 0) {
    sections.push('  - Other Tools:')
    otherTools
      .sort(([, a], [, b]) => b - a)
      .slice(0, 5)
      .forEach(([tool, count]) => {
        sections.push(`    - ${tool}: ${count}`)
      })
  }
  sections.push('')

  // Timing
  sections.push('Timing:')
  sections.push(`  - Total Duration: ${formatDuration(summary.timing.totalSeconds)}`)
  if (summary.timing.timeToTarget !== null) {
    sections.push(`  - Time to Target: ${formatDuration(summary.timing.timeToTarget)}`)
  }
  sections.push('')

  // Generic checks
  if (summary.results.length > 0) {
    sections.push('Quality Checks:')
    summary.results.forEach((check) => {
      const status = check.passed ? '✓' : '✗'
      sections.push(`  ${status} ${check.name}${check.details ? ` - ${check.details}` : ''}`)
    })
    sections.push('')
  }

  // Details
  sections.push('Details:')
  sections.push(summary.taskScore.details)
  sections.push('')

  return sections.join('\n')
}

/**
 * Generate a compact summary line
 *
 * @param summary - The evaluation summary
 * @returns Single-line summary
 */
export function generateCompactSummary(summary: SearchEvaluationSummary): string {
  const score = formatPercentage(summary.compositeScore)
  const found = summary.searchMetrics.targetFound ? '✓' : '✗'
  const searches = summary.searchMetrics.searchCount
  const duration = formatDuration(summary.timing.totalSeconds)

  return `[${score}] ${summary.task.name} - Target: ${found}, Searches: ${searches}, Time: ${duration}`
}

/**
 * Generate a JSON report
 *
 * @param summary - The evaluation summary
 * @returns JSON string
 */
export function generateJsonReport(summary: SearchEvaluationSummary): string {
  return JSON.stringify(summary, null, 2)
}

/**
 * Generate a CSV row for comparison
 *
 * @param summary - The evaluation summary
 * @returns CSV row
 */
export function generateCsvRow(summary: SearchEvaluationSummary): string {
  return [
    summary.task.id,
    summary.task.name,
    summary.compositeScore.toFixed(3),
    summary.taskScore.searchQuality.toFixed(3),
    summary.taskScore.taskCompletion.toFixed(3),
    summary.taskScore.efficiency.toFixed(3),
    summary.searchMetrics.searchCount,
    summary.searchMetrics.targetFound ? 'yes' : 'no',
    summary.searchMetrics.targetFoundInTop || 'N/A',
    summary.toolUsage.totalToolCalls,
    summary.timing.totalSeconds.toFixed(1),
  ].join(',')
}

/**
 * Get CSV header
 */
export function getCsvHeader(): string {
  return [
    'task_id',
    'task_name',
    'composite_score',
    'search_quality',
    'task_completion',
    'efficiency',
    'search_count',
    'target_found',
    'target_rank',
    'total_tool_calls',
    'duration_seconds',
  ].join(',')
}

/**
 * Format percentage
 */
function formatPercentage(value: number): string {
  return `${(value * 100).toFixed(1)}%`
}

/**
 * Format duration in seconds
 */
function formatDuration(seconds: number): string {
  if (seconds < 60) {
    return `${seconds.toFixed(1)}s`
  }

  const minutes = Math.floor(seconds / 60)
  const remainingSeconds = seconds % 60

  return `${minutes}m ${remainingSeconds.toFixed(0)}s`
}
