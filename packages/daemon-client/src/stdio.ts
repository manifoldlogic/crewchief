/**
 * Stdio-based connection implementation for daemon communication
 *
 * Spawns a daemon process and communicates via stdin/stdout using
 * newline-delimited JSON-RPC messages.
 */

import { createInterface } from 'node:readline'
import { Connection } from './connection.js'
import { RpcProtocol, type JsonRpcResponse } from './rpc.js'
import { DaemonLifecycle } from './lifecycle.js'
import {
  DaemonCommunicationError,
  DaemonCrashError,
  DaemonStartupError,
  DaemonTimeoutError,
} from './errors.js'
import type { DaemonConfig, DaemonProcessDef, PendingRequest } from './types.js'

/**
 * Request ID type (sequential integers)
 */
export type RequestId = number

/**
 * Stdio connection to daemon process
 *
 * Handles:
 * - Process spawning and lifecycle
 * - Line-based JSON-RPC communication
 * - Request/response multiplexing via request IDs
 * - Timeout handling for individual requests
 * - Graceful shutdown and cleanup
 * - Auto-restart on crashes (configurable)
 */
export class StdioConnection implements Connection {
  private daemonProcess?: DaemonProcessDef
  private lifecycle: DaemonLifecycle
  private pendingRequests = new Map<RequestId, PendingRequest>()
  private nextId = 1
  private connected = false
  private errorHandlers: Array<(err?: Error) => void> = []
  private closeHandlers: Array<(err?: Error) => void> = []
  private isShuttingDown = false

  constructor(private readonly binaryPath: string) {
    // Create lifecycle manager with stdio-specific defaults
    const config: DaemonConfig = {
      binaryPath,
      timeout: 30000,
      autoRestart: true,
      maxRestartAttempts: 5,
      restartBackoffMs: 1000,
      startTimeout: 5000,
      shutdownTimeout: 5000,
    }
    this.lifecycle = new DaemonLifecycle(config)
  }

  /**
   * Start the daemon process and wait for it to be ready
   *
   * @throws {DaemonStartupError} if daemon fails to start
   */
  async connect(): Promise<void> {
    if (this.daemonProcess) {
      return // Already connected
    }

    try {
      this.daemonProcess = await this.lifecycle.start()
      this.setupHandlers()
      this.connected = true

      // Wait for daemon to be ready (send ping to verify)
      await this.waitForReady()
    } catch (error) {
      this.connected = false
      this.daemonProcess = undefined
      throw error
    }
  }

  /**
   * Set up process event handlers
   */
  private setupHandlers(): void {
    if (!this.daemonProcess) return

    // Set up stdout reader for JSON-RPC responses
    const reader = createInterface({
      input: this.daemonProcess.stdout,
      crlfDelay: Infinity,
    })

    reader.on('line', (line) => {
      try {
        const response = RpcProtocol.parseResponse(line)
        this.handleMessage(response)
      } catch (error) {
        console.error(
          `Failed to parse daemon response: ${error instanceof Error ? error.message : String(error)}`
        )
      }
    })

    reader.on('close', () => {
      // Stdout closed - daemon likely exited
      this.handleDaemonExit()
    })

    // Set up stderr logging
    const stderrReader = createInterface({
      input: this.daemonProcess.stderr,
      crlfDelay: Infinity,
    })

    stderrReader.on('line', (line) => {
      console.error(`[daemon stderr] ${line}`)
    })

    // Process exit handling
    this.daemonProcess.process.on('exit', (code, signal) => {
      this.handleDaemonExit(code, signal)
    })

    this.daemonProcess.process.on('error', (error) => {
      console.error(`Daemon process error: ${error.message}`)
      this.errorHandlers.forEach((h) => h(error))
    })
  }

  /**
   * Wait for daemon to be ready by sending a ping request
   */
  private async waitForReady(timeoutMs: number = 5000): Promise<void> {
    try {
      await this.sendRequest('ping', undefined, timeoutMs)
    } catch (err) {
      throw new DaemonStartupError('Daemon failed to respond to ping', {
        cause: err as Error,
      })
    }
  }

  /**
   * Handle a complete JSON-RPC response message
   */
  private handleMessage(response: JsonRpcResponse): void {
    if (response.id === null) {
      // Notification (no response expected) - ignore
      return
    }

    const pending = this.pendingRequests.get(response.id as number)
    if (!pending) {
      console.warn('Received response for unknown request ID:', response.id)
      return
    }

    this.pendingRequests.delete(response.id as number)
    clearTimeout(pending.timer)

    try {
      const result = RpcProtocol.extractResult(response)
      pending.resolve(result)

      // Reset restart attempts on successful operation
      this.lifecycle.resetRestartAttempts()
    } catch (error) {
      pending.reject(error instanceof Error ? error : new Error(String(error)))
    }
  }

  /**
   * Send a JSON-RPC request and wait for response
   *
   * @param method - The RPC method name
   * @param params - Optional parameters
   * @param timeoutMs - Request timeout in milliseconds (default: 30000)
   * @returns Promise resolving to the result
   * @throws {DaemonCommunicationError} If not connected or write fails
   * @throws {DaemonTimeoutError} If request times out
   */
  async sendRequest<T = unknown>(
    method: string,
    params?: unknown,
    timeoutMs: number = 30000
  ): Promise<T> {
    // Reject new requests during shutdown
    if (this.isShuttingDown) {
      throw new DaemonCommunicationError('Connection is shutting down')
    }

    if (!this.connected || !this.daemonProcess) {
      throw new DaemonCommunicationError('Not connected')
    }

    const id = this.getNextRequestId()
    const request = RpcProtocol.createRequest(method, params, id)
    const requestLine = RpcProtocol.serializeRequest(request)

    return new Promise((resolve, reject) => {
      // Set up timeout
      const timer = setTimeout(() => {
        this.pendingRequests.delete(id)
        reject(
          new DaemonTimeoutError(
            `Request ${id} (${method}) timed out after ${timeoutMs}ms`
          )
        )
      }, timeoutMs)

      // Store pending request
      this.pendingRequests.set(id, {
        promise: Promise.resolve(undefined) as Promise<unknown>,
        resolve: resolve as (value: unknown) => void,
        reject,
        timestamp: Date.now(),
        timer,
      })

      // Send request to daemon
      try {
        this.daemonProcess!.stdin.write(requestLine)
      } catch (error) {
        this.pendingRequests.delete(id)
        clearTimeout(timer)
        reject(
          new DaemonCommunicationError(
            `Failed to send request to daemon: ${error instanceof Error ? error.message : String(error)}`,
            { cause: error as Error }
          )
        )
      }
    })
  }

  /**
   * Close the connection gracefully
   *
   * Waits for pending requests to complete and stops the daemon.
   */
  async close(): Promise<void> {
    if (!this.daemonProcess || this.isShuttingDown) {
      return
    }

    this.isShuttingDown = true

    try {
      // Wait for pending requests to complete (with timeout)
      if (this.pendingRequests.size > 0) {
        const shutdownTimeout = 5000
        await Promise.race([
          this.waitForPendingRequests(),
          new Promise<void>((resolve) => setTimeout(resolve, shutdownTimeout)),
        ])
      }

      // Reject any remaining pending requests
      this.rejectAllPending(
        new DaemonCommunicationError('Connection closed by client')
      )

      // Stop daemon process
      await this.lifecycle.stop(this.daemonProcess)
      this.daemonProcess = undefined
      this.connected = false
    } finally {
      this.isShuttingDown = false
    }
  }

  /**
   * Wait for all pending requests to complete
   */
  private async waitForPendingRequests(): Promise<void> {
    const pendingPromises = Array.from(this.pendingRequests.values()).map(
      (req) => req.promise.catch(() => {})
    )
    await Promise.all(pendingPromises)
  }

  /**
   * Check if the connection is active
   */
  isConnected(): boolean {
    return this.connected && this.daemonProcess !== undefined
  }

  /**
   * Register event handlers
   *
   * @param event - Event type ('error' or 'close')
   * @param handler - Handler function
   */
  on(event: 'error' | 'close', handler: (err?: Error) => void): void {
    if (event === 'error') {
      this.errorHandlers.push(handler)
    } else if (event === 'close') {
      this.closeHandlers.push(handler)
    }
  }

  /**
   * Handle daemon exit
   */
  private handleDaemonExit(code?: number | null, signal?: string | null): void {
    const wasRunning = this.daemonProcess !== undefined
    this.connected = false
    this.daemonProcess = undefined

    // Notify close handlers
    this.closeHandlers.forEach((h) => h())

    // Reject all pending requests
    const error = new DaemonCrashError(
      `Daemon exited unexpectedly (code: ${code ?? 'unknown'}, signal: ${signal ?? 'none'})`,
      code ?? undefined,
      signal ?? undefined
    )
    this.rejectAllPending(error)

    // Notify error handlers if daemon crashed
    if (code !== 0 && code !== null) {
      this.errorHandlers.forEach((h) => h(error))
    }

    // Auto-restart if configured and not shutting down
    if (wasRunning && !this.isShuttingDown && this.lifecycle.shouldRestart()) {
      const delay = this.lifecycle.getBackoffDelay()
      console.log(`Daemon crashed, restarting in ${delay}ms...`)
      setTimeout(() => {
        this.connect().catch((error) => {
          console.error(
            `Failed to restart daemon: ${error instanceof Error ? error.message : String(error)}`
          )
        })
      }, delay)
    }
  }

  /**
   * Get next request ID with rollover handling
   */
  private getNextRequestId(): number {
    this.nextId++

    // Handle overflow - rollover to 1 (not 0, which is reserved for notifications)
    if (this.nextId > Number.MAX_SAFE_INTEGER) {
      this.nextId = 1
    }

    return this.nextId
  }

  /**
   * Reject all pending requests with the given error
   */
  private rejectAllPending(error: Error): void {
    for (const [id, pending] of this.pendingRequests) {
      clearTimeout(pending.timer)
      pending.reject(error)
    }
    this.pendingRequests.clear()
  }
}
