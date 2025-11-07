/**
 * Evaluation infrastructure for search optimization
 *
 * Exports baseline runner for grep-only evaluation and future
 * comparison utilities.
 */

export {
  BaselineConfig,
  BaselineResult,
  BaselineMetrics,
  runBaseline,
  formatBaselineReport,
} from './baseline-runner.js'
