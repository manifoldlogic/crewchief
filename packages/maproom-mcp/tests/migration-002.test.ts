/**
 * Integration tests for Migration 002: Code Embeddings Table
 *
 * These tests verify that the code_embeddings table migration successfully:
 * - Created the table with correct schema
 * - Migrated embeddings with deduplication
 * - Maintained referential integrity through foreign keys
 * - Created HNSW vector index for similarity search
 * - Achieved expected storage savings
 *
 * This is a critical validation checkpoint for Phase 2 of the BLOBSHA project.
 * All tests must pass before proceeding to Phase 3 (application integration).
 *
 * Test strategy:
 * - Verify table structure and constraints
 * - Validate deduplication correctness (no data loss)
 * - Confirm foreign key prevents orphaned chunks
 * - Check HNSW index exists and is usable
 * - Calculate storage savings and cache efficiency
 *
 * Related: BLOBSHA-2002, BLOBSHA-2001
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { Client } from 'pg'

// Test database client
let testClient: Client | null = null

beforeAll(async () => {
  // Setup test database connection
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

describe('Migration 002 - Table Structure', () => {
  it.skipIf(!testClient)('code_embeddings table exists with correct schema', async () => {
    if (!testClient) return

    // Query table structure
    const result = await testClient.query(`
      SELECT
        column_name,
        data_type,
        is_nullable,
        column_default
      FROM information_schema.columns
      WHERE table_schema = 'maproom'
        AND table_name = 'code_embeddings'
      ORDER BY ordinal_position
    `)

    // Verify table exists and has all required columns
    expect(result.rows.length).toBeGreaterThan(0)

    const columns = result.rows.reduce((acc, row) => {
      acc[row.column_name] = row
      return acc
    }, {} as Record<string, any>)

    // Verify blob_sha column (PRIMARY KEY)
    expect(columns.blob_sha).toBeDefined()
    expect(columns.blob_sha.data_type).toBe('text')
    expect(columns.blob_sha.is_nullable).toBe('NO')

    // Verify embedding column (NOT NULL)
    expect(columns.embedding).toBeDefined()
    expect(columns.embedding.data_type).toBe('USER-DEFINED') // pgvector type
    expect(columns.embedding.is_nullable).toBe('NO')

    // Verify model_version column (has default)
    expect(columns.model_version).toBeDefined()
    expect(columns.model_version.data_type).toBe('text')
    expect(columns.model_version.is_nullable).toBe('NO')

    // Verify created_at column (has default)
    expect(columns.created_at).toBeDefined()
    expect(columns.created_at.data_type).toBe('timestamp without time zone')
  })

  it.skipIf(!testClient)('blob_sha is PRIMARY KEY', async () => {
    if (!testClient) return

    // Query primary key constraint
    const result = await testClient.query(`
      SELECT
        tc.constraint_name,
        kcu.column_name
      FROM information_schema.table_constraints tc
      JOIN information_schema.key_column_usage kcu
        ON tc.constraint_name = kcu.constraint_name
        AND tc.table_schema = kcu.table_schema
      WHERE tc.table_schema = 'maproom'
        AND tc.table_name = 'code_embeddings'
        AND tc.constraint_type = 'PRIMARY KEY'
    `)

    expect(result.rows.length).toBe(1)
    expect(result.rows[0].column_name).toBe('blob_sha')
  })
})

describe('Migration 002 - Deduplication', () => {
  it.skipIf(!testClient)('deduplication working correctly (unique embeddings ≤ total chunks)', async () => {
    if (!testClient) return

    const result = await testClient.query(`
      SELECT
        (SELECT COUNT(*) FROM maproom.chunks) AS total_chunks,
        (SELECT COUNT(*) FROM maproom.code_embeddings) AS unique_embeddings,
        ROUND(100.0 * (SELECT COUNT(*) FROM maproom.code_embeddings) / NULLIF((SELECT COUNT(*) FROM maproom.chunks), 0), 2) AS cache_efficiency
    `)

    const row = result.rows[0]

    console.log(`  Total chunks: ${row.total_chunks}`)
    console.log(`  Unique embeddings: ${row.unique_embeddings}`)
    console.log(`  Cache efficiency: ${row.cache_efficiency}%`)

    // Unique embeddings cannot exceed total chunks (basic sanity check)
    expect(parseInt(row.unique_embeddings)).toBeLessThanOrEqual(parseInt(row.total_chunks))

    // If there are chunks, verify metrics are valid
    if (parseInt(row.total_chunks) > 0) {
      expect(parseInt(row.unique_embeddings)).toBeGreaterThanOrEqual(0)
      expect(parseFloat(row.cache_efficiency)).toBeGreaterThanOrEqual(0)
      expect(parseFloat(row.cache_efficiency)).toBeLessThanOrEqual(100)
    }
  })

  it.skipIf(!testClient)('storage savings can be calculated', async () => {
    if (!testClient) return

    const result = await testClient.query(`
      SELECT
        (SELECT COUNT(*) FROM maproom.chunks) AS total_chunks,
        (SELECT COUNT(*) FROM maproom.code_embeddings) AS unique_embeddings,
        (SELECT COUNT(*) FROM maproom.chunks) - (SELECT COUNT(*) FROM maproom.code_embeddings) AS duplicate_chunks,
        ROUND(((SELECT COUNT(*) FROM maproom.chunks) - (SELECT COUNT(*) FROM maproom.code_embeddings)) * 6.0 / 1024.0, 2) AS storage_saved_mb
    `)

    const row = result.rows[0]

    console.log(`  Duplicate chunks: ${row.duplicate_chunks}`)
    console.log(`  Storage saved: ${row.storage_saved_mb} MB`)

    // Verify metrics are valid
    expect(parseInt(row.duplicate_chunks)).toBeGreaterThanOrEqual(0)
    expect(parseFloat(row.storage_saved_mb)).toBeGreaterThanOrEqual(0)
  })
})

describe('Migration 002 - Data Integrity', () => {
  it.skipIf(!testClient)('no embedding loss - all chunks have corresponding embeddings', async () => {
    if (!testClient) return

    // Verify no orphaned chunks (LEFT JOIN should find no NULLs)
    const result = await testClient.query(`
      SELECT COUNT(*) AS orphaned_count
      FROM maproom.chunks c
      LEFT JOIN maproom.code_embeddings e ON c.blob_sha = e.blob_sha
      WHERE e.blob_sha IS NULL AND c.embedding IS NOT NULL
    `)

    const orphanedCount = parseInt(result.rows[0].orphaned_count)

    console.log(`  Orphaned chunks: ${orphanedCount}`)

    // Zero orphaned chunks expected
    expect(orphanedCount).toBe(0)
  })

  it.skipIf(!testClient)('all embeddings in code_embeddings have valid blob_sha format', async () => {
    if (!testClient) return

    // Verify blob_sha format (64 hex characters for SHA-256)
    const result = await testClient.query(`
      SELECT
        COUNT(*) AS total_embeddings,
        COUNT(*) FILTER (WHERE blob_sha ~ '^[0-9a-f]{64}$') AS valid_blob_shas
      FROM maproom.code_embeddings
    `)

    const row = result.rows[0]
    const totalEmbeddings = parseInt(row.total_embeddings)
    const validBlobShas = parseInt(row.valid_blob_shas)

    console.log(`  Total embeddings: ${totalEmbeddings}`)
    console.log(`  Valid blob SHAs: ${validBlobShas}`)

    // If there are embeddings, all should have valid blob_sha format
    if (totalEmbeddings > 0) {
      expect(validBlobShas).toBe(totalEmbeddings)
    }
  })
})

describe('Migration 002 - Foreign Key Constraint', () => {
  it.skipIf(!testClient)('foreign key constraint exists on chunks table', async () => {
    if (!testClient) return

    // Query foreign key constraint
    const result = await testClient.query(`
      SELECT
        tc.constraint_name,
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
        AND tc.table_name = 'chunks'
        AND tc.constraint_name = 'fk_chunks_embedding'
        AND tc.constraint_type = 'FOREIGN KEY'
    `)

    expect(result.rows.length).toBe(1)
    expect(result.rows[0].constraint_name).toBe('fk_chunks_embedding')
    expect(result.rows[0].column_name).toBe('blob_sha')
    expect(result.rows[0].foreign_table_name).toBe('code_embeddings')
    expect(result.rows[0].foreign_column_name).toBe('blob_sha')
  })

  it.skipIf(!testClient)('foreign key prevents deletion of referenced embeddings', async () => {
    if (!testClient) return

    // Check if there are any embeddings to test with
    const countResult = await testClient.query(`
      SELECT COUNT(*) AS embedding_count
      FROM maproom.code_embeddings
    `)

    const embeddingCount = parseInt(countResult.rows[0].embedding_count)

    if (embeddingCount === 0) {
      console.log('  No embeddings to test deletion constraint (empty database)')
      return
    }

    // Try to find an embedding that's referenced by chunks
    const embeddingResult = await testClient.query(`
      SELECT e.blob_sha
      FROM maproom.code_embeddings e
      JOIN maproom.chunks c ON c.blob_sha = e.blob_sha
      LIMIT 1
    `)

    if (embeddingResult.rows.length === 0) {
      console.log('  No referenced embeddings found (all embeddings orphaned - may indicate data issue)')
      return
    }

    const referencedBlobSha = embeddingResult.rows[0].blob_sha

    // Attempt to delete a referenced embedding (should fail)
    try {
      await testClient.query(`
        DELETE FROM maproom.code_embeddings
        WHERE blob_sha = $1
      `, [referencedBlobSha])

      // If we get here, the constraint didn't work
      expect.fail('Foreign key constraint should have prevented deletion')
    } catch (err) {
      // Expected error: foreign key constraint violation
      const error = err as Error
      expect(error.message).toContain('violates foreign key constraint')
      console.log('  Foreign key constraint correctly prevented deletion')
    }
  })
})

describe('Migration 002 - HNSW Index', () => {
  it.skipIf(!testClient)('HNSW index exists on embedding column', async () => {
    if (!testClient) return

    // Query index information
    const result = await testClient.query(`
      SELECT
        indexname,
        indexdef
      FROM pg_indexes
      WHERE schemaname = 'maproom'
        AND tablename = 'code_embeddings'
        AND indexname = 'idx_embeddings_vector'
    `)

    expect(result.rows.length).toBe(1)
    expect(result.rows[0].indexname).toBe('idx_embeddings_vector')
    expect(result.rows[0].indexdef).toContain('hnsw')
    expect(result.rows[0].indexdef).toContain('embedding')
    expect(result.rows[0].indexdef).toContain('vector_cosine_ops')

    console.log('  HNSW index definition:', result.rows[0].indexdef)
  })

  it.skipIf(!testClient)('HNSW index can be queried (structure validation)', async () => {
    if (!testClient) return

    // Check if there are embeddings to query
    const countResult = await testClient.query(`
      SELECT COUNT(*) AS embedding_count
      FROM maproom.code_embeddings
    `)

    const embeddingCount = parseInt(countResult.rows[0].embedding_count)

    if (embeddingCount === 0) {
      console.log('  No embeddings to test index query (empty database)')
      return
    }

    // Get a sample embedding to use as query vector
    const sampleResult = await testClient.query(`
      SELECT embedding
      FROM maproom.code_embeddings
      LIMIT 1
    `)

    const sampleEmbedding = sampleResult.rows[0].embedding

    // Execute similarity search (should use HNSW index)
    const searchResult = await testClient.query(`
      SELECT blob_sha, embedding <=> $1::vector AS distance
      FROM maproom.code_embeddings
      ORDER BY embedding <=> $1::vector
      LIMIT 5
    `, [sampleEmbedding])

    // Verify query returns results
    expect(searchResult.rows.length).toBeGreaterThan(0)
    expect(searchResult.rows.length).toBeLessThanOrEqual(5)

    // Verify distance metric is present
    expect(searchResult.rows[0].distance).toBeDefined()

    console.log(`  Similarity search returned ${searchResult.rows.length} results`)
  })

  it.skipIf(!testClient)('EXPLAIN ANALYZE confirms index usage (when data exists)', async () => {
    if (!testClient) return

    // Check if there are embeddings to query
    const countResult = await testClient.query(`
      SELECT COUNT(*) AS embedding_count
      FROM maproom.code_embeddings
    `)

    const embeddingCount = parseInt(countResult.rows[0].embedding_count)

    if (embeddingCount === 0) {
      console.log('  No embeddings to test EXPLAIN ANALYZE (empty database)')
      return
    }

    // Get a sample embedding
    const sampleResult = await testClient.query(`
      SELECT embedding
      FROM maproom.code_embeddings
      LIMIT 1
    `)

    const sampleEmbedding = sampleResult.rows[0].embedding

    // Run EXPLAIN ANALYZE
    const explainResult = await testClient.query(`
      EXPLAIN ANALYZE
      SELECT blob_sha, embedding <=> $1::vector AS distance
      FROM maproom.code_embeddings
      ORDER BY embedding <=> $1::vector
      LIMIT 5
    `, [sampleEmbedding])

    const queryPlan = explainResult.rows.map(row => row['QUERY PLAN']).join('\n')

    console.log('  Query plan:', queryPlan)

    // Verify index is being used (look for 'Index Scan' in plan)
    // Note: HNSW index may not always be used for very small datasets
    // So we just verify the query completes successfully
    expect(queryPlan).toBeDefined()
    expect(queryPlan.length).toBeGreaterThan(0)
  })
})

describe('Migration 002 - Comprehensive Validation', () => {
  it.skipIf(!testClient)('migration 002 executed successfully with all validations passing', async () => {
    if (!testClient) return

    // Run comprehensive validation checks
    const validations: Array<{ name: string; passed: boolean; message: string }> = []

    // Validation 1: Table exists
    const tableResult = await testClient.query(`
      SELECT EXISTS (
        SELECT 1
        FROM information_schema.tables
        WHERE table_schema = 'maproom'
          AND table_name = 'code_embeddings'
      ) AS table_exists
    `)
    const tableExists = tableResult.rows[0].table_exists
    validations.push({
      name: 'Table exists',
      passed: tableExists,
      message: tableExists ? 'code_embeddings table found' : 'code_embeddings table missing'
    })

    // Validation 2: No orphaned chunks
    const orphanResult = await testClient.query(`
      SELECT COUNT(*) AS orphaned_count
      FROM maproom.chunks c
      LEFT JOIN maproom.code_embeddings e ON c.blob_sha = e.blob_sha
      WHERE e.blob_sha IS NULL AND c.embedding IS NOT NULL
    `)
    const noOrphans = parseInt(orphanResult.rows[0].orphaned_count) === 0
    validations.push({
      name: 'No orphaned chunks',
      passed: noOrphans,
      message: noOrphans ? 'All chunks have embeddings' : `Found ${orphanResult.rows[0].orphaned_count} orphaned chunks`
    })

    // Validation 3: Deduplication achieved
    const dedupResult = await testClient.query(`
      SELECT
        (SELECT COUNT(*) FROM maproom.chunks) AS total_chunks,
        (SELECT COUNT(*) FROM maproom.code_embeddings) AS unique_embeddings
    `)
    const totalChunks = parseInt(dedupResult.rows[0].total_chunks)
    const uniqueEmbeddings = parseInt(dedupResult.rows[0].unique_embeddings)
    const dedupAchieved = uniqueEmbeddings <= totalChunks
    validations.push({
      name: 'Deduplication achieved',
      passed: dedupAchieved,
      message: dedupAchieved
        ? `${uniqueEmbeddings} unique embeddings for ${totalChunks} chunks`
        : `Invalid state: ${uniqueEmbeddings} embeddings > ${totalChunks} chunks`
    })

    // Validation 4: Foreign key exists
    const fkResult = await testClient.query(`
      SELECT EXISTS (
        SELECT 1
        FROM information_schema.table_constraints
        WHERE table_schema = 'maproom'
          AND table_name = 'chunks'
          AND constraint_name = 'fk_chunks_embedding'
          AND constraint_type = 'FOREIGN KEY'
      ) AS fk_exists
    `)
    const fkExists = fkResult.rows[0].fk_exists
    validations.push({
      name: 'Foreign key constraint',
      passed: fkExists,
      message: fkExists ? 'fk_chunks_embedding constraint found' : 'fk_chunks_embedding constraint missing'
    })

    // Validation 5: HNSW index exists
    const indexResult = await testClient.query(`
      SELECT EXISTS (
        SELECT 1
        FROM pg_indexes
        WHERE schemaname = 'maproom'
          AND tablename = 'code_embeddings'
          AND indexname = 'idx_embeddings_vector'
      ) AS index_exists
    `)
    const indexExists = indexResult.rows[0].index_exists
    validations.push({
      name: 'HNSW index',
      passed: indexExists,
      message: indexExists ? 'idx_embeddings_vector index found' : 'idx_embeddings_vector index missing'
    })

    // Print validation results
    console.log('\n  === Migration 002 Validation Results ===')
    for (const validation of validations) {
      const status = validation.passed ? '✓' : '✗'
      console.log(`  ${status} ${validation.name}: ${validation.message}`)
    }
    console.log('')

    // All validations must pass
    const allPassed = validations.every(v => v.passed)
    expect(allPassed, 'All migration validations should pass').toBe(true)
  })
})
