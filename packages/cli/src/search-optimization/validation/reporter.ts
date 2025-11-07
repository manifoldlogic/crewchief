/**
 * Validation Report Generator
 *
 * Produces comprehensive markdown reports showing:
 * - Per-task validation results across 5 quality dimensions
 * - Failure pattern identification and grouping
 * - Suite-level summaries and statistics
 * - Actionable recommendations for improvement
 *
 * Supports multiple output formats:
 * - Markdown (primary, human-readable)
 * - JSON (structured data)
 * - Console (quick terminal view)
 *
 * @example Basic usage
 * ```typescript
 * import { ReportGenerator } from './reporter'
 * import { validateSuite } from './task-validator'
 * import { TIER1_SUITE } from '../benchmarks/tier1-impossible'
 *
 * // Validate suite
 * const suiteResult = await validateSuite(TIER1_SUITE)
 *
 * // Generate report
 * const generator = new ReportGenerator({ format: 'markdown' })
 * const report = generator.generate(
 *   suiteResult.taskResults,
 *   'tier1-impossible',
 *   'tier1'
 * )
 *
 * // Save to file
 * await generator.save(report)
 * // Report saved to: ./reports/validation-report-tier1-impossible-2025-01-01T00-00-00.md
 *
 * // Or print to console
 * generator.print(report)
 * ```
 *
 * @example Custom configuration
 * ```typescript
 * const generator = new ReportGenerator({
 *   format: 'json',
 *   outputDir: './custom-reports',
 *   includePatterns: true,
 *   includeRecommendations: true,
 *   verbose: true
 * })
 *
 * const report = generator.generate(results, 'my-suite', 'tier2')
 * await generator.save(report, 'custom-report-name.json')
 * ```
 *
 * @example Programmatic analysis
 * ```typescript
 * const report = generator.generate(results)
 *
 * // Access summary statistics
 * console.log(`Pass rate: ${report.summary.passRate}%`)
 * console.log(`Failed: ${report.summary.failed} tasks`)
 *
 * // Analyze patterns
 * const tooEasy = report.patterns?.find(p => p.pattern === 'too-easy')
 * if (tooEasy) {
 *   console.log(`Found ${tooEasy.count} tasks that are too easy for grep`)
 * }
 *
 * // Get high priority recommendations
 * const highPriority = report.recommendations?.filter(r => r.priority === 'high')
 * console.log(`${highPriority?.length} tasks need immediate attention`)
 * ```
 */

import { promises as fs } from 'node:fs'
import path from 'node:path'
import type { ValidationResult } from './task-validator.js'

/**
 * Configuration for report generation
 */
export interface ReportConfig {
  /** Output format */
  format: 'markdown' | 'json' | 'console'

  /** Directory for saving reports */
  outputDir?: string

  /** Include failure pattern analysis */
  includePatterns?: boolean

  /** Include recommendations */
  includeRecommendations?: boolean

  /** Include verbose details */
  verbose?: boolean
}

/**
 * Report metadata
 */
export interface ReportMetadata {
  /** When the report was generated */
  timestamp: Date

  /** Framework version */
  version: string

  /** Total number of tasks validated */
  totalTasks: number

  /** Suite name if applicable */
  suiteName?: string

  /** Tier level */
  tier?: string
}

/**
 * Summary section with aggregate statistics
 */
export interface SummarySection {
  /** Total tasks */
  total: number

  /** Number of tasks that passed */
  passed: number

  /** Number of tasks that failed */
  failed: number

  /** Pass rate percentage */
  passRate: number

  /** Breakdown by dimension */
  dimensionBreakdown: {
    constructValidity: { passed: number; failed: number }
    discriminantValidity: { passed: number; failed: number }
    ecologicalValidity: { passed: number; failed: number }
    reliability: { passed: number; failed: number }
    statisticalPower: { passed: number; failed: number }
  }
}

/**
 * Per-task result section
 */
export interface TaskResultSection {
  /** Task ID */
  taskId: string

  /** Task name */
  taskName: string

  /** Overall pass/fail */
  passed: boolean

  /** Dimension results */
  dimensions: {
    name: string
    passed: boolean
    actual: string | number
    expected: string | number
    details: string
  }[]

  /** Task-specific recommendations */
  recommendations: string[]
}

/**
 * Failure pattern types
 */
export type FailurePattern = 'too-easy' | 'too-hard' | 'insufficient-advantage' | 'unreliable' | 'ecologically-invalid'

/**
 * Pattern section grouping similar failures
 */
export interface PatternSection {
  /** Pattern type */
  pattern: FailurePattern

  /** Pattern description */
  description: string

  /** Number of tasks matching this pattern */
  count: number

  /** Task IDs matching this pattern */
  taskIds: string[]

  /** Common characteristics */
  characteristics: string[]

  /** Recommended fixes */
  fixes: string[]
}

/**
 * Recommendation section
 */
export interface RecommendationSection {
  /** Task ID */
  taskId: string

  /** Priority (high/medium/low) */
  priority: 'high' | 'medium' | 'low'

  /** Specific actionable recommendations */
  actions: string[]
}

/**
 * Complete validation report
 */
export interface Report {
  /** Report metadata */
  metadata: ReportMetadata

  /** Summary statistics */
  summary: SummarySection

  /** Per-task results */
  perTaskResults: TaskResultSection[]

  /** Failure patterns (optional) */
  patterns?: PatternSection[]

  /** Recommendations (optional) */
  recommendations?: RecommendationSection[]

  /** Markdown formatted report */
  markdown: string

  /** JSON formatted report */
  json: string
}

/**
 * Default report configuration
 */
const DEFAULT_CONFIG: ReportConfig = {
  format: 'markdown',
  outputDir: './reports',
  includePatterns: true,
  includeRecommendations: true,
  verbose: false,
}

/**
 * Report Generator
 *
 * Main class for generating validation reports.
 */
export class ReportGenerator {
  private config: Required<ReportConfig>

  constructor(config: Partial<ReportConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config } as Required<ReportConfig>
  }

  /**
   * Generate a complete validation report
   *
   * @param results - Array of validation results
   * @param suiteName - Optional suite name
   * @param tier - Optional tier level
   * @returns Complete report
   */
  generate(results: ValidationResult[], suiteName?: string, tier?: string): Report {
    // Generate metadata
    const metadata: ReportMetadata = {
      timestamp: new Date(),
      version: '1.0.0',
      totalTasks: results.length,
      suiteName,
      tier,
    }

    // Generate summary
    const summary = generateSummary(results)

    // Generate per-task results
    const perTaskResults = generatePerTaskResults(results)

    // Generate patterns if requested
    const patterns = this.config.includePatterns ? identifyPatterns(results) : undefined

    // Generate recommendations if requested
    const recommendations = this.config.includeRecommendations ? generateRecommendations(results) : undefined

    // Format as markdown
    const markdown = formatMarkdown({
      metadata,
      summary,
      perTaskResults,
      patterns,
      recommendations,
      markdown: '',
      json: '',
    })

    // Format as JSON
    const json = formatJSON({
      metadata,
      summary,
      perTaskResults,
      patterns,
      recommendations,
      markdown: '',
      json: '',
    })

    return {
      metadata,
      summary,
      perTaskResults,
      patterns,
      recommendations,
      markdown,
      json,
    }
  }

  /**
   * Save report to file
   *
   * @param report - The report to save
   * @param filename - Optional custom filename
   */
  async save(report: Report, filename?: string): Promise<void> {
    // Create output directory if it doesn't exist
    await fs.mkdir(this.config.outputDir, { recursive: true })

    // Generate filename
    const timestamp = report.metadata.timestamp.toISOString().replace(/[:.]/g, '-').slice(0, 19)
    const suiteName = report.metadata.suiteName ?? 'validation'
    const defaultFilename = `validation-report-${suiteName}-${timestamp}.${this.config.format === 'json' ? 'json' : 'md'}`
    const outputFilename = filename ?? defaultFilename

    // Full path
    const outputPath = path.join(this.config.outputDir, outputFilename)

    // Write content based on format
    const content = this.config.format === 'json' ? report.json : report.markdown

    await fs.writeFile(outputPath, content, 'utf-8')

    console.log(`Report saved to: ${outputPath}`)
  }

  /**
   * Print report to console
   *
   * @param report - The report to print
   */
  print(report: Report): void {
    const consoleOutput = formatConsole(report)
    console.log(consoleOutput)
  }
}

/**
 * Generate summary section
 */
export function generateSummary(results: ValidationResult[]): SummarySection {
  const total = results.length
  const passed = results.filter((r) => r.passed).length
  const failed = total - passed
  const passRate = total > 0 ? (passed / total) * 100 : 0

  // Dimension breakdown
  const dimensionBreakdown = {
    constructValidity: { passed: 0, failed: 0 },
    discriminantValidity: { passed: 0, failed: 0 },
    ecologicalValidity: { passed: 0, failed: 0 },
    reliability: { passed: 0, failed: 0 },
    statisticalPower: { passed: 0, failed: 0 },
  }

  for (const result of results) {
    for (const [key, dimension] of Object.entries(result.dimensions)) {
      const dimKey = key as keyof typeof dimensionBreakdown
      if (dimension.passed) {
        dimensionBreakdown[dimKey].passed++
      } else {
        dimensionBreakdown[dimKey].failed++
      }
    }
  }

  return {
    total,
    passed,
    failed,
    passRate,
    dimensionBreakdown,
  }
}

/**
 * Generate per-task result sections
 */
export function generatePerTaskResults(results: ValidationResult[]): TaskResultSection[] {
  return results.map((result) => ({
    taskId: result.task.id,
    taskName: result.task.name,
    passed: result.passed,
    dimensions: Object.values(result.dimensions).map((dim) => ({
      name: dim.dimension,
      passed: dim.passed,
      actual: dim.actual,
      expected: dim.expected,
      details: dim.details,
    })),
    recommendations: result.recommendations,
  }))
}

/**
 * Identify failure patterns across tasks
 */
export function identifyPatterns(results: ValidationResult[]): PatternSection[] {
  const patterns: PatternSection[] = []

  // Pattern 1: Too Easy (grep success > 60%)
  const tooEasyTasks = results.filter((r) => {
    if (r.dimensions.constructValidity.passed) return false
    const actual = parseFloat(r.dimensions.constructValidity.actual.toString())
    return actual > 0.6
  })

  if (tooEasyTasks.length > 0) {
    patterns.push({
      pattern: 'too-easy',
      description: 'Tasks that are too easy for grep-based search',
      count: tooEasyTasks.length,
      taskIds: tooEasyTasks.map((t) => t.task.id),
      characteristics: [
        'Grep success rate > 60%',
        'Keywords directly match search intent',
        'Direct relationships without indirection',
      ],
      fixes: [
        'Add semantic complexity (concepts vs keywords)',
        'Require transitive relationships',
        'Introduce ambiguity that needs context',
      ],
    })
  }

  // Pattern 2: Too Hard (grep < 10% AND search < 50%)
  const tooHardTasks = results.filter((r) => {
    const grepActual = parseFloat(r.dimensions.constructValidity.actual.toString())
    const searchActual = r.dimensions.discriminantValidity.actual.toString()
    const searchSuccess = parseFloat(searchActual.split('%')[0])
    return grepActual < 0.1 && searchSuccess < 50
  })

  if (tooHardTasks.length > 0) {
    patterns.push({
      pattern: 'too-hard',
      description: 'Tasks that are too difficult even for semantic search',
      count: tooHardTasks.length,
      taskIds: tooHardTasks.map((t) => t.task.id),
      characteristics: ['Grep success < 10%', 'Search success < 50%', 'May require domain expertise'],
      fixes: ['Simplify task requirements', 'Add more context or hints', 'Break into smaller sub-tasks'],
    })
  }

  // Pattern 3: Insufficient Advantage (search - grep < 20%)
  const insufficientAdvantageTasks = results.filter((r) => {
    if (r.dimensions.discriminantValidity.passed) return false
    const searchActual = r.dimensions.discriminantValidity.actual.toString()
    const advantageMatch = searchActual.match(/Δ \+(\d+)pp/)
    if (!advantageMatch) return false
    const advantage = parseFloat(advantageMatch[1])
    return advantage < 20
  })

  if (insufficientAdvantageTasks.length > 0) {
    patterns.push({
      pattern: 'insufficient-advantage',
      description: 'Tasks where semantic search advantage is too small',
      count: insufficientAdvantageTasks.length,
      taskIds: insufficientAdvantageTasks.map((t) => t.task.id),
      characteristics: [
        'Search improvement < 20 percentage points',
        'Both grep and search succeed/fail similarly',
        'Unclear value proposition',
      ],
      fixes: [
        'Make task harder for grep (add indirection)',
        'Make task easier for search (semantic cues)',
        'Clarify why semantic search helps',
      ],
    })
  }

  // Pattern 4: Unreliable (variance > 20%)
  const unreliableTasks = results.filter((r) => {
    if (r.dimensions.reliability.passed) return false
    const actual = r.dimensions.reliability.actual.toString()
    const cvMatch = actual.match(/CV = ([\d.]+)%/)
    if (!cvMatch) return false
    const cv = parseFloat(cvMatch[1])
    return cv > 20
  })

  if (unreliableTasks.length > 0) {
    patterns.push({
      pattern: 'unreliable',
      description: 'Tasks with high variance and inconsistent results',
      count: unreliableTasks.length,
      taskIds: unreliableTasks.map((t) => t.task.id),
      characteristics: ['Coefficient of variation > 20%', 'Subjective validation criteria', 'Inconsistent results'],
      fixes: [
        'Use objective validators (code_change vs explanation)',
        'Tighten success criteria',
        'Remove ambiguous requirements',
      ],
    })
  }

  // Pattern 5: Ecologically Invalid (realism score < 60%)
  const ecologicallyInvalidTasks = results.filter((r) => {
    if (r.dimensions.ecologicalValidity.passed) return false
    const actual = r.dimensions.ecologicalValidity.actual.toString()
    const scoreMatch = actual.match(/Score: (\d+)%/)
    if (!scoreMatch) return false
    const score = parseFloat(scoreMatch[1])
    return score < 60
  })

  if (ecologicallyInvalidTasks.length > 0) {
    patterns.push({
      pattern: 'ecologically-invalid',
      description: 'Tasks that do not reflect realistic developer scenarios',
      count: ecologicallyInvalidTasks.length,
      taskIds: ecologicallyInvalidTasks.map((t) => t.task.id),
      characteristics: ['Realism score < 60%', 'Artificial or contrived scenario', 'Low frequency in real work'],
      fixes: [
        'Base on actual developer scenarios',
        'Add realistic context and motivation',
        'Mark basedOnRealScenario if applicable',
      ],
    })
  }

  return patterns
}

/**
 * Generate prioritized recommendations
 */
export function generateRecommendations(results: ValidationResult[]): RecommendationSection[] {
  return results
    .filter((r) => !r.passed) // Only for failed tasks
    .map((result) => {
      // Determine priority based on failure severity
      let priority: 'high' | 'medium' | 'low' = 'medium'

      const failedCount = Object.values(result.dimensions).filter((d) => !d.passed).length

      if (failedCount >= 3) {
        priority = 'high' // Multiple failures
      } else if (!result.dimensions.ecologicalValidity.passed) {
        priority = 'high' // Ecological validity is critical
      }
      // Single or double failures remain 'medium' priority

      return {
        taskId: result.task.id,
        priority,
        actions: result.recommendations,
      }
    })
    .sort((a, b) => {
      // Sort by priority
      const priorityOrder = { high: 0, medium: 1, low: 2 }
      return priorityOrder[a.priority] - priorityOrder[b.priority]
    })
}

/**
 * Format report as markdown
 */
export function formatMarkdown(report: Omit<Report, 'markdown' | 'json'>): string {
  const lines: string[] = []

  // Header
  lines.push('# Validation Report')
  lines.push('')
  lines.push(`**Generated:** ${report.metadata.timestamp.toISOString()}`)
  lines.push(`**Framework Version:** ${report.metadata.version}`)
  lines.push(`**Total Tasks:** ${report.metadata.totalTasks}`)
  if (report.metadata.suiteName) {
    lines.push(`**Suite:** ${report.metadata.suiteName}`)
  }
  if (report.metadata.tier) {
    lines.push(`**Tier:** ${report.metadata.tier}`)
  }
  lines.push('')

  // Summary
  lines.push('## Summary')
  lines.push('')
  const { summary } = report
  const statusEmoji = summary.failed === 0 ? '✅' : '❌'
  lines.push(
    `${statusEmoji} **Overall:** ${summary.passed}/${summary.total} tasks passed (${summary.passRate.toFixed(1)}%)`,
  )
  lines.push('')

  // Summary table
  lines.push('| Status | Count | Percentage |')
  lines.push('|--------|-------|------------|')
  lines.push(`| ✅ Passed | ${summary.passed} | ${summary.passRate.toFixed(1)}% |`)
  lines.push(`| ❌ Failed | ${summary.failed} | ${(100 - summary.passRate).toFixed(1)}% |`)
  lines.push('')

  // Dimension breakdown
  lines.push('### Dimension Breakdown')
  lines.push('')
  lines.push('| Dimension | Passed | Failed |')
  lines.push('|-----------|--------|--------|')
  for (const [key, value] of Object.entries(summary.dimensionBreakdown)) {
    const displayName = key
      .replace(/([A-Z])/g, ' $1')
      .replace(/^./, (str) => str.toUpperCase())
      .trim()
    lines.push(`| ${displayName} | ${value.passed} | ${value.failed} |`)
  }
  lines.push('')

  // Per-task results
  lines.push('## Per-Task Results')
  lines.push('')
  for (const taskResult of report.perTaskResults) {
    const status = taskResult.passed ? '✅' : '❌'
    lines.push(`### ${status} ${taskResult.taskId}: ${taskResult.taskName}`)
    lines.push('')
    lines.push('| Dimension | Status | Actual | Expected |')
    lines.push('|-----------|--------|--------|----------|')
    for (const dim of taskResult.dimensions) {
      const dimStatus = dim.passed ? '✅' : '❌'
      lines.push(`| ${dim.name} | ${dimStatus} | ${dim.actual} | ${dim.expected} |`)
    }
    lines.push('')

    // Show failed dimension details
    const failedDims = taskResult.dimensions.filter((d) => !d.passed)
    if (failedDims.length > 0) {
      lines.push('**Issues:**')
      lines.push('')
      for (const dim of failedDims) {
        lines.push(`- **${dim.name}:** ${dim.details}`)
      }
      lines.push('')
    }
  }

  // Failure patterns
  if (report.patterns && report.patterns.length > 0) {
    lines.push('## Failure Patterns')
    lines.push('')
    for (const pattern of report.patterns) {
      lines.push(`### ${pattern.description}`)
      lines.push('')
      lines.push(`**Count:** ${pattern.count} tasks`)
      lines.push('')
      lines.push(`**Affected Tasks:** ${pattern.taskIds.join(', ')}`)
      lines.push('')
      lines.push('**Characteristics:**')
      for (const char of pattern.characteristics) {
        lines.push(`- ${char}`)
      }
      lines.push('')
      lines.push('**Recommended Fixes:**')
      for (const fix of pattern.fixes) {
        lines.push(`- ${fix}`)
      }
      lines.push('')
    }
  }

  // Recommendations
  if (report.recommendations && report.recommendations.length > 0) {
    lines.push('## Recommendations')
    lines.push('')
    for (const rec of report.recommendations) {
      const priorityEmoji = rec.priority === 'high' ? '🔴' : rec.priority === 'medium' ? '🟡' : '🟢'
      lines.push(`### ${priorityEmoji} ${rec.taskId} (${rec.priority.toUpperCase()} Priority)`)
      lines.push('')
      for (let i = 0; i < rec.actions.length; i++) {
        lines.push(`${i + 1}. ${rec.actions[i]}`)
      }
      lines.push('')
    }
  }

  return lines.join('\n')
}

/**
 * Format report as JSON
 */
export function formatJSON(report: Omit<Report, 'markdown' | 'json'>): string {
  return JSON.stringify(
    {
      metadata: {
        ...report.metadata,
        timestamp: report.metadata.timestamp.toISOString(),
      },
      summary: report.summary,
      perTaskResults: report.perTaskResults,
      patterns: report.patterns,
      recommendations: report.recommendations,
    },
    null,
    2,
  )
}

/**
 * Format report for console output
 */
export function formatConsole(report: Report): string {
  const lines: string[] = []

  // Header
  lines.push('═'.repeat(80))
  lines.push(`VALIDATION REPORT${report.metadata.suiteName ? `: ${report.metadata.suiteName}` : ''}`)
  lines.push('═'.repeat(80))
  lines.push('')

  // Summary
  const { summary } = report
  const statusSymbol = summary.failed === 0 ? '✅' : '❌'
  lines.push(`${statusSymbol} Overall: ${summary.passed}/${summary.total} passed (${summary.passRate.toFixed(1)}%)`)
  lines.push('')

  // Quick dimension summary
  lines.push('Dimension Status:')
  for (const [key, value] of Object.entries(summary.dimensionBreakdown)) {
    const displayName = key.replace(/([A-Z])/g, ' $1').trim()
    const rate = (value.passed / report.metadata.totalTasks) * 100
    const symbol = value.failed === 0 ? '✅' : '⚠️'
    lines.push(`  ${symbol} ${displayName}: ${value.passed}/${report.metadata.totalTasks} (${rate.toFixed(0)}%)`)
  }
  lines.push('')

  // Failed tasks
  const failedTasks = report.perTaskResults.filter((t) => !t.passed)
  if (failedTasks.length > 0) {
    lines.push(`Failed Tasks (${failedTasks.length}):`)
    lines.push('─'.repeat(80))
    for (const task of failedTasks) {
      const failedDims = task.dimensions.filter((d) => !d.passed).map((d) => d.name)
      lines.push(`  ❌ ${task.taskId}: ${task.taskName}`)
      lines.push(`     Failed: ${failedDims.join(', ')}`)
    }
    lines.push('')
  }

  // Patterns summary
  if (report.patterns && report.patterns.length > 0) {
    lines.push('Failure Patterns:')
    lines.push('─'.repeat(80))
    for (const pattern of report.patterns) {
      lines.push(`  • ${pattern.description}: ${pattern.count} tasks`)
    }
    lines.push('')
  }

  lines.push('═'.repeat(80))
  lines.push(`Full report: ${report.metadata.totalTasks} tasks validated at ${report.metadata.timestamp.toISOString()}`)
  lines.push('═'.repeat(80))

  return lines.join('\n')
}
