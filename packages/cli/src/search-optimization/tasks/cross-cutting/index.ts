/**
 * Cross-Cutting Concerns Tasks (Tier 2)
 *
 * Tasks for scattered patterns that need aggregation across the codebase.
 * Grep struggles (30-60% success) because:
 * - Patterns are scattered across many files
 * - Requires aggregation and synthesis
 * - No single location contains all instances
 * - Context needed to identify relevant instances
 *
 * Semantic search excels (>70% success) because:
 * - Aggregates scattered patterns effectively
 * - Recognizes cross-cutting concerns
 * - Synthesizes patterns from multiple locations
 */

export { TASK_ASYNC_ERROR_HANDLING } from './async-error-handling.js'
export { TASK_SECURITY_LOGGING } from './security-logging.js'
export { TASK_INPUT_VALIDATION } from './input-validation.js'
