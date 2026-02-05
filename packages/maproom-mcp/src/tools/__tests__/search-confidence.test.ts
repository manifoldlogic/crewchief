/**
 * Integration test for search confidence signals (MRIMP-4.2001)
 *
 * Tests verify that confidence signals flow correctly through the complete
 * stack (daemon -> daemon-client -> MCP) when include_confidence is set.
 *
 * Test coverage:
 * 1. Returns confidence when include_confidence=true
 * 2. Omits confidence when include_confidence=false
 * 3. Omits confidence when include_confidence not provided (backward compat)
 * 4. Confidence fields match TypeScript types (number, number, boolean)
 * 5. ConfidenceSignals type can be imported and used
 *
 * Tests skip gracefully if database unavailable with warning message.
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { getDaemonClient } from '../../daemon.js'
import type { DaemonClient } from '../../daemon-client/index.js'
import type { ConfidenceSignals } from '../../daemon-client/types.js'
import { existsSync } from 'node:fs'
import { homedir } from 'node:os'

describe('Search Confidence Integration Tests', () => {
  let client: DaemonClient | null = null
  let dbAvailable = false

  beforeAll(async () => {
    // Suppress Rust daemon debug logs to prevent JSON-RPC parsing errors
    process.env.RUST_LOG = 'error'

    // Check if database exists
    const dbPath = process.env.MAPROOM_DATABASE_URL?.replace('sqlite://', '') ||
                   `${homedir()}/.maproom/maproom.db`

    if (!existsSync(dbPath)) {
      console.warn(`Warning: Test database not found at ${dbPath} - skipping integration tests`)
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
        console.log(`Database connected, ${testResult.total} chunks indexed`)
      } else {
        console.warn('Warning: Database returned no results - skipping integration tests')
      }
    } catch (error) {
      console.warn(`Warning: Failed to connect to database: ${error instanceof Error ? error.message : String(error)}`)
      console.warn('   Skipping integration tests')
      dbAvailable = false
    }
  })

  afterAll(async () => {
    // Daemon cleanup is handled by singleton lifecycle
    // No explicit cleanup needed
  })

  it('should return confidence when include_confidence is true', async () => {
    if (!dbAvailable || !client) {
      console.log('Skipping: database not available')
      return
    }

    const result = await client.search({
      query: 'function',
      repo: 'crewchief',
      limit: 10,
      include_confidence: true,
    })

    expect(result).toBeDefined()
    expect(result.hits).toBeDefined()
    expect(Array.isArray(result.hits)).toBe(true)
    expect(result.hits.length).toBeGreaterThan(0)

    // Every hit should have confidence when include_confidence=true
    for (const hit of result.hits) {
      expect(hit.confidence).toBeDefined()
      expect(hit.confidence).toHaveProperty('source_count')
      expect(hit.confidence).toHaveProperty('score_gap')
      expect(hit.confidence).toHaveProperty('is_exact_match')
    }
  })

  it('should omit confidence when include_confidence is false', async () => {
    if (!dbAvailable || !client) {
      console.log('Skipping: database not available')
      return
    }

    const result = await client.search({
      query: 'function',
      repo: 'crewchief',
      limit: 10,
      include_confidence: false,
    })

    expect(result).toBeDefined()
    expect(result.hits).toBeDefined()
    expect(result.hits.length).toBeGreaterThan(0)

    // No hit should have confidence when include_confidence=false
    for (const hit of result.hits) {
      expect(hit.confidence).toBeUndefined()
    }
  })

  it('should omit confidence when include_confidence not provided (backward compat)', async () => {
    if (!dbAvailable || !client) {
      console.log('Skipping: database not available')
      return
    }

    const result = await client.search({
      query: 'function',
      repo: 'crewchief',
      limit: 10,
      // include_confidence intentionally omitted
    })

    expect(result).toBeDefined()
    expect(result.hits).toBeDefined()
    expect(result.hits.length).toBeGreaterThan(0)

    // No hit should have confidence when include_confidence is omitted
    for (const hit of result.hits) {
      expect(hit.confidence).toBeUndefined()
    }
  })

  it('should return confidence fields matching TypeScript types', async () => {
    if (!dbAvailable || !client) {
      console.log('Skipping: database not available')
      return
    }

    const result = await client.search({
      query: 'function',
      repo: 'crewchief',
      limit: 10,
      include_confidence: true,
    })

    expect(result).toBeDefined()
    expect(result.hits).toBeDefined()
    expect(result.hits.length).toBeGreaterThan(0)

    for (const hit of result.hits) {
      expect(hit.confidence).toBeDefined()

      const confidence = hit.confidence!

      // source_count must be a number (integer >= 1)
      expect(typeof confidence.source_count).toBe('number')
      expect(confidence.source_count).toBeGreaterThanOrEqual(1)
      expect(Number.isInteger(confidence.source_count)).toBe(true)

      // score_gap must be a non-negative number
      expect(typeof confidence.score_gap).toBe('number')
      expect(confidence.score_gap).toBeGreaterThanOrEqual(0)

      // is_exact_match must be a boolean
      expect(typeof confidence.is_exact_match).toBe('boolean')
    }
  })

  it('should allow ConfidenceSignals type to be imported and used', () => {
    // This test validates that the ConfidenceSignals type is importable
    // and structurally correct at compile time. If the import at the top
    // of this file fails, this test file won't even load.

    // Runtime validation: create a value conforming to ConfidenceSignals
    const signals: ConfidenceSignals = {
      source_count: 2,
      score_gap: 0.15,
      is_exact_match: false,
    }

    expect(signals.source_count).toBe(2)
    expect(signals.score_gap).toBe(0.15)
    expect(signals.is_exact_match).toBe(false)

    // Verify all expected keys are present
    const keys = Object.keys(signals).sort()
    expect(keys).toEqual(['is_exact_match', 'score_gap', 'source_count'])
  })
})
