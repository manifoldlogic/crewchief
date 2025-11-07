/**
 * Benchmark Suite Module
 *
 * Provides benchmark suites for evaluating search tool performance,
 * validation utilities, and reporting capabilities.
 *
 * @example
 * ```typescript
 * import {
 *   TIER1_GREP_IMPOSSIBLE_SUITE,
 *   validateSuiteComposition,
 *   generateSuiteReport,
 *   formatReportMarkdown
 * } from './benchmarks'
 *
 * // Get the suite
 * const suite = TIER1_GREP_IMPOSSIBLE_SUITE
 *
 * // Validate it
 * const validation = validateSuiteComposition(suite)
 * console.log('Valid:', validation.passed)
 *
 * // Generate report
 * const report = generateSuiteReport(suite)
 * const markdown = formatReportMarkdown(report)
 * console.log(markdown)
 * ```
 */

// Suite definition
export {
  TIER1_GREP_IMPOSSIBLE_SUITE,
  getTasksByCategory,
  getTasksByDifficulty,
  getSuiteStatistics,
  type BenchmarkSuite,
  type SuiteMetadata,
  type CategoryStatistics,
  type DifficultyStatistics,
  type SuiteStatistics,
} from './tier1-impossible.js'

// Validation
export {
  validateSuiteComposition,
  formatValidationSummary,
  type ValidationResult,
  type ValidationFailure,
} from './validation.js'

// Reporting
export {
  generateSuiteReport,
  formatReportMarkdown,
  formatReportText,
  formatCompactSummary,
  type SuiteReport,
  type SuiteSummary,
  type TaskInfo,
} from './reporter.js'

// Suite execution
export {
  runBenchmarkSuite,
  calculateAggregateMetrics,
  validateSuiteResults,
  formatSuiteSummary,
  type SuiteResult,
  type TaskResult,
  type AggregateMetrics,
  type ValidationStatus,
  type SuiteRunConfig,
} from './suite-runner.js'
