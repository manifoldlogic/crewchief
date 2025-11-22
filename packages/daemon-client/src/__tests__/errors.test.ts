/**
 * Unit tests for error type hierarchy
 */

import { describe, it, expect } from 'vitest'
import {
  DaemonError,
  DaemonStartError,
  DaemonCrashError,
  DaemonTimeoutError,
  RpcError,
  DaemonUnhealthyError,
} from '../errors.js'

describe('DaemonError', () => {
  it('should create error with message, code, and optional cause', () => {
    const cause = new Error('underlying error')
    const error = new DaemonError('test message', 'TEST_CODE', cause)

    expect(error).toBeInstanceOf(Error)
    expect(error).toBeInstanceOf(DaemonError)
    expect(error.message).toBe('test message')
    expect(error.code).toBe('TEST_CODE')
    expect(error.cause).toBe(cause)
    expect(error.name).toBe('DaemonError')
  })

  it('should work without cause', () => {
    const error = new DaemonError('test message', 'TEST_CODE')

    expect(error.message).toBe('test message')
    expect(error.code).toBe('TEST_CODE')
    expect(error.cause).toBeUndefined()
  })

  it('should have stack trace', () => {
    const error = new DaemonError('test', 'TEST')
    expect(error.stack).toBeDefined()
    expect(error.stack).toContain('DaemonError')
  })
})

describe('DaemonStartError', () => {
  it('should extend DaemonError with correct code', () => {
    const error = new DaemonStartError('Failed to start daemon')

    expect(error).toBeInstanceOf(DaemonError)
    expect(error).toBeInstanceOf(DaemonStartError)
    expect(error.message).toBe('Failed to start daemon')
    expect(error.code).toBe('DAEMON_START_FAILED')
    expect(error.name).toBe('DaemonStartError')
  })

  it('should preserve cause chain', () => {
    const cause = new Error('ENOENT: binary not found')
    const error = new DaemonStartError('Failed to start daemon', cause)

    expect(error.cause).toBe(cause)
  })
})

describe('DaemonCrashError', () => {
  it('should include exitCode and signal', () => {
    const error = new DaemonCrashError('Daemon crashed', 1, 'SIGTERM')

    expect(error).toBeInstanceOf(DaemonError)
    expect(error).toBeInstanceOf(DaemonCrashError)
    expect(error.message).toBe('Daemon crashed')
    expect(error.code).toBe('DAEMON_CRASHED')
    expect(error.exitCode).toBe(1)
    expect(error.signal).toBe('SIGTERM')
    expect(error.name).toBe('DaemonCrashError')
  })

  it('should work with undefined exitCode and signal', () => {
    const error = new DaemonCrashError('Daemon crashed')

    expect(error.exitCode).toBeUndefined()
    expect(error.signal).toBeUndefined()
  })

  it('should work with cause', () => {
    const cause = new Error('Process terminated unexpectedly')
    const error = new DaemonCrashError('Daemon crashed', 137, 'SIGKILL', cause)

    expect(error.cause).toBe(cause)
    expect(error.exitCode).toBe(137)
    expect(error.signal).toBe('SIGKILL')
  })
})

describe('DaemonTimeoutError', () => {
  it('should extend DaemonError with correct code', () => {
    const error = new DaemonTimeoutError('Request timed out after 30s')

    expect(error).toBeInstanceOf(DaemonError)
    expect(error).toBeInstanceOf(DaemonTimeoutError)
    expect(error.message).toBe('Request timed out after 30s')
    expect(error.code).toBe('DAEMON_TIMEOUT')
    expect(error.name).toBe('DaemonTimeoutError')
  })

  it('should preserve cause', () => {
    const cause = new Error('Network timeout')
    const error = new DaemonTimeoutError('Request timed out', cause)

    expect(error.cause).toBe(cause)
  })
})

describe('RpcError', () => {
  it('should include rpcCode and data', () => {
    const errorData = { requestId: 42, method: 'search' }
    const error = new RpcError('Method not found', -32601, errorData)

    expect(error).toBeInstanceOf(DaemonError)
    expect(error).toBeInstanceOf(RpcError)
    expect(error.message).toBe('Method not found')
    expect(error.code).toBe('RPC_ERROR')
    expect(error.rpcCode).toBe(-32601)
    expect(error.data).toBe(errorData)
    expect(error.name).toBe('RpcError')
  })

  it('should work without data', () => {
    const error = new RpcError('Parse error', -32700)

    expect(error.rpcCode).toBe(-32700)
    expect(error.data).toBeUndefined()
  })

  describe('helper methods', () => {
    it('isParseError() should detect -32700', () => {
      const parseError = new RpcError('Parse error', -32700)
      const otherError = new RpcError('Other', -32600)

      expect(parseError.isParseError()).toBe(true)
      expect(otherError.isParseError()).toBe(false)
    })

    it('isInvalidRequest() should detect -32600', () => {
      const invalidRequest = new RpcError('Invalid request', -32600)
      const otherError = new RpcError('Other', -32601)

      expect(invalidRequest.isInvalidRequest()).toBe(true)
      expect(otherError.isInvalidRequest()).toBe(false)
    })

    it('isMethodNotFound() should detect -32601', () => {
      const methodNotFound = new RpcError('Method not found', -32601)
      const otherError = new RpcError('Other', -32600)

      expect(methodNotFound.isMethodNotFound()).toBe(true)
      expect(otherError.isMethodNotFound()).toBe(false)
    })

    it('isInvalidParams() should detect -32602', () => {
      const invalidParams = new RpcError('Invalid params', -32602)
      const otherError = new RpcError('Other', -32600)

      expect(invalidParams.isInvalidParams()).toBe(true)
      expect(otherError.isInvalidParams()).toBe(false)
    })

    it('isInternalError() should detect -32603', () => {
      const internalError = new RpcError('Internal error', -32603)
      const otherError = new RpcError('Other', -32600)

      expect(internalError.isInternalError()).toBe(true)
      expect(otherError.isInternalError()).toBe(false)
    })
  })
})

describe('DaemonUnhealthyError', () => {
  it('should extend DaemonError with correct code', () => {
    const error = new DaemonUnhealthyError('Daemon is not healthy')

    expect(error).toBeInstanceOf(DaemonError)
    expect(error).toBeInstanceOf(DaemonUnhealthyError)
    expect(error.message).toBe('Daemon is not healthy')
    expect(error.code).toBe('DAEMON_UNHEALTHY')
    expect(error.name).toBe('DaemonUnhealthyError')
  })

  it('should preserve cause', () => {
    const cause = new Error('Health check failed')
    const error = new DaemonUnhealthyError('Daemon is not healthy', cause)

    expect(error.cause).toBe(cause)
  })
})
