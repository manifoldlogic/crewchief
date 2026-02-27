/**
 * @maproom/daemon-client - TypeScript client for maproom daemon
 *
 * Provides a high-level interface for communicating with the maproom daemon
 * via JSON-RPC 2.0 over stdio.
 *
 * @example
 * ```typescript
 * import { DaemonClient } from '@maproom/daemon-client'
 *
 * const client = new DaemonClient({
 *   binaryPath: '/path/to/maproom',
 *   env: {
 *     MAPROOM_DATABASE_URL: 'sqlite://~/.maproom/maproom.db',
 *     OPENAI_API_KEY: 'sk-...',
 *   },
 *   timeout: 30000,
 *   autoRestart: true,
 * })
 *
 * // Search (daemon auto-starts on first request)
 * const results = await client.search({
 *   query: 'function parseConfig',
 *   repo: 'my-repo',
 *   worktree: 'main',
 *   limit: 10,
 * })
 *
 * // Cleanup
 * await client.stop()
 * ```
 */

// Main client
export { DaemonClient } from './client.js'
export type {
  SearchParams,
  SearchResult,
  ContextParams,
  RustContextItem,
  RustContextBundle,
  StatusParams,
  StatusResult,
  RepoStatus,
  WorktreeStatus,
} from './client.js'

// Configuration and types
export type {
  DaemonConfig,
  DaemonProcessDef,
  PendingRequest,
  ErrorType,
  PipelineStage,
  SearchErrorDetails,
  QueryUnderstanding,
  QueryFilters,
  TimingBreakdown,
  ConfidenceSignals,
  SearchMetadata,
} from './types.js'

// Errors
export {
  DaemonError,
  DaemonStartError,
  DaemonCrashError,
  DaemonTimeoutError,
  RpcError,
  DaemonUnhealthyError,
  DaemonCommunicationError,
  SocketConnectionError,
  SocketTimeoutError,
  DaemonStartupError,
  DaemonLockError,
} from './errors.js'

// Connection interface and implementations
export { Connection, ConnectionMode, ConnectionConfig } from './connection.js'
export { SocketConnection } from './socket.js'
export { StdioConnection } from './stdio.js'
export type { RequestId } from './socket.js'

// Connection factory
export { createConnection, detectConnectionMode } from './connection-factory.js'

// JSON-RPC protocol (for advanced usage)
export { RpcProtocol } from './rpc.js'
export type {
  JsonRpcRequest,
  JsonRpcResponse,
  JsonRpcErrorObject,
} from './rpc.js'

// Daemon discovery and auto-start
export { connectOrSpawn, getDefaultConfig } from './discovery.js'
export type { DiscoveryConfig } from './discovery.js'

// Filtering functionality
export { FilterableSearchResult } from './filterable-result.js'
export type { SearchHit } from './filterable-result.js'
export type { FilterCriteria, SortField, SortOrder } from './filter-types.js'
