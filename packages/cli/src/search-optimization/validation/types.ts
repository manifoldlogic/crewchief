/**
 * Types for pre-flight validation
 */

/**
 * Result of a single validation check
 */
export interface CheckResult {
  passed: boolean
  message: string
  details?: Record<string, unknown>
}

/**
 * Index status for a worktree
 */
export interface IndexStatus {
  indexed: boolean
  chunkCount: number
}

/**
 * Validation error with troubleshooting guidance
 */
export interface ValidationError {
  check: string
  message: string
  troubleshooting?: string
}

/**
 * Validation warning (non-blocking)
 */
export interface ValidationWarning {
  check: string
  message: string
}

/**
 * Environment for a single variant
 * This represents a worktree environment where the variant will run
 */
export interface VariantEnvironment {
  variantId: string
  worktreePath: string
  repo: string
  worktree: string
}

/**
 * Validation result for a single variant
 */
export interface VariantValidation {
  variantId: string
  worktreePath: string
  checks: {
    worktreeExists: CheckResult
    worktreeScanned: CheckResult
    mcpConfigValid: CheckResult
    toolsAccessible: CheckResult
    filePermissions: CheckResult
  }
  overall: 'pass' | 'fail'
  failureReason?: string
}

/**
 * Overall validation result for competition setup
 */
export interface ValidationResult {
  valid: boolean
  errors: ValidationError[]
  warnings: ValidationWarning[]
  variantResults: Map<string, VariantValidation>
}

/**
 * Competition configuration (referenced from competition-runner)
 */
export interface CompetitionConfig {
  task: Record<string, unknown> // SearchTask
  variants: Array<Record<string, unknown>> // Variant[]
  parallelExecution?: boolean
  timeout?: number
  baseDir?: string
}
