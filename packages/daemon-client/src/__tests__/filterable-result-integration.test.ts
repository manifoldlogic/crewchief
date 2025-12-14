/**
 * Integration tests for FilterableSearchResult chained operations
 *
 * Tests focus on:
 * - Chaining filter → sort → slice
 * - Multiple filter chains
 * - Order sensitivity (sort → filter vs filter → sort)
 * - Immutability preservation across chains
 * - Performance of combined operations
 */

import { describe, it, expect } from 'vitest'
import type { SearchResult } from '../client.js'
import { FilterableSearchResult } from '../filterable-result.js'

/**
 * Generate mock search results for testing
 *
 * Creates realistic data with varied kinds and file types for
 * comprehensive integration testing.
 *
 * @param count - Number of results to generate
 * @returns SearchResult with generated hits
 */
function generateMockResults(count: number): SearchResult {
  const hits: SearchResult['hits'] = []
  const kinds = ['function', 'class', 'interface', 'type']
  const extensions = ['.ts', '.tsx', '.js', '.jsx']

  for (let i = 0; i < count; i++) {
    hits.push({
      chunk_id: i,
      file_path: `src/file${i % 10}${extensions[i % extensions.length]}`,
      symbol_name: `symbol${i}`,
      kind: kinds[i % kinds.length],
      start_line: (i % 50) + 1,
      end_line: (i % 50) + 10,
      content: `content ${i}`,
      score: Math.random(),
    })
  }

  return { hits, total: count }
}

describe('FilterableSearchResult - Integration Tests', () => {
  const mockResult100 = generateMockResults(100)

  it('chains filter → sort → slice', () => {
    const result = new FilterableSearchResult(mockResult100)

    const filtered = result
      .filter({ kind: 'function' })
      .sortBy('relpath')
      .slice(0, 10)

    expect(filtered.hits.length).toBeLessThanOrEqual(10)
    expect(filtered.hits.every((h) => h.kind === 'function')).toBe(true)
    // Verify sorted
    for (let i = 0; i < filtered.hits.length - 1; i++) {
      expect(filtered.hits[i].file_path <= filtered.hits[i + 1].file_path).toBe(
        true,
      )
    }
  })

  it('chains multiple filters', () => {
    const result = new FilterableSearchResult(mockResult100)

    const filtered = result
      .filter({ kind: 'function' })
      .filter({ file_type: 'ts' })
      .filter({ min_score: 0.3 })

    expect(
      filtered.hits.every(
        (h) =>
          h.kind === 'function' &&
          h.file_path.endsWith('.ts') &&
          h.score >= 0.3,
      ),
    ).toBe(true)
  })

  it('chains sort → filter (order matters)', () => {
    const result = new FilterableSearchResult(mockResult100)

    // Sort first, then filter
    const filtered = result.sortBy('score', 'desc').filter({ kind: 'function' })

    expect(filtered.hits.every((h) => h.kind === 'function')).toBe(true)
    // Should still be sorted by score
    for (let i = 0; i < filtered.hits.length - 1; i++) {
      expect(filtered.hits[i].score >= filtered.hits[i + 1].score).toBe(true)
    }
  })

  it('preserves immutability across chain', () => {
    const result = new FilterableSearchResult(mockResult100)
    const originalHits = result.hits
    const originalLength = result.hits.length

    result.filter({ kind: 'function' }).sortBy('relpath').slice(0, 10)

    expect(result.hits).toBe(originalHits)
    expect(result.hits.length).toBe(originalLength)
  })

  it('handles empty results in chain', () => {
    const result = new FilterableSearchResult(mockResult100)

    const filtered = result
      .filter({ kind: 'nonexistent' })
      .sortBy('relpath')
      .slice(0, 10)

    expect(filtered.hits.length).toBe(0)
    expect(filtered.count).toBe(0)
  })

  // Performance tests (thresholds relaxed for CI environment variability)
  it('filter() completes in <5ms for 100 items', () => {
    const result = new FilterableSearchResult(mockResult100)
    const start = performance.now()

    result.filter({ kind: 'function' })

    const elapsed = performance.now() - start
    expect(elapsed).toBeLessThan(5)
  })

  it('sortBy() completes in <5ms for 100 items', () => {
    const result = new FilterableSearchResult(mockResult100)
    const start = performance.now()

    result.sortBy('relpath')

    const elapsed = performance.now() - start
    expect(elapsed).toBeLessThan(5)
  })

  it('chained operations complete in <10ms for 100 items', () => {
    const result = new FilterableSearchResult(mockResult100)
    const start = performance.now()

    result.filter({ kind: 'function' }).sortBy('relpath').slice(0, 10)

    const elapsed = performance.now() - start
    expect(elapsed).toBeLessThan(10)
  })
})
