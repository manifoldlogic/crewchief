/**
 * Architectural Understanding Tasks
 *
 * Tasks that require understanding system-level data flows, initialization sequences,
 * and component interactions. These are "grep-impossible" tasks because they require
 * piecing together multiple components and understanding their relationships across
 * architectural boundaries.
 *
 * Grep can find individual components but cannot:
 * - Trace data flow across multiple layers
 * - Understand initialization ordering and dependencies
 * - Connect client-server interactions across process boundaries
 * - Identify architectural patterns and their purpose
 *
 * Semantic search with architectural understanding can solve these by:
 * - Querying for conceptual flows ("request flow", "startup sequence")
 * - Understanding layer separation (CLI, business logic, infrastructure)
 * - Recognizing communication patterns (IPC, protocols, message passing)
 * - Identifying temporal relationships (initialization order, dependencies)
 */

export { TASK_DATA_FLOW_WORKTREE_CREATION } from './data-flow.js'
export { TASK_INIT_SEQUENCE_ORCHESTRATOR } from './init-sequence.js'
export { TASK_SYSTEM_INTERACTIONS_MCP_SEARCH } from './system-interactions.js'
