/**
 * E2E tests for FilterableSearchResult with real daemon search results
 *
 * These tests validate the complete integration: daemon search → FilterableSearchResult
 * wrapper → filtering/sorting/pagination. They catch integration issues that unit tests
 * might miss, such as type mismatches or unexpected data formats.
 *
 * IMPORTANT: These tests require a running crewchief-maproom daemon.
 * They will gracefully skip if the daemon connection fails.
 */

import { describe, it, expect, beforeAll } from 'vitest'
import { DaemonClient } from '../client.js'
import { FilterableSearchResult } from '../filterable-result.js'

describe('FilterableSearchResult - E2E Tests', () => {
  let daemon: DaemonClient
  let isDaemonAvailable = false

  beforeAll(async () => {
    daemon = new DaemonClient()
    try {
      await daemon.connect()
      // Test if daemon is actually responsive
      await daemon.ping()
      isDaemonAvailable = true
    } catch (error) {
      console.log(
        'Daemon not available, E2E tests will be skipped:',
        error instanceof Error ? error.message : String(error),
      )
      isDaemonAvailable = false
    }
  })

  it.skipIf(!isDaemonAvailable)(
    'performs real search and filters functions',
    async () => {
      // Perform real search against crewchief codebase
      const searchResult = await daemon.search({
        query: 'function',
        repo: 'crewchief',
      })

      expect(searchResult.hits.length).toBeGreaterThan(0)

      // Wrap and filter
      const result = new FilterableSearchResult(searchResult)
      const functions = result.filter({ kind: 'function' })

      // Verify filtering worked
      expect(functions.hits.length).toBeGreaterThan(0)
      expect(functions.hits.every((h) => h.kind === 'function')).toBe(true)
      expect(functions.count).toBe(functions.hits.length)

      // Verify no data corruption
      expect(functions.hits[0].chunk_id).toBeDefined()
      expect(functions.hits[0].file_path).toBeDefined()
      expect(functions.hits[0].score).toBeDefined()
      expect(functions.hits[0].content).toBeDefined()
    },
  )

  it.skipIf(!isDaemonAvailable)(
    'filters TypeScript files on real data',
    async () => {
      const searchResult = await daemon.search({
        query: 'export',
        repo: 'crewchief',
      })

      const result = new FilterableSearchResult(searchResult)
      const tsFiles = result.filter({ file_type: 'ts' })

      // Verify all results are TypeScript files
      expect(tsFiles.hits.length).toBeGreaterThan(0)
      expect(tsFiles.hits.every((h) => h.file_path.endsWith('.ts'))).toBe(true)

      // Verify count consistency
      expect(tsFiles.count).toBe(tsFiles.hits.length)

      // Verify no data corruption
      for (const hit of tsFiles.hits) {
        expect(hit.chunk_id).toBeDefined()
        expect(typeof hit.chunk_id).toBe('number')
        expect(hit.file_path).toBeDefined()
        expect(typeof hit.file_path).toBe('string')
        expect(hit.score).toBeDefined()
        expect(typeof hit.score).toBe('number')
      }
    },
  )

  it.skipIf(!isDaemonAvailable)(
    'chains filter + sort + slice on real data',
    async () => {
      const searchResult = await daemon.search({
        query: 'search',
        repo: 'crewchief',
      })

      expect(searchResult.hits.length).toBeGreaterThan(0)

      const result = new FilterableSearchResult(searchResult)
      const filtered = result
        .filter({ kind: 'function', file_type: 'ts' })
        .sortBy('relpath')
        .slice(0, 10)

      // Verify chain worked
      expect(filtered.hits.length).toBeLessThanOrEqual(10)
      expect(filtered.hits.length).toBeGreaterThan(0)
      expect(
        filtered.hits.every(
          (h) => h.kind === 'function' && h.file_path.endsWith('.ts'),
        ),
      ).toBe(true)

      // Verify sorted by relpath (file_path)
      for (let i = 0; i < filtered.hits.length - 1; i++) {
        expect(filtered.hits[i].file_path <= filtered.hits[i + 1].file_path).toBe(
          true,
        )
      }

      // Verify no data corruption
      expect(filtered.hits[0].chunk_id).toBeDefined()
      expect(filtered.hits[0].symbol_name).toBeDefined()
      expect(filtered.hits[0].start_line).toBeDefined()
      expect(filtered.hits[0].end_line).toBeDefined()
      expect(filtered.hits[0].content).toBeDefined()
    },
  )

  it.skipIf(!isDaemonAvailable)(
    'maintains backward compatibility',
    async () => {
      // Existing code pattern - should work unchanged
      const searchResult = await daemon.search({
        query: 'test',
        repo: 'crewchief',
      })

      // Access hits directly (existing pattern)
      expect(searchResult.hits).toBeDefined()
      expect(Array.isArray(searchResult.hits)).toBe(true)
      expect(searchResult.total).toBeDefined()
      expect(searchResult.hits.length).toBeGreaterThan(0)

      // Can still use SearchResult without FilterableSearchResult
      const firstHit = searchResult.hits[0]
      expect(firstHit.chunk_id).toBeDefined()
      expect(typeof firstHit.chunk_id).toBe('number')
      expect(firstHit.file_path).toBeDefined()
      expect(typeof firstHit.file_path).toBe('string')
      expect(firstHit.score).toBeDefined()
      expect(typeof firstHit.score).toBe('number')
      expect(firstHit.kind).toBeDefined()
      expect(typeof firstHit.kind).toBe('string')
      expect(firstHit.content).toBeDefined()
      expect(typeof firstHit.content).toBe('string')
      expect(firstHit.start_line).toBeDefined()
      expect(typeof firstHit.start_line).toBe('number')
      expect(firstHit.end_line).toBeDefined()
      expect(typeof firstHit.end_line).toBe('number')

      // Verify symbol_name can be null (as per type definition)
      expect(
        firstHit.symbol_name === null || typeof firstHit.symbol_name === 'string',
      ).toBe(true)
    },
  )

  it.skipIf(!isDaemonAvailable)(
    'has minimal performance overhead (<5ms)',
    async () => {
      const searchResult = await daemon.search({
        query: 'function',
        repo: 'crewchief',
      })

      expect(searchResult.hits.length).toBeGreaterThan(0)

      // Measure filtering overhead
      const start = performance.now()

      const result = new FilterableSearchResult(searchResult)
      const filtered = result
        .filter({ kind: 'function' })
        .sortBy('relpath')
        .slice(0, 20)

      const elapsed = performance.now() - start

      // Should be very fast (<5ms for typical result sets)
      expect(elapsed).toBeLessThan(5)

      // Verify results are valid
      expect(filtered.hits.length).toBeGreaterThan(0)
      expect(filtered.hits.length).toBeLessThanOrEqual(20)
      expect(filtered.hits.every((h) => h.kind === 'function')).toBe(true)

      // Verify no data corruption after fast operations
      expect(filtered.hits[0].chunk_id).toBeDefined()
      expect(filtered.hits[0].file_path).toBeDefined()
      expect(filtered.hits[0].score).toBeDefined()
    },
  )
})
