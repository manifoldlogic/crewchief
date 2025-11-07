/**
 * Relationship Discovery Tasks
 *
 * Tasks that require understanding code graphs and transitive relationships.
 * These are "grep-impossible" tasks because they require traversing dependency
 * chains, tracing execution flows, and analyzing impact across multiple files.
 *
 * Grep can find direct references but cannot:
 * - Traverse transitive dependencies
 * - Connect execution flows across layers
 * - Perform impact analysis for API changes
 * - Reason about indirect relationships
 *
 * Semantic search with code graph understanding can solve these by:
 * - Querying for conceptual relationships ("depends on", "calls", "affects")
 * - Following import and call chains
 * - Understanding architectural patterns
 * - Filtering by semantic relevance
 */

export { TASK_TRANSITIVE_DEPENDENCIES } from './transitive-dependencies.js'
export { TASK_CALL_CHAIN_TRACING } from './call-chain.js'
export { TASK_API_IMPACT_ANALYSIS } from './impact-analysis.js'
