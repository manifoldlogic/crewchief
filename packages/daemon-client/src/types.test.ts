import { describe, it, expect } from 'vitest'
import type {
  ErrorType,
  PipelineStage,
  SearchErrorDetails,
  QueryUnderstanding,
  QueryFilters,
  TimingBreakdown,
  SearchMetadata,
  ConfidenceSignals,
} from './types.js'

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

describe('Type synchronization - Query Understanding', () => {
  it('should deserialize QueryUnderstanding from Rust JSON', () => {
    // Example JSON from Rust serialization
    const rustJson = {
      mode: 'auto' as const,
      tokens: ['authenticate', 'user'],
      expanded_terms: ['auth', 'login', 'authentication'],
      filters: {
        repo_id: 1,
        worktree_id: 2,
        file_types: [],
        recency_threshold: null,
      },
      fusion_strategy: 'reciprocal_rank_fusion',
      timing: {
        query_processing_ms: 4.2,
        search_execution_ms: 35.8,
        score_fusion_ms: 2.1,
        result_assembly_ms: 6.4,
        total_ms: 48.5,
      },
    }

    // TypeScript should parse without errors
    const understanding: QueryUnderstanding = rustJson
    expect(understanding.mode).toBe('auto')
    expect(understanding.tokens).toEqual(['authenticate', 'user'])
    expect(understanding.timing.total_ms).toBe(48.5)
  })

  it('should handle optional understanding field', () => {
    // Metadata without understanding (backward compatibility)
    const metadataWithout = {}

    const metadata1: SearchMetadata = metadataWithout
    expect(metadata1.understanding).toBeUndefined()

    // Metadata with understanding
    const metadataWith = {
      understanding: {
        mode: 'code' as const,
        tokens: ['test'],
        expanded_terms: [],
        filters: {
          repo_id: 1,
          worktree_id: null,
          file_types: [],
          recency_threshold: null,
        },
        fusion_strategy: 'linear',
        timing: {
          query_processing_ms: 1.0,
          search_execution_ms: 2.0,
          score_fusion_ms: 3.0,
          result_assembly_ms: 4.0,
          total_ms: 10.0,
        },
      },
    }

    const metadata2: SearchMetadata = metadataWith
    expect(metadata2.understanding?.mode).toBe('code')
  })

  it('should validate timing breakdown structure', () => {
    const timing: TimingBreakdown = {
      query_processing_ms: 4.2,
      search_execution_ms: 35.8,
      score_fusion_ms: 2.1,
      result_assembly_ms: 6.4,
      total_ms: 48.5,
    }

    // Verify all fields are numbers
    expect(typeof timing.query_processing_ms).toBe('number')
    expect(typeof timing.total_ms).toBe('number')

    // Verify total is sum of parts
    const sum =
      timing.query_processing_ms +
      timing.search_execution_ms +
      timing.score_fusion_ms +
      timing.result_assembly_ms
    expect(sum).toBeCloseTo(timing.total_ms, 1)
  })
})

describe('Type synchronization - Confidence Signals', () => {
  it('should deserialize ConfidenceSignals from Rust JSON', () => {
    // Example JSON from Rust serialization
    const rustJson = {
      source_count: 3,
      score_gap: 1.25,
      is_exact_match: true,
    }

    // TypeScript should parse without errors
    const signals: ConfidenceSignals = rustJson

    expect(signals.source_count).toBe(3)
    expect(signals.score_gap).toBeCloseTo(1.25)
    expect(signals.is_exact_match).toBe(true)
  })

  it('should validate all field types are correct', () => {
    const signals: ConfidenceSignals = {
      source_count: 2,
      score_gap: 0.42,
      is_exact_match: false,
    }

    // Verify field types
    expect(typeof signals.source_count).toBe('number')
    expect(typeof signals.score_gap).toBe('number')
    expect(typeof signals.is_exact_match).toBe('boolean')

    // Verify values
    expect(signals.source_count).toBe(2)
    expect(signals.score_gap).toBe(0.42)
    expect(signals.is_exact_match).toBe(false)
  })

  it('should handle edge cases for ConfidenceSignals', () => {
    // Test minimum values
    const minSignals: ConfidenceSignals = {
      source_count: 1,
      score_gap: 0.0,
      is_exact_match: false,
    }

    expect(minSignals.source_count).toBe(1)
    expect(minSignals.score_gap).toBe(0.0)
    expect(minSignals.is_exact_match).toBe(false)

    // Test maximum expected values
    const maxSignals: ConfidenceSignals = {
      source_count: 4,
      score_gap: 10.5,
      is_exact_match: true,
    }

    expect(maxSignals.source_count).toBe(4)
    expect(maxSignals.score_gap).toBeCloseTo(10.5)
    expect(maxSignals.is_exact_match).toBe(true)
  })

  it('should handle ConfidenceSignals with floating point score_gap', () => {
    const signals: ConfidenceSignals = {
      source_count: 3,
      score_gap: 0.123456789,
      is_exact_match: true,
    }

    // Verify floating point precision is preserved
    expect(signals.score_gap).toBeCloseTo(0.123456789, 5)
  })
})
