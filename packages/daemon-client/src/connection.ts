/**
 * Connection interface for daemon communication
 *
 * This interface abstracts the transport mechanism (socket or stdio)
 * for communicating with the daemon.
 */

/**
 * Connection interface for sending requests and receiving responses
 */
export interface Connection {
  /**
   * Send a JSON-RPC request to the daemon
   *
   * @param method - The RPC method name
   * @param params - Optional parameters for the method
   * @returns Promise resolving to the result
   */
  sendRequest<T = unknown>(method: string, params?: unknown): Promise<T>

  /**
   * Close the connection gracefully
   *
   * Waits for pending requests to complete and cleans up resources.
   */
  close(): Promise<void>

  /**
   * Check if the connection is currently active
   *
   * @returns true if connected, false otherwise
   */
  isConnected(): boolean

  /**
   * Register event handlers for connection events
   *
   * @param event - Event type ('error' or 'close')
   * @param handler - Handler function, receives error for 'error' events
   */
  on(event: 'error' | 'close', handler: (err?: Error) => void): void
}
