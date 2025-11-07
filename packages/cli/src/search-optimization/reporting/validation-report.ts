/**
 * Validation Report Generator
 *
 * Generates comprehensive markdown reports showing:
 * - Executive summary
 * - Per-tier results
 * - Statistical analysis
 * - Tool selection patterns
 * - Failure analysis
 */

import { writeFileSync } from 'fs'
import { join } from 'path'
import type { ValidationResults } from '../scripts/run-full-validation.js'

/**
 * Generate validation report as markdown
 */
export function generateValidationReport(results: ValidationResults): string {
  const lines: string[] = []

  // Header
  lines.push('# Full Validation Report')
  lines.push('')
  lines.push(`**Generated**: ${results.timestamp.toISOString()}`)
  lines.push(`**Total Tasks**: ${results.summary.totalTasks}`)
  lines.push('')
  lines.push('='.repeat(80))
  lines.push('')

  // Executive Summary
  lines.push('## Executive Summary')
  lines.push('')
  lines.push('| Metric | Value |')
  lines.push('|--------|-------|')
  lines.push(`| Total Tasks | ${results.summary.totalTasks} |`)
  lines.push(`| Grep Baseline Success | ${(results.summary.grepSuccessRate * 100).toFixed(1)}% |`)
  lines.push(`| Search Available Success | ${(results.summary.searchSuccessRate * 100).toFixed(1)}% |`)
  lines.push(`| **Improvement** | **+${(results.summary.improvement * 100).toFixed(1)}%** |`)
  lines.push(
    `| Statistical Significance | ${results.summary.statisticallySignificant ? '✅ YES (p < 0.05)' : '❌ NO'} |`,
  )
  lines.push(`| p-value | ${results.statisticalAnalysis.pValue.toFixed(4)} |`)
  lines.push(
    `| Effect Size (Cohen's d) | ${results.statisticalAnalysis.cohensD.toFixed(2)} (${results.statisticalAnalysis.effectSize}) |`,
  )
  lines.push('')

  if (results.summary.statisticallySignificant) {
    lines.push('✅ **Strong evidence** that semantic search provides measurable value over grep-only baseline.')
  } else {
    lines.push(
      '⚠️  **Insufficient evidence** of statistical significance. Results may be due to chance or small sample size.',
    )
  }
  lines.push('')
  lines.push('='.repeat(80))
  lines.push('')

  // Tier 1: Grep-Impossible
  lines.push('## Tier 1: Grep-Impossible Tasks')
  lines.push('')
  lines.push(
    'These tasks fundamentally defeat grep through relationship traversal, architectural understanding, or negative space detection.',
  )
  lines.push('')
  lines.push('| Metric | Value |')
  lines.push('|--------|-------|')
  lines.push(`| Task Count | ${results.perTierSummary.tier1.taskCount} |`)
  lines.push(`| Grep Baseline | ${(results.perTierSummary.tier1.grepSuccess * 100).toFixed(1)}% |`)
  lines.push(`| Search Available | ${(results.perTierSummary.tier1.searchSuccess * 100).toFixed(1)}% |`)
  lines.push(`| **Improvement** | **+${(results.perTierSummary.tier1.improvement * 100).toFixed(1)}%** |`)
  lines.push(`| p-value | ${results.perTierSummary.tier1.pValue.toFixed(4)} |`)
  lines.push('')

  if (results.perTierSummary.tier1.grepSuccess < 0.3 && results.perTierSummary.tier1.searchSuccess > 0.7) {
    lines.push('✅ **Tier 1 Success**: Tasks defeat grep (<30% success) and search solves them (>70% success).')
  } else if (results.perTierSummary.tier1.grepSuccess >= 0.3) {
    lines.push(
      `⚠️  **Tier 1 Warning**: Grep success rate (${(results.perTierSummary.tier1.grepSuccess * 100).toFixed(1)}%) is too high. Tasks may not be grep-impossible.`,
    )
  } else if (results.perTierSummary.tier1.searchSuccess <= 0.7) {
    lines.push(
      `⚠️  **Tier 1 Warning**: Search success rate (${(results.perTierSummary.tier1.searchSuccess * 100).toFixed(1)}%) is too low. Tasks may be too difficult.`,
    )
  }
  lines.push('')
  lines.push('='.repeat(80))
  lines.push('')

  // Tier 2: Grep-Hard
  lines.push('## Tier 2: Grep-Hard Tasks')
  lines.push('')
  lines.push('These tasks are solvable with grep but semantic search provides efficiency advantages.')
  lines.push('')
  lines.push('| Metric | Value |')
  lines.push('|--------|-------|')
  lines.push(`| Task Count | ${results.perTierSummary.tier2.taskCount} |`)
  lines.push(`| Grep Baseline | ${(results.perTierSummary.tier2.grepSuccess * 100).toFixed(1)}% |`)
  lines.push(`| Search Available | ${(results.perTierSummary.tier2.searchSuccess * 100).toFixed(1)}% |`)
  lines.push(`| **Improvement** | **+${(results.perTierSummary.tier2.improvement * 100).toFixed(1)}%** |`)
  lines.push(`| p-value | ${results.perTierSummary.tier2.pValue.toFixed(4)} |`)
  lines.push('')

  if (
    results.perTierSummary.tier2.grepSuccess >= 0.3 &&
    results.perTierSummary.tier2.grepSuccess <= 0.6 &&
    results.perTierSummary.tier2.improvement > 0.3
  ) {
    lines.push(
      '✅ **Tier 2 Success**: Grep works (30-60%) but search provides significant advantage (>30% improvement).',
    )
  } else if (results.perTierSummary.tier2.grepSuccess < 0.3) {
    lines.push(
      `⚠️  **Tier 2 Warning**: Grep success too low (${(results.perTierSummary.tier2.grepSuccess * 100).toFixed(1)}%). These may be grep-impossible tasks (Tier 1).`,
    )
  } else if (results.perTierSummary.tier2.grepSuccess > 0.6) {
    lines.push(
      `⚠️  **Tier 2 Warning**: Grep success too high (${(results.perTierSummary.tier2.grepSuccess * 100).toFixed(1)}%). Tasks may be too easy.`,
    )
  }
  lines.push('')
  lines.push('='.repeat(80))
  lines.push('')

  // Tier 3: Real-World
  lines.push('## Tier 3: Real-World Tasks')
  lines.push('')
  lines.push('These tasks measure voluntary search adoption in realistic development scenarios.')
  lines.push('')
  lines.push('| Metric | Value |')
  lines.push('|--------|-------|')
  lines.push(`| Task Count | ${results.perTierSummary.tier3.taskCount} |`)
  lines.push(`| Grep Baseline | ${(results.perTierSummary.tier3.grepSuccess * 100).toFixed(1)}% |`)
  lines.push(`| Search Available | ${(results.perTierSummary.tier3.searchSuccess * 100).toFixed(1)}% |`)
  lines.push(`| **Improvement** | **+${(results.perTierSummary.tier3.improvement * 100).toFixed(1)}%** |`)
  lines.push(
    `| Voluntary Search Adoption | ${(results.searchResults.toolUsageStats.searchUsageRate * 100).toFixed(0)}% |`,
  )
  lines.push(`| p-value | ${results.perTierSummary.tier3.pValue.toFixed(4)} |`)
  lines.push('')

  if (results.searchResults.toolUsageStats.searchUsageRate >= 0.4) {
    lines.push(
      `✅ **Tier 3 Success**: Agents voluntarily adopt search (${(results.searchResults.toolUsageStats.searchUsageRate * 100).toFixed(0)}% usage rate) when available.`,
    )
  } else {
    lines.push(
      `⚠️  **Tier 3 Warning**: Low voluntary search adoption (${(results.searchResults.toolUsageStats.searchUsageRate * 100).toFixed(0)}%). Search may not be naturally useful.`,
    )
  }
  lines.push('')
  lines.push('='.repeat(80))
  lines.push('')

  // Statistical Tests
  lines.push('## Statistical Analysis')
  lines.push('')
  lines.push('### Hypothesis Test')
  lines.push('- **Null Hypothesis (H₀)**: Semantic search provides no improvement over grep')
  lines.push('- **Alternative Hypothesis (H₁)**: Semantic search provides significant improvement over grep')
  lines.push('')
  lines.push('### Paired t-test Results')
  lines.push(`- **t-statistic**: ${results.statisticalAnalysis.tStatistic.toFixed(2)}`)
  lines.push(`- **Degrees of freedom**: ${results.statisticalAnalysis.degreesOfFreedom}`)
  lines.push(`- **p-value**: ${results.statisticalAnalysis.pValue.toFixed(4)}`)
  lines.push(
    `- **Significance**: ${results.statisticalAnalysis.pValue < 0.05 ? '✅ Significant at α = 0.05' : '❌ Not significant'}`,
  )
  lines.push('')

  lines.push('### Effect Size')
  lines.push(`- **Cohen's d**: ${results.statisticalAnalysis.cohensD.toFixed(2)}`)
  lines.push(`- **Magnitude**: ${results.statisticalAnalysis.effectSize}`)
  lines.push('- **Interpretation**:')
  lines.push('  - d < 0.5: small effect')
  lines.push('  - 0.5 ≤ d < 0.8: medium effect')
  lines.push('  - 0.8 ≤ d < 1.2: large effect')
  lines.push('  - d ≥ 1.2: very large effect')
  lines.push('')

  lines.push('### Summary Statistics')
  lines.push(`- **Mean improvement**: ${(results.statisticalAnalysis.meanDifference * 100).toFixed(1)}%`)
  lines.push(`- **Median improvement**: ${(results.statisticalAnalysis.medianDifference * 100).toFixed(1)}%`)
  lines.push(`- **Standard deviation**: ${(results.statisticalAnalysis.standardDeviation * 100).toFixed(1)}%`)
  lines.push('')

  lines.push('### Confidence Interval (95%)')
  lines.push(
    `- **95% CI**: [${(results.statisticalAnalysis.confidenceInterval95.lower * 100).toFixed(1)}%, ${(results.statisticalAnalysis.confidenceInterval95.upper * 100).toFixed(1)}%]`,
  )
  lines.push(
    `- We are 95% confident the true improvement is between ${(results.statisticalAnalysis.confidenceInterval95.lower * 100).toFixed(1)}% and ${(results.statisticalAnalysis.confidenceInterval95.upper * 100).toFixed(1)}%.`,
  )
  lines.push('')

  lines.push('### Statistical Power')
  lines.push(`- **Power**: ${(results.statisticalAnalysis.statisticalPower * 100).toFixed(0)}%`)
  lines.push(`- **Sample Size**: ${results.statisticalAnalysis.sampleSize} tasks`)
  lines.push(
    `- Probability of detecting a true effect of this size: ${(results.statisticalAnalysis.statisticalPower * 100).toFixed(0)}%`,
  )
  lines.push('')
  lines.push('='.repeat(80))
  lines.push('')

  // Per-Category Results
  lines.push('## Per-Category Results')
  lines.push('')
  if (results.perCategorySummary.size > 0) {
    lines.push('| Category | Tasks | Grep | Search | Improvement |')
    lines.push('|----------|-------|------|--------|-------------|')
    for (const [category, summary] of results.perCategorySummary) {
      lines.push(
        `| ${category} | ${summary.taskCount} | ${(summary.grepSuccess * 100).toFixed(1)}% | ${(summary.searchSuccess * 100).toFixed(1)}% | +${(summary.improvement * 100).toFixed(1)}% |`,
      )
    }
  } else {
    lines.push('No per-category data available.')
  }
  lines.push('')
  lines.push('='.repeat(80))
  lines.push('')

  // Failure Analysis
  lines.push('## Failure Analysis')
  lines.push('')
  lines.push('### Tasks Where Grep Failed (Score < 0.6)')
  lines.push('')

  // Collect grep failures by category
  const grepFailuresByCategory = new Map<string, number>()
  for (const [category, summary] of results.perCategorySummary) {
    if (summary.grepSuccess < 0.6) {
      grepFailuresByCategory.set(category, summary.taskCount)
    }
  }

  if (grepFailuresByCategory.size > 0) {
    for (const [category, count] of grepFailuresByCategory) {
      const summary = results.perCategorySummary.get(category)!
      lines.push(
        `- **${category}**: ${count} tasks, ${(summary.grepSuccess * 100).toFixed(1)}% success rate (search: ${(summary.searchSuccess * 100).toFixed(1)}%)`,
      )
    }
  } else {
    lines.push('No significant grep failures detected.')
  }
  lines.push('')

  lines.push('### Tasks Where Search Failed (Score < 0.6)')
  lines.push('')

  // Collect search failures by category
  const searchFailuresByCategory = new Map<string, number>()
  for (const [category, summary] of results.perCategorySummary) {
    if (summary.searchSuccess < 0.6) {
      searchFailuresByCategory.set(category, summary.taskCount)
    }
  }

  if (searchFailuresByCategory.size > 0) {
    for (const [category, count] of searchFailuresByCategory) {
      const summary = results.perCategorySummary.get(category)!
      lines.push(`- **${category}**: ${count} tasks, ${(summary.searchSuccess * 100).toFixed(1)}% success rate`)
    }
  } else {
    lines.push('No significant search failures detected.')
  }
  lines.push('')
  lines.push('='.repeat(80))
  lines.push('')

  // Tool Usage
  lines.push('## Tool Selection Analysis')
  lines.push('')
  lines.push('### Grep-Only Condition')
  lines.push(`- Search usage: ${(results.grepResults.toolUsageStats.searchUsageRate * 100).toFixed(0)}% (should be 0%)`)
  lines.push(`- Grep usage: ${(results.grepResults.toolUsageStats.grepUsageRate * 100).toFixed(0)}%`)
  lines.push('')
  lines.push('### Search-Available Condition')
  lines.push(`- Search usage: ${(results.searchResults.toolUsageStats.searchUsageRate * 100).toFixed(0)}%`)
  lines.push(`- Grep usage: ${(results.searchResults.toolUsageStats.grepUsageRate * 100).toFixed(0)}%`)
  lines.push('')
  lines.push('='.repeat(80))
  lines.push('')

  // Conclusion
  lines.push('## Conclusion')
  lines.push('')

  if (results.summary.statisticallySignificant && results.summary.improvement > 0.3) {
    lines.push('### ✅ Strong Evidence for Semantic Search Value')
    lines.push('')
    lines.push('The validation provides **strong evidence** that semantic search delivers measurable value:')
    lines.push('')
    lines.push(
      `1. **Defeats grep on impossible tasks** (Tier 1: +${(results.perTierSummary.tier1.improvement * 100).toFixed(1)}%)`,
    )
    lines.push(
      `2. **Shows efficiency gains on hard tasks** (Tier 2: +${(results.perTierSummary.tier2.improvement * 100).toFixed(1)}%)`,
    )
    lines.push(
      `3. **Natural adoption in real-world scenarios** (Tier 3: ${(results.searchResults.toolUsageStats.searchUsageRate * 100).toFixed(0)}% usage)`,
    )
    lines.push(
      `4. **Statistically significant** (p = ${results.statisticalAnalysis.pValue.toFixed(4)}, d = ${results.statisticalAnalysis.cohensD.toFixed(2)})`,
    )
  } else if (results.summary.statisticallySignificant) {
    lines.push('### ⚠️  Moderate Evidence for Semantic Search Value')
    lines.push('')
    lines.push('The validation shows statistically significant results but modest practical improvement.')
    lines.push('Consider refining tasks to demonstrate larger effect sizes.')
  } else {
    lines.push('### ❌ Insufficient Evidence')
    lines.push('')
    lines.push('The validation did not show statistically significant improvements.')
    lines.push('This suggests:')
    lines.push('- Sample size may be too small')
    lines.push('- Tasks may not effectively demonstrate search advantages')
    lines.push('- Search implementation may need improvement')
  }
  lines.push('')
  lines.push('='.repeat(80))
  lines.push('')

  // Metadata
  lines.push('## Validation Metadata')
  lines.push('')
  lines.push(`- **Timestamp**: ${results.timestamp.toISOString()}`)
  lines.push(`- **Grep execution time**: ${results.grepResults.durationSeconds.toFixed(1)}s`)
  lines.push(`- **Search execution time**: ${results.searchResults.durationSeconds.toFixed(1)}s`)
  lines.push(
    `- **Total execution time**: ${(results.grepResults.durationSeconds + results.searchResults.durationSeconds).toFixed(1)}s`,
  )
  lines.push('')

  return lines.join('\n')
}

/**
 * Save validation report to file
 */
export async function saveValidationReport(report: string, baseDir: string): Promise<void> {
  const reportPath = join(baseDir, 'report.md')
  writeFileSync(reportPath, report)
  console.log(`\nReport saved to: ${reportPath}`)
}
