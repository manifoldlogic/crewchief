/**
 * Main daemon client implementation
 */

import { createInterface } from "node:readline";
import {
  DaemonError,
  DaemonTimeoutError,
  DaemonCrashError,
  DaemonUnhealthyError,
} from "./errors.js";
import { RpcProtocol, type JsonRpcResponse } from "./rpc.js";
import { DaemonLifecycle } from "./lifecycle.js";
import type {
  ConfidenceSignals,
  DaemonConfig,
  DaemonProcessDef,
  PendingRequest,
  SearchMetadata,
} from "./types.js";

/**
 * Search parameters for daemon search method
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
 * Daemon client for communicating with crewchief-maproom daemon
 */
export class DaemonClient {
  private daemonProcess?: DaemonProcessDef;
  private lifecycle: DaemonLifecycle;
  private requestId = 0;
  private pendingRequests = new Map<number, PendingRequest>();
  private isStarting = false;
  private isShuttingDown = false;

  constructor(private readonly config: DaemonConfig) {
    this.lifecycle = new DaemonLifecycle(config);
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
   * Explicitly start the daemon (optional - daemon will auto-start on first request)
   */
  async start(): Promise<void> {
    if (this.daemonProcess) {
      return; // Already started
    }

    if (this.isStarting) {
      // Wait for existing start operation
      while (this.isStarting) {
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      return;
    }

    this.isStarting = true;

    try {
      this.daemonProcess = await this.lifecycle.start();
      this.setupProcessHandlers();
      this.setupStdoutReader();
    } finally {
      this.isStarting = false;
    }
  }

  /**
   * Stop the daemon gracefully
   *
   * Waits for in-flight requests to complete (up to shutdownTimeout),
   * then stops the daemon process. New requests are rejected during shutdown.
   */
  async stop(): Promise<void> {
    if (!this.daemonProcess || this.isShuttingDown) {
      return;
    }

    this.isShuttingDown = true;

    try {
      // Wait for in-flight requests to complete (with timeout)
      if (this.pendingRequests.size > 0) {
        const shutdownTimeout = this.config.shutdownTimeout ?? 5000;
        const pendingPromises = Array.from(this.pendingRequests.values()).map(
          (req) =>
            new Promise<void>((resolve) => {
              // Wrap in timeout to ensure we don't wait forever
              const originalResolve = req.resolve;
              const originalReject = req.reject;

              req.resolve = (value: unknown) => {
                originalResolve(value);
                resolve();
              };

              req.reject = (error: Error) => {
                originalReject(error);
                resolve();
              };
            }),
        );

        // Wait for all requests to complete OR timeout
        await Promise.race([
          Promise.all(pendingPromises),
          new Promise<void>((resolve) => setTimeout(resolve, shutdownTimeout)),
        ]);
      }

      // Reject any remaining pending requests (if timeout occurred)
      for (const pending of this.pendingRequests.values()) {
        clearTimeout(pending.timer);
        pending.reject(
          new DaemonError("Daemon is shutting down", "DAEMON_SHUTTING_DOWN"),
        );
      }
      this.pendingRequests.clear();

      await this.lifecycle.stop(this.daemonProcess);
      this.daemonProcess = undefined;
    } finally {
      this.isShuttingDown = false;
    }
  }

  /**
   * Restart the daemon
   */
  async restart(): Promise<void> {
    await this.stop();
    await this.start();
  }

  /**
   * Check if daemon is healthy (running and responsive)
   */
  async isHealthy(): Promise<boolean> {
    try {
      await this.ping();
      return true;
    } catch (error) {
      return false;
    }
  }

  /**
   * Get next request ID with rollover handling
   *
   * Request IDs are sequential integers (1, 2, 3...) that reset to 1
   * when reaching Number.MAX_SAFE_INTEGER to prevent overflow.
   *
   * Note: Node.js is single-threaded, so no mutex needed for increment.
   */
  private getNextRequestId(): number {
    this.requestId++;

    // Handle overflow - rollover to 1 (not 0, which is reserved for notifications)
    if (this.requestId > Number.MAX_SAFE_INTEGER) {
      this.requestId = 1;
    }

    return this.requestId;
  }

  /**
   * Send a JSON-RPC request to the daemon
   */
  private async sendRequest<T>(method: string, params?: unknown): Promise<T> {
    // Reject new requests during shutdown
    if (this.isShuttingDown) {
      throw new DaemonError("Daemon is shutting down", "DAEMON_SHUTTING_DOWN");
    }

    // Ensure daemon is running
    if (!this.daemonProcess) {
      await this.start();
    }

    if (!this.daemonProcess) {
      throw new DaemonUnhealthyError("Failed to start daemon");
    }

    const id = this.getNextRequestId();
    const request = RpcProtocol.createRequest(method, params, id);
    const requestLine = RpcProtocol.serializeRequest(request);

    // Create promise for response
    let promiseResolve: (value: T) => void;
    let promiseReject: (error: Error) => void;

    const promise = new Promise<T>((resolve, reject) => {
      promiseResolve = resolve;
      promiseReject = reject;
    });

    const timeout = this.config.timeout ?? 30000;
    const timestamp = Date.now();

    // Set up timeout
    const timer = setTimeout(() => {
      this.pendingRequests.delete(id);
      promiseReject(
        new DaemonTimeoutError(
          `Request ${id} (${method}) timed out after ${timeout}ms`,
        ),
      );
    }, timeout);

    // Store pending request
    this.pendingRequests.set(id, {
      promise,
      resolve: promiseResolve! as (value: unknown) => void,
      reject: promiseReject!,
      timestamp,
      timer,
    });

    // Send request to daemon
    try {
      this.daemonProcess!.stdin.write(requestLine);
    } catch (error) {
      this.pendingRequests.delete(id);
      clearTimeout(timer);
      promiseReject!(
        new DaemonError(
          `Failed to send request to daemon: ${error instanceof Error ? error.message : String(error)}`,
          "WRITE_FAILED",
          error instanceof Error ? error : undefined,
        ),
      );
    }

    return promise;
  }

  /**
   * Handle response from daemon
   */
  private handleResponse(response: JsonRpcResponse): void {
    if (response.id === null) {
      // Notification (no response expected) - ignore
      return;
    }

    const pending = this.pendingRequests.get(response.id);
    if (!pending) {
      // Response for unknown request - ignore
      console.warn(`Received response for unknown request ID: ${response.id}`);
      return;
    }

    // Remove pending request
    this.pendingRequests.delete(response.id);
    clearTimeout(pending.timer);

    // Handle response
    try {
      const result = RpcProtocol.extractResult(response);
      pending.resolve(result);

      // Reset restart attempts on successful operation
      this.lifecycle.resetRestartAttempts();
    } catch (error) {
      pending.reject(error instanceof Error ? error : new Error(String(error)));
    }
  }

  /**
   * Set up stdout reader for responses
   */
  private setupStdoutReader(): void {
    if (!this.daemonProcess) {
      return;
    }

    const reader = createInterface({
      input: this.daemonProcess.stdout,
      crlfDelay: Infinity,
    });

    reader.on("line", (line) => {
      try {
        const response = RpcProtocol.parseResponse(line);
        this.handleResponse(response);
      } catch (error) {
        console.error(
          `Failed to parse daemon response: ${error instanceof Error ? error.message : String(error)}`,
        );
      }
    });

    reader.on("close", () => {
      // Stdout closed - daemon likely exited
      this.handleDaemonExit();
    });
  }

  /**
   * Set up process event handlers
   */
  private setupProcessHandlers(): void {
    if (!this.daemonProcess) {
      return;
    }

    this.daemonProcess.process.on("exit", (code, signal) => {
      this.handleDaemonExit(code, signal);
    });

    this.daemonProcess.process.on("error", (error) => {
      console.error(`Daemon process error: ${error.message}`);
    });

    // Log stderr for debugging
    const stderrReader = createInterface({
      input: this.daemonProcess.stderr,
      crlfDelay: Infinity,
    });

    stderrReader.on("line", (line) => {
      console.error(`[Daemon stderr] ${line}`);
    });
  }

  /**
   * Handle daemon exit
   */
  private handleDaemonExit(code?: number | null, signal?: string | null): void {
    const wasRunning = this.daemonProcess !== undefined;
    this.daemonProcess = undefined;

    // Reject all pending requests
    for (const pending of this.pendingRequests.values()) {
      clearTimeout(pending.timer);
      pending.reject(
        new DaemonCrashError(
          `Daemon exited unexpectedly (code: ${code ?? "unknown"}, signal: ${signal ?? "none"})`,
          code ?? undefined,
          signal ?? undefined,
        ),
      );
    }
    this.pendingRequests.clear();

    // Auto-restart if configured and not shutting down
    if (wasRunning && !this.isShuttingDown && this.lifecycle.shouldRestart()) {
      const delay = this.lifecycle.getBackoffDelay();
      console.log(`Daemon crashed, restarting in ${delay}ms...`);
      setTimeout(() => {
        this.start().catch((error) => {
          console.error(
            `Failed to restart daemon: ${error instanceof Error ? error.message : String(error)}`,
          );
        });
      }, delay);
    }
  }
}
