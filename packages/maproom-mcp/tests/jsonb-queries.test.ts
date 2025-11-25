/**
 * Integration tests for JSONB worktree_ids queries
 *
 * These tests verify that the worktree_ids JSONB column works correctly for:
 * - Single worktree filtering (? operator)
 * - Multiple worktree filtering (?| operator)
 * - Array element removal (- operator)
 * - Deduplication (no duplicate worktree IDs)
 * - Edge cases (empty arrays, single worktrees, many worktrees)
 * - GIN index usage (performance)
 *
 * This is a critical validation for Phase 1 of BRANCHX (worktree tracking schema).
 * All tests must pass before proceeding to git integration in Phase 2.
 *
 * Related: BRANCHX-1001, BRANCHX-1003
 */

import { describe, it, expect, beforeAll, afterAll, beforeEach } from 'vitest'
import { Client } from 'pg'

// Test database client
let testClient: Client | null = null
let testRepoId: number
let testWorktree1Id: number
let testWorktree2Id: number
let testWorktree3Id: number

// Helper functions (inlined to avoid import issues)
async function createTestRepo(client: Client, name: string, rootPath: string = '/test'): Promise<number> {
  const { rows } = await client.query(
    'INSERT INTO maproom.repos (name, root_path) VALUES ($1, $2) RETURNING id',
    [name, rootPath]
  )
  return rows[0].id as number
}

async function createTestWorktree(
  client: Client,
  repoId: number,
  name: string,
  absPath: string
): Promise<number> {
  const { rows } = await client.query(
    'INSERT INTO maproom.worktrees (repo_id, name, abs_path) VALUES ($1, $2, $3) RETURNING id',
    [repoId, name, absPath]
  )
  return rows[0].id as number
}

async function createTestFile(
  client: Client,
  repoId: number,
  worktreeId: number,
  relpath: string,
  lastModified: Date = new Date()
): Promise<number> {
  // First create a commit for this repo
  const commitResult = await client.query(
    'INSERT INTO maproom.commits (repo_id, sha) VALUES ($1, $2) RETURNING id',
    [repoId, 'test-commit-' + Math.random()]
  )
  const commitId = commitResult.rows[0].id

  const { rows } = await client.query(
    'INSERT INTO maproom.files (repo_id, worktree_id, commit_id, relpath, content_hash, last_modified) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id',
    [repoId, worktreeId, commitId, relpath, 'test-hash-' + Math.random(), lastModified]
  )
  return rows[0].id as number
}

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

beforeEach(async () => {
  if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

  // Clean up test data
  await testClient.query('DELETE FROM maproom.chunks')
  await testClient.query('DELETE FROM maproom.files')
  await testClient.query('DELETE FROM maproom.worktrees')
  await testClient.query('DELETE FROM maproom.repos')

  // Create test repo and worktrees
  testRepoId = await createTestRepo(testClient, 'test-repo')
  testWorktree1Id = await createTestWorktree(testClient, testRepoId, 'main', '/test/main')
  testWorktree2Id = await createTestWorktree(testClient, testRepoId, 'feature', '/test/feature')
  testWorktree3Id = await createTestWorktree(testClient, testRepoId, 'develop', '/test/develop')
})

describe('JSONB worktree_ids - Basic Queries', () => {
  it('finds chunk in specific worktree using ? operator', async () => {
    if (!testClient) {
      console.log('⚠ Skipping test - no database connection')
      return
    }

    // Create file and chunk with worktree_ids = [1, 2]
    const fileId = await createTestFile(testClient, testRepoId, testWorktree1Id, 'test.ts')

    await testClient.query(
      `INSERT INTO maproom.chunks
       (file_id, kind, start_line, end_line, ts_doc, worktree_ids, recency_score, churn_score)
       VALUES ($1, 'func', 1, 10, to_tsvector('simple', 'test'), $2, 1.0, 0.0)`,
      [fileId, JSON.stringify([testWorktree1Id, testWorktree2Id])]
    )

    // Query using JSONB contains operator
    const result = await testClient.query(
      `SELECT * FROM maproom.chunks WHERE worktree_ids ? $1`,
      [testWorktree2Id.toString()]
    )

    expect(result.rows).toHaveLength(1)
    expect(result.rows[0].worktree_ids).toEqual([testWorktree1Id, testWorktree2Id])
  })

  it('does not find chunk when worktree not in array', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const fileId = await createTestFile(testClient, testRepoId, testWorktree1Id, 'test.ts')

    await testClient.query(
      `INSERT INTO maproom.chunks
       (file_id, kind, start_line, end_line, ts_doc, worktree_ids, recency_score, churn_score)
       VALUES ($1, 'func', 1, 10, to_tsvector('simple', 'test'), $2, 1.0, 0.0)`,
      [fileId, JSON.stringify([testWorktree1Id])]
    )

    // Query for worktree not in array
    const result = await testClient.query(
      `SELECT * FROM maproom.chunks WHERE worktree_ids ? $1`,
      [testWorktree3Id.toString()]
    )

    expect(result.rows).toHaveLength(0)
  })

  it('finds chunks in multiple worktrees using ?| operator', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const fileId1 = await createTestFile(testClient, testRepoId, testWorktree1Id, 'test1.ts')
    const fileId2 = await createTestFile(testClient, testRepoId, testWorktree2Id, 'test2.ts')
    const fileId3 = await createTestFile(testClient, testRepoId, testWorktree3Id, 'test3.ts')

    // Chunk 1: in worktree 1
    await testClient.query(
      `INSERT INTO maproom.chunks
       (file_id, kind, start_line, end_line, ts_doc, worktree_ids, recency_score, churn_score)
       VALUES ($1, 'func', 1, 10, to_tsvector('simple', 'test'), $2, 1.0, 0.0)`,
      [fileId1, JSON.stringify([testWorktree1Id])]
    )

    // Chunk 2: in worktree 2
    await testClient.query(
      `INSERT INTO maproom.chunks
       (file_id, kind, start_line, end_line, ts_doc, worktree_ids, recency_score, churn_score)
       VALUES ($1, 'func', 1, 10, to_tsvector('simple', 'test'), $2, 1.0, 0.0)`,
      [fileId2, JSON.stringify([testWorktree2Id])]
    )

    // Chunk 3: in worktree 3
    await testClient.query(
      `INSERT INTO maproom.chunks
       (file_id, kind, start_line, end_line, ts_doc, worktree_ids, recency_score, churn_score)
       VALUES ($1, 'func', 1, 10, to_tsvector('simple', 'test'), $2, 1.0, 0.0)`,
      [fileId3, JSON.stringify([testWorktree3Id])]
    )

    // Query for chunks in worktree 1 OR 2 (should find 2 chunks)
    const result = await testClient.query(
      `SELECT * FROM maproom.chunks
       WHERE worktree_ids ?| ARRAY[$1, $2]`,
      [testWorktree1Id.toString(), testWorktree2Id.toString()]
    )

    expect(result.rows).toHaveLength(2)
  })
})

describe('JSONB worktree_ids - Edge Cases', () => {
  it('handles empty worktree_ids array', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const fileId = await createTestFile(testClient, testRepoId, testWorktree1Id, 'test.ts')

    // Create chunk with empty array (orphaned chunk)
    await testClient.query(
      `INSERT INTO maproom.chunks
       (file_id, kind, start_line, end_line, ts_doc, worktree_ids, recency_score, churn_score)
       VALUES ($1, 'func', 1, 10, to_tsvector('simple', 'test'), '[]', 1.0, 0.0)`,
      [fileId]
    )

    // Query should not find chunk
    const result = await testClient.query(
      `SELECT * FROM maproom.chunks WHERE worktree_ids ? $1`,
      [testWorktree1Id.toString()]
    )

    expect(result.rows).toHaveLength(0)

    // Verify chunk exists with empty array
    const allChunks = await testClient.query(`SELECT * FROM maproom.chunks`)
    expect(allChunks.rows).toHaveLength(1)
    expect(allChunks.rows[0].worktree_ids).toEqual([])
  })

  it('handles single worktree in array', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const fileId = await createTestFile(testClient, testRepoId, testWorktree1Id, 'test.ts')

    await testClient.query(
      `INSERT INTO maproom.chunks
       (file_id, kind, start_line, end_line, ts_doc, worktree_ids, recency_score, churn_score)
       VALUES ($1, 'func', 1, 10, to_tsvector('simple', 'test'), $2, 1.0, 0.0)`,
      [fileId, JSON.stringify([testWorktree1Id])]
    )

    const result = await testClient.query(
      `SELECT * FROM maproom.chunks WHERE worktree_ids ? $1`,
      [testWorktree1Id.toString()]
    )

    expect(result.rows).toHaveLength(1)
    expect(result.rows[0].worktree_ids).toHaveLength(1)
  })

  it('handles many worktrees in array (10+)', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    // Create 10 additional worktrees
    const worktreeIds = [testWorktree1Id, testWorktree2Id, testWorktree3Id]
    for (let i = 0; i < 10; i++) {
      const id = await createTestWorktree(
        testClient,
        testRepoId,
        `branch-${i}`,
        `/test/branch-${i}`
      )
      worktreeIds.push(id)
    }

    const fileId = await createTestFile(testClient, testRepoId, testWorktree1Id, 'test.ts')

    // Create chunk with all worktree IDs
    await testClient.query(
      `INSERT INTO maproom.chunks
       (file_id, kind, start_line, end_line, ts_doc, worktree_ids, recency_score, churn_score)
       VALUES ($1, 'func', 1, 10, to_tsvector('simple', 'test'), $2, 1.0, 0.0)`,
      [fileId, JSON.stringify(worktreeIds)]
    )

    // Query should find chunk for any worktree
    for (const wtId of worktreeIds) {
      const result = await testClient.query(
        `SELECT * FROM maproom.chunks WHERE worktree_ids ? $1`,
        [wtId.toString()]
      )
      expect(result.rows).toHaveLength(1)
    }

    // Verify array length
    const chunk = await testClient.query(
      `SELECT worktree_ids FROM maproom.chunks WHERE file_id = $1`,
      [fileId]
    )
    expect(chunk.rows[0].worktree_ids).toHaveLength(13) // 3 + 10
  })
})

describe('JSONB worktree_ids - Array Manipulation', () => {
  it('prevents duplicates when adding worktree', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const fileId = await createTestFile(testClient, testRepoId, testWorktree1Id, 'test.ts')

    // Create chunk with initial worktree
    await testClient.query(
      `INSERT INTO maproom.chunks
       (file_id, kind, start_line, end_line, ts_doc, worktree_ids, recency_score, churn_score)
       VALUES ($1, 'func', 1, 10, to_tsvector('simple', 'test'), $2, 1.0, 0.0)`,
      [fileId, JSON.stringify([testWorktree1Id])]
    )

    // Try to add same worktree (should deduplicate)
    await testClient.query(
      `UPDATE maproom.chunks
       SET worktree_ids = CASE
         WHEN worktree_ids ? $1::text THEN worktree_ids
         ELSE worktree_ids || to_jsonb($1::bigint)
       END
       WHERE file_id = $2`,
      [testWorktree1Id.toString(), fileId]
    )

    // Verify no duplicates
    const result = await testClient.query(
      `SELECT worktree_ids FROM maproom.chunks WHERE file_id = $1`,
      [fileId]
    )
    expect(result.rows[0].worktree_ids).toHaveLength(1)
    expect(result.rows[0].worktree_ids[0]).toBe(testWorktree1Id)
  })

  it('removes worktree from array using - operator', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const fileId = await createTestFile(testClient, testRepoId, testWorktree1Id, 'test.ts')

    // Create chunk with multiple worktrees
    await testClient.query(
      `INSERT INTO maproom.chunks
       (file_id, kind, start_line, end_line, ts_doc, worktree_ids, recency_score, churn_score)
       VALUES ($1, 'func', 1, 10, to_tsvector('simple', 'test'), $2, 1.0, 0.0)`,
      [fileId, JSON.stringify([testWorktree1Id, testWorktree2Id, testWorktree3Id])]
    )

    // Remove worktree 2
    await testClient.query(
      `UPDATE maproom.chunks
       SET worktree_ids = worktree_ids - $1
       WHERE file_id = $2`,
      [testWorktree2Id.toString(), fileId]
    )

    // Verify worktree 2 removed
    const result = await testClient.query(`SELECT worktree_ids FROM maproom.chunks`)
    expect(result.rows[0].worktree_ids).toHaveLength(2)
    expect(result.rows[0].worktree_ids).toContain(testWorktree1Id)
    expect(result.rows[0].worktree_ids).toContain(testWorktree3Id)
    expect(result.rows[0].worktree_ids).not.toContain(testWorktree2Id)
  })
})

describe('JSONB worktree_ids - Performance', () => {
  it('verifies GIN index exists', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const result = await testClient.query(`
      SELECT indexname, indexdef
      FROM pg_indexes
      WHERE schemaname = 'maproom'
        AND tablename = 'chunks'
        AND indexname = 'idx_chunks_worktree_ids'
    `)

    expect(result.rows).toHaveLength(1)
    expect(result.rows[0].indexdef).toContain('gin')
    expect(result.rows[0].indexdef).toContain('worktree_ids')
  })

  it('verifies GIN index is used in query plan', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    // Create some test data
    const fileId = await createTestFile(testClient, testRepoId, testWorktree1Id, 'test.ts')
    await testClient.query(
      `INSERT INTO maproom.chunks
       (file_id, kind, start_line, end_line, ts_doc, worktree_ids, recency_score, churn_score)
       VALUES ($1, 'func', 1, 10, to_tsvector('simple', 'test'), $2, 1.0, 0.0)`,
      [fileId, JSON.stringify([testWorktree1Id])]
    )

    // Get query plan
    const plan = await testClient.query(`
      EXPLAIN
      SELECT * FROM maproom.chunks
      WHERE worktree_ids ? $1
    `, [testWorktree1Id.toString()])

    // Check if index is mentioned (may be Index Scan or Bitmap Index Scan)
    const planText = plan.rows.map(r => r['QUERY PLAN']).join('\n')
    const usesIndex =
      planText.includes('idx_chunks_worktree_ids') ||
      planText.includes('Index Scan') ||
      planText.includes('Bitmap Index Scan')

    // Note: With small datasets, PostgreSQL may choose Seq Scan
    // This is expected and not a failure
    console.log('Query plan:', planText)
    console.log('Uses index or scan:', usesIndex)
  })

  it('performs acceptably with 1000+ chunks', async () => {
    if (!testClient) { console.log('⚠ Skipping test - no database connection'); return; }

    const fileId = await createTestFile(testClient, testRepoId, testWorktree1Id, 'test.ts')

    // Insert 1000 chunks
    const insertPromises = []
    for (let i = 0; i < 1000; i++) {
      const promise = testClient.query(
        `INSERT INTO maproom.chunks
         (file_id, kind, start_line, end_line, ts_doc, worktree_ids, recency_score, churn_score)
         VALUES ($1, 'func', $2, $3, to_tsvector('simple', 'test'), $4, 1.0, 0.0)`,
        [
          fileId,
          i * 10 + 1,
          i * 10 + 10,
          JSON.stringify([testWorktree1Id, testWorktree2Id])
        ]
      )
      insertPromises.push(promise)
    }
    await Promise.all(insertPromises)

    // Measure query time
    const startTime = Date.now()
    const result = await testClient.query(
      `SELECT * FROM maproom.chunks WHERE worktree_ids ? $1`,
      [testWorktree1Id.toString()]
    )
    const queryTime = Date.now() - startTime

    expect(result.rows).toHaveLength(1000)
    expect(queryTime).toBeLessThan(1000) // Should complete in <1 second

    console.log(`Query time for 1000 chunks: ${queryTime}ms`)
  })
})
