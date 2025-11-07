/**
 * Search Optimization Module
 *
 * Exports for search description optimization, benchmarking, and validation.
 */

// Validation runner
export {
  runFullValidation,
  estimateCost,
  type ValidationOptions,
  type ValidationResults,
  type ConditionResults,
  type TierSummary,
  type CategorySummary,
} from './scripts/run-full-validation.js'

// Statistical analysis
export {
  performStatisticalAnalysis,
  pairedTTest,
  cohensD,
  interpretEffectSize,
  confidenceInterval95,
  median,
  type StatisticalAnalysis,
} from './reporting/statistics.js'

// Report generation
export { generateValidationReport, saveValidationReport } from './reporting/validation-report.js'

// Multi-tier scoring
export {
  calculateMultiTierScore,
  aggregateMultiTierScores,
  checkMultiTierConvergence,
  formatMultiTierScore,
  DEFAULT_TIER_WEIGHTS,
  type MultiTierScore,
  type TierWeights,
  type TierSuiteResult,
  type TierMetrics,
  type ToolSelectionMetrics,
} from './multi-tier-scoring.js'

// Competition runner
export { runCompetition, type CompetitionConfig, type CompetitionResult } from './competition-runner.js'

// Genetic iterator
export {
  runGeneticIterations,
  generateNextGeneration,
  type IterationConfig,
  type GenerationResult,
} from './genetic-iterator.js'

// Types
export type { SearchTask, AgentOutput, TaskScore } from './types.js'

// Benchmarks
export {
  TIER1_GREP_IMPOSSIBLE_SUITE,
  TIER2_GREP_HARD_SUITE,
  TIER3_REALWORLD_SUITE,
  type BenchmarkSuite,
} from './benchmarks/index.js'
