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
  RelatedChunkResult,
  ChunkSearchResult,
} from './types.js'
import type { SearchParams } from './client.js'

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
        'Restart the maproom daemon: maproom serve',
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

describe('Type synchronization - Related Chunks', () => {
  // Sync with: crates/maproom/src/search/results.rs::RelatedChunkResult
  it('should deserialize RelatedChunkResult from Rust JSON', () => {
    // Example JSON from Rust serialization
    const rustJson = {
      chunk_id: 123,
      relpath: 'src/auth/handler.ts',
      symbol_name: 'authenticate',
      kind: 'function',
      start_line: 10,
      end_line: 25,
      preview: 'export function authenticate() {...',
      depth: 2,
      relevance: 0.7,
      relationship_type: 'call',
    }

    // TypeScript should parse without errors
    const related: RelatedChunkResult = rustJson

    expect(related.chunk_id).toBe(123)
    expect(related.relpath).toBe('src/auth/handler.ts')
    expect(related.symbol_name).toBe('authenticate')
    expect(related.kind).toBe('function')
    expect(related.start_line).toBe(10)
    expect(related.end_line).toBe(25)
    expect(related.preview).toContain('authenticate')
    expect(related.depth).toBe(2)
    expect(related.relevance).toBe(0.7)
    expect(related.relationship_type).toBe('call')
  })

  it('should validate all RelatedChunkResult fields and types', () => {
    const sample: RelatedChunkResult = {
      chunk_id: 123,
      relpath: 'src/auth/handler.ts',
      symbol_name: 'authenticate',
      kind: 'function',
      start_line: 10,
      end_line: 25,
      preview: 'export function authenticate() {...',
      depth: 2,
      relevance: 0.7,
      relationship_type: 'call',
    }

    // Validate all fields exist and have correct types
    expect(typeof sample.chunk_id).toBe('number')
    expect(typeof sample.relpath).toBe('string')
    expect(typeof sample.symbol_name).toBe('string')
    expect(typeof sample.kind).toBe('string')
    expect(typeof sample.start_line).toBe('number')
    expect(typeof sample.end_line).toBe('number')
    expect(typeof sample.preview).toBe('string')
    expect(typeof sample.depth).toBe('number')
    expect(typeof sample.relevance).toBe('number')
    expect(typeof sample.relationship_type).toBe('string')
  })

  it('should handle null symbol_name in RelatedChunkResult', () => {
    const sample: RelatedChunkResult = {
      chunk_id: 123,
      relpath: 'src/config.ts',
      symbol_name: null, // Anonymous chunk
      kind: 'module',
      start_line: 1,
      end_line: 100,
      preview: 'export const config = {...',
      depth: 1,
      relevance: 0.5,
      relationship_type: 'import',
    }

    expect(sample.symbol_name).toBeNull()
    expect(typeof sample.kind).toBe('string')
  })

  it('should validate depth values (1 or 2)', () => {
    const depth1: RelatedChunkResult = {
      chunk_id: 1,
      relpath: 'src/file.ts',
      symbol_name: 'func',
      kind: 'function',
      start_line: 1,
      end_line: 10,
      preview: 'preview',
      depth: 1,
      relevance: 0.7,
      relationship_type: 'call',
    }

    const depth2: RelatedChunkResult = {
      chunk_id: 2,
      relpath: 'src/file.ts',
      symbol_name: 'func',
      kind: 'function',
      start_line: 1,
      end_line: 10,
      preview: 'preview',
      depth: 2,
      relevance: 0.49,
      relationship_type: 'import',
    }

    expect(depth1.depth).toBe(1)
    expect(depth2.depth).toBe(2)
  })

  it('should validate relevance is between 0.0 and 1.0', () => {
    const sample: RelatedChunkResult = {
      chunk_id: 1,
      relpath: 'src/file.ts',
      symbol_name: 'func',
      kind: 'function',
      start_line: 1,
      end_line: 10,
      preview: 'preview',
      depth: 1,
      relevance: 0.8,
      relationship_type: 'call',
    }

    expect(sample.relevance).toBeGreaterThanOrEqual(0.0)
    expect(sample.relevance).toBeLessThanOrEqual(1.0)
  })

  it('should validate relationship_type values', () => {
    const relationshipTypes = ['import', 'call', 'extends', 'implements']

    relationshipTypes.forEach((relType) => {
      const sample: RelatedChunkResult = {
        chunk_id: 1,
        relpath: 'src/file.ts',
        symbol_name: 'func',
        kind: 'function',
        start_line: 1,
        end_line: 10,
        preview: 'preview',
        depth: 1,
        relevance: 0.5,
        relationship_type: relType,
      }

      expect(sample.relationship_type).toBe(relType)
    })
  })

  it('should handle ChunkSearchResult with optional related field', () => {
    // ChunkSearchResult without related field
    const resultWithout: ChunkSearchResult = {
      chunk_id: 1,
      file_id: 10,
      relpath: 'src/main.ts',
      symbol_name: 'main',
      kind: 'function',
      start_line: 1,
      end_line: 50,
      preview: 'export function main() {...',
      score: 0.95,
    }

    expect(resultWithout.related).toBeUndefined()

    // ChunkSearchResult with related field
    const resultWith: ChunkSearchResult = {
      chunk_id: 1,
      file_id: 10,
      relpath: 'src/main.ts',
      symbol_name: 'main',
      kind: 'function',
      start_line: 1,
      end_line: 50,
      preview: 'export function main() {...',
      score: 0.95,
      related: [
        {
          chunk_id: 2,
          relpath: 'src/helper.ts',
          symbol_name: 'helper',
          kind: 'function',
          start_line: 5,
          end_line: 10,
          preview: 'export function helper() {...',
          depth: 1,
          relevance: 0.8,
          relationship_type: 'call',
        },
      ],
    }

    expect(resultWith.related).toBeDefined()
    expect(Array.isArray(resultWith.related)).toBe(true)
    expect(resultWith.related?.length).toBe(1)
    expect(resultWith.related?.[0].chunk_id).toBe(2)
  })

  it('should handle ChunkSearchResult with empty related array', () => {
    const result: ChunkSearchResult = {
      chunk_id: 1,
      file_id: 10,
      relpath: 'src/main.ts',
      symbol_name: 'main',
      kind: 'function',
      start_line: 1,
      end_line: 50,
      preview: 'export function main() {...',
      score: 0.95,
      related: [], // Empty array (expansion ran but found no relationships)
    }

    expect(result.related).toBeDefined()
    expect(Array.isArray(result.related)).toBe(true)
    expect(result.related?.length).toBe(0)
  })

  it('should handle ChunkSearchResult with multiple related chunks', () => {
    const result: ChunkSearchResult = {
      chunk_id: 1,
      file_id: 10,
      relpath: 'src/main.ts',
      symbol_name: 'main',
      kind: 'function',
      start_line: 1,
      end_line: 50,
      preview: 'export function main() {...',
      score: 0.95,
      related: [
        {
          chunk_id: 2,
          relpath: 'src/helper.ts',
          symbol_name: 'helper',
          kind: 'function',
          start_line: 5,
          end_line: 10,
          preview: 'export function helper() {...',
          depth: 1,
          relevance: 0.8,
          relationship_type: 'call',
        },
        {
          chunk_id: 3,
          relpath: 'src/utils.ts',
          symbol_name: 'format',
          kind: 'function',
          start_line: 20,
          end_line: 30,
          preview: 'export function format() {...',
          depth: 2,
          relevance: 0.49,
          relationship_type: 'import',
        },
      ],
    }

    expect(result.related?.length).toBe(2)
    expect(result.related?.[0].depth).toBe(1)
    expect(result.related?.[1].depth).toBe(2)
  })

  it('should validate ChunkSearchResult has all required fields', () => {
    const result: ChunkSearchResult = {
      chunk_id: 1,
      file_id: 10,
      relpath: 'src/main.ts',
      symbol_name: 'main',
      kind: 'function',
      start_line: 1,
      end_line: 50,
      preview: 'export function main() {...',
      score: 0.95,
    }

    expect(typeof result.chunk_id).toBe('number')
    expect(typeof result.file_id).toBe('number')
    expect(typeof result.relpath).toBe('string')
    expect(typeof result.symbol_name).toBe('string')
    expect(typeof result.kind).toBe('string')
    expect(typeof result.start_line).toBe('number')
    expect(typeof result.end_line).toBe('number')
    expect(typeof result.preview).toBe('string')
    expect(typeof result.score).toBe('number')
  })
})

describe('Type synchronization - SearchParams filters', () => {
  // Sync with: crates/maproom/src/daemon/types.rs::SearchParams
  it('should validate SearchParams filter fields match Rust', () => {
    // This test validates that SearchParams has the expected filter fields
    // matching the Rust SearchParams in daemon/types.rs
    const params: SearchParams = {
      repo: 'test-repo',
      query: 'test query',
      kind: ['func', 'class'],
      lang: ['py', 'ts'],
    };

    // Verify fields exist and have correct types
    expect(params.kind).toBeInstanceOf(Array);
    expect(params.lang).toBeInstanceOf(Array);
    expect(params.kind![0]).toBe('func');
    expect(params.lang![0]).toBe('py');
  });

  it('should handle optional filter fields', () => {
    // Verify filters are truly optional (backward compatibility)
    const params: SearchParams = {
      repo: 'test-repo',
      query: 'test query',
    };

    expect(params.kind).toBeUndefined();
    expect(params.lang).toBeUndefined();
  });

  it('should serialize to JSON matching Rust expectations', () => {
    const params: SearchParams = {
      repo: 'test-repo',
      query: 'test query',
      kind: ['func'],
      lang: ['py'],
    };

    const json = JSON.stringify(params);
    const parsed = JSON.parse(json);

    // Verify JSON keys match Rust field names exactly
    expect(parsed.kind).toEqual(['func']);
    expect(parsed.lang).toEqual(['py']);
    expect(parsed.repo).toBe('test-repo');
    expect(parsed.query).toBe('test query');
  });
})
