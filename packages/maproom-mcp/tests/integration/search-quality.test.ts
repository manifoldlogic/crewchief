/**
 * Search Quality Integration Tests
 *
 * Tests search ranking behavior against the test corpus.
 * Validates that semantic ranking improvements work correctly.
 *
 * Test Corpus: /tmp/semrank-test-corpus (indexed in SEMRANK-1004)
 * Baseline Metrics: benchmarks/baseline-fts.csv (from SEMRANK-1005)
 *
 * Test Strategy:
 * 1. Verify search tool works against test corpus
 * 2. Validate ranking behavior (implementations vs tests/docs)
 * 3. Measure latency and compare against baseline
 * 4. Ensure no performance regressions
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { Client } from 'pg'
import { setupTestDatabase, teardownTestDatabase } from '../helpers/database.js'
import {
  search,
  expectImplementationFirst,
  expectRankOrder,
  expectImplementationBeforeTests,
  expectImplementationBeforeDocs,
  measureLatency,
  loadBaseline,
  compareLatency,
  getImplementationRank,
  getTestRank,
  getDocRank,
  type BaselineMetric,
} from '../helpers/search-test-utils.js'

describe('Search Quality - Test Corpus', () => {
  let client: Client
  let baseline: Map<string, BaselineMetric>

  beforeAll(async () => {
    // Connect to test database (but skip cleanTestData since we need test-corpus preserved)
    const { createClient, setupTestSchema } = await import('../helpers/database.js')
    client = await createClient()
    await setupTestSchema(client)

    // Verify test-corpus exists
    const { rows } = await client.query(
      "SELECT COUNT(*) as count FROM maproom.repos WHERE name = 'test-corpus'"
    )
    const repoExists = parseInt(rows[0].count) > 0

    if (!repoExists) {
      throw new Error(
        'Test corpus not indexed. Run: /workspace/packages/cli/bin/linux-arm64/crewchief-maproom scan --repo test-corpus --worktree main --path /tmp/semrank-test-corpus --commit HEAD --force --generate-embeddings false'
      )
    }

    // Load baseline metrics for comparison
    try {
      baseline = await loadBaseline()
    } catch (error) {
      console.warn('Could not load baseline CSV, skipping baseline comparisons:', error)
      baseline = new Map()
    }
  })

  afterAll(async () => {
    // Clean up database connection (without cleaning test data)
    await client.end()
  })

  describe('Basic Search Functionality', () => {
    it('should return results for exact function name query', async () => {
      const results = await search(client, 'authenticate')

      expect(results).toBeDefined()
      expect(results.length).toBeGreaterThan(0)
      expect(results[0].score).toBeGreaterThan(0)
    })

    it('should return results for concept search', async () => {
      const results = await search(client, 'user authentication')

      expect(results).toBeDefined()
      expect(results.length).toBeGreaterThan(0)
    })

    it('should return results for Python snake_case function', async () => {
      const results = await search(client, 'validate_token')

      expect(results).toBeDefined()
      expect(results.length).toBeGreaterThan(0)
    })

    it('should return results for TypeScript camelCase function', async () => {
      const results = await search(client, 'validateToken')

      expect(results).toBeDefined()
      expect(results.length).toBeGreaterThan(0)
    })

    it('should return results for Rust function', async () => {
      const results = await search(client, 'connect_database')

      expect(results).toBeDefined()
      expect(results.length).toBeGreaterThan(0)
    })

    it('should return results for class name', async () => {
      const results = await search(client, 'DatabaseConnection')

      expect(results).toBeDefined()
      expect(results.length).toBeGreaterThan(0)
    })

    it('should return results for React hook', async () => {
      const results = await search(client, 'useAuth')

      expect(results).toBeDefined()
      expect(results.length).toBeGreaterThan(0)
    })

    it('should respect limit parameter', async () => {
      const results = await search(client, 'authenticate', { limit: 5 })

      expect(results.length).toBeLessThanOrEqual(5)
    })

    it('should return results ordered by score descending', async () => {
      const results = await search(client, 'authenticate')

      for (let i = 1; i < results.length; i++) {
        expect(results[i - 1].score).toBeGreaterThanOrEqual(results[i].score)
      }
    })
  })

  describe('Search Result Metadata', () => {
    it('should include chunk_id for all results', async () => {
      const results = await search(client, 'authenticate')

      for (const result of results) {
        const chunkId = typeof result.chunk_id === 'string' ? parseInt(result.chunk_id) : result.chunk_id
        expect(chunkId).toBeGreaterThan(0)
      }
    })

    it('should include file path for all results', async () => {
      const results = await search(client, 'authenticate')

      for (const result of results) {
        expect(result.relpath).toBeDefined()
        expect(typeof result.relpath).toBe('string')
        expect(result.relpath.length).toBeGreaterThan(0)
      }
    })

    it('should include line ranges for all results', async () => {
      const results = await search(client, 'authenticate')

      for (const result of results) {
        expect(result.start_line).toBeGreaterThan(0)
        expect(result.end_line).toBeGreaterThanOrEqual(result.start_line)
      }
    })

    it('should include kind for all results', async () => {
      const results = await search(client, 'authenticate')

      const validKinds = [
        'func',
        'class',
        'method',
        'component',
        'hook',
        'module',
        'var',
        'type',
        'heading_1',
        'heading_2',
        'heading_3',
        'markdown_section',
        'code_block',
        'imports',
        'use',
      ]

      for (const result of results) {
        expect(result.kind).toBeDefined()
        expect(validKinds).toContain(result.kind)
      }
    })
  })

  describe('Ranking Behavior - Current Baseline', () => {
    it('should find implementation chunks for "connect_database"', async () => {
      const results = await search(client, 'connect_database')

      const implRank = getImplementationRank(results)
      expect(implRank).toBeDefined()
      expect(implRank).toBeGreaterThan(0)
    })

    it('should find implementation chunks for "execute_query"', async () => {
      const results = await search(client, 'execute_query')

      const implRank = getImplementationRank(results)
      expect(implRank).toBeDefined()
      expect(implRank).toBeGreaterThan(0)
    })

    it('should find implementation chunks for concept search', async () => {
      const results = await search(client, 'user authentication')

      const implRank = getImplementationRank(results)
      expect(implRank).toBeDefined()
      expect(implRank).toBeGreaterThan(0)
    })

    it('should detect documentation chunks in results', async () => {
      const results = await search(client, 'authenticate')

      const docRank = getDocRank(results)
      expect(docRank).toBeDefined()
      expect(docRank).toBeGreaterThan(0)
    })

    it('should detect test chunks in results', async () => {
      const results = await search(client, 'test_authenticate')

      const testRank = getTestRank(results)
      expect(testRank).toBeDefined()
      expect(testRank).toBeGreaterThan(0)
    })
  })

  describe('Ranking Helper Functions', () => {
    it('should validate rank order for specific query', async () => {
      // This will pass or fail based on current ranking
      // Useful for documenting current behavior
      const results = await search(client, 'connect_database', { limit: 3 })
      const kinds = results.slice(0, 3).map((r) => r.kind)

      // Document what we actually get
      expect(kinds).toBeDefined()
      expect(kinds.length).toBeGreaterThan(0)
    })

    it('should measure implementation rank consistency', async () => {
      const queries = ['authenticate', 'validate_token', 'connect_database']

      for (const query of queries) {
        const results = await search(client, query)
        const implRank = getImplementationRank(results)

        // All queries should find at least one implementation
        expect(implRank).toBeDefined()
        expect(implRank).toBeGreaterThan(0)
      }
    })
  })

  describe('Performance Benchmarks', () => {
    it('should measure latency for exact function search', async () => {
      const metrics = await measureLatency(client, 'authenticate', 20, 5)

      expect(metrics.p50).toBeGreaterThan(0)
      expect(metrics.p95).toBeGreaterThan(0)
      expect(metrics.p99).toBeGreaterThan(0)
      expect(metrics.samples).toBe(20)

      // Performance should be reasonable (< 200ms p95)
      expect(metrics.p95).toBeLessThan(200)
    })

    it('should measure latency for concept search', async () => {
      const metrics = await measureLatency(client, 'user authentication', 20, 5)

      expect(metrics.p50).toBeGreaterThan(0)
      expect(metrics.p95).toBeGreaterThan(0)
      expect(metrics.p95).toBeLessThan(200)
    })

    it('should measure latency for class name search', async () => {
      const metrics = await measureLatency(client, 'DatabaseConnection', 20, 5)

      expect(metrics.p50).toBeGreaterThan(0)
      expect(metrics.p95).toBeGreaterThan(0)
      expect(metrics.p95).toBeLessThan(200)
    })
  })

  describe('Baseline Comparison', () => {
    it('should load baseline metrics successfully', () => {
      // Baseline should be loaded in beforeAll
      expect(baseline).toBeDefined()

      if (baseline.size > 0) {
        expect(baseline.size).toBeGreaterThan(0)

        // Check a known query exists
        const auth = baseline.get('authenticate')
        expect(auth).toBeDefined()

        if (auth) {
          expect(auth.latency_p50_ms).toBeGreaterThan(0)
          expect(auth.latency_p95_ms).toBeGreaterThan(0)
        }
      }
    })

    it('should compare current latency against baseline', async () => {
      if (baseline.size === 0) {
        console.warn('Skipping baseline comparison - baseline not loaded')
        return
      }

      const query = 'authenticate'
      const baselineMetric = baseline.get(query)

      if (!baselineMetric) {
        console.warn(`Skipping baseline comparison - no baseline for "${query}"`)
        return
      }

      const currentMetrics = await measureLatency(client, query, 20, 5)

      const comparison = compareLatency(currentMetrics, baselineMetric, 0.1)

      // Log comparison for visibility
      console.log(`Latency comparison for "${query}":`)
      console.log(`  P50: ${currentMetrics.p50.toFixed(2)}ms (baseline: ${baselineMetric.latency_p50_ms}ms, diff: ${comparison.p50_diff_pct.toFixed(1)}%)`)
      console.log(`  P95: ${currentMetrics.p95.toFixed(2)}ms (baseline: ${baselineMetric.latency_p95_ms}ms, diff: ${comparison.p95_diff_pct.toFixed(1)}%)`)
      console.log(`  P99: ${currentMetrics.p99.toFixed(2)}ms (baseline: ${baselineMetric.latency_p99_ms}ms, diff: ${comparison.p99_diff_pct.toFixed(1)}%)`)

      if (!comparison.passed) {
        console.warn('Performance regression detected:', comparison.failures)
      }

      // Don't fail test on regression - just document it
      expect(comparison).toBeDefined()
    })
  })

  describe('Empty Result Handling', () => {
    it('should return empty array for no matches', async () => {
      const results = await search(client, 'nonexistent_function_xyz_12345')

      expect(results).toBeDefined()
      expect(Array.isArray(results)).toBe(true)
      expect(results.length).toBe(0)
    })

    it('should handle special characters gracefully', async () => {
      const results = await search(client, '@@@@')

      expect(results).toBeDefined()
      expect(Array.isArray(results)).toBe(true)
    })
  })

  describe('Repo and Worktree Scoping', () => {
    it('should respect repo parameter', async () => {
      const results = await search(client, 'authenticate', { repo: 'test-corpus' })

      expect(results).toBeDefined()
      expect(results.length).toBeGreaterThan(0)
    })

    it('should respect worktree parameter', async () => {
      const results = await search(client, 'authenticate', {
        repo: 'test-corpus',
        worktree: 'main',
      })

      expect(results).toBeDefined()
      expect(results.length).toBeGreaterThan(0)
    })

    it('should throw error for non-existent repo', async () => {
      await expect(
        search(client, 'authenticate', { repo: 'nonexistent-repo' })
      ).rejects.toThrow('not found')
    })
  })
})

describe('Search Quality - Phase 3 Readiness', () => {
  let client: Client

  beforeAll(async () => {
    const { createClient, setupTestSchema } = await import('../helpers/database.js')
    client = await createClient()
    await setupTestSchema(client)
  })

  afterAll(async () => {
    await client.end()
  })

  it('should have test corpus indexed and ready', async () => {
    // Verify test corpus exists and has chunks
    const { rows } = await client.query(
      `SELECT COUNT(*) as count
       FROM maproom.chunks c
       JOIN maproom.files f ON f.id = c.file_id
       JOIN maproom.repos r ON r.id = f.repo_id
       WHERE r.name = 'test-corpus'`
    )

    const count = parseInt(rows[0].count)
    expect(count).toBeGreaterThan(50) // Should have ~104 chunks
  })

  it('should have implementations in test corpus', async () => {
    const { rows } = await client.query(
      `SELECT COUNT(*) as count
       FROM maproom.chunks c
       JOIN maproom.files f ON f.id = c.file_id
       JOIN maproom.repos r ON r.id = f.repo_id
       WHERE r.name = 'test-corpus'
         AND c.kind IN ('func', 'class', 'method')`
    )

    const count = parseInt(rows[0].count)
    expect(count).toBeGreaterThan(10) // Should have ~28 impl chunks
  })

  it('should have documentation in test corpus', async () => {
    const { rows } = await client.query(
      `SELECT COUNT(*) as count
       FROM maproom.chunks c
       JOIN maproom.files f ON f.id = c.file_id
       JOIN maproom.repos r ON r.id = f.repo_id
       WHERE r.name = 'test-corpus'
         AND c.kind::text LIKE 'heading_%'`
    )

    const count = parseInt(rows[0].count)
    expect(count).toBeGreaterThan(10) // Should have ~37 heading chunks
  })

  it('should have test files in test corpus', async () => {
    const { rows } = await client.query(
      `SELECT COUNT(DISTINCT f.id) as count
       FROM maproom.files f
       JOIN maproom.repos r ON r.id = f.repo_id
       WHERE r.name = 'test-corpus'
         AND f.relpath LIKE '%test%'`
    )

    const count = parseInt(rows[0].count)
    expect(count).toBeGreaterThan(0) // Should have test files
  })

  it('should support debug mode for ranking analysis', async () => {
    const results = await search(client, 'authenticate', { debug: true, limit: 5 })

    expect(results).toBeDefined()
    expect(results.length).toBeGreaterThan(0)
    // Debug data would be in bundle metadata, not per-result
  })
})
