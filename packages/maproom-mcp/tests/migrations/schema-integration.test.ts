/**
 * Integration tests for SCHMAFIX migrations 0018-0020
 *
 * These tests verify that the MCP server works correctly with the new database schema
 * after integrating migrations 0018 (blob_sha), 0019 (code_embeddings), and 0020 (BRANCHX).
 *
 * Background: The original problem was that MCP TypeScript code (line 511 of src/index.ts)
 * references a `code_embeddings` table that didn't exist, causing vector search to crash
 * with "relation does not exist" error. After integrating these migrations, the schema
 * should be complete and MCP queries should work without errors.
 *
 * Related: SCHMAFIX-4001, SCHMAFIX-1001, SCHMAFIX-2001, SCHMAFIX-3901
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import pg from 'pg'
const { Client } = pg

// Test database client
let testClient: Client | null = null

beforeAll(async () => {
  const connectionString = process.env.MAPROOM_DATABASE_URL || 'postgresql://maproom:maproom@localhost:5432/maproom'

  try {
    testClient = new Client({ connectionString })
    await testClient.connect()
    console.log('✓ Connected to test database')
  } catch (err) {
    console.warn('⚠ Unable to connect to database, skipping integration tests')
    console.warn(`  Connection string: ${connectionString}`)
    console.warn(`  Error: ${err instanceof Error ? err.message : String(err)}`)
    testClient = null
  }
})

afterAll(async () => {
  if (testClient) {
    await testClient.end()
    console.log('✓ Disconnected from test database')
  }
})

describe('Migration 0019 - code_embeddings Table', () => {
  it('code_embeddings table exists and is queryable', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return }

    // Verify table exists in information_schema
    const result = await testClient.query(`
      SELECT table_name FROM information_schema.tables
      WHERE table_schema = 'maproom' AND table_name = 'code_embeddings'
    `)

    expect(result.rows).toHaveLength(1)
    expect(result.rows[0].table_name).toBe('code_embeddings')
  })

  it('vector search query executes without crashing', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return }

    // This is the exact query from src/index.ts line 511 that was failing before migration 0019
    // The query should succeed (not crash) even if the table is empty
    const result = await testClient.query(
      'SELECT COUNT(*) as count FROM maproom.code_embeddings LIMIT 1'
    )

    expect(result.rows).toHaveLength(1)
    expect(result.rows[0].count).toBeDefined()
    // Count might be '0' (empty table) - that's fine, we just care it doesn't crash
  })

  it('code_embeddings table has correct schema structure', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return }

    // Verify the table has the expected columns
    const result = await testClient.query(`
      SELECT
        column_name,
        data_type,
        is_nullable
      FROM information_schema.columns
      WHERE table_schema = 'maproom'
        AND table_name = 'code_embeddings'
      ORDER BY ordinal_position
    `)

    expect(result.rows.length).toBeGreaterThan(0)

    const columns = result.rows.reduce((acc, row) => {
      acc[row.column_name] = row
      return acc
    }, {} as Record<string, any>)

    // Should have key columns from migration 0019:
    // - blob_sha (primary key)
    // - embedding (vector data)
    // - model_version (embedding model identifier)
    // - created_at (timestamp)
    expect(columns.blob_sha).toBeDefined()
    expect(columns.embedding).toBeDefined()
    expect(columns.model_version).toBeDefined()
    expect(columns.created_at).toBeDefined()
  })
})

describe('Migration 0018 - blob_sha Column Integration', () => {
  it('chunks table has blob_sha column', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return }

    const result = await testClient.query(`
      SELECT column_name, data_type FROM information_schema.columns
      WHERE table_schema = 'maproom' AND table_name = 'chunks' AND column_name = 'blob_sha'
    `)

    expect(result.rows).toHaveLength(1)
    expect(result.rows[0].data_type).toBe('text')
  })

  it('blob_sha column is accessible in queries', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return }

    // Verify we can query the blob_sha column without errors
    const result = await testClient.query(`
      SELECT blob_sha FROM maproom.chunks LIMIT 1
    `)

    // Query should succeed (even if no rows are returned)
    expect(result).toBeDefined()
  })

  it('blob_sha index exists for performance', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return }

    const result = await testClient.query(`
      SELECT indexname FROM pg_indexes
      WHERE schemaname = 'maproom'
        AND tablename = 'chunks'
        AND indexname = 'idx_chunks_blob_sha'
    `)

    expect(result.rows).toHaveLength(1)
  })
})

describe('Migration 0020 - BRANCHX Schema Integration', () => {
  it('chunks table has worktree_ids JSONB column', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return }

    const result = await testClient.query(`
      SELECT column_name, data_type FROM information_schema.columns
      WHERE table_schema = 'maproom' AND table_name = 'chunks' AND column_name = 'worktree_ids'
    `)

    expect(result.rows).toHaveLength(1)
    expect(result.rows[0].data_type).toBe('jsonb')
  })

  it('worktree_index_state table exists for BRANCHX tracking', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return }

    const result = await testClient.query(`
      SELECT table_name FROM information_schema.tables
      WHERE table_schema = 'maproom' AND table_name = 'worktree_index_state'
    `)

    expect(result.rows).toHaveLength(1)
  })

  it('worktree_ids GIN index exists', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return }

    const result = await testClient.query(`
      SELECT indexname FROM pg_indexes
      WHERE schemaname = 'maproom'
        AND tablename = 'chunks'
        AND indexname = 'idx_chunks_worktree_ids'
    `)

    expect(result.rows).toHaveLength(1)
  })

  it('worktree_index_state has required tracking columns', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return }

    const result = await testClient.query(`
      SELECT column_name, data_type
      FROM information_schema.columns
      WHERE table_schema = 'maproom'
        AND table_name = 'worktree_index_state'
      ORDER BY ordinal_position
    `)

    expect(result.rows.length).toBeGreaterThan(0)

    const columns = result.rows.reduce((acc, row) => {
      acc[row.column_name] = row
      return acc
    }, {} as Record<string, any>)

    // Verify critical columns exist for worktree tracking
    expect(columns.worktree_id).toBeDefined()
    expect(columns.last_tree_sha).toBeDefined()
    expect(columns.last_indexed).toBeDefined()
    expect(columns.chunks_processed).toBeDefined()
    expect(columns.embeddings_generated).toBeDefined()
  })
})

describe('Schema Integration - End-to-End Validation', () => {
  it('all three migration schemas work together', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return }

    // Query that touches all three migrations:
    // - blob_sha (migration 0018)
    // - worktree_ids (migration 0020)
    // - code_embeddings referenced but not joined (migration 0019)
    const result = await testClient.query(`
      SELECT
        c.blob_sha,
        c.worktree_ids,
        (SELECT COUNT(*) FROM maproom.code_embeddings) as embedding_count
      FROM maproom.chunks c
      LIMIT 1
    `)

    // Query should succeed without errors
    expect(result).toBeDefined()
  })

  it('database ready for MCP vector search operations', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return }

    // Verify all schema elements needed for MCP operations exist
    const tables = await testClient.query(`
      SELECT table_name FROM information_schema.tables
      WHERE table_schema = 'maproom'
        AND table_name IN ('chunks', 'code_embeddings', 'worktree_index_state')
      ORDER BY table_name
    `)

    expect(tables.rows).toHaveLength(3)
    expect(tables.rows[0].table_name).toBe('chunks')
    expect(tables.rows[1].table_name).toBe('code_embeddings')
    expect(tables.rows[2].table_name).toBe('worktree_index_state')
  })

  it('displays schema integration statistics', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return }

    // Get statistics about the integrated schema
    const chunksCount = await testClient.query('SELECT COUNT(*) as count FROM maproom.chunks')
    const embeddingsCount = await testClient.query('SELECT COUNT(*) as count FROM maproom.code_embeddings')
    const worktreeStateCount = await testClient.query('SELECT COUNT(*) as count FROM maproom.worktree_index_state')

    // Get chunks with blob_sha
    const blobShaCount = await testClient.query(`
      SELECT COUNT(*) as count FROM maproom.chunks WHERE blob_sha IS NOT NULL
    `)

    // Get chunks with worktree_ids
    const worktreeIdsCount = await testClient.query(`
      SELECT COUNT(*) as count FROM maproom.chunks
      WHERE jsonb_array_length(worktree_ids) > 0
    `)

    console.log('\n=== SCHMAFIX Schema Integration Statistics ===')
    console.log(`Total chunks:                     ${chunksCount.rows[0].count}`)
    console.log(`Code embeddings:                  ${embeddingsCount.rows[0].count}`)
    console.log(`Worktree index states:            ${worktreeStateCount.rows[0].count}`)
    console.log(`Chunks with blob_sha:             ${blobShaCount.rows[0].count}`)
    console.log(`Chunks with worktree_ids:         ${worktreeIdsCount.rows[0].count}`)
    console.log('===============================================\n')

    // Basic sanity checks - counts should be >= 0
    expect(parseInt(chunksCount.rows[0].count)).toBeGreaterThanOrEqual(0)
    expect(parseInt(embeddingsCount.rows[0].count)).toBeGreaterThanOrEqual(0)
    expect(parseInt(worktreeStateCount.rows[0].count)).toBeGreaterThanOrEqual(0)
  })
})
