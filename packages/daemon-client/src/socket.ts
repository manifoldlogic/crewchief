/**
 * Socket-based connection implementation for daemon communication
 *
 * Implements the Connection interface using Unix domain sockets with
 * length-prefixed message framing (4-byte big-endian length prefix).
 *
 * Wire format matches Rust JsonRpcCodec:
 * - 4 bytes: message length (big-endian u32)
 * - N bytes: UTF-8 encoded JSON message
 */

import * as net from 'node:net'
import { Connection } from './connection.js'
import { JsonRpcRequest, JsonRpcResponse, RpcProtocol } from './rpc.js'
import { SocketConnectionError, SocketTimeoutError } from './errors.js'

/**
 * Request ID type (sequential integers)
 */
export type RequestId = number

/**
 * Pending request waiting for response
 */
interface PendingRequest {
  resolve: (value: unknown) => void
  reject: (error: Error) => void
  method: string
  timeout?: NodeJS.Timeout
}

/**
 * Socket-based connection to daemon
 *
 * Handles:
 * - Connection establishment with timeout
 * - Length-prefixed message encoding/decoding
 * - Buffer management for partial reads
 * - Request/response multiplexing via request IDs
 * - Timeout handling for individual requests
 * - Graceful shutdown and cleanup
 */
export class SocketConnection implements Connection {
  private socket: net.Socket | null = null
  private pendingRequests = new Map<RequestId, PendingRequest>()
  private buffer = Buffer.alloc(0)
  private nextId = 1
  private connected = false
  private errorHandlers: Array<(err?: Error) => void> = []
  private closeHandlers: Array<(err?: Error) => void> = []

  constructor(private socketPath: string) {}

  /**
   * Connect to the Unix socket
   *
   * @param timeoutMs - Connection timeout in milliseconds (default: 10000)
   * @throws {SocketTimeoutError} If connection times out
   * @throws {SocketConnectionError} If connection fails
   */
  async connect(timeoutMs: number = 10000): Promise<void> {
    return new Promise((resolve, reject) => {
      this.socket = net.createConnection(this.socketPath)

      const timeout = setTimeout(() => {
        this.socket?.destroy()
        reject(new SocketTimeoutError(this.socketPath, timeoutMs))
      }, timeoutMs)

      this.socket.on('connect', () => {
        clearTimeout(timeout)
        this.connected = true
        this.setupHandlers()
        resolve()
      })

      this.socket.on('error', (err) => {
        clearTimeout(timeout)
        reject(
          new SocketConnectionError(`Failed to connect to ${this.socketPath}`, {
            cause: err,
          })
        )
      })
    })
  }

  /**
   * Set up socket event handlers after connection
   */
  private setupHandlers(): void {
    if (!this.socket) return

    this.socket.on('data', (data: Buffer) => {
      this.handleData(data)
    })

    this.socket.on('error', (err) => {
      this.connected = false
      this.errorHandlers.forEach((h) => h(err))
      this.rejectAllPending(
        new SocketConnectionError('Socket error', { cause: err })
      )
    })

    this.socket.on('close', () => {
      this.connected = false
      this.closeHandlers.forEach((h) => h())
      this.rejectAllPending(
        new SocketConnectionError('Socket closed unexpectedly')
      )
    })
  }

  /**
   * Handle incoming data from socket
   *
   * Accumulates data in buffer and extracts complete messages.
   * Each message has a 4-byte big-endian length prefix followed by
   * the UTF-8 encoded JSON payload.
   *
   * This implements the same wire format as the Rust JsonRpcCodec.
   */
  private handleData(data: Buffer): void {
    // Append to buffer
    this.buffer = Buffer.concat([this.buffer, data])

    // Process complete messages
    while (this.buffer.length >= 4) {
      // Read length prefix (4 bytes, big-endian)
      const messageLength = this.buffer.readUInt32BE(0)

      // Check if we have the full message
      if (this.buffer.length < 4 + messageLength) {
        // Need more data
        break
      }

      // Extract message
      const messageBytes = this.buffer.slice(4, 4 + messageLength)
      this.buffer = this.buffer.slice(4 + messageLength)

      // Parse JSON
      try {
        const json = messageBytes.toString('utf8')
        const message = JSON.parse(json) as JsonRpcResponse
        this.handleMessage(message)
      } catch (err) {
        console.error('Failed to parse JSON-RPC message:', err)
      }
    }
  }

  /**
   * Handle a complete JSON-RPC response message
   */
  private handleMessage(message: JsonRpcResponse): void {
    const pending = this.pendingRequests.get(message.id as number)
    if (!pending) {
      console.warn('Received response for unknown request ID:', message.id)
      return
    }

    this.pendingRequests.delete(message.id as number)

    if (pending.timeout) {
      clearTimeout(pending.timeout)
    }

    if (message.error) {
      pending.reject(new Error(`JSON-RPC error: ${message.error.message}`))
    } else {
      pending.resolve(message.result)
    }
  }

  /**
   * Send a JSON-RPC request and wait for response
   *
   * @param method - The RPC method name
   * @param params - Optional parameters
   * @param timeoutMs - Request timeout in milliseconds (default: 30000)
   * @returns Promise resolving to the result
   * @throws {SocketConnectionError} If not connected or write fails
   * @throws {SocketTimeoutError} If request times out
   */
  async sendRequest<T = unknown>(
    method: string,
    params?: unknown,
    timeoutMs: number = 30000
  ): Promise<T> {
    if (!this.connected || !this.socket) {
      throw new SocketConnectionError('Not connected')
    }

    const id = this.nextId++
    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      method,
      params,
      id,
    }

    return new Promise((resolve, reject) => {
      // Set up timeout
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id)
        reject(
          new SocketTimeoutError(
            `Request ${method} timed out after ${timeoutMs}ms`,
            timeoutMs
          )
        )
      }, timeoutMs)

      // Store pending request
      this.pendingRequests.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
        method,
        timeout,
      })

      // Encode and send
      try {
        const json = JSON.stringify(request)
        const messageBytes = Buffer.from(json, 'utf8')

        // Write length prefix (4 bytes, big-endian)
        const lengthPrefix = Buffer.alloc(4)
        lengthPrefix.writeUInt32BE(messageBytes.length, 0)

        // Write to socket
        this.socket!.write(lengthPrefix)
        this.socket!.write(messageBytes)
      } catch (err) {
        clearTimeout(timeout)
        this.pendingRequests.delete(id)
        reject(
          new SocketConnectionError('Failed to send request', {
            cause: err as Error,
          })
        )
      }
    })
  }

  /**
   * Close the connection gracefully
   *
   * Rejects all pending requests and closes the socket.
   */
  async close(): Promise<void> {
    if (this.socket) {
      // Reject all pending requests
      this.rejectAllPending(
        new SocketConnectionError('Connection closed by client')
      )

      return new Promise((resolve) => {
        this.socket!.once('close', () => {
          this.connected = false
          resolve()
        })
        this.socket!.end()
      })
    }
  }

  /**
   * Check if the connection is active
   */
  isConnected(): boolean {
    return this.connected
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
   * Reject all pending requests with the given error
   *
   * Used during shutdown or error conditions.
   */
  private rejectAllPending(error: Error): void {
    for (const [id, pending] of this.pendingRequests) {
      if (pending.timeout) {
        clearTimeout(pending.timeout)
      }
      pending.reject(error)
    }
    this.pendingRequests.clear()
  }
}
