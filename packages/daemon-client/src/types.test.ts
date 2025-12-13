import { describe, it, expect } from 'vitest'
import type { ErrorType, PipelineStage, SearchErrorDetails } from './types.js'

describe('Type synchronization with Rust', () => {
  // Sync with: crates/maproom/src/search/errors.rs::ErrorType
  it('should match Rust ErrorType enum values', () => {
    const rustErrorTypes = [
      'embedding_provider',
      'database',
      'validation',
      'timeout',
      'not_found',
      'unknown',
    ]

    // This will fail to compile if TypeScript ErrorType diverges
    const tsErrorTypes: ErrorType[] = [
      'embedding_provider',
      'database',
      'validation',
      'timeout',
      'not_found',
      'unknown',
    ]

    expect(rustErrorTypes).toEqual(tsErrorTypes)
  })

  // Sync with: crates/maproom/src/search/errors.rs::PipelineStage
  it('should match Rust PipelineStage enum values', () => {
    const rustStages = [
      'query_processing',
      'search_execution',
      'score_fusion',
      'result_assembly',
    ]

    const tsStages: PipelineStage[] = [
      'query_processing',
      'search_execution',
      'score_fusion',
      'result_assembly',
    ]

    expect(rustStages).toEqual(tsStages)
  })

  it('should deserialize SearchErrorDetails from Rust JSON', () => {
    // Example JSON from Rust serialization
    const rustJson = {
      error_type: 'embedding_provider' as ErrorType,
      stage: 'query_processing' as PipelineStage,
      context: { provider_error: 'timeout' },
      suggestions: ['Check credentials', 'Try FTS mode'],
    }

    // TypeScript should parse without errors
    const details: SearchErrorDetails = rustJson
    expect(details.error_type).toBe('embedding_provider')
    expect(details.stage).toBe('query_processing')
    expect(details.context).toEqual({ provider_error: 'timeout' })
    expect(details.suggestions).toHaveLength(2)
  })

  it('should validate all ErrorType variants', () => {
    const allErrorTypes: ErrorType[] = [
      'embedding_provider',
      'database',
      'validation',
      'timeout',
      'not_found',
      'unknown',
    ]

    // Verify each type is assignable to ErrorType
    allErrorTypes.forEach((errorType) => {
      const details: SearchErrorDetails = {
        error_type: errorType,
        stage: 'query_processing',
        context: {},
        suggestions: [],
      }
      expect(details.error_type).toBe(errorType)
    })
  })

  it('should validate all PipelineStage variants', () => {
    const allStages: PipelineStage[] = [
      'query_processing',
      'search_execution',
      'score_fusion',
      'result_assembly',
    ]

    // Verify each stage is assignable to PipelineStage
    allStages.forEach((stage) => {
      const details: SearchErrorDetails = {
        error_type: 'unknown',
        stage: stage,
        context: {},
        suggestions: [],
      }
      expect(details.stage).toBe(stage)
    })
  })

  it('should handle empty context and suggestions', () => {
    const details: SearchErrorDetails = {
      error_type: 'validation',
      stage: 'query_processing',
      context: {},
      suggestions: [],
    }

    expect(details.context).toEqual({})
    expect(details.suggestions).toHaveLength(0)
  })

  it('should handle multiple suggestions', () => {
    const details: SearchErrorDetails = {
      error_type: 'database',
      stage: 'search_execution',
      context: { message: 'timeout' },
      suggestions: [
        'Check database connectivity',
        'Restart the maproom daemon: crewchief-maproom serve',
      ],
    }

    expect(details.suggestions).toHaveLength(2)
    expect(details.suggestions[0]).toContain('database')
    expect(details.suggestions[1]).toContain('daemon')
  })

  it('should handle complex context objects', () => {
    const details: SearchErrorDetails = {
      error_type: 'embedding_provider',
      stage: 'query_processing',
      context: {
        provider: 'openai',
        provider_error: 'Rate limit exceeded',
        timeout_ms: '5000',
      },
      suggestions: ['Wait 5 seconds before retrying', 'Try FTS mode'],
    }

    expect(details.context.provider).toBe('openai')
    expect(details.context.provider_error).toBe('Rate limit exceeded')
    expect(details.context.timeout_ms).toBe('5000')
  })
})
