/**
 * Conceptual Similarity Tasks (Tier 2)
 *
 * Tasks that find implementations using different terminology but the same concept.
 * Grep struggles (30-60% success) because:
 * - Multiple valid keywords for the same concept
 * - Different implementation patterns (functional vs imperative)
 * - Implicit patterns without explicit keywords
 *
 * Semantic search excels (>70% success) because:
 * - Understands conceptual relationships
 * - Recognizes patterns across syntax variations
 * - Connects related implementations
 */

export { TASK_RETRY_IMPLEMENTATIONS } from './retry-implementations.js'
export { TASK_ERROR_HANDLING_PATTERNS } from './error-handling-patterns.js'
export { TASK_RATE_LIMITING } from './rate-limiting.js'
export { TASK_CACHING_STRATEGIES } from './caching-strategies.js'
