/**
 * @maproom/daemon-client - TypeScript client for crewchief-maproom daemon
 *
 * Provides a high-level interface for communicating with the maproom daemon
 * via JSON-RPC 2.0 over stdio.
 *
 * @example
 * ```typescript
 * import { DaemonClient } from '@maproom/daemon-client'
 *
 * const client = new DaemonClient({
 *   binaryPath: '/path/to/crewchief-maproom',
 *   env: {
 *     MAPROOM_DATABASE_URL: 'postgresql://...',
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
export type { SearchParams, SearchResult } from './client.js'

// Configuration
export type {DaemonConfig } from './lifecycle.js'

// Errors
export {
  DaemonError,
  DaemonStartError,
  DaemonCrashError,
  DaemonTimeoutError,
  RpcError,
  DaemonUnhealthyError,
} from './errors.js'

// JSON-RPC protocol (for advanced usage)
export { RpcProtocol } from './rpc.js'
export type {
  JsonRpcRequest,
  JsonRpcResponse,
  JsonRpcErrorObject,
} from './rpc.js'
