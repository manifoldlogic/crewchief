/**
 * Unit tests for FilterableSearchResult class
 *
 * Tests focus on filter() method, edge cases, and immutability.
 * Coverage target: ≥80% on filterable-result.ts
 */

import { describe, it, expect } from 'vitest'
import type { SearchResult } from '../client.js'
import { FilterableSearchResult } from '../filterable-result.js'

const mockHits: SearchResult['hits'] = [
  {
    chunk_id: 1,
    file_path: 'src/auth.ts',
    symbol_name: 'login',
    kind: 'function',
    start_line: 10,
    end_line: 20,
    content: 'function login() {...}',
    score: 0.95,
  },
  {
    chunk_id: 2,
    file_path: 'src/user.tsx',
    symbol_name: 'UserProfile',
    kind: 'class',
    start_line: 5,
    end_line: 50,
    content: 'class UserProfile {...}',
    score: 0.85,
  },
  {
    chunk_id: 3,
    file_path: 'test/auth.test.ts',
    symbol_name: 'testLogin',
    kind: 'function',
    start_line: 15,
    end_line: 25,
    content: 'function testLogin() {...}',
    score: 0.75,
  },
  {
    chunk_id: 4,
    file_path: 'README.md',
    symbol_name: null,
    kind: 'markdown',
    start_line: 1,
    end_line: 100,
    content: '# Documentation',
    score: 0.5,
  },
]

const mockSearchResult: SearchResult = {
  hits: mockHits,
  total: mockHits.length,
}

describe('FilterableSearchResult', () => {
  describe('filter()', () => {
    it('filters by kind (exact match)', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({ kind: 'function' })

      expect(filtered.hits.length).toBe(2)
      expect(filtered.hits.every((h) => h.kind === 'function')).toBe(true)
    })

    it('filters by file_type with dot', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({ file_type: '.ts' })

      expect(filtered.hits.length).toBe(2)
      expect(filtered.hits.every((h) => h.file_path.endsWith('.ts'))).toBe(true)
    })

    it('filters by file_type without dot', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({ file_type: 'tsx' })

      expect(filtered.hits.length).toBe(1)
      expect(filtered.hits[0].file_path).toContain('.tsx')
    })

    it('filters by path substring', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({ path: 'test/' })

      expect(filtered.hits.length).toBe(1)
      expect(filtered.hits[0].file_path).toContain('test/')
    })

    it('filters by min_score', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({ min_score: 0.8 })

      expect(filtered.hits.length).toBe(2)
      expect(filtered.hits.every((h) => h.score >= 0.8)).toBe(true)
    })

    it('filters by max_score', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({ max_score: 0.8 })

      expect(filtered.hits.length).toBe(2)
      expect(filtered.hits.every((h) => h.score <= 0.8)).toBe(true)
    })

    it('filters by custom function', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({
        custom: (hit) =>
          hit.symbol_name?.toLowerCase().includes('login') ?? false,
      })

      expect(filtered.hits.length).toBe(2)
      expect(
        filtered.hits.every((h) =>
          h.symbol_name?.toLowerCase().includes('login'),
        ),
      ).toBe(true)
    })

    it('combines multiple criteria with AND logic', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({
        kind: 'function',
        file_type: 'ts',
        min_score: 0.7,
      })

      expect(filtered.hits.length).toBe(2)
      expect(
        filtered.hits.every(
          (h) =>
            h.kind === 'function' &&
            h.file_path.endsWith('.ts') &&
            h.score >= 0.7,
        ),
      ).toBe(true)
    })

    it('handles empty results gracefully', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({ kind: 'nonexistent' })

      expect(filtered.hits.length).toBe(0)
      expect(filtered.count).toBe(0)
      expect(filtered.total).toBe(0)
    })

    it('handles null symbol_name', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({ kind: 'markdown' })

      expect(filtered.hits.length).toBe(1)
      expect(filtered.hits[0].symbol_name).toBeNull()
    })

    it('handles invalid scores gracefully', () => {
      const invalidResult: SearchResult = {
        hits: [{ ...mockHits[0], score: NaN }],
        total: 1,
      }
      const result = new FilterableSearchResult(invalidResult)
      const filtered = result.filter({ min_score: 0.5 })

      // Should filter out NaN scores
      expect(filtered.hits.length).toBe(0)
    })

    it('preserves immutability (original unchanged)', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const originalHits = result.hits.length

      result.filter({ kind: 'function' })

      expect(result.hits.length).toBe(originalHits)
      expect(result.raw.hits.length).toBe(originalHits)
    })
  })

  describe('constructor and getters', () => {
    it('initializes with SearchResult', () => {
      const result = new FilterableSearchResult(mockSearchResult)

      expect(result.hits).toBe(mockSearchResult.hits)
      expect(result.total).toBe(mockSearchResult.total)
      expect(result.count).toBe(mockSearchResult.hits.length)
    })

    it('raw getter returns original SearchResult', () => {
      const result = new FilterableSearchResult(mockSearchResult)

      expect(result.raw).toBe(mockSearchResult)
    })

    it('count equals hits.length', () => {
      const result = new FilterableSearchResult(mockSearchResult)

      expect(result.count).toBe(result.hits.length)
    })
  })

  describe('edge cases', () => {
    it('handles empty SearchResult', () => {
      const emptyResult: SearchResult = { hits: [], total: 0 }
      const result = new FilterableSearchResult(emptyResult)

      expect(result.hits.length).toBe(0)
      expect(result.total).toBe(0)
      expect(result.count).toBe(0)
    })

    it('handles filter with no criteria', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({})

      expect(filtered.hits.length).toBe(mockSearchResult.hits.length)
      expect(filtered.total).toBe(mockSearchResult.hits.length)
    })

    it('handles filter with all criteria excluding all results', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({
        kind: 'function',
        file_type: 'md', // No functions in .md files
      })

      expect(filtered.hits.length).toBe(0)
      expect(filtered.total).toBe(0)
    })

    it('handles score boundary conditions', () => {
      const result = new FilterableSearchResult(mockSearchResult)

      // Exact match on min_score
      const exactMin = result.filter({ min_score: 0.75 })
      expect(exactMin.hits.some((h) => h.score === 0.75)).toBe(true)

      // Exact match on max_score
      const exactMax = result.filter({ max_score: 0.75 })
      expect(exactMax.hits.some((h) => h.score === 0.75)).toBe(true)
    })

    it('handles custom filter returning false for all hits', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({
        custom: () => false,
      })

      expect(filtered.hits.length).toBe(0)
    })

    it('handles custom filter returning true for all hits', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result.filter({
        custom: () => true,
      })

      expect(filtered.hits.length).toBe(mockSearchResult.hits.length)
    })

    it('handles path with special characters', () => {
      const specialResult: SearchResult = {
        hits: [
          {
            ...mockHits[0],
            file_path: 'src/features/[id]/page.tsx',
          },
        ],
        total: 1,
      }
      const result = new FilterableSearchResult(specialResult)
      const filtered = result.filter({ path: '[id]' })

      expect(filtered.hits.length).toBe(1)
      expect(filtered.hits[0].file_path).toContain('[id]')
    })

    it('handles Infinity scores gracefully', () => {
      const infinityResult: SearchResult = {
        hits: [{ ...mockHits[0], score: Infinity }],
        total: 1,
      }
      const result = new FilterableSearchResult(infinityResult)
      const filtered = result.filter({ max_score: 1.0 })

      // Infinity > 1.0, should be filtered out
      expect(filtered.hits.length).toBe(0)
    })
  })

  describe('chaining filters', () => {
    it('allows chaining multiple filter calls', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const filtered = result
        .filter({ kind: 'function' })
        .filter({ min_score: 0.8 })

      expect(filtered.hits.length).toBe(1)
      expect(filtered.hits[0].kind).toBe('function')
      expect(filtered.hits[0].score).toBeGreaterThanOrEqual(0.8)
    })

    it('preserves immutability through chain', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const originalCount = result.count

      const step1 = result.filter({ kind: 'function' })
      const step2 = step1.filter({ min_score: 0.8 })

      // Original unchanged
      expect(result.count).toBe(originalCount)
      // Intermediate step unchanged
      expect(step1.count).toBe(2)
      // Final result has both filters applied
      expect(step2.count).toBe(1)
    })
  })

  describe('sortBy()', () => {
    it('sorts by score descending (default)', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const sorted = result.sortBy("score")

      expect(sorted.hits[0].score).toBeGreaterThanOrEqual(sorted.hits[1].score)
    })

    it('sorts by relpath ascending', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const sorted = result.sortBy("relpath")

      expect(sorted.hits[0].file_path <= sorted.hits[1].file_path).toBe(true)
    })

    it('sorts by symbol_name ascending', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const sorted = result.sortBy("symbol_name")

      const names = sorted.hits.map(h => h.symbol_name ?? "")
      expect(names[0] <= names[1]).toBe(true)
    })

    it('sorts by start_line ascending', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const sorted = result.sortBy("start_line")

      expect(sorted.hits[0].start_line <= sorted.hits[1].start_line).toBe(true)
    })

    it('sorts by kind ascending', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const sorted = result.sortBy("kind")

      expect(sorted.hits[0].kind <= sorted.hits[1].kind).toBe(true)
    })

    it('sorts descending when explicitly specified', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const sorted = result.sortBy("relpath", "desc")

      expect(sorted.hits[0].file_path >= sorted.hits[1].file_path).toBe(true)
    })

    it('preserves immutability (original unchanged)', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const originalFirst = result.hits[0]

      result.sortBy("relpath")

      expect(result.hits[0]).toBe(originalFirst)
    })
  })

  describe('slice()', () => {
    it('slices with start only', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const sliced = result.slice(2)

      expect(sliced.hits.length).toBe(mockHits.length - 2)
      expect(sliced.hits[0]).toBe(result.hits[2])
    })

    it('slices with start and end', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const sliced = result.slice(1, 3)

      expect(sliced.hits.length).toBe(2)
      expect(sliced.hits[0]).toBe(result.hits[1])
      expect(sliced.hits[1]).toBe(result.hits[2])
    })

    it('handles out of bounds gracefully', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const sliced = result.slice(100, 200)

      expect(sliced.hits.length).toBe(0)
    })

    it('preserves immutability (original unchanged)', () => {
      const result = new FilterableSearchResult(mockSearchResult)
      const originalLength = result.hits.length

      result.slice(0, 2)

      expect(result.hits.length).toBe(originalLength)
    })
  })
})
