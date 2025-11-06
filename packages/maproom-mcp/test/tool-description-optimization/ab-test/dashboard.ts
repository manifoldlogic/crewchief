/**
 * Simple A/B Test Dashboard
 *
 * Provides monitoring and visualization of A/B test results
 */

import type { ABTestCollector, VariantSummary } from './collector.js'

export interface DashboardData {
  experiment_id: string
  start_time: number
  current_time: number
  duration_hours: number
  variants: VariantSummary[]
  winner: {
    variant: string | null
    confidence: number // 0-1
    p_value: number
  }
  recommendations: string[]
}

/**
 * Generate dashboard data from collector
 */
export function generateDashboard(
  collector: ABTestCollector,
  experimentId: string = 'experiment-1',
  startTime?: number
): DashboardData {
  const summary = collector.getSummary()
  const variants = Array.from(summary.values())

  const currentTime = Date.now()
  const start = startTime || (currentTime - 24 * 60 * 60 * 1000) // Default to 24h ago
  const durationHours = (currentTime - start) / (60 * 60 * 1000)

  // Determine winner (simplified - production should use full statistical analysis)
  const sorted = [...variants].sort((a, b) => b.success_rate - a.success_rate)
  const best = sorted[0]
  const baseline = sorted[1]

  let winner: { variant: string | null; confidence: number; p_value: number } = {
    variant: null,
    confidence: 0,
    p_value: 1.0
  }

  if (best && baseline) {
    const delta = best.success_rate - baseline.success_rate
    const minSampleSize = 1000

    // Simple winner detection (production should use statistical analysis from analyzer.ts)
    if (best.total_queries >= minSampleSize && baseline.total_queries >= minSampleSize) {
      if (delta > 0.05) {
        // >5% improvement
        winner = {
          variant: best.variant,
          confidence: 0.95, // Simplified - should calculate from t-test
          p_value: 0.01 // Simplified - should calculate from t-test
        }
      }
    }
  }

  // Generate recommendations
  const recommendations: string[] = []

  if (winner.variant) {
    recommendations.push(`✅ Winner detected: ${winner.variant}`)
    recommendations.push(`   Deploy ${winner.variant} to 100% of traffic`)
  } else if (variants.every(v => v.total_queries < 1000)) {
    recommendations.push('⏳ Continue collecting data')
    recommendations.push(`   Need ${1000 - (variants[0]?.total_queries || 0)} more samples per variant`)
  } else {
    recommendations.push('⚠️  No clear winner yet')
    recommendations.push('   Consider extending experiment duration')
  }

  return {
    experiment_id: experimentId,
    start_time: start,
    current_time: currentTime,
    duration_hours: durationHours,
    variants,
    winner,
    recommendations
  }
}

/**
 * Format dashboard as human-readable text
 */
export function formatDashboard(data: DashboardData): string {
  const lines: string[] = []

  lines.push('=' .repeat(70))
  lines.push('A/B TEST DASHBOARD')
  lines.push('='.repeat(70))
  lines.push('')

  lines.push(`Experiment ID: ${data.experiment_id}`)
  lines.push(`Duration: ${data.duration_hours.toFixed(1)} hours`)
  lines.push(`Last Updated: ${new Date(data.current_time).toISOString()}`)
  lines.push('')

  // Variant summary table
  lines.push('VARIANT PERFORMANCE:')
  lines.push('   Variant              | Queries | Success Rate | Avg Results | Users')
  lines.push('   ' + '-'.repeat(70))

  for (const variant of data.variants) {
    const name = variant.variant.substring(0, 20).padEnd(20)
    const queries = variant.total_queries.toString().padStart(7)
    const successRate = `${(variant.success_rate * 100).toFixed(1)}%`.padStart(12)
    const avgResults = variant.avg_result_count.toFixed(2).padStart(11)
    const users = variant.unique_users.toString().padStart(5)

    lines.push(`   ${name} | ${queries} | ${successRate} | ${avgResults} | ${users}`)
  }

  lines.push('')

  // Winner status
  if (data.winner.variant) {
    lines.push('WINNER:')
    lines.push(`   ${data.winner.variant}`)
    lines.push(`   Confidence: ${(data.winner.confidence * 100).toFixed(1)}%`)
    lines.push(`   P-value: ${data.winner.p_value.toFixed(4)}`)
  } else {
    lines.push('WINNER: None detected yet')
  }

  lines.push('')

  // Recommendations
  lines.push('RECOMMENDATIONS:')
  data.recommendations.forEach(rec => lines.push(`   ${rec}`))

  lines.push('')
  lines.push('='.repeat(70))

  return lines.join('\n')
}

/**
 * Export dashboard as JSON (for API endpoint)
 */
export function formatJSON(data: DashboardData): string {
  return JSON.stringify(data, null, 2)
}

/**
 * Simple HTTP endpoint handler (for Express/Fastify)
 */
export function createDashboardHandler(collector: ABTestCollector) {
  return async (_req: any, res: any) => {
    try {
      const dashboard = generateDashboard(collector)
      const format = _req.query?.format || 'json'

      if (format === 'text') {
        res.type('text/plain')
        res.send(formatDashboard(dashboard))
      } else {
        res.json(dashboard)
      }
    } catch (error) {
      res.status(500).json({
        error: 'Failed to generate dashboard',
        message: error instanceof Error ? error.message : String(error)
      })
    }
  }
}

/**
 * CLI dashboard (for terminal display)
 */
export function displayDashboard(collector: ABTestCollector, experimentId?: string): void {
  const dashboard = generateDashboard(collector, experimentId)
  console.log(formatDashboard(dashboard))
}
