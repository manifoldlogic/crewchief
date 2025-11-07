/**
 * Negative Space Tasks
 *
 * Tasks that search for what's MISSING rather than what's present.
 * These are "grep-impossible" tasks because grep can only find patterns that exist,
 * not patterns that are absent.
 *
 * Grep can find:
 * - Code that HAS error handling
 * - Code that HAS validation
 * - Code that HAS security checks
 *
 * Grep CANNOT find:
 * - Code that LACKS error handling
 * - Code that LACKS validation
 * - Code that LACKS security checks
 *
 * To solve with grep requires:
 * 1. Find ALL instances of pattern A (e.g., all async functions)
 * 2. Find ALL instances of pattern B (e.g., all error handlers)
 * 3. Manually cross-reference to find A without B
 * 4. Account for multiple forms of B (try-catch, .catch(), helpers, wrappers)
 *
 * This becomes exponentially complex as:
 * - Codebase grows (more instances to cross-reference)
 * - Protection patterns vary (multiple ways to protect code)
 * - Context matters (same code may be safe in one context, unsafe in another)
 *
 * Semantic search solves this by:
 * - Understanding concepts like "unprotected" and "without validation"
 * - Reasoning about code quality and risk
 * - Identifying patterns by their absence, not presence
 * - Understanding context and semantic relationships
 */

export { TASK_MISSING_ERROR_HANDLING } from './missing-error-handling.js'
export { TASK_UNPROTECTED_FILE_OPERATIONS } from './unprotected-operations.js'
