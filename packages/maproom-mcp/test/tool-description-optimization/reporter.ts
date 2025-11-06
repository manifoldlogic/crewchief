/**
 * Result Formatter and Reporter
 *
 * Outputs experiment results in multiple formats:
 * - JSON (for statistical analysis)
 * - Human-readable console output
 * - Markdown reports
 */

import type { VariantMetrics, CategoryMetrics, VariantComparison } from './metrics.js'

/**
 * Format metrics as JSON
 */
export function formatJSON(metrics: VariantMetrics | VariantMetrics[]): string {
  return JSON.stringify(metrics, null, 2)
}

/**
 * Format metrics as human-readable text
 */
export function formatText(metrics: VariantMetrics): string {
  const lines: string[] = []

  lines.push('=' .repeat(60))
  lines.push(`Variant: ${metrics.variant_name} (${metrics.variant_id})`)
  lines.push('='.repeat(60))
  lines.push('')

  lines.push('SUMMARY:')
  lines.push(`  Total Queries:        ${metrics.total_queries}`)
  lines.push(`  Successful Queries:   ${metrics.successful_queries} (${(metrics.success_rate * 100).toFixed(1)}%)`)
  lines.push(`  Avg Results/Query:    ${metrics.avg_result_count.toFixed(2)}`)
  lines.push(`  Avg Execution Time:   ${metrics.avg_execution_time_ms.toFixed(0)}ms`)
  lines.push(`  Avg Confidence:       ${(metrics.avg_transformation_confidence * 100).toFixed(1)}%`)
  lines.push(`  Total Time:           ${(metrics.total_execution_time_ms / 1000).toFixed(1)}s`)
  lines.push('')

  // Failed queries
  const failed = metrics.query_results.filter(r => !r.success)
  if (failed.length > 0) {
    lines.push(`FAILED QUERIES (${failed.length}):`)
    for (const result of failed.slice(0, 10)) {
      lines.push(`  ${result.query_id}: "${result.original_query}"`)
      lines.push(`    → "${result.transformed_query}" (${result.result_count} results)`)
    }
    if (failed.length > 10) {
      lines.push(`  ... and ${failed.length - 10} more`)
    }
    lines.push('')
  }

  // Top queries
  const top = [...metrics.query_results]
    .sort((a, b) => b.result_count - a.result_count)
    .slice(0, 5)

  lines.push('TOP 5 QUERIES BY RESULTS:')
  for (const result of top) {
    lines.push(`  ${result.query_id}: ${result.result_count} results`)
    lines.push(`    "${result.original_query}" → "${result.transformed_query}"`)
  }
  lines.push('')

  return lines.join('\n')
}

/**
 * Format category metrics
 */
export function formatCategoryMetrics(categoryMetrics: CategoryMetrics[]): string {
  const lines: string[] = []

  lines.push('METRICS BY CATEGORY:')
  lines.push('-'.repeat(60))

  for (const cat of categoryMetrics) {
    lines.push(`  ${cat.category}:`)
    lines.push(`    Success Rate:  ${(cat.success_rate * 100).toFixed(1)}% (${cat.successful_queries}/${cat.total_queries})`)
    lines.push(`    Avg Results:   ${cat.avg_result_count.toFixed(2)}`)
  }
  lines.push('')

  return lines.join('\n')
}

/**
 * Format comparison between variants
 */
export function formatComparison(comparison: VariantComparison): string {
  const lines: string[] = []

  lines.push('='.repeat(60))
  lines.push('VARIANT COMPARISON')
  lines.push('='.repeat(60))
  lines.push('')

  lines.push(`Variant A: ${comparison.variant_a}`)
  lines.push(`Variant B: ${comparison.variant_b}`)
  lines.push('')

  lines.push('DELTA (A - B):')
  lines.push(`  Success Rate:    ${comparison.success_rate_delta > 0 ? '+' : ''}${(comparison.success_rate_delta * 100).toFixed(1)}%`)
  lines.push(`  Avg Results:     ${comparison.avg_result_count_delta > 0 ? '+' : ''}${comparison.avg_result_count_delta.toFixed(2)}`)
  lines.push(`  Execution Time:  ${comparison.execution_time_delta_ms > 0 ? '+' : ''}${comparison.execution_time_delta_ms.toFixed(0)}ms`)
  lines.push(`  Confidence:      ${comparison.confidence_delta > 0 ? '+' : ''}${(comparison.confidence_delta * 100).toFixed(1)}%`)
  lines.push('')

  const winnerName = comparison.winner === 'a' ? comparison.variant_a :
                     comparison.winner === 'b' ? comparison.variant_b :
                     'TIE'

  lines.push(`WINNER: ${winnerName}`)
  lines.push('')

  return lines.join('\n')
}

/**
 * Format multiple variants for comparison
 */
export function formatLeaderboard(allMetrics: VariantMetrics[]): string {
  const lines: string[] = []

  lines.push('='.repeat(60))
  lines.push('VARIANT LEADERBOARD')
  lines.push('='.repeat(60))
  lines.push('')

  // Sort by success rate, then avg result count
  const sorted = [...allMetrics].sort((a, b) => {
    if (Math.abs(a.success_rate - b.success_rate) > 0.01) {
      return b.success_rate - a.success_rate
    }
    return b.avg_result_count - a.avg_result_count
  })

  lines.push('Rank | Variant                | Success | Avg Results | Avg Time')
  lines.push('-----+------------------------+---------+-------------+---------')

  sorted.forEach((m, idx) => {
    const rank = (idx + 1).toString().padStart(4)
    const name = m.variant_name.substring(0, 22).padEnd(22)
    const success = `${(m.success_rate * 100).toFixed(1)}%`.padStart(7)
    const avgResults = m.avg_result_count.toFixed(2).padStart(11)
    const avgTime = `${m.avg_execution_time_ms.toFixed(0)}ms`.padStart(7)

    lines.push(`${rank} | ${name} | ${success} | ${avgResults} | ${avgTime}`)
  })

  lines.push('')

  return lines.join('\n')
}

/**
 * Format as Markdown report
 */
export function formatMarkdown(metrics: VariantMetrics, categoryMetrics?: CategoryMetrics[]): string {
  const lines: string[] = []

  lines.push(`# Experiment Report: ${metrics.variant_name}`)
  lines.push('')
  lines.push(`**Variant ID:** ${metrics.variant_id}`)
  lines.push(`**Timestamp:** ${metrics.timestamp.toISOString()}`)
  lines.push('')

  lines.push('## Summary')
  lines.push('')
  lines.push('| Metric | Value |')
  lines.push('|--------|-------|')
  lines.push(`| Total Queries | ${metrics.total_queries} |`)
  lines.push(`| Successful Queries | ${metrics.successful_queries} (${(metrics.success_rate * 100).toFixed(1)}%) |`)
  lines.push(`| Avg Results/Query | ${metrics.avg_result_count.toFixed(2)} |`)
  lines.push(`| Avg Execution Time | ${metrics.avg_execution_time_ms.toFixed(0)}ms |`)
  lines.push(`| Avg Confidence | ${(metrics.avg_transformation_confidence * 100).toFixed(1)}% |`)
  lines.push(`| Total Time | ${(metrics.total_execution_time_ms / 1000).toFixed(1)}s |`)
  lines.push('')

  if (categoryMetrics && categoryMetrics.length > 0) {
    lines.push('## Results by Category')
    lines.push('')
    lines.push('| Category | Success Rate | Avg Results |')
    lines.push('|----------|--------------|-------------|')
    for (const cat of categoryMetrics) {
      lines.push(`| ${cat.category} | ${(cat.success_rate * 100).toFixed(1)}% (${cat.successful_queries}/${cat.total_queries}) | ${cat.avg_result_count.toFixed(2)} |`)
    }
    lines.push('')
  }

  const failed = metrics.query_results.filter(r => !r.success)
  if (failed.length > 0) {
    lines.push(`## Failed Queries (${failed.length})`)
    lines.push('')
    for (const result of failed.slice(0, 20)) {
      lines.push(`- **${result.query_id}**: "${result.original_query}"`)
      lines.push(`  - Transformed: "${result.transformed_query}"`)
      lines.push(`  - Results: ${result.result_count}`)
    }
    if (failed.length > 20) {
      lines.push(`- ... and ${failed.length - 20} more`)
    }
    lines.push('')
  }

  return lines.join('\n')
}

/**
 * Console logger with colors (optional)
 */
export class ConsoleReporter {
  report(metrics: VariantMetrics): void {
    console.log(formatText(metrics))
  }

  reportCategory(categoryMetrics: CategoryMetrics[]): void {
    console.log(formatCategoryMetrics(categoryMetrics))
  }

  reportComparison(comparison: VariantComparison): void {
    console.log(formatComparison(comparison))
  }

  reportLeaderboard(allMetrics: VariantMetrics[]): void {
    console.log(formatLeaderboard(allMetrics))
  }
}
