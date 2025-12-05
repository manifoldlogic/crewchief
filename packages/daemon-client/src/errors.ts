/**
 * Error types for daemon-client operations
 */

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
  constructor(
    message: string,
    public readonly rpcCode: number,
    public readonly data?: unknown
  ) {
    super(message, 'RPC_ERROR')
    this.name = 'RpcError'
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
