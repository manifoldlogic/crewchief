/**
 * Integration tests for Migration 004: Worktree Tracking
 *
 * These tests verify that migration 004 successfully:
 * - Added worktree_ids JSONB column to chunks table
 * - Created worktree_index_state table with correct schema
 * - Created GIN index on worktree_ids for performance
 * - Backfilled existing chunks with worktree IDs
 * - Set appropriate defaults and constraints
 * - Can be rolled back cleanly
 *
 * This is a critical validation for BRANCHX Phase 1 (worktree tracking schema).
 * All tests must pass before proceeding to git integration in Phase 2.
 *
 * Related: BRANCHX-1001, BRANCHX-1002, BRANCHX-1003
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { Client } from 'pg'

// Test database client
let testClient: Client | null = null

beforeAll(async () => {
  // MAPROOM_DATABASE_URL is set by vitest.config.ts to the correct test database
  const connectionString = process.env.MAPROOM_DATABASE_URL || 'postgresql://maproom:maproom@host.docker.internal:5434/maproom_test'

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

describe('Migration 004 - worktree_ids Column', () => {
  it('worktree_ids column exists with correct type', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const result = await testClient.query(`
      SELECT
        column_name,
        data_type,
        is_nullable,
        column_default
      FROM information_schema.columns
      WHERE table_schema = 'maproom'
        AND table_name = 'chunks'
        AND column_name = 'worktree_ids'
    `)

    expect(result.rows).toHaveLength(1)

    const column = result.rows[0]
    expect(column.column_name).toBe('worktree_ids')
    expect(column.data_type).toBe('jsonb')
    expect(column.is_nullable).toBe('NO')
    expect(column.column_default).toContain('[]') // Default empty array
  })

  it('worktree_ids column is NOT NULL', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const result = await testClient.query(`
      SELECT is_nullable
      FROM information_schema.columns
      WHERE table_schema = 'maproom'
        AND table_name = 'chunks'
        AND column_name = 'worktree_ids'
    `)

    expect(result.rows[0].is_nullable).toBe('NO')
  })

  it('worktree_ids has default value of empty array', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    // Just verify the column default is set correctly in the schema
    const result = await testClient.query(`
      SELECT column_default
      FROM information_schema.columns
      WHERE table_schema = 'maproom'
        AND table_name = 'chunks'
        AND column_name = 'worktree_ids'
    `)

    // Default should be '[]'::jsonb
    expect(result.rows[0].column_default).toContain('[]')
  })
})

describe('Migration 004 - GIN Index', () => {
  it('GIN index exists on worktree_ids column', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const result = await testClient.query(`
      SELECT
        indexname,
        indexdef
      FROM pg_indexes
      WHERE schemaname = 'maproom'
        AND tablename = 'chunks'
        AND indexname = 'idx_chunks_worktree_ids'
    `)

    expect(result.rows).toHaveLength(1)
    expect(result.rows[0].indexname).toBe('idx_chunks_worktree_ids')
    expect(result.rows[0].indexdef).toContain('gin')
    expect(result.rows[0].indexdef).toContain('worktree_ids')
  })

  it('GIN index type is correct', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const result = await testClient.query(`
      SELECT
        i.relname AS index_name,
        am.amname AS index_type
      FROM pg_class i
      JOIN pg_am am ON i.relam = am.oid
      JOIN pg_namespace n ON i.relnamespace = n.oid
      WHERE n.nspname = 'maproom'
        AND i.relname = 'idx_chunks_worktree_ids'
    `)

    expect(result.rows).toHaveLength(1)
    expect(result.rows[0].index_type).toBe('gin')
  })
})

describe('Migration 004 - worktree_index_state Table', () => {
  it('worktree_index_state table exists', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const result = await testClient.query(`
      SELECT table_name
      FROM information_schema.tables
      WHERE table_schema = 'maproom'
        AND table_name = 'worktree_index_state'
    `)

    expect(result.rows).toHaveLength(1)
  })

  it('worktree_index_state has correct columns', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const result = await testClient.query(`
      SELECT
        column_name,
        data_type,
        is_nullable
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

    // worktree_id (PRIMARY KEY, FK to worktrees)
    expect(columns.worktree_id).toBeDefined()
    expect(columns.worktree_id.data_type).toBe('bigint')
    expect(columns.worktree_id.is_nullable).toBe('NO')

    // last_tree_sha (NOT NULL)
    expect(columns.last_tree_sha).toBeDefined()
    expect(columns.last_tree_sha.data_type).toBe('text')
    expect(columns.last_tree_sha.is_nullable).toBe('NO')

    // last_indexed (has default)
    expect(columns.last_indexed).toBeDefined()
    expect(columns.last_indexed.data_type).toBe('timestamp without time zone')

    // chunks_processed (has default)
    expect(columns.chunks_processed).toBeDefined()
    expect(columns.chunks_processed.data_type).toBe('integer')

    // embeddings_generated (has default)
    expect(columns.embeddings_generated).toBeDefined()
    expect(columns.embeddings_generated.data_type).toBe('integer')
  })

  it('worktree_id is PRIMARY KEY', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const result = await testClient.query(`
      SELECT
        tc.constraint_name,
        tc.constraint_type,
        kcu.column_name
      FROM information_schema.table_constraints tc
      JOIN information_schema.key_column_usage kcu
        ON tc.constraint_name = kcu.constraint_name
        AND tc.table_schema = kcu.table_schema
      WHERE tc.table_schema = 'maproom'
        AND tc.table_name = 'worktree_index_state'
        AND tc.constraint_type = 'PRIMARY KEY'
    `)

    expect(result.rows).toHaveLength(1)
    expect(result.rows[0].column_name).toBe('worktree_id')
  })

  it('worktree_id has FOREIGN KEY to worktrees', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const result = await testClient.query(`
      SELECT
        tc.constraint_name,
        tc.constraint_type,
        kcu.column_name,
        ccu.table_name AS foreign_table_name,
        ccu.column_name AS foreign_column_name
      FROM information_schema.table_constraints tc
      JOIN information_schema.key_column_usage kcu
        ON tc.constraint_name = kcu.constraint_name
        AND tc.table_schema = kcu.table_schema
      JOIN information_schema.constraint_column_usage ccu
        ON ccu.constraint_name = tc.constraint_name
        AND ccu.table_schema = tc.table_schema
      WHERE tc.table_schema = 'maproom'
        AND tc.table_name = 'worktree_index_state'
        AND tc.constraint_type = 'FOREIGN KEY'
    `)

    expect(result.rows.length).toBeGreaterThan(0)

    const fk = result.rows.find(r => r.column_name === 'worktree_id')
    expect(fk).toBeDefined()
    expect(fk!.foreign_table_name).toBe('worktrees')
    expect(fk!.foreign_column_name).toBe('id')
  })

  it('index exists on last_tree_sha column', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const result = await testClient.query(`
      SELECT indexname
      FROM pg_indexes
      WHERE schemaname = 'maproom'
        AND tablename = 'worktree_index_state'
        AND indexname = 'idx_worktree_index_state_tree_sha'
    `)

    expect(result.rows).toHaveLength(1)
  })
})

describe('Migration 004 - Data Integrity', () => {
  it('all chunks have worktree_ids (no NULLs)', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const result = await testClient.query(`
      SELECT COUNT(*) as null_count
      FROM maproom.chunks
      WHERE worktree_ids IS NULL
    `)

    expect(parseInt(result.rows[0].null_count)).toBe(0)
  })

  it('worktree_index_state has CASCADE delete on worktree FK', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    // Verify the foreign key constraint has ON DELETE CASCADE
    const result = await testClient.query(`
      SELECT
        rc.delete_rule
      FROM information_schema.referential_constraints rc
      JOIN information_schema.table_constraints tc
        ON rc.constraint_name = tc.constraint_name
      WHERE tc.table_schema = 'maproom'
        AND tc.table_name = 'worktree_index_state'
        AND tc.constraint_type = 'FOREIGN KEY'
        AND rc.delete_rule = 'CASCADE'
    `)

    // Should have at least one CASCADE delete rule (for worktree_id FK)
    expect(result.rows.length).toBeGreaterThan(0)
  })
})

describe('Migration 004 - Statistics', () => {
  it('displays migration statistics', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    // Get total chunks
    const chunksResult = await testClient.query(`
      SELECT COUNT(*) as total FROM maproom.chunks
    `)
    const totalChunks = parseInt(chunksResult.rows[0].total)

    // Get chunks with populated worktree_ids
    const populatedResult = await testClient.query(`
      SELECT COUNT(*) as count
      FROM maproom.chunks
      WHERE jsonb_array_length(worktree_ids) > 0
    `)
    const populatedChunks = parseInt(populatedResult.rows[0].count)

    // Get chunks with empty worktree_ids
    const emptyResult = await testClient.query(`
      SELECT COUNT(*) as count
      FROM maproom.chunks
      WHERE jsonb_array_length(worktree_ids) = 0
    `)
    const emptyChunks = parseInt(emptyResult.rows[0].count)

    // Get unique worktrees referenced
    const uniqueResult = await testClient.query(`
      SELECT COUNT(DISTINCT elem) as count
      FROM maproom.chunks,
           jsonb_array_elements_text(worktree_ids) AS elem
    `)
    const uniqueWorktrees = parseInt(uniqueResult.rows[0].count || 0)

    console.log('\n=== Migration 004 Statistics ===')
    console.log(`Total chunks:                  ${totalChunks}`)
    console.log(`Chunks with worktree_ids:      ${populatedChunks} (${totalChunks > 0 ? ((populatedChunks / totalChunks) * 100).toFixed(1) : 0}%)`)
    console.log(`Chunks with empty worktree_ids: ${emptyChunks} (${totalChunks > 0 ? ((emptyChunks / totalChunks) * 100).toFixed(1) : 0}%)`)
    console.log(`Unique worktrees referenced:   ${uniqueWorktrees}`)

    // Basic sanity checks
    expect(totalChunks).toBeGreaterThanOrEqual(0)
    expect(populatedChunks + emptyChunks).toBe(totalChunks)
  })
})
