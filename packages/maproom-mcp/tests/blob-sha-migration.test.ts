/**
 * Integration tests for blob SHA computation compatibility
 *
 * These tests verify that the PostgreSQL compute_git_blob_sha() function
 * produces identical output to the Rust compute_blob_sha() function.
 *
 * This is critical for content-addressed storage and embedding deduplication,
 * as both the application (Rust) and database (PostgreSQL) must compute
 * identical SHA-256 hashes for the same content.
 *
 * Test strategy:
 * - Use known SHA values from Rust test suite (crates/maproom/src/content_hash.rs)
 * - Query PostgreSQL function with same inputs
 * - Verify byte-for-byte identical output (64 hex characters)
 *
 * Related: BLOBSHA-1002
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { Client } from 'pg'

// Test database client
let testClient: Client | null = null

beforeAll(async () => {
  // Setup test database connection
  const connectionString = process.env.DATABASE_URL || 'postgresql://maproom:maproom@localhost:5432/maproom'

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

/**
 * Known SHA-256 values from Rust test suite
 *
 * These values are verified against the Rust implementation in:
 * crates/maproom/src/content_hash.rs::test_blob_sha_git_compatibility()
 *
 * Format: SHA256("blob <size>\0<content>")
 *
 * Computed using: printf 'blob <size>\0<content>' | sha256sum
 */
const KNOWN_HASHES: Record<string, string> = {
  // Empty content: "blob 0\0"
  '': '473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813',

  // Simple ASCII: "blob 4\0test"
  'test': 'aa19560d465e7d43915547490a1f6b73eb55702e3d12cb82fb577df60bad4928',
}

/**
 * Additional test cases to cover edge cases
 * These hashes can be verified using: printf 'blob <size>\0<content>' | sha256sum
 */
const TEST_CASES: Array<{ content: string; description: string }> = [
  { content: '', description: 'empty string' },
  { content: 'test', description: 'simple ASCII' },
  { content: 'function foo() {\n  return 42;\n}', description: 'multi-line code' },
  { content: 'Hello 世界 🌍', description: 'unicode content' },
  { content: 'x'.repeat(1024), description: 'large content (1KB)' },
  { content: 'special!@#$%^&*()_+-=[]{}|;:\'",.<>?/', description: 'special characters' },
  { content: 'line1\nline2\nline3', description: 'newlines' },
  { content: 'line1\r\nline2', description: 'CRLF newlines' },
  { content: '  leading and trailing spaces  ', description: 'whitespace' },
]

describe('Blob SHA Migration - PostgreSQL Function Availability', () => {
  it.skipIf(!testClient)('should have compute_git_blob_sha function', async () => {
    if (!testClient) return

    // Query PostgreSQL function catalog to verify function exists
    const result = await testClient.query(`
      SELECT
        proname,
        pg_get_function_identity_arguments(oid) as args,
        prosrc
      FROM pg_proc
      WHERE proname = 'compute_git_blob_sha'
        AND pronamespace = 'maproom'::regnamespace
    `)

    expect(result.rows.length).toBeGreaterThan(0)
    expect(result.rows[0].proname).toBe('compute_git_blob_sha')
    expect(result.rows[0].args).toBe('content text')
  })

  it.skipIf(!testClient)('should have function in maproom schema', async () => {
    if (!testClient) return

    // Verify function is callable via maproom.compute_git_blob_sha()
    const result = await testClient.query(`
      SELECT maproom.compute_git_blob_sha('test') as sha
    `)

    expect(result.rows.length).toBe(1)
    expect(result.rows[0].sha).toBeDefined()
    expect(result.rows[0].sha.length).toBe(64) // SHA-256 = 32 bytes = 64 hex chars
  })
})

describe('Blob SHA Migration - Rust/PostgreSQL Compatibility', () => {
  it.skipIf(!testClient)('should match known Rust hash for empty content', async () => {
    if (!testClient) return

    const content = ''
    const expectedHash = KNOWN_HASHES[content]

    const result = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [content]
    )

    const actualHash = result.rows[0].sha

    // Verify hash format
    expect(actualHash).toBeDefined()
    expect(actualHash.length).toBe(64)
    expect(actualHash).toMatch(/^[0-9a-f]{64}$/)

    // Verify exact match with Rust implementation
    expect(actualHash).toBe(expectedHash)
  })

  it.skipIf(!testClient)('should match known Rust hash for "test" content', async () => {
    if (!testClient) return

    const content = 'test'
    const expectedHash = KNOWN_HASHES[content]

    const result = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [content]
    )

    const actualHash = result.rows[0].sha

    // Verify hash format
    expect(actualHash).toBeDefined()
    expect(actualHash.length).toBe(64)
    expect(actualHash).toMatch(/^[0-9a-f]{64}$/)

    // Verify exact match with Rust implementation
    expect(actualHash).toBe(expectedHash)
  })
})

describe('Blob SHA Migration - Determinism', () => {
  it.skipIf(!testClient)('should produce identical hashes for repeated calls', async () => {
    if (!testClient) return

    const content = 'function foo() { return 1; }'

    // Call PostgreSQL function twice
    const result1 = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [content]
    )
    const result2 = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [content]
    )

    const hash1 = result1.rows[0].sha
    const hash2 = result2.rows[0].sha

    // Both hashes should be identical
    expect(hash1).toBe(hash2)
    expect(hash1.length).toBe(64)
  })

  it.skipIf(!testClient)('should produce different hashes for different content', async () => {
    if (!testClient) return

    const content1 = 'function foo() { return 1; }'
    const content2 = 'function bar() { return 2; }'

    const result1 = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [content1]
    )
    const result2 = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [content2]
    )

    const hash1 = result1.rows[0].sha
    const hash2 = result2.rows[0].sha

    // Hashes should be different
    expect(hash1).not.toBe(hash2)
    expect(hash1.length).toBe(64)
    expect(hash2.length).toBe(64)
  })
})

describe('Blob SHA Migration - Edge Cases', () => {
  it.skipIf(!testClient)('should handle whitespace sensitivity', async () => {
    if (!testClient) return

    const content1 = 'function foo() { return 1; }'
    const content2 = 'function foo() { return 1;  }' // Extra space

    const result1 = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [content1]
    )
    const result2 = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [content2]
    )

    const hash1 = result1.rows[0].sha
    const hash2 = result2.rows[0].sha

    // Even a single extra space should change the hash
    expect(hash1).not.toBe(hash2)
  })

  it.skipIf(!testClient)('should handle unicode correctly', async () => {
    if (!testClient) return

    const content = '函数 foo() { return "привет"; } // こんにちは'

    const result1 = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [content]
    )
    const result2 = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [content]
    )

    const hash1 = result1.rows[0].sha
    const hash2 = result2.rows[0].sha

    // Unicode should be handled deterministically
    expect(hash1).toBe(hash2)
    expect(hash1.length).toBe(64)
  })

  it.skipIf(!testClient)('should handle newlines correctly', async () => {
    if (!testClient) return

    const contentLF = 'line1\nline2'
    const contentCRLF = 'line1\r\nline2'

    const result1 = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [contentLF]
    )
    const result2 = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [contentCRLF]
    )

    const hash1 = result1.rows[0].sha
    const hash2 = result2.rows[0].sha

    // Different newline types should produce different hashes
    expect(hash1).not.toBe(hash2)
    expect(hash1.length).toBe(64)
    expect(hash2.length).toBe(64)
  })

  it.skipIf(!testClient)('should handle large content', async () => {
    if (!testClient) return

    const largeContent = 'x'.repeat(10000) // 10KB

    const result1 = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [largeContent]
    )
    const result2 = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [largeContent]
    )

    const hash1 = result1.rows[0].sha
    const hash2 = result2.rows[0].sha

    // Large content should be handled deterministically
    expect(hash1).toBe(hash2)
    expect(hash1.length).toBe(64)
  })
})

describe('Blob SHA Migration - Comprehensive Test Suite', () => {
  it.skipIf(!testClient)('should handle all edge case scenarios consistently', async () => {
    if (!testClient) return

    // Test each case and verify determinism
    for (const testCase of TEST_CASES) {
      const result1 = await testClient.query(
        'SELECT maproom.compute_git_blob_sha($1) as sha',
        [testCase.content]
      )
      const result2 = await testClient.query(
        'SELECT maproom.compute_git_blob_sha($1) as sha',
        [testCase.content]
      )

      const hash1 = result1.rows[0].sha
      const hash2 = result2.rows[0].sha

      // Verify format
      expect(hash1.length, `Failed for: ${testCase.description}`).toBe(64)
      expect(hash1, `Failed for: ${testCase.description}`).toMatch(/^[0-9a-f]{64}$/)

      // Verify determinism
      expect(hash1, `Not deterministic for: ${testCase.description}`).toBe(hash2)
    }
  })

  it.skipIf(!testClient)('should handle batch processing efficiently', async () => {
    if (!testClient) return

    // Process multiple content items in a single query
    const contents = TEST_CASES.map(tc => tc.content)

    const result = await testClient.query(`
      SELECT
        content,
        maproom.compute_git_blob_sha(content) as sha
      FROM unnest($1::text[]) as content
    `, [contents])

    expect(result.rows.length).toBe(TEST_CASES.length)

    // Verify all hashes have correct format
    for (const row of result.rows) {
      expect(row.sha.length).toBe(64)
      expect(row.sha).toMatch(/^[0-9a-f]{64}$/)
    }
  })
})

describe('Blob SHA Migration - Git Compatibility', () => {
  it.skipIf(!testClient)('should use Git blob object format', async () => {
    if (!testClient) return

    // Verify the function uses the correct format: "blob <size>\0<content>"
    // We can test this by checking against known hashes

    const content = 'test'
    const expectedHash = KNOWN_HASHES[content]

    const result = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      [content]
    )

    const actualHash = result.rows[0].sha

    // This hash was computed using: printf 'blob 4\0test' | sha256sum
    // If it matches, we know the format is correct
    expect(actualHash).toBe(expectedHash)
  })

  it.skipIf(!testClient)('should handle size calculation correctly', async () => {
    if (!testClient) return

    // The size in the Git blob header is the byte length of the content
    // For "test", size = 4
    // For "", size = 0

    const emptyHash = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      ['']
    )

    const testHash = await testClient.query(
      'SELECT maproom.compute_git_blob_sha($1) as sha',
      ['test']
    )

    // Verify against known hashes
    expect(emptyHash.rows[0].sha).toBe(KNOWN_HASHES[''])
    expect(testHash.rows[0].sha).toBe(KNOWN_HASHES['test'])
  })
})

describe('Blob SHA Migration - Database Integration', () => {
  it.skipIf(!testClient)('should verify chunks.blob_sha column exists', async () => {
    if (!testClient) return

    // Check that the blob_sha column was added to chunks table
    const result = await testClient.query(`
      SELECT
        column_name,
        data_type,
        is_nullable
      FROM information_schema.columns
      WHERE table_schema = 'maproom'
        AND table_name = 'chunks'
        AND column_name = 'blob_sha'
    `)

    expect(result.rows.length).toBe(1)
    expect(result.rows[0].column_name).toBe('blob_sha')
    expect(result.rows[0].data_type).toBe('text')
    // After migration completes, should be NOT NULL
  })

  it.skipIf(!testClient)('should verify index on blob_sha exists', async () => {
    if (!testClient) return

    // Check that the index was created on blob_sha column
    const result = await testClient.query(`
      SELECT
        indexname,
        indexdef
      FROM pg_indexes
      WHERE schemaname = 'maproom'
        AND tablename = 'chunks'
        AND indexname = 'idx_chunks_blob_sha'
    `)

    expect(result.rows.length).toBe(1)
    expect(result.rows[0].indexname).toBe('idx_chunks_blob_sha')
    expect(result.rows[0].indexdef).toContain('blob_sha')
  })
})

describe('Blob SHA Migration - Migration Validation Tests', () => {
  it.skipIf(!testClient)('migration 001 executed successfully', async () => {
    if (!testClient) return

    // Verify migration 001 was applied by checking:
    // 1. Function exists
    // 2. Column exists
    // 3. Index exists

    // Check function exists
    const funcResult = await testClient.query(`
      SELECT proname
      FROM pg_proc
      WHERE proname = 'compute_git_blob_sha'
        AND pronamespace = 'maproom'::regnamespace
    `)
    expect(funcResult.rows.length).toBeGreaterThan(0)

    // Check column exists
    const colResult = await testClient.query(`
      SELECT column_name
      FROM information_schema.columns
      WHERE table_schema = 'maproom'
        AND table_name = 'chunks'
        AND column_name = 'blob_sha'
    `)
    expect(colResult.rows.length).toBe(1)

    // Check index exists
    const idxResult = await testClient.query(`
      SELECT indexname
      FROM pg_indexes
      WHERE schemaname = 'maproom'
        AND tablename = 'chunks'
        AND indexname = 'idx_chunks_blob_sha'
    `)
    expect(idxResult.rows.length).toBe(1)
  })

  it.skipIf(!testClient)('all chunks have blob_sha values (no NULLs)', async () => {
    if (!testClient) return

    // Verify no NULL values in blob_sha column
    const result = await testClient.query(`
      SELECT COUNT(*) as null_count
      FROM maproom.chunks
      WHERE blob_sha IS NULL
    `)

    expect(result.rows[0].null_count).toBe('0')
  })

  it.skipIf(!testClient)('deduplication metrics can be calculated', async () => {
    if (!testClient) return

    // Calculate deduplication metrics
    const result = await testClient.query(`
      SELECT
        COUNT(*) AS total_chunks,
        COUNT(DISTINCT blob_sha) AS unique_blobs,
        ROUND(100.0 * (COUNT(*) - COUNT(DISTINCT blob_sha)) / NULLIF(COUNT(*), 0), 2) AS dedup_pct
      FROM maproom.chunks
    `)

    const row = result.rows[0]

    // Verify metrics are defined
    expect(row.total_chunks).toBeDefined()
    expect(row.unique_blobs).toBeDefined()
    expect(row.dedup_pct).toBeDefined()

    // Unique blobs cannot exceed total chunks
    expect(parseInt(row.unique_blobs)).toBeLessThanOrEqual(parseInt(row.total_chunks))

    // If there are chunks, dedup_pct should be a valid number >= 0
    if (parseInt(row.total_chunks) > 0) {
      expect(parseFloat(row.dedup_pct)).toBeGreaterThanOrEqual(0)
      expect(parseFloat(row.dedup_pct)).toBeLessThanOrEqual(100)
    }
  })
})
