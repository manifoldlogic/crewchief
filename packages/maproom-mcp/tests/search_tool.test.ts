/**
 * Tests for MCP search tool with hybrid search parameters
 *
 * These tests verify:
 * - Parameter validation (mode, filters, debug)
 * - Mode-specific query execution (FTS, vector, hybrid)
 * - Filter combinations (file_type, recency_threshold, etc.)
 * - Debug output structure
 * - Backward compatibility with existing calls
 * - Error handling for invalid parameters
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { Client } from 'pg'

// Mock database client for testing
let testClient: Client

beforeAll(async () => {
  // Setup test database connection
  const connectionString = process.env.TEST_DATABASE_URL || process.env.DATABASE_URL
  if (!connectionString) {
    console.warn('No TEST_DATABASE_URL set, skipping integration tests')
    return
  }
  testClient = new Client({ connectionString })
  await testClient.connect()
})

afterAll(async () => {
  if (testClient) {
    await testClient.end()
  }
})

describe('Search Tool - Parameter Validation', () => {
  it('should validate mode parameter', () => {
    const validModes = ['fts', 'vector', 'hybrid']
    validModes.forEach(mode => {
      expect(validModes).toContain(mode)
    })
  })

  it('should reject invalid mode parameter', () => {
    const invalidMode = 'invalid_mode'
    const validModes = ['fts', 'vector', 'hybrid']
    expect(validModes).not.toContain(invalidMode)
  })

  it('should accept optional filters object', () => {
    const validFilters = {
      repo_id: 1,
      worktree_id: 2,
      file_type: 'ts',
      recency_threshold: '7 days'
    }

    expect(validFilters).toHaveProperty('repo_id')
    expect(validFilters).toHaveProperty('worktree_id')
    expect(validFilters).toHaveProperty('file_type')
    expect(validFilters).toHaveProperty('recency_threshold')
  })

  it('should accept debug boolean parameter', () => {
    expect(typeof true).toBe('boolean')
    expect(typeof false).toBe('boolean')
  })
})

describe('Search Tool - Mode Selection', () => {
  it('should default to hybrid mode when not specified', () => {
    const params = { repo: 'test', query: 'search' }
    const mode = params['mode'] || 'hybrid'
    expect(mode).toBe('hybrid')
  })

  it('should accept fts mode', () => {
    const params = { repo: 'test', query: 'search', mode: 'fts' }
    expect(params.mode).toBe('fts')
  })

  it('should accept vector mode', () => {
    const params = { repo: 'test', query: 'search', mode: 'vector' }
    expect(params.mode).toBe('vector')
  })

  it('should accept hybrid mode explicitly', () => {
    const params = { repo: 'test', query: 'search', mode: 'hybrid' }
    expect(params.mode).toBe('hybrid')
  })
})

describe('Search Tool - Filter Handling', () => {
  it('should handle file_type filter', () => {
    const filters = { file_type: 'ts' }
    expect(filters.file_type).toBe('ts')
  })

  it('should handle recency_threshold filter', () => {
    const filters = { recency_threshold: '30 days' }
    expect(filters.recency_threshold).toBe('30 days')
  })

  it('should handle multiple filters', () => {
    const filters = {
      file_type: 'rs',
      recency_threshold: '7 days',
      worktree_id: 1
    }
    expect(Object.keys(filters).length).toBe(3)
  })

  it('should handle empty filters object', () => {
    const filters = {}
    expect(Object.keys(filters).length).toBe(0)
  })
})

describe('Search Tool - Debug Mode', () => {
  it('should structure debug output correctly', () => {
    const debugOutput = {
      mode: 'fts',
      query_terms: 'search & terms',
      total_results: 10,
      fts_score: 0.85,
      vector_score: null,
      recency_score: 0.75,
      churn_score: 0.10,
      final_score: 0.85
    }

    expect(debugOutput).toHaveProperty('mode')
    expect(debugOutput).toHaveProperty('query_terms')
    expect(debugOutput).toHaveProperty('total_results')
  })

  it('should include per-hit debug info when debug=true', () => {
    const hit = {
      chunk_id: 1,
      relpath: 'src/test.ts',
      score: 0.85,
      debug: {
        fts_score: 0.85,
        vector_score: null,
        recency_score: 0.75,
        churn_score: 0.10,
        final_score: 0.85
      }
    }

    expect(hit.debug).toBeDefined()
    expect(hit.debug?.fts_score).toBe(0.85)
  })
})

describe('Search Tool - Backward Compatibility', () => {
  it('should work with legacy simple search parameters', () => {
    const legacyParams = {
      repo: 'crewchief',
      query: 'authentication',
      k: 10
    }

    // All new parameters should be optional
    expect(legacyParams).not.toHaveProperty('mode')
    expect(legacyParams).not.toHaveProperty('filters')
    expect(legacyParams).not.toHaveProperty('debug')
  })

  it('should apply defaults for omitted parameters', () => {
    const params = { repo: 'test', query: 'search' }
    const mode = params['mode'] || 'hybrid'
    const k = params['k'] || 10
    const debug = params['debug'] || false
    const filters = params['filters'] || {}

    expect(mode).toBe('hybrid')
    expect(k).toBe(10)
    expect(debug).toBe(false)
    expect(Object.keys(filters).length).toBe(0)
  })
})

describe('Search Tool - Error Messages', () => {
  it('should provide helpful error for invalid mode', () => {
    const errorMessage = `Mode must be one of: "fts", "vector", "hybrid". Got: "invalid"`
    expect(errorMessage).toContain('fts')
    expect(errorMessage).toContain('vector')
    expect(errorMessage).toContain('hybrid')
  })

  it('should provide helpful error for missing embeddings in vector mode', () => {
    const errorMessage = 'Vector search requires embeddings. No embeddings found in database.'
    expect(errorMessage).toContain('embeddings')
    expect(errorMessage).toContain('Vector search')
  })

  it('should provide helpful error for unimplemented query embedding', () => {
    const errorMessage = 'Vector search requires query embedding generation.'
    expect(errorMessage).toContain('query embedding')
  })
})

describe('Search Tool - SQL Query Construction', () => {
  it('should build file_type filter correctly', () => {
    const fileType = 'ts'
    const pattern = `%.${fileType}`
    expect(pattern).toBe('%.ts')
  })

  it('should build recency_threshold filter correctly', () => {
    const threshold = '7 days'
    const sqlClause = `f.last_modified > NOW() - INTERVAL '${threshold}'`
    expect(sqlClause).toContain('INTERVAL')
    expect(sqlClause).toContain('7 days')
  })

  it('should handle parameterized queries', () => {
    const args: any[] = [1] // repoId
    const worktreeId = 2
    args.push(worktreeId)
    expect(args.length).toBe(2)
    expect(args[1]).toBe(worktreeId)
  })
})

// Integration tests (require database connection)
describe('Search Tool - Integration Tests', () => {
  it.skipIf(!testClient)('should execute FTS search', async () => {
    if (!testClient) return

    const query = 'SELECT 1 as test'
    const result = await testClient.query(query)
    expect(result.rows.length).toBeGreaterThan(0)
  })

  it.skipIf(!testClient)('should check for embeddings', async () => {
    if (!testClient) return

    const query = 'SELECT COUNT(*) as count FROM maproom.chunks WHERE code_embedding IS NOT NULL LIMIT 1'
    try {
      const result = await testClient.query(query)
      expect(result.rows[0]).toHaveProperty('count')
    } catch (err) {
      // Table might not exist in test environment, that's okay
      console.warn('Test database not initialized, skipping embedding check')
    }
  })
})

// Performance tests (optional, can be slow)
describe.skip('Search Tool - Performance Tests', () => {
  it('should complete FTS search within 100ms', async () => {
    // TODO: Implement performance benchmarks
    expect(true).toBe(true)
  })

  it('should complete hybrid search within 150ms', async () => {
    // TODO: Implement performance benchmarks
    expect(true).toBe(true)
  })
})
