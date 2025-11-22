/**
 * Unit tests for JSON-RPC 2.0 protocol implementation
 */

import { describe, it, expect } from 'vitest'
import { RpcProtocol, type JsonRpcResponse } from '../rpc.js'
import { RpcError } from '../errors.js'

describe('RpcProtocol.createRequest', () => {
  it('should create valid JSON-RPC 2.0 request with all fields', () => {
    const request = RpcProtocol.createRequest('search', { query: 'test' }, 1)

    expect(request).toEqual({
      jsonrpc: '2.0',
      method: 'search',
      params: { query: 'test' },
      id: 1,
    })
  })

  it('should omit params if undefined', () => {
    const request = RpcProtocol.createRequest('ping', undefined, 2)

    expect(request).toEqual({
      jsonrpc: '2.0',
      method: 'ping',
      id: 2,
    })
    expect(request.params).toBeUndefined()
  })

  it('should use sequential IDs', () => {
    const req1 = RpcProtocol.createRequest('method1', null, 1)
    const req2 = RpcProtocol.createRequest('method2', null, 2)
    const req3 = RpcProtocol.createRequest('method3', null, 3)

    expect(req1.id).toBe(1)
    expect(req2.id).toBe(2)
    expect(req3.id).toBe(3)
  })

  it('should handle large request IDs', () => {
    const largeId = Number.MAX_SAFE_INTEGER
    const request = RpcProtocol.createRequest('method', null, largeId)

    expect(request.id).toBe(largeId)
  })
})

describe('RpcProtocol.serializeRequest', () => {
  it('should serialize request to line-delimited JSON', () => {
    const request = RpcProtocol.createRequest('ping', undefined, 1)
    const serialized = RpcProtocol.serializeRequest(request)

    expect(serialized).toBe('{"jsonrpc":"2.0","method":"ping","id":1}\n')
  })

  it('should include newline terminator', () => {
    const request = RpcProtocol.createRequest('search', { query: 'test' }, 42)
    const serialized = RpcProtocol.serializeRequest(request)

    expect(serialized.endsWith('\n')).toBe(true)
  })
})

describe('RpcProtocol.parseResponse', () => {
  it('should parse valid success response', () => {
    const line = '{"jsonrpc":"2.0","result":{"data":"test"},"id":1}'
    const response = RpcProtocol.parseResponse(line)

    expect(response).toEqual({
      jsonrpc: '2.0',
      result: { data: 'test' },
      id: 1,
    })
  })

  it('should parse valid error response', () => {
    const line = '{"jsonrpc":"2.0","error":{"code":-32601,"message":"Method not found"},"id":2}'
    const response = RpcProtocol.parseResponse(line)

    expect(response).toEqual({
      jsonrpc: '2.0',
      error: { code: -32601, message: 'Method not found' },
      id: 2,
    })
  })

  it('should parse response with null id (notification)', () => {
    const line = '{"jsonrpc":"2.0","result":"ok","id":null}'
    const response = RpcProtocol.parseResponse(line)

    expect(response.id).toBeNull()
  })

  it('should throw RpcError on malformed JSON', () => {
    const line = '{invalid json}'

    expect(() => RpcProtocol.parseResponse(line)).toThrow(RpcError)

    try {
      RpcProtocol.parseResponse(line)
    } catch (error) {
      expect(error).toBeInstanceOf(RpcError)
      const rpcError = error as RpcError
      expect(rpcError.rpcCode).toBe(-32700) // Parse error
      expect(rpcError.data).toMatchObject({ line })
    }
  })

  it('should throw RpcError on missing jsonrpc field', () => {
    const line = '{"result":"ok","id":1}'

    expect(() => RpcProtocol.parseResponse(line)).toThrow(RpcError)

    try {
      RpcProtocol.parseResponse(line)
    } catch (error) {
      const rpcError = error as RpcError
      expect(rpcError.rpcCode).toBe(-32700)
      expect(rpcError.message).toContain('Invalid JSON-RPC version')
    }
  })

  it('should throw RpcError on invalid jsonrpc version', () => {
    const line = '{"jsonrpc":"1.0","result":"ok","id":1}'

    expect(() => RpcProtocol.parseResponse(line)).toThrow(RpcError)

    try {
      RpcProtocol.parseResponse(line)
    } catch (error) {
      const rpcError = error as RpcError
      expect(rpcError.message).toContain('Invalid JSON-RPC version: 1.0')
    }
  })

  it('should throw RpcError on missing id field', () => {
    const line = '{"jsonrpc":"2.0","result":"ok"}'

    expect(() => RpcProtocol.parseResponse(line)).toThrow(RpcError)

    try {
      RpcProtocol.parseResponse(line)
    } catch (error) {
      const rpcError = error as RpcError
      expect(rpcError.message).toContain('Missing id field')
    }
  })

  it('should preserve error data from response', () => {
    const line = '{"jsonrpc":"2.0","error":{"code":-32000,"message":"Server error","data":{"detail":"extra info"}},"id":5}'
    const response = RpcProtocol.parseResponse(line)

    expect(response.error).toMatchObject({
      code: -32000,
      message: 'Server error',
      data: { detail: 'extra info' },
    })
  })
})

describe('RpcProtocol.isError', () => {
  it('should return true for error response', () => {
    const response: JsonRpcResponse = {
      jsonrpc: '2.0',
      error: { code: -32601, message: 'Method not found' },
      id: 1,
    }

    expect(RpcProtocol.isError(response)).toBe(true)
  })

  it('should return false for success response', () => {
    const response: JsonRpcResponse = {
      jsonrpc: '2.0',
      result: { data: 'test' },
      id: 1,
    }

    expect(RpcProtocol.isError(response)).toBe(false)
  })

  it('should return false when both result and error are undefined', () => {
    const response: JsonRpcResponse = {
      jsonrpc: '2.0',
      id: 1,
    }

    expect(RpcProtocol.isError(response)).toBe(false)
  })
})

describe('RpcProtocol.throwIfError', () => {
  it('should throw RpcError if response contains error', () => {
    const response: JsonRpcResponse = {
      jsonrpc: '2.0',
      error: { code: -32601, message: 'Method not found', data: { method: 'unknown' } },
      id: 1,
    }

    expect(() => RpcProtocol.throwIfError(response)).toThrow(RpcError)

    try {
      RpcProtocol.throwIfError(response)
    } catch (error) {
      const rpcError = error as RpcError
      expect(rpcError.message).toBe('Method not found')
      expect(rpcError.rpcCode).toBe(-32601)
      expect(rpcError.data).toMatchObject({ method: 'unknown' })
    }
  })

  it('should not throw for success response', () => {
    const response: JsonRpcResponse = {
      jsonrpc: '2.0',
      result: { data: 'test' },
      id: 1,
    }

    expect(() => RpcProtocol.throwIfError(response)).not.toThrow()
  })
})

describe('RpcProtocol.extractResult', () => {
  it('should extract result from success response', () => {
    const response: JsonRpcResponse = {
      jsonrpc: '2.0',
      result: { hits: [], total: 0 },
      id: 1,
    }

    const result = RpcProtocol.extractResult(response)
    expect(result).toEqual({ hits: [], total: 0 })
  })

  it('should throw RpcError if response contains error', () => {
    const response: JsonRpcResponse = {
      jsonrpc: '2.0',
      error: { code: -32000, message: 'Server error' },
      id: 1,
    }

    expect(() => RpcProtocol.extractResult(response)).toThrow(RpcError)
  })

  it('should throw RpcError if both result and error are missing', () => {
    const response: JsonRpcResponse = {
      jsonrpc: '2.0',
      id: 1,
    }

    expect(() => RpcProtocol.extractResult(response)).toThrow(RpcError)

    try {
      RpcProtocol.extractResult(response)
    } catch (error) {
      const rpcError = error as RpcError
      expect(rpcError.rpcCode).toBe(-32603) // Internal error
      expect(rpcError.message).toContain('missing both result and error')
    }
  })

  it('should preserve type information with generics', () => {
    interface SearchResult {
      hits: string[]
      total: number
    }

    const response: JsonRpcResponse = {
      jsonrpc: '2.0',
      result: { hits: ['a', 'b'], total: 2 },
      id: 1,
    }

    const result = RpcProtocol.extractResult<SearchResult>(response)
    expect(result.hits).toEqual(['a', 'b'])
    expect(result.total).toBe(2)
  })
})
