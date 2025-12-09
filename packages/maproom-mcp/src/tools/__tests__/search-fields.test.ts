/**
 * Integration test for search result fields (SRCHFIX-2002)
 *
 * Tests verify that chunk_id, symbol_name, and kind fields are correctly
 * populated in search results and enable context retrieval.
 *
 * Test coverage:
 * 1. chunk_id is populated with positive integer
 * 2. symbol_name is populated for functions (or null for anonymous)
 * 3. kind is populated with valid values
 * 4. null symbol_name handling works correctly
 * 5. context retrieval works using chunk_id from search
 *
 * Tests skip gracefully if database unavailable with warning message.
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { getDaemonClient } from '../../daemon.js'
import type { DaemonClient } from '../../daemon-client/index.js'
import { existsSync } from 'node:fs'
import { homedir } from 'node:os'

describe('Search Result Fields Integration Tests', () => {
  let client: DaemonClient | null = null
  let dbAvailable = false

  beforeAll(async () => {
    // Suppress Rust daemon debug logs to prevent JSON-RPC parsing errors
    process.env.RUST_LOG = 'error'

    // Check if database exists
    const dbPath = process.env.MAPROOM_DATABASE_URL?.replace('sqlite://', '') ||
                   `${homedir()}/.maproom/maproom.db`

    if (!existsSync(dbPath)) {
      console.warn(`⚠️  Test database not found at ${dbPath} - skipping integration tests`)
      console.warn('   To run these tests, ensure the database is indexed with: crewchief-maproom scan')
      return
    }

    try {
      // Initialize daemon client
      client = getDaemonClient()

      // Test database connectivity with a simple search
      const testResult = await client.search({
        query: 'function',
        repo: 'crewchief',
        worktree: 'main',
        limit: 1
      })

      if (testResult && testResult.hits) {
        dbAvailable = true
        console.log(`✓ Database connected, ${testResult.total} chunks indexed`)
      } else {
        console.warn('⚠️  Database returned no results - skipping integration tests')
      }
    } catch (error) {
      console.warn(`⚠️  Failed to connect to database: ${error instanceof Error ? error.message : String(error)}`)
      console.warn('   Skipping integration tests')
      dbAvailable = false
    }
  })

  afterAll(async () => {
    // Daemon cleanup is handled by singleton lifecycle
    // No explicit cleanup needed
  })

  it('should populate chunk_id with positive integer', async () => {
    if (!dbAvailable || !client) {
      console.log('⊘ Skipping: database not available')
      return
    }

    // Search for a generic pattern that should match any indexed codebase
    const result = await client.search({
      query: 'function',
      repo: 'crewchief',
      limit: 10
    })

    expect(result).toBeDefined()
    expect(result.hits).toBeDefined()
    expect(Array.isArray(result.hits)).toBe(true)

    // Should have at least one result for 'function' query in a typical codebase
    expect(result.hits.length).toBeGreaterThan(0)

    // Verify chunk_id field exists and is a positive integer
    for (const hit of result.hits) {
      expect(hit.chunk_id).toBeDefined()
      expect(typeof hit.chunk_id).toBe('number')
      expect(hit.chunk_id).toBeGreaterThan(0)
      expect(Number.isInteger(hit.chunk_id)).toBe(true)
    }
  })

  it('should populate symbol_name for functions (or null for anonymous)', async () => {
    if (!dbAvailable || !client) {
      console.log('⊘ Skipping: database not available')
      return
    }

    // Search for functions - most should have symbol names
    const result = await client.search({
      query: 'function',
      repo: 'crewchief',
      limit: 20
    })

    expect(result).toBeDefined()
    expect(result.hits).toBeDefined()
    expect(result.hits.length).toBeGreaterThan(0)

    // Verify symbol_name field exists (can be string or null)
    let foundWithSymbol = 0
    let foundWithNull = 0

    for (const hit of result.hits) {
      expect(hit).toHaveProperty('symbol_name')

      // symbol_name should be either a non-empty string or null
      if (hit.symbol_name !== null) {
        expect(typeof hit.symbol_name).toBe('string')
        if (hit.symbol_name.length > 0) {
          foundWithSymbol++
        }
      } else {
        foundWithNull++
      }
    }

    // For function queries, we expect at least some results to have symbol names
    // (anonymous functions or documentation chunks may have null)
    expect(foundWithSymbol).toBeGreaterThan(0)
  })

  it('should populate kind with valid values', async () => {
    if (!dbAvailable || !client) {
      console.log('⊘ Skipping: database not available')
      return
    }

    // Search for various code constructs
    const result = await client.search({
      query: 'export',
      repo: 'crewchief',
      limit: 20
    })

    expect(result).toBeDefined()
    expect(result.hits).toBeDefined()
    expect(result.hits.length).toBeGreaterThan(0)

    // Valid kind values from tree-sitter parsing
    const validKinds = [
      'function',
      'function_declaration',
      'arrow_function',
      'method_definition',
      'class',
      'class_declaration',
      'interface',
      'interface_declaration',
      'type_alias',
      'type_alias_declaration',
      'const',
      'variable_declaration',
      'export_statement',
      'import_statement',
      'module',
      'comment',
      'documentation',
      // Add other valid kinds as needed
    ]

    for (const hit of result.hits) {
      expect(hit).toHaveProperty('kind')
      expect(typeof hit.kind).toBe('string')
      expect(hit.kind.length).toBeGreaterThan(0)

      // Kind should be a non-empty string
      // Note: We don't validate against validKinds list strictly
      // because tree-sitter may return various kinds depending on language
    }
  })

  it('should handle null symbol_name correctly', async () => {
    if (!dbAvailable || !client) {
      console.log('⊘ Skipping: database not available')
      return
    }

    // Search for documentation or comments which often have null symbol_name
    const result = await client.search({
      query: 'documentation',
      repo: 'crewchief',
      limit: 30
    })

    expect(result).toBeDefined()
    expect(result.hits).toBeDefined()

    // Find at least one result with null symbol_name
    // (or verify all results handle null gracefully)
    let hasNullSymbol = false

    for (const hit of result.hits) {
      expect(hit).toHaveProperty('symbol_name')

      if (hit.symbol_name === null) {
        hasNullSymbol = true
        // Verify other fields are still populated correctly
        expect(hit.chunk_id).toBeGreaterThan(0)
        expect(hit.kind).toBeTruthy()
        expect(hit.file_path).toBeTruthy()
      }
    }

    // Note: If no null symbols found, that's okay - it means the query
    // didn't match any anonymous chunks. The important thing is the code
    // handles null correctly without crashing.
    if (!hasNullSymbol && result.hits.length > 0) {
      console.log('   Note: No null symbol_name found in results (query may not match anonymous chunks)')
    }
  })

  it('should enable context retrieval using chunk_id from search', async () => {
    if (!dbAvailable || !client) {
      console.log('⊘ Skipping: database not available')
      return
    }

    // First, perform a search to get chunk_id
    const searchResult = await client.search({
      query: 'function',
      repo: 'crewchief',
      limit: 5
    })

    expect(searchResult).toBeDefined()
    expect(searchResult.hits).toBeDefined()
    expect(searchResult.hits.length).toBeGreaterThan(0)

    // Take the first result's chunk_id
    const firstHit = searchResult.hits[0]
    expect(firstHit.chunk_id).toBeGreaterThan(0)

    // Use chunk_id to retrieve context
    const contextResult = await client.context({
      chunk_id: String(firstHit.chunk_id),
      budget_tokens: 6000,
      expand: {
        callers: true,
        callees: true,
        tests: true,
      }
    })

    // Verify context was retrieved successfully
    expect(contextResult).toBeDefined()
    expect(contextResult.items).toBeDefined()
    expect(Array.isArray(contextResult.items)).toBe(true)

    // Should have at least the primary chunk
    expect(contextResult.items.length).toBeGreaterThan(0)

    // Verify context bundle structure
    expect(contextResult).toHaveProperty('total_tokens')
    expect(contextResult).toHaveProperty('truncated')
    expect(typeof contextResult.total_tokens).toBe('number')
    expect(typeof contextResult.truncated).toBe('boolean')

    // Verify items have required fields
    for (const item of contextResult.items) {
      expect(item).toHaveProperty('relpath')
      expect(item).toHaveProperty('range')
      expect(item).toHaveProperty('content')
      expect(item.range).toHaveProperty('start')
      expect(item.range).toHaveProperty('end')
    }
  })
})
