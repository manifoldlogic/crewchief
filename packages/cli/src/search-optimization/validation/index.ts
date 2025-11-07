/**
 * Task validation infrastructure
 *
 * Validates search tasks across 5 quality dimensions:
 * 1. Construct Validity - Grep baseline difficulty
 * 2. Discriminant Validity - Search advantage
 * 3. Ecological Validity - Real-world realism
 * 4. Test-Retest Reliability - Result consistency
 * 5. Statistical Power - Sample size adequacy
 */

export {
  validateTask,
  validateSuite,
  formatValidationReport,
  formatSuiteValidationReport,
  DEFAULT_THRESHOLDS,
} from './task-validator.js'

export type {
  ValidationConfig,
  ValidationThresholds,
  TierThresholds,
  DimensionResult,
  ValidationResult,
  SuiteValidationResult,
} from './task-validator.js'
