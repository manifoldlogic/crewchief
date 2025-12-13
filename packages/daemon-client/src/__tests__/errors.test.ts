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
import type { SearchErrorDetails } from '../types.js'

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

  describe('SearchErrorDetails deserialization', () => {
    it('should parse error details from data field', () => {
      const errorData: SearchErrorDetails = {
        error_type: 'embedding_provider',
        stage: 'query_processing',
        context: { provider_error: 'timeout' },
        suggestions: ['Check credentials', 'Try FTS mode'],
      }

      const error = new RpcError('Embedding failed', -32000, errorData)

      expect(error.getDetails()).toEqual(errorData)
      expect(error.details?.error_type).toBe('embedding_provider')
    })

    it('should format user message with context and suggestions', () => {
      const errorData: SearchErrorDetails = {
        error_type: 'database',
        stage: 'search_execution',
        context: { message: 'Connection failed', repo: 'crewchief' },
        suggestions: ['Check database connectivity', 'Verify repository indexed'],
      }

      const error = new RpcError('Database error', -32000, errorData)
      const message = error.getUserMessage()

      expect(message).toContain('search_execution')
      expect(message).toContain('Connection failed')
      expect(message).toContain('Check database connectivity')
      expect(message).toContain('Verify repository indexed')
    })

    it('should handle missing error details gracefully', () => {
      const error = new RpcError('Generic error', -32000)

      expect(error.getDetails()).toBeUndefined()
      expect(error.getUserMessage()).toEqual('Generic error')
    })

    it('should handle invalid error details structure', () => {
      const invalidData = { foo: 'bar' }

      const error = new RpcError('Invalid error', -32000, invalidData)

      expect(error.getDetails()).toBeUndefined()
      expect(error.getUserMessage()).toEqual('Invalid error')
    })

    it('should deserialize all error types correctly', () => {
      const errorTypes: Array<SearchErrorDetails['error_type']> = [
        'embedding_provider',
        'database',
        'validation',
        'timeout',
        'not_found',
        'unknown',
      ]

      for (const type of errorTypes) {
        const errorData: SearchErrorDetails = {
          error_type: type,
          stage: 'query_processing',
          context: {},
          suggestions: ['Test suggestion'],
        }

        const error = new RpcError('Test error', -32000, errorData)
        expect(error.getDetails()?.error_type).toBe(type)
      }
    })

    it('should handle error details with empty context', () => {
      const errorData: SearchErrorDetails = {
        error_type: 'timeout',
        stage: 'search_execution',
        context: {},
        suggestions: ['Increase timeout', 'Simplify query'],
      }

      const error = new RpcError('Search timeout', -32000, errorData)
      const message = error.getUserMessage()

      expect(message).toContain('search_execution')
      expect(message).toContain('Increase timeout')
      expect(message).not.toContain('Context:')
    })

    it('should handle error details with empty suggestions', () => {
      const errorData: SearchErrorDetails = {
        error_type: 'unknown',
        stage: 'result_assembly',
        context: { error: 'unexpected failure' },
        suggestions: [],
      }

      const error = new RpcError('Unknown error', -32000, errorData)
      const message = error.getUserMessage()

      expect(message).toContain('result_assembly')
      expect(message).toContain('unexpected failure')
      expect(message).not.toContain('Suggestions:')
    })

    it('should validate all required fields in type guard', () => {
      // Missing error_type
      const missingErrorType = {
        stage: 'query_processing',
        context: {},
        suggestions: [],
      }
      const error1 = new RpcError('Error', -32000, missingErrorType)
      expect(error1.getDetails()).toBeUndefined()

      // Missing stage
      const missingStage = {
        error_type: 'database',
        context: {},
        suggestions: [],
      }
      const error2 = new RpcError('Error', -32000, missingStage)
      expect(error2.getDetails()).toBeUndefined()

      // Missing context
      const missingContext = {
        error_type: 'database',
        stage: 'query_processing',
        suggestions: [],
      }
      const error3 = new RpcError('Error', -32000, missingContext)
      expect(error3.getDetails()).toBeUndefined()

      // Missing suggestions
      const missingSuggestions = {
        error_type: 'database',
        stage: 'query_processing',
        context: {},
      }
      const error4 = new RpcError('Error', -32000, missingSuggestions)
      expect(error4.getDetails()).toBeUndefined()

      // context is null
      const nullContext = {
        error_type: 'database',
        stage: 'query_processing',
        context: null,
        suggestions: [],
      }
      const error5 = new RpcError('Error', -32000, nullContext)
      expect(error5.getDetails()).toBeUndefined()

      // suggestions is not an array
      const invalidSuggestions = {
        error_type: 'database',
        stage: 'query_processing',
        context: {},
        suggestions: 'not an array',
      }
      const error6 = new RpcError('Error', -32000, invalidSuggestions)
      expect(error6.getDetails()).toBeUndefined()
    })

    it('should format user message with multiple context entries', () => {
      const errorData: SearchErrorDetails = {
        error_type: 'validation',
        stage: 'query_processing',
        context: {
          query: 'test query',
          mode: 'hybrid',
          issue: 'invalid parameter',
        },
        suggestions: ['Check query syntax', 'Use simpler query'],
      }

      const error = new RpcError('Validation failed', -32000, errorData)
      const message = error.getUserMessage()

      expect(message).toContain('query_processing')
      expect(message).toContain('query: test query')
      expect(message).toContain('mode: hybrid')
      expect(message).toContain('issue: invalid parameter')
      expect(message).toContain('Check query syntax')
      expect(message).toContain('Use simpler query')
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
