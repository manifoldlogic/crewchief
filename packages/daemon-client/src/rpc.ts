/**
 * JSON-RPC 2.0 protocol implementation for daemon communication
 */

import { RpcError } from './errors'

/**
 * JSON-RPC 2.0 request structure
 */
export interface JsonRpcRequest {
  jsonrpc: '2.0'
  method: string
  params?: unknown
  id: number
}

/**
 * JSON-RPC 2.0 response structure
 */
export interface JsonRpcResponse {
  jsonrpc: '2.0'
  result?: unknown
  error?: JsonRpcErrorObject
  id: number | null
}

/**
 * JSON-RPC 2.0 standard error structure
 */
export interface JsonRpcErrorObject {
  code: number
  message: string
  data?: unknown
}

/**
 * JSON-RPC protocol handler
 */
export class RpcProtocol {
  /**
   * Create a JSON-RPC request
   */
  static createRequest(method: string, params: unknown, id: number): JsonRpcRequest {
    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      method,
      id,
    }

    if (params !== undefined) {
      request.params = params
    }

    return request
  }

  /**
   * Serialize a JSON-RPC request to a line for stdio
   */
  static serializeRequest(request: JsonRpcRequest): string {
    return JSON.stringify(request) + '\n'
  }

  /**
   * Parse a JSON-RPC response from a line
   */
  static parseResponse(line: string): JsonRpcResponse {
    try {
      const response = JSON.parse(line) as JsonRpcResponse

      // Validate JSON-RPC 2.0 structure
      if (response.jsonrpc !== '2.0') {
        throw new Error(`Invalid JSON-RPC version: ${response.jsonrpc}`)
      }

      if (response.id === undefined) {
        throw new Error('Missing id field in JSON-RPC response')
      }

      return response
    } catch (error) {
      throw new RpcError(
        `Failed to parse JSON-RPC response: ${error instanceof Error ? error.message : String(error)}`,
        -32700, // Parse error
        { line, error: error instanceof Error ? error.message : String(error) }
      )
    }
  }

  /**
   * Check if a response contains an error
   */
  static isError(response: JsonRpcResponse): boolean {
    return response.error !== undefined
  }

  /**
   * Extract error from response and throw RpcError
   */
  static throwIfError(response: JsonRpcResponse): void {
    if (response.error) {
      throw new RpcError(
        response.error.message,
        response.error.code,
        response.error.data
      )
    }
  }

  /**
   * Extract result from response, throwing if there's an error
   */
  static extractResult<T>(response: JsonRpcResponse): T {
    RpcProtocol.throwIfError(response)

    if (response.result === undefined) {
      throw new RpcError(
        'JSON-RPC response missing both result and error fields',
        -32603, // Internal error
        { response }
      )
    }

    return response.result as T
  }
}
