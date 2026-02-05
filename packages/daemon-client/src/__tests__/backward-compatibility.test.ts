import { describe, it, expect } from 'vitest'
import type { SearchParams, SearchResult } from '../client.js'
import type { ConfidenceSignals, ChunkSearchResult } from '../types.js'

describe('Backward Compatibility', () => {
  it('should allow SearchParams without include_confidence', () => {
    // Old API code that doesn't know about include_confidence
    const params: SearchParams = {
      query: 'test',
      repo: 'test-repo',
    }
    expect(params.include_confidence).toBeUndefined()
  })

  it('should allow SearchResult hits without confidence field', () => {
    // Simulated daemon response without confidence (backward compat)
    const result: SearchResult = {
      hits: [
        {
          chunk_id: 1,
          file_path: 'src/test.rs',
          start_line: 1,
          end_line: 10,
          symbol_name: 'test',
          kind: 'function',
          content: 'fn test() {}',
          score: 0.95,
          // NO confidence field
        },
      ],
      total: 1,
    }
    expect(result.hits[0].confidence).toBeUndefined()
  })

  it('should allow SearchResult hits with confidence field', () => {
    const result: SearchResult = {
      hits: [
        {
          chunk_id: 1,
          file_path: 'src/test.rs',
          start_line: 1,
          end_line: 10,
          symbol_name: 'test',
          kind: 'function',
          content: 'fn test() {}',
          score: 0.95,
          confidence: {
            source_count: 2,
            score_gap: 0.15,
            is_exact_match: true,
          },
        },
      ],
      total: 1,
    }
    expect(result.hits[0].confidence).toBeDefined()
    expect(result.hits[0].confidence!.source_count).toBe(2)
  })

  it('should verify Rust ConfidenceSignals field types match TypeScript', () => {
    // Simulated Rust JSON output
    const rustJson = {
      source_count: 3, // Rust usize -> TS number
      score_gap: 0.42, // Rust f32 -> TS number
      is_exact_match: true, // Rust bool -> TS boolean
    }
    const signals: ConfidenceSignals = rustJson
    expect(typeof signals.source_count).toBe('number')
    expect(typeof signals.score_gap).toBe('number')
    expect(typeof signals.is_exact_match).toBe('boolean')
  })

  it('should verify ChunkSearchResult confidence is optional', () => {
    const withoutConfidence: ChunkSearchResult = {
      chunk_id: 1,
      file_id: 1,
      relpath: 'src/test.rs',
      symbol_name: 'test',
      kind: 'function',
      start_line: 1,
      end_line: 10,
      preview: 'fn test() {}',
      score: 0.95,
    }
    expect(withoutConfidence.confidence).toBeUndefined()

    const withConfidence: ChunkSearchResult = {
      chunk_id: 1,
      file_id: 1,
      relpath: 'src/test.rs',
      symbol_name: 'test',
      kind: 'function',
      start_line: 1,
      end_line: 10,
      preview: 'fn test() {}',
      score: 0.95,
      confidence: { source_count: 1, score_gap: 0.0, is_exact_match: false },
    }
    expect(withConfidence.confidence).toBeDefined()
  })

  it('should produce identical response structure with and without confidence', () => {
    // When confidence is disabled (or not requested), the response shape
    // should be the same minus the confidence field
    const responseWithout: SearchResult = {
      hits: [
        {
          chunk_id: 1,
          file_path: 'src/test.rs',
          start_line: 1,
          end_line: 10,
          symbol_name: 'test',
          kind: 'function',
          content: 'fn test() {}',
          score: 0.95,
        },
      ],
      total: 1,
    }

    const responseWith: SearchResult = {
      hits: [
        {
          chunk_id: 1,
          file_path: 'src/test.rs',
          start_line: 1,
          end_line: 10,
          symbol_name: 'test',
          kind: 'function',
          content: 'fn test() {}',
          score: 0.95,
          confidence: {
            source_count: 2,
            score_gap: 0.15,
            is_exact_match: true,
          },
        },
      ],
      total: 1,
    }

    // Core structure is the same (same top-level keys)
    const keysWithout = Object.keys(responseWithout)
    const keysWith = Object.keys(responseWith)
    expect(keysWithout).toEqual(keysWith)

    // Hit structure shares the same base keys
    const hitKeysWithout = Object.keys(responseWithout.hits[0])
    const hitKeysWith = Object.keys(responseWith.hits[0])

    // The "without" set should be a subset of the "with" set
    for (const key of hitKeysWithout) {
      expect(hitKeysWith).toContain(key)
    }

    // The only extra key is "confidence"
    const extraKeys = hitKeysWith.filter((k) => !hitKeysWithout.includes(k))
    expect(extraKeys).toEqual(['confidence'])
  })
})
