/**
 * Ambiguity Resolution Tasks (Tier 2)
 *
 * Tasks where keywords have multiple meanings requiring context.
 * Grep struggles (30-60% success) because:
 * - Keywords appear in multiple contexts (transaction: financial vs database)
 * - Must distinguish implementation from discussion/comments
 * - Same keyword for different operation types (cache read vs write)
 *
 * Semantic search excels (>70% success) because:
 * - Disambiguates context from surrounding code
 * - Distinguishes implementation from mentions
 * - Understands operation intent from patterns
 */

export { TASK_TRANSACTION_MANAGEMENT } from './transaction-management.js'
export { TASK_AUTHENTICATION_CHECKS } from './authentication-checks.js'
export { TASK_RESOURCE_CLEANUP } from './resource-cleanup.js'
export { TASK_CACHE_OPERATIONS } from './cache-operations.js'
