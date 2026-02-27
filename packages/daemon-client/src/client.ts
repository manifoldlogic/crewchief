/**
 * Main daemon client implementation
 *
 * Provides a high-level interface for communicating with the maproom daemon.
 * Supports both socket and stdio connection modes with automatic fallback.
 */

import { Connection, ConnectionMode, ConnectionConfig } from "./connection.js";
import { createConnection } from "./connection-factory.js";
import { DaemonError } from "./errors.js";
import type { ConfidenceSignals, SearchMetadata } from "./types.js";

/**
 * Search parameters for daemon search method
 * TYPE_SYNC: Must match crates/maproom/src/daemon/types.rs::SearchParams
 */
export interface SearchParams {
  query: string;
  repo: string;
  worktree?: string;
  limit?: number;
  threshold?: number;
  debug?: boolean;
  /** Deduplicate results across worktrees (default: true) */
  deduplicate?: boolean;
  /** Include confidence signals for result quality assessment (default: false) */
  include_confidence?: boolean;
  /** Include related chunks via graph traversal (default: false) */
  include_related?: boolean;
  /** Filter by chunk kind (e.g., ["func", "class"]) */
  kind?: string[];
  /** Filter by file language (e.g., ["py", "ts"]) */
  lang?: string[];
}

/**
 * Search result from daemon
 *
 * Sync with: crates/maproom/src/db/mod.rs SearchHit
 */
export interface SearchResult {
  hits: Array<{
    chunk_id: number;
    file_path: string;
    start_line: number;
    end_line: number;
    symbol_name: string | null;
    kind: string;
    content: string;
    score: number;
    /** Confidence signals for result quality assessment (present when include_confidence=true) */
    confidence?: ConfidenceSignals;
  }>;
  total: number;
  query_embedding_time_ms?: number;
  search_time_ms?: number;
  metadata?: SearchMetadata;
}

/**
 * Context parameters for daemon context method
 *
 * Sync with: crates/maproom/src/daemon/types.rs ContextParams
 */
export interface ContextParams {
  chunk_id: string;
  budget_tokens?: number;
  expand?: {
    callers?: boolean;
    callees?: boolean;
    tests?: boolean;
    docs?: boolean;
    config?: boolean;
    max_depth?: number;
    routes?: boolean;
    hooks?: boolean;
    jsx_parents?: boolean;
    jsx_children?: boolean;
  };
}

/**
 * Context item in a bundle
 *
 * Sync with: crates/maproom/src/context/types.rs ContextItem
 */
export interface RustContextItem {
  relpath: string;
  range: {
    start: number;
    end: number;
  };
  role: string;
  reason: string;
  content: string;
  tokens: number;
}

/**
 * Context bundle from daemon
 *
 * Sync with: crates/maproom/src/context/types.rs ContextBundle
 */
export interface RustContextBundle {
  items: RustContextItem[];
  total_tokens: number;
  truncated: boolean;
}

/**
 * Parameters for status request
 *
 * Sync with: crates/maproom/src/daemon/types.rs StatusParams
 */
export interface StatusParams {
  repo?: string;
}

/**
 * Worktree statistics in status response
 *
 * Sync with: crates/maproom/src/daemon/types.rs WorktreeStatus
 */
export interface WorktreeStatus {
  name: string;
  path: string;
  file_count: number;
  chunk_count: number;
}

/**
 * Repository statistics in status response
 *
 * Sync with: crates/maproom/src/daemon/types.rs RepoStatus
 */
export interface RepoStatus {
  name: string;
  worktrees: WorktreeStatus[];
}

/**
 * Status result from daemon
 *
 * Sync with: crates/maproom/src/daemon/types.rs StatusResult
 */
export interface StatusResult {
  repos: RepoStatus[];
  total_repos: number;
  total_files: number;
  total_chunks: number;
}

/**
 * Configuration for DaemonClient
 *
 * Supports legacy DaemonConfig fields for backward compatibility.
 */
export interface DaemonClientConfig extends Partial<ConnectionConfig> {
  // Legacy fields (mapped to ConnectionConfig)
  binaryPath?: string;
  timeout?: number;
  env?: NodeJS.ProcessEnv;
  startTimeout?: number;
  shutdownTimeout?: number;
  maxRestartAttempts?: number;
  restartBackoffMs?: number;
  autoRestart?: boolean;
}

/**
 * Daemon client for communicating with maproom daemon
 *
 * Automatically selects the best connection mode (socket or stdio) based on
 * platform and configuration. Supports both explicit mode selection and
 * automatic fallback.
 *
 * @example
 * ```typescript
 * // Auto-detect connection mode (backward compatible)
 * const client = new DaemonClient()
 * await client.connect()
 * const results = await client.search({ query: 'test', repo: 'my-repo' })
 * await client.close()
 * ```
 *
 * @example
 * ```typescript
 * // Explicit mode selection
 * const client = new DaemonClient({ mode: ConnectionMode.Socket })
 * await client.connect()
 * ```
 *
 * @example
 * ```typescript
 * // Legacy config (still works)
 * const client = new DaemonClient({ binaryPath: '/path/to/daemon' })
 * await client.connect()
 * ```
 */
export class DaemonClient {
  private connection: Connection | null = null;
  private isConnecting = false;

  constructor(private readonly config: DaemonClientConfig = {}) {}

  /**
   * Connect to the daemon
   *
   * Creates a connection using the configured mode (or auto-detect).
   * This is optional - the client will auto-connect on first request.
   *
   * @throws {DaemonError} if connection fails
   */
  async connect(): Promise<void> {
    if (this.connection) {
      return; // Already connected
    }

    if (this.isConnecting) {
      // Wait for existing connection attempt
      while (this.isConnecting) {
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      return;
    }

    this.isConnecting = true;

    try {
      // Map legacy config to ConnectionConfig
      const connectionConfig: Partial<ConnectionConfig> = {
        mode: this.config.mode,
        socketPath: this.config.socketPath,
        binaryPath: this.config.binaryPath,
        startupTimeout: this.config.startupTimeout ?? this.config.startTimeout,
      };

      this.connection = await createConnection(connectionConfig);
    } finally {
      this.isConnecting = false;
    }
  }

  /**
   * Send ping request to check daemon health
   */
  async ping(): Promise<string> {
    return await this.sendRequest<string>("ping");
  }

  /**
   * Send search request to daemon
   */
  async search(params: SearchParams): Promise<SearchResult> {
    // Ensure deduplicate has a default value (true)
    const searchParams = {
      ...params,
      deduplicate: params.deduplicate ?? true,
    };
    return await this.sendRequest<SearchResult>("search", searchParams);
  }

  /**
   * Send context request to daemon
   *
   * Retrieves a context bundle for a chunk, optionally including
   * related code (callers, callees, tests, etc.) within a token budget.
   */
  async context(params: ContextParams): Promise<RustContextBundle> {
    // Apply default budget_tokens
    const contextParams = {
      ...params,
      budget_tokens: params.budget_tokens ?? 6000,
    };
    return await this.sendRequest<RustContextBundle>("context", contextParams);
  }

  /**
   * Send status request to daemon
   *
   * Retrieves repository and worktree statistics from the database.
   */
  async status(params: StatusParams = {}): Promise<StatusResult> {
    return await this.sendRequest<StatusResult>("status", params);
  }

  /**
   * Close the connection gracefully
   *
   * Waits for pending requests to complete and cleans up resources.
   */
  async close(): Promise<void> {
    if (this.connection) {
      await this.connection.close();
      this.connection = null;
    }
  }

  /**
   * Check if the client is connected
   */
  isConnected(): boolean {
    return this.connection?.isConnected() ?? false;
  }

  /**
   * Register error event handler
   */
  onError(handler: (err?: Error) => void): void {
    this.connection?.on("error", handler);
  }

  /**
   * Register close event handler
   */
  onClose(handler: (err?: Error) => void): void {
    this.connection?.on("close", handler);
  }

  /**
   * Send a JSON-RPC request to the daemon
   *
   * Auto-connects if not already connected.
   *
   * @param method - RPC method name
   * @param params - Optional method parameters
   * @returns Promise resolving to the result
   * @throws {DaemonError} if request fails
   */
  private async sendRequest<T>(method: string, params?: unknown): Promise<T> {
    // Auto-connect if needed
    if (!this.connection) {
      await this.connect();
    }

    if (!this.connection) {
      throw new DaemonError("Failed to connect to daemon", "CONNECTION_FAILED");
    }

    return await this.connection.sendRequest<T>(method, params);
  }

  // Legacy methods for backward compatibility

  /**
   * Start the daemon (alias for connect() for backward compatibility)
   *
   * @deprecated Use connect() instead
   */
  async start(): Promise<void> {
    await this.connect();
  }

  /**
   * Stop the daemon (alias for close() for backward compatibility)
   *
   * @deprecated Use close() instead
   */
  async stop(): Promise<void> {
    await this.close();
  }

  /**
   * Check if daemon is healthy (same as isConnected for connection mode)
   *
   * @deprecated Use isConnected() instead, or call ping() to verify responsiveness
   */
  async isHealthy(): Promise<boolean> {
    if (!this.isConnected()) {
      return false;
    }

    try {
      await this.ping();
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Restart the daemon (reconnect)
   *
   * @deprecated Connection mode handles reconnection automatically
   */
  async restart(): Promise<void> {
    await this.close();
    await this.connect();
  }
}
