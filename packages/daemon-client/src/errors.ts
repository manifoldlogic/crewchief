/**
 * Error types for daemon-client operations
 */

import type { SearchErrorDetails } from './types.js'

/**
 * Base error class for all daemon-related errors
 */
export class DaemonError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly cause?: Error
  ) {
    super(message)
    this.name = 'DaemonError'
    Error.captureStackTrace(this, this.constructor)
  }
}

/**
 * Error thrown when daemon fails to start
 */
export class DaemonStartError extends DaemonError {
  constructor(message: string, cause?: Error) {
    super(message, 'DAEMON_START_FAILED', cause)
    this.name = 'DaemonStartError'
  }
}

/**
 * Error thrown when daemon crashes unexpectedly
 */
export class DaemonCrashError extends DaemonError {
  constructor(
    message: string,
    public readonly exitCode?: number,
    public readonly signal?: string,
    cause?: Error
  ) {
    super(message, 'DAEMON_CRASHED', cause)
    this.name = 'DaemonCrashError'
  }
}

/**
 * Error thrown when daemon operation times out
 */
export class DaemonTimeoutError extends DaemonError {
  constructor(message: string, cause?: Error) {
    super(message, 'DAEMON_TIMEOUT', cause)
    this.name = 'DaemonTimeoutError'
  }
}

/**
 * Error thrown for JSON-RPC protocol errors
 */
export class RpcError extends DaemonError {
  public readonly details?: SearchErrorDetails

  constructor(
    message: string,
    public readonly rpcCode: number,
    public readonly data?: unknown
  ) {
    super(message, 'RPC_ERROR')
    this.name = 'RpcError'

    // Attempt to parse structured error details from data field
    if (data && typeof data === 'object' && this.isSearchErrorDetails(data)) {
      this.details = data as SearchErrorDetails
    }
  }

  /**
   * Type guard to validate SearchErrorDetails structure
   */
  private isSearchErrorDetails(data: unknown): boolean {
    if (typeof data !== 'object' || data === null) return false
    const obj = data as Record<string, unknown>
    return (
      typeof obj.error_type === 'string' &&
      typeof obj.stage === 'string' &&
      typeof obj.context === 'object' &&
      obj.context !== null &&
      Array.isArray(obj.suggestions)
    )
  }

  /**
   * Get parsed error details if available
   */
  getDetails(): SearchErrorDetails | undefined {
    return this.details
  }

  /**
   * Format error with context and suggestions for user display
   */
  getUserMessage(): string {
    if (!this.details) {
      return this.message // Fallback to generic error
    }

    const { stage, context, suggestions } = this.details

    let formatted = `Search failed at ${stage}: ${this.message}\n`

    // Add context if available
    if (Object.keys(context).length > 0) {
      formatted += '\nContext:\n'
      for (const [key, value] of Object.entries(context)) {
        formatted += `  ${key}: ${value}\n`
      }
    }

    // Add suggestions
    if (suggestions.length > 0) {
      formatted += '\nSuggestions:\n'
      for (const suggestion of suggestions) {
        formatted += `  - ${suggestion}\n`
      }
    }

    return formatted
  }

  /**
   * Check if this is a parse error (-32700)
   */
  isParseError(): boolean {
    return this.rpcCode === -32700
  }

  /**
   * Check if this is an invalid request error (-32600)
   */
  isInvalidRequest(): boolean {
    return this.rpcCode === -32600
  }

  /**
   * Check if this is a method not found error (-32601)
   */
  isMethodNotFound(): boolean {
    return this.rpcCode === -32601
  }

  /**
   * Check if this is an invalid params error (-32602)
   */
  isInvalidParams(): boolean {
    return this.rpcCode === -32602
  }

  /**
   * Check if this is an internal error (-32603)
   */
  isInternalError(): boolean {
    return this.rpcCode === -32603
  }
}

/**
 * Error thrown when daemon is not healthy
 */
export class DaemonUnhealthyError extends DaemonError {
  constructor(message: string, cause?: Error) {
    super(message, 'DAEMON_UNHEALTHY', cause)
    this.name = 'DaemonUnhealthyError'
  }
}

/**
 * Base error class for daemon communication errors
 */
export class DaemonCommunicationError extends DaemonError {
  constructor(message: string, options?: { cause?: Error }) {
    super(message, 'DAEMON_COMMUNICATION_ERROR', options?.cause)
    this.name = 'DaemonCommunicationError'
  }
}

/**
 * Error thrown for socket connection failures
 */
export class SocketConnectionError extends DaemonError {
  constructor(message: string, options?: { cause?: Error }) {
    super(message, 'SOCKET_CONNECTION_ERROR', options?.cause)
    this.name = 'SocketConnectionError'
  }
}

/**
 * Error thrown when socket operations time out
 */
export class SocketTimeoutError extends DaemonError {
  constructor(message: string, public timeoutMs: number) {
    super(message, 'SOCKET_TIMEOUT')
    this.name = 'SocketTimeoutError'
  }
}

/**
 * Error thrown when daemon fails to start or become ready
 */
export class DaemonStartupError extends DaemonError {
  constructor(message: string, options?: { cause?: Error }) {
    super(message, 'DAEMON_STARTUP_ERROR', options?.cause)
    this.name = 'DaemonStartupError'
  }
}

/**
 * Error thrown when lock acquisition fails
 */
export class DaemonLockError extends DaemonError {
  constructor(message: string, options?: { cause?: Error }) {
    super(message, 'DAEMON_LOCK_ERROR', options?.cause)
    this.name = 'DaemonLockError'
  }
}
