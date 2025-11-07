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
 *   TIER2_GREP_HARD_SUITE,
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

// Tier 1 suite (grep-impossible)
export {
  TIER1_GREP_IMPOSSIBLE_SUITE,
  getTasksByCategory as getTier1TasksByCategory,
  getTasksByDifficulty as getTier1TasksByDifficulty,
  getSuiteStatistics as getTier1SuiteStatistics,
  type BenchmarkSuite,
  type SuiteMetadata,
  type CategoryStatistics,
  type DifficultyStatistics,
  type SuiteStatistics,
} from './tier1-impossible.js'

// Tier 2 suite (grep-hard)
export {
  TIER2_GREP_HARD_SUITE,
  getTasksByCategory as getTier2TasksByCategory,
  getTasksByDifficulty as getTier2TasksByDifficulty,
  getSuiteStatistics as getTier2SuiteStatistics,
} from './tier2-hard.js'

// Tier 3 suite (real-world)
export {
  TIER3_REALWORLD_SUITE,
  getTasksByCategory as getTier3TasksByCategory,
  getTasksByDifficulty as getTier3TasksByDifficulty,
  getTasksByFrequency as getTier3TasksByFrequency,
  getSuiteStatistics as getTier3SuiteStatistics,
} from './tier3-realworld.js'

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
