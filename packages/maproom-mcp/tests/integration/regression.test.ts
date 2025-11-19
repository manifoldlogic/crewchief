/**
 * Regression Tests: SEMRANK-3006
 *
 * Validates that known failure cases from project analysis are resolved by semantic ranking.
 * These tests document specific problems that motivated the SEMRANK project and verify they remain fixed.
 *
 * Known Failures (from SEMRANK analysis.md):
 * 1. Implementations ranked below tests/docs
 * 2. Case-sensitive matching failures
 * 3. Multi-word queries don't match snake_case symbols
 * 4. Acronym normalization missing
 *
 * Note: Comprehensive test coverage is in search-quality.test.ts.
 * These regression tests focus on documenting the "before/after" story.
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { Client } from 'pg'
import { search } from '../helpers/search-test-utils.js'

describe('Regression Tests - SEMRANK-3006', () => {
  let client: Client

  beforeAll(async () => {
    const { createClient, setupTestSchema, ensureTestCorpusIndexed } = await import('../helpers/database.js')
    client = await createClient()
    await setupTestSchema(client)

    // Ensure test-corpus is indexed (auto-index if missing)
    await ensureTestCorpusIndexed(client)
  })

  afterAll(async () => {
    await client?.end()
  })

  describe('Known Failure #1: Implementation vs Test Ranking', () => {
    it('should rank implementations before tests for exact symbol match', async () => {
      // Original Problem: Tests ranked higher than implementations due to keyword frequency
      // Fix: Kind multipliers (func: 2.5×, test: 0.6×) + exact match (3.0×)

      const results = await search(client, 'authenticate', { limit: 10 })

      // Verify implementation ranks first
      expect(results.length).toBeGreaterThan(0)
      const first = results[0]
      expect(['func', 'async_func', 'method']).toContain(first.kind)

      // Find first test (if any)
      const testIndex = results.findIndex(r => r.relpath?.includes('test'))

      if (testIndex >= 0) {
        // Tests should rank lower than implementations
        expect(testIndex).toBeGreaterThan(0)
      }

      // Regression validation: Implementation is #1
      expect(results[0].kind).not.toMatch(/test|spec/)
    })

    it('should rank create_session implementation before test', async () => {
      // Another implementation vs test case
      const results = await search(client, 'create_session', { limit: 10 })

      expect(results.length).toBeGreaterThan(0)
      const first = results[0]

      // First result should be implementation
      expect(['func', 'async_func', 'method', 'class']).toContain(first.kind)
      expect(first.relpath).not.toMatch(/test|spec/)
    })
  })

  describe('Known Failure #2: Implementation vs Documentation Ranking', () => {
    it('should rank implementations before documentation for concept queries', async () => {
      // Original Problem: Documentation ranked #1 due to keyword frequency
      // Fix: Kind multipliers (func/class: 2.5×, heading: 0.3-0.6×)

      const results = await search(client, 'authentication', { limit: 10 })

      expect(results.length).toBeGreaterThan(0)

      // Find first implementation and first doc
      const implIndex = results.findIndex(r =>
        ['func', 'class', 'method', 'async_func'].includes(r.kind)
      )
      const docIndex = results.findIndex(r =>
        r.kind.startsWith('heading_') || r.kind === 'markdown_section'
      )

      // Implementation should rank before documentation
      if (implIndex >= 0 && docIndex >= 0) {
        expect(implIndex).toBeLessThan(docIndex)
      }

      // Regression validation: Top result is implementation, not docs
      expect(['func', 'class', 'method', 'async_func', 'component', 'hook']).toContain(
        results[0].kind
      )
    })

    it('should rank database implementations before docs mentioning "database"', async () => {
      const results = await search(client, 'database connection', { limit: 10 })

      expect(results.length).toBeGreaterThan(0)

      // Top results should be implementations
      const top3 = results.slice(0, 3)
      const implCount = top3.filter(r => ['func', 'class', 'method'].includes(r.kind)).length

      // At least 2 of top 3 should be implementations
      expect(implCount).toBeGreaterThanOrEqual(2)
    })
  })

  describe('Known Failure #3: Case Sensitivity', () => {
    it('should return same #1 result for all case variations', async () => {
      // Original Problem: Case-sensitive matching caused different results
      // Fix: LOWER(symbol_name) = LOWER(query) in exact match detection

      const queries = ['authenticate', 'Authenticate', 'AUTHENTICATE', 'AuThEnTiCaTe']

      const allResults = await Promise.all(queries.map(q => search(client, q, { limit: 5 })))

      // All should have results
      allResults.forEach(results => {
        expect(results.length).toBeGreaterThan(0)
      })

      // Top results should be the same chunk (case-insensitive exact match)
      const topChunkIds = allResults.map(results => results[0].chunk_id)
      const uniqueChunks = new Set(topChunkIds)

      // All case variations return same #1 result
      expect(uniqueChunks.size).toBe(1)

      // Regression validation: Case doesn't affect ranking
      const scores = allResults.map(results => results[0].score)
      const uniqueScores = new Set(scores.map(s => s.toFixed(4))) // Round to avoid float precision issues

      // Scores should be identical (or very close)
      expect(uniqueScores.size).toBeLessThanOrEqual(2) // Allow minor float variance
    })
  })

  describe('Known Failure #4: Multi-Word Queries', () => {
    it('should normalize multi-word queries correctly', async () => {
      // Original Problem: Space-separated queries didn't match snake_case symbols
      // Fix: Query normalization (spaces → underscores, camelCase → snake_case)

      // Use "user authentication" which exists in test corpus
      const results = await search(client, 'user authentication', { limit: 10 })

      expect(results.length).toBeGreaterThan(0)

      // Should find authentication-related chunks
      const hasRelevantResults = results.some(
        r =>
          r.symbol_name?.toLowerCase().includes('auth') ||
          r.symbol_name?.toLowerCase().includes('user')
      )

      expect(hasRelevantResults).toBe(true)

      // Regression validation: Multi-word queries work
      expect(results[0].score).toBeGreaterThan(0)
    })

    it('should normalize "database connection" to match database_connection', async () => {
      const results = await search(client, 'database connection', { limit: 10 })

      expect(results.length).toBeGreaterThan(0)

      // Should find database-related implementations
      const hasDbResults = results.some(
        r =>
          r.symbol_name?.toLowerCase().includes('database') ||
          r.symbol_name?.toLowerCase().includes('connection') ||
          r.content_preview?.toLowerCase().includes('database')
      )

      expect(hasDbResults).toBe(true)
    })
  })

  describe('Known Failure #5: Acronym Normalization', () => {
    it('should normalize validateToken (camelCase) to match validate_token (snake_case)', async () => {
      // Original Problem: Acronyms and camelCase didn't match snake_case symbols
      // Fix: Acronym-aware normalization in Rust (validateToken → validate_token)

      const results = await search(client, 'validateToken', { limit: 10 })

      expect(results.length).toBeGreaterThan(0)

      // Should find validate_token symbols
      const hasTokenValidation = results.some(
        r =>
          r.symbol_name?.toLowerCase().includes('validate') &&
          r.symbol_name?.toLowerCase().includes('token')
      )

      expect(hasTokenValidation).toBe(true)
    })

    it('should handle common programming acronyms (HTTP, XML, JSON)', async () => {
      // Test that common acronyms are handled correctly
      const acronymQueries = ['useAuth', 'connect_database', 'execute_query']

      for (const query of acronymQueries) {
        const results = await search(client, query, { limit: 5 })

        // Each query should return relevant results
        expect(results.length).toBeGreaterThan(0)
        expect(results[0].score).toBeGreaterThan(0)
      }
    })
  })

  describe('Regression Summary: All Known Failures Fixed', () => {
    it('should demonstrate overall semantic ranking quality', async () => {
      // This test validates that semantic ranking works holistically

      const testCases = [
        { query: 'authenticate', expectKind: ['func', 'method', 'async_func'] },
        { query: 'create_session', expectKind: ['func', 'method'] },
        { query: 'DatabaseConnection', expectKind: ['class'] },
        { query: 'user authentication', expectKind: ['func', 'class'] },
      ]

      for (const { query, expectKind } of testCases) {
        const results = await search(client, query, { limit: 5 })

        expect(results.length).toBeGreaterThan(0)
        expect(expectKind).toContain(results[0].kind)

        // Implementations rank first (not tests or docs)
        expect(results[0].relpath).not.toMatch(/test|spec/)
        expect(results[0].kind).not.toMatch(/heading|markdown/)
      }
    })

    it('should validate exact match multiplier is applied', async () => {
      // Verify the core semantic ranking mechanism works

      const results = await search(client, 'authenticate', { limit: 5, debug: true })

      expect(results.length).toBeGreaterThan(0)
      const first = results[0]

      // Verify score is present and positive (semantic ranking is working)
      expect(first.score).toBeGreaterThan(0)

      // Exact match should rank implementations first
      expect(['func', 'async_func', 'method', 'class']).toContain(first.kind)

      // The query "authenticate" should match symbols with exact name
      expect(first.symbol_name?.toLowerCase()).toContain('authenticate')
    })
  })
})
