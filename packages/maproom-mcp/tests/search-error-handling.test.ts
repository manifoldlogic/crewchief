/**
 * Integration tests for search error handling (SRCHTRN-1005)
 *
 * Tests the end-to-end error flow from Rust daemon → TypeScript → MCP
 * Validates structured error formatting with context and suggestions
 */

import { describe, it, expect } from 'vitest'
import { formatSearchError } from '../src/tools/search.js'
import { RpcError } from '../src/daemon-client/index.js'
import { ValidationError } from '../src/utils/validation.js'
import { ProcessError } from '../src/utils/process.js'

describe('Search error handling integration (SRCHTRN-1005)', () => {
  it('should format embedding provider error with suggestions', () => {
    // Mock RpcError with structured details (from daemon)
    const error = new RpcError(
      'Query processing failed: Embedding generation failed: request timeout',
      -32000,
      {
        error_type: 'embedding_provider',
        stage: 'query_processing',
        context: { provider_error: 'request timeout' },
        suggestions: [
          'Check your embedding provider credentials',
          'Verify network connectivity',
          'Try FTS mode while debugging: --mode fts',
        ],
      }
    )

    const result = formatSearchError(error)

    expect(result.isError).toBe(true)
    expect(result.content).toHaveLength(1)
    expect(result.content[0].type).toBe('text')

    const errorData = JSON.parse(result.content[0].text)
    expect(errorData.error).toBe('embedding_provider')
    expect(errorData.stage).toBe('query_processing')
    expect(errorData.message).toContain('request timeout')
    expect(errorData.context).toEqual({ provider_error: 'request timeout' })
    expect(errorData.suggestions).toHaveLength(3)
    expect(errorData.suggestions).toContain('Try FTS mode while debugging: --mode fts')
  })

  it('should format database error with context', () => {
    const error = new RpcError(
      'Search execution failed: Database query timeout',
      -32000,
      {
        error_type: 'database',
        stage: 'search_execution',
        context: {
          query_type: 'vector',
          timeout_ms: '30000'
        },
        suggestions: [
          'Check database connectivity',
          'Try reducing search scope with filters',
        ],
      }
    )

    const result = formatSearchError(error)

    expect(result.isError).toBe(true)
    const errorData = JSON.parse(result.content[0].text)
    expect(errorData.error).toBe('database')
    expect(errorData.stage).toBe('search_execution')
    expect(errorData.context.query_type).toBe('vector')
    expect(errorData.suggestions).toHaveLength(2)
  })

  it('should format validation error with suggestions', () => {
    const error = new RpcError(
      'Validation failed: Repository not found',
      -32602,
      {
        error_type: 'validation',
        stage: 'query_processing',
        context: {
          repo: 'nonexistent-repo',
          available_repos: 'crewchief'
        },
        suggestions: [
          'Check repository name spelling',
          'Use status tool to see available repositories',
        ],
      }
    )

    const result = formatSearchError(error)

    expect(result.isError).toBe(true)
    const errorData = JSON.parse(result.content[0].text)
    expect(errorData.error).toBe('validation')
    expect(errorData.context.repo).toBe('nonexistent-repo')
    expect(errorData.suggestions).toContain('Use status tool to see available repositories')
  })

  it('should handle RpcError without details (backward compatibility)', () => {
    // Old-style RpcError without structured details
    const error = new RpcError('Generic RPC error', -32000)

    const result = formatSearchError(error)

    expect(result.isError).toBe(true)
    const errorData = JSON.parse(result.content[0].text)
    expect(errorData.error).toBe('RPC_ERROR')
    expect(errorData.message).toBe('Generic RPC error')
    // Should not have structured fields
    expect(errorData.stage).toBeUndefined()
    expect(errorData.context).toBeUndefined()
    expect(errorData.suggestions).toBeUndefined()
  })

  it('should handle ValidationError (existing behavior)', () => {
    const error = new ValidationError('Invalid parameters', 'VALIDATION_FAILED')

    const result = formatSearchError(error)

    expect(result.isError).toBe(true)
    const errorData = JSON.parse(result.content[0].text)
    expect(errorData.error).toBe('VALIDATION_FAILED')
    expect(errorData.message).toBe('Invalid parameters')
  })

  it('should handle ProcessError (existing behavior)', () => {
    const error = new ProcessError('Binary not found', 'BINARY_NOT_FOUND')

    const result = formatSearchError(error)

    expect(result.isError).toBe(true)
    const errorData = JSON.parse(result.content[0].text)
    expect(errorData.error).toBe('BINARY_NOT_FOUND')
    expect(errorData.message).toBe('Binary not found')
    expect(errorData.hint).toContain('maproom')
  })

  it('should handle generic Error (fallback)', () => {
    const error = new Error('Something went wrong')

    const result = formatSearchError(error)

    expect(result.isError).toBe(true)
    const errorData = JSON.parse(result.content[0].text)
    expect(errorData.error).toBe('INTERNAL_ERROR')
    expect(errorData.message).toBe('Something went wrong')
  })

  it('should format error as valid JSON in MCP text field', () => {
    const error = new RpcError(
      'Test error',
      -32000,
      {
        error_type: 'timeout',
        stage: 'search_execution',
        context: { elapsed_ms: '30000' },
        suggestions: ['Increase timeout', 'Reduce scope'],
      }
    )

    const result = formatSearchError(error)

    // Verify structure follows MCP protocol
    expect(result).toHaveProperty('isError', true)
    expect(result).toHaveProperty('content')
    expect(Array.isArray(result.content)).toBe(true)
    expect(result.content[0]).toHaveProperty('type', 'text')
    expect(result.content[0]).toHaveProperty('text')

    // Verify JSON is valid and parseable
    expect(() => JSON.parse(result.content[0].text)).not.toThrow()

    // Verify JSON is formatted (has indentation)
    const text = result.content[0].text
    expect(text).toContain('\n')
    expect(text).toContain('  ') // 2-space indentation
  })

  it('should preserve all error fields in structured format', () => {
    const error = new RpcError(
      'Complex error message',
      -32000,
      {
        error_type: 'not_found',
        stage: 'result_assembly',
        context: {
          field1: 'value1',
          field2: 'value2',
          field3: 'value3',
        },
        suggestions: [
          'Suggestion 1',
          'Suggestion 2',
        ],
      }
    )

    const result = formatSearchError(error)
    const errorData = JSON.parse(result.content[0].text)

    // Verify all fields are present
    expect(errorData).toHaveProperty('error')
    expect(errorData).toHaveProperty('stage')
    expect(errorData).toHaveProperty('message')
    expect(errorData).toHaveProperty('context')
    expect(errorData).toHaveProperty('suggestions')

    // Verify context preserves all fields
    expect(Object.keys(errorData.context)).toHaveLength(3)
    expect(errorData.context.field1).toBe('value1')
    expect(errorData.context.field2).toBe('value2')
    expect(errorData.context.field3).toBe('value3')
  })
})
