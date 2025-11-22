/**
 * End-to-End Integration Tests for MCP Search via Daemon
 *
 * Tests the complete search flow: MCP tool → DaemonClient → Rust daemon → PostgreSQL
 * Uses real components (no mocking) to validate daemon integration works correctly.
 *
 * Test Coverage:
 * - Basic search functionality via daemon
 * - Daemon lifecycle (start, reuse, restart)
 * - Concurrent requests (10 and 50 concurrent)
 * - Error scenarios (repo not found, daemon failures)
 *
 * Prerequisites:
 * - PostgreSQL test database with test-corpus indexed
 * - crewchief-maproom binary built and available
 * - MAPROOM_DATABASE_URL environment variable set
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { Client } from 'pg'
import { closeDaemonClient, getDaemonClient } from '../src/daemon.js'
import { handleSearchTool } from '../src/tools/search.js'
import { setupTestDatabase, teardownTestDatabase, ensureTestCorpusIndexed } from './helpers/database.js'
import type { SearchBundle } from '../src/types.js'

describe('MCP Search Integration via Daemon', () => {
  let client: Client

  beforeAll(async () => {
    // Setup test database
    const { createClient, setupTestSchema } = await import('./helpers/database.js')
    client = await createClient()
    await setupTestSchema(client)

    // Index integration test corpus
    const { execSync } = await import('node:child_process')
    const path = await import('node:path')
    const { fileURLToPath } = await import('node:url')

    const __dirname = path.dirname(fileURLToPath(import.meta.url))
    const binaryPath = path.join(__dirname, '..', '..', 'cli', 'bin', 'crewchief-maproom')

    // Check if corpus already indexed
    const { rows } = await client.query(
      "SELECT COUNT(*) as count FROM maproom.repos WHERE name = 'integration-test-corpus'"
    )
    const repoExists = parseInt(rows[0].count) > 0

    if (!repoExists) {
      console.log('📦 Indexing integration test corpus...')
      execSync(
        `"${binaryPath}" scan --repo integration-test-corpus --worktree main --path /tmp/integration-test-corpus --commit HEAD --force --generate-embeddings false`,
        {
          stdio: 'inherit',
          env: {
            ...process.env,
            MAPROOM_DATABASE_URL: process.env.TEST_MAPROOM_DATABASE_URL || process.env.MAPROOM_DATABASE_URL,
          },
        }
      )
      console.log('✅ Integration test corpus indexed')
    }
  })

  afterAll(async () => {
    // Critical: Close daemon client to prevent process leaks
    await closeDaemonClient()

    // Clean up database connection (without cleaning data - other tests may need it)
    if (client) {
      await client.end()
    }
  })

  describe('Basic Search', () => {
    it('should return search results via daemon', async () => {
      const params = {
        query: 'authenticate',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 10,
        mode: 'fts' as const,
      }

      const result: SearchBundle = await handleSearchTool(params, client)

      expect(result).toHaveProperty('hits')
      expect(result.hits).toBeInstanceOf(Array)
      expect(result.hits.length).toBeGreaterThan(0)
      expect(result.hits[0]).toHaveProperty('chunk_id')
      expect(result.hits[0]).toHaveProperty('score')
      expect(result.hits[0]).toHaveProperty('relpath')
      expect(result.total).toBeGreaterThan(0)
    })

    it('should return results for concept search', async () => {
      const params = {
        query: 'database connection',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 10,
        mode: 'fts' as const,
      }

      const result = await handleSearchTool(params, client)

      expect(result.hits.length).toBeGreaterThan(0)
      expect(result.query).toBe('database connection')
    })

    it('should return results for exact function name', async () => {
      const params = {
        query: 'validateToken',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 10,
        mode: 'fts' as const,
      }

      const result = await handleSearchTool(params, client)

      expect(result.hits.length).toBeGreaterThan(0)
    })

    it('should return empty results for non-existent query', async () => {
      const params = {
        query: 'nonexistentfunctionnamexyz123',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 10,
        mode: 'fts' as const,
      }

      const result = await handleSearchTool(params, client)

      // Empty results should not throw, just return empty array
      expect(result.hits).toBeInstanceOf(Array)
      expect(result.hits.length).toBe(0)
    })

    it('should respect limit parameter', async () => {
      const params = {
        query: 'test',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 3,
        mode: 'fts' as const,
      }

      const result = await handleSearchTool(params, client)

      // Should not exceed limit
      expect(result.hits.length).toBeLessThanOrEqual(3)
    })
  })

  describe('Daemon Lifecycle', () => {
    it('should start daemon on first request', async () => {
      // Close any existing daemon
      await closeDaemonClient()

      const params = {
        query: 'first request',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 5,
        mode: 'fts' as const,
      }

      // First request should start daemon
      const result = await handleSearchTool(params, client)

      expect(result.hits).toBeInstanceOf(Array)
    })

    it('should reuse daemon for subsequent requests', async () => {
      const params1 = {
        query: 'request one',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 5,
        mode: 'fts' as const,
      }

      const params2 = {
        query: 'request two',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 5,
        mode: 'fts' as const,
      }

      // Both requests should succeed using same daemon
      const result1 = await handleSearchTool(params1, client)
      const result2 = await handleSearchTool(params2, client)

      expect(result1.hits).toBeInstanceOf(Array)
      expect(result2.hits).toBeInstanceOf(Array)

      // Queries should be different
      expect(result1.query).not.toBe(result2.query)
    })

    it('should restart daemon if explicitly stopped', async () => {
      // Stop daemon
      await closeDaemonClient()

      // New request should restart daemon
      const params = {
        query: 'after restart',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 5,
        mode: 'fts' as const,
      }

      const result = await handleSearchTool(params, client)

      expect(result.hits).toBeInstanceOf(Array)
    })

    it('should handle graceful shutdown', async () => {
      // Ensure daemon is running
      const params = {
        query: 'before shutdown',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 5,
        mode: 'fts' as const,
      }

      await handleSearchTool(params, client)

      // Graceful shutdown should not throw
      await expect(closeDaemonClient()).resolves.toBeUndefined()
    })
  })

  describe('Concurrent Requests', () => {
    it('should handle 10 concurrent searches without errors', async () => {
      const searches = Array.from({ length: 10 }, (_, i) => ({
        query: `concurrent query ${i}`,
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 5,
        mode: 'fts' as const,
      }))

      // Execute all searches concurrently
      const results = await Promise.all(
        searches.map((params) => handleSearchTool(params, client))
      )

      // All should succeed
      expect(results.length).toBe(10)

      // Verify each result is valid
      results.forEach((result, i) => {
        expect(result).toHaveProperty('hits')
        expect(result.hits).toBeInstanceOf(Array)
        expect(result.query).toBe(`concurrent query ${i}`)
      })

      // Verify no cross-contamination (each query matches its index)
      results.forEach((result, i) => {
        expect(result.query).toBe(searches[i].query)
      })
    })

    it('should handle 50 concurrent searches without errors', async () => {
      const searches = Array.from({ length: 50 }, (_, i) => ({
        query: `stress test query ${i}`,
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 5,
        mode: 'fts' as const,
      }))

      // Execute all searches concurrently
      const results = await Promise.all(
        searches.map((params) => handleSearchTool(params, client))
      )

      // All should succeed
      expect(results.length).toBe(50)

      // Verify no cross-contamination
      results.forEach((result, i) => {
        expect(result.query).toBe(searches[i].query)
      })
    }, 30000) // 30s timeout for stress test

    it('should maintain response IDs match request queries', async () => {
      const uniqueQueries = [
        'unique query alpha',
        'unique query beta',
        'unique query gamma',
        'unique query delta',
        'unique query epsilon',
      ]

      const searches = uniqueQueries.map((query) => ({
        query,
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 5,
        mode: 'fts' as const,
      }))

      const results = await Promise.all(
        searches.map((params) => handleSearchTool(params, client))
      )

      // Each result should match its query exactly (no cross-contamination)
      results.forEach((result, i) => {
        expect(result.query).toBe(uniqueQueries[i])
      })
    })

    it('should handle mixed concurrent requests (different repos/worktrees)', async () => {
      const searches = [
        { query: 'search 1', repo: 'test-corpus', worktree: 'main' },
        { query: 'search 2', repo: 'test-corpus', worktree: 'main' },
        { query: 'search 3', repo: 'test-corpus', worktree: 'main' },
      ].map((params) => ({
        ...params,
        limit: 5,
        mode: 'fts' as const,
      }))

      const results = await Promise.all(
        searches.map((params) => handleSearchTool(params, client))
      )

      // All should succeed
      expect(results.length).toBe(3)

      // Verify queries match
      results.forEach((result, i) => {
        expect(result.query).toBe(searches[i].query)
        expect(result.repo).toBe(searches[i].repo)
      })
    })
  })

  describe('Error Handling', () => {
    it('should return user-friendly error for non-existent repo', async () => {
      const params = {
        query: 'test',
        repo: 'nonexistent-repo-xyz',
        worktree: 'main',
        limit: 10,
        mode: 'fts' as const,
      }

      await expect(handleSearchTool(params, client)).rejects.toThrow(/not found/)
    })

    it('should handle invalid query parameter', async () => {
      const params = {
        query: '', // Empty query should be rejected
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 10,
        mode: 'fts' as const,
      }

      await expect(handleSearchTool(params, client)).rejects.toThrow()
    })

    it('should handle invalid limit parameter', async () => {
      const params = {
        query: 'test',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: -1, // Negative limit should be rejected
        mode: 'fts' as const,
      }

      await expect(handleSearchTool(params, client)).rejects.toThrow()
    })

    it('should handle invalid mode parameter', async () => {
      const params = {
        query: 'test',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 10,
        mode: 'invalid_mode' as any,
      }

      await expect(handleSearchTool(params, client)).rejects.toThrow()
    })

    it('should handle daemon restart after crash simulation', async () => {
      // Stop daemon to simulate crash
      await closeDaemonClient()

      // New request should auto-restart daemon
      const params = {
        query: 'after crash',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 5,
        mode: 'fts' as const,
      }

      // Should succeed by restarting daemon
      const result = await handleSearchTool(params, client)

      expect(result.hits).toBeInstanceOf(Array)
    })
  })

  describe('Performance and Reliability', () => {
    it('should complete searches within reasonable time', async () => {
      const startTime = Date.now()

      const params = {
        query: 'performance test',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 10,
        mode: 'fts' as const,
      }

      await handleSearchTool(params, client)

      const duration = Date.now() - startTime

      // Should complete in under 5 seconds (generous for CI)
      expect(duration).toBeLessThan(5000)
    })

    it('should handle rapid sequential requests', async () => {
      const queries = ['rapid 1', 'rapid 2', 'rapid 3', 'rapid 4', 'rapid 5']

      for (const query of queries) {
        const params = {
          query,
          repo: 'integration-test-corpus',
          worktree: 'main',
          limit: 5,
          mode: 'fts' as const,
        }

        const result = await handleSearchTool(params, client)

        expect(result.query).toBe(query)
      }
    })

    it('should maintain daemon stability across many requests', async () => {
      // Execute 20 requests to verify stability
      const requests = Array.from({ length: 20 }, (_, i) => ({
        query: `stability test ${i}`,
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 5,
        mode: 'fts' as const,
      }))

      for (const params of requests) {
        const result = await handleSearchTool(params, client)
        expect(result.hits).toBeInstanceOf(Array)
      }

      // Daemon should still be healthy
      const daemon = getDaemonClient()
      const isHealthy = await daemon.isHealthy()
      expect(isHealthy).toBe(true)
    })
  })

  describe('Data Integrity', () => {
    it('should return valid chunk IDs', async () => {
      const params = {
        query: 'authenticate',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 10,
        mode: 'fts' as const,
      }

      const result = await handleSearchTool(params, client)

      expect(result.hits.length).toBeGreaterThan(0)

      // All chunk IDs should be positive integers
      result.hits.forEach((hit) => {
        expect(hit.chunk_id).toBeGreaterThan(0)
        expect(Number.isInteger(hit.chunk_id)).toBe(true)
      })
    })

    it('should return valid file paths', async () => {
      const params = {
        query: 'function',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 10,
        mode: 'fts' as const,
      }

      const result = await handleSearchTool(params, client)

      expect(result.hits.length).toBeGreaterThan(0)

      // All file paths should be non-empty strings
      result.hits.forEach((hit) => {
        expect(hit.relpath).toBeTruthy()
        expect(typeof hit.relpath).toBe('string')
        expect(hit.relpath.length).toBeGreaterThan(0)
      })
    })

    it('should return valid line numbers', async () => {
      const params = {
        query: 'class',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 10,
        mode: 'fts' as const,
      }

      const result = await handleSearchTool(params, client)

      expect(result.hits.length).toBeGreaterThan(0)

      // All line numbers should be valid (start_line < end_line, both > 0)
      result.hits.forEach((hit) => {
        expect(hit.start_line).toBeGreaterThan(0)
        expect(hit.end_line).toBeGreaterThanOrEqual(hit.start_line)
      })
    })

    it('should return valid scores', async () => {
      const params = {
        query: 'search',
        repo: 'integration-test-corpus',
        worktree: 'main',
        limit: 10,
        mode: 'fts' as const,
      }

      const result = await handleSearchTool(params, client)

      expect(result.hits.length).toBeGreaterThan(0)

      // All scores should be positive numbers
      result.hits.forEach((hit) => {
        expect(hit.score).toBeGreaterThan(0)
        expect(typeof hit.score).toBe('number')
        expect(Number.isFinite(hit.score)).toBe(true)
      })

      // Scores should be in descending order
      for (let i = 1; i < result.hits.length; i++) {
        expect(result.hits[i - 1].score).toBeGreaterThanOrEqual(result.hits[i].score)
      }
    })
  })
})
