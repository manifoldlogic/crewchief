/**
 * Evaluation infrastructure for search optimization
 *
 * Exports baseline runner for grep-only evaluation, comparison framework
 * for side-by-side evaluations, and statistical analysis utilities.
 */

// Baseline runner (grep-only evaluation)
export {
  BaselineConfig,
  BaselineResult,
  BaselineMetrics,
  runBaseline,
  formatBaselineReport,
} from './baseline-runner.js'

// Comparison framework (side-by-side evaluation)
export { ComparisonConfig, ComparisonResult, runComparison } from './comparison.js'

// Metrics calculation
export {
  AdvantageMetrics,
  AggregatedMetrics,
  calculateAdvantage,
  aggregateMetrics,
  calculateSearchUsageRate,
  extractBaselineScores,
  extractSearchScores,
  extractBaselineTimes,
  extractSearchTimes,
  formatAdvantageMetrics,
  formatAggregatedMetrics,
} from './metrics.js'

// Statistical analysis
export {
  TTestResult,
  EffectSizeResult,
  ConfidenceInterval,
  mean,
  variance,
  standardDeviation,
  tTest,
  cohensD,
  confidenceInterval,
  confidenceIntervalDifference,
} from './statistics.js'
