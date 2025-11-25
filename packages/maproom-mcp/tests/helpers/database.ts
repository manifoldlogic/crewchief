/**
 * Database helper utilities for E2E testing
 *
 * Provides functions for:
 * - Setting up test database
 * - Tearing down test data
 * - Creating test repos and worktrees
 * - Indexing test fixtures
 */

import { Client } from 'pg'
import { execSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export interface TestRepo {
  id: number
  name: string
}

export interface TestWorktree {
  id: number
  name: string
  abs_path: string
  repo_id: number
}

export interface TestFile {
  id: number
  relpath: string
  worktree_id: number
}

export interface TestChunk {
  id: number
  file_id: number
  symbol_name: string | null
  kind: string
  start_line: number
  end_line: number
}

/**
 * Get database connection from environment
 */
export function getDatabaseUrl(): string {
  const dbUrl = process.env.TEST_DATABASE_URL || process.env.MAPROOM_DATABASE_URL
  if (!dbUrl) {
    throw new Error(
      'No TEST_DATABASE_URL or MAPROOM_DATABASE_URL environment variable set. ' +
      'Set TEST_DATABASE_URL to run E2E tests with a test database.'
    )
  }
  return dbUrl
}

/**
 * Create a new database client
 */
export async function createClient(): Promise<Client> {
  const client = new Client({ connectionString: getDatabaseUrl() })
  await client.connect()
  return client
}

/**
 * Setup test database schema if needed
 */
export async function setupTestSchema(client: Client): Promise<void> {
  // Check if maproom schema exists
  const { rows } = await client.query(
    "SELECT schema_name FROM information_schema.schemata WHERE schema_name = 'maproom'"
  )

  if (rows.length === 0) {
    throw new Error(
      'Maproom schema does not exist in test database. ' +
      'Run database migrations before running E2E tests.'
    )
  }
}

/**
 * Setup test database (creates client and ensures schema exists)
 */
export async function setupTestDatabase(): Promise<Client> {
  const client = await createClient()
  await setupTestSchema(client)
  await cleanTestData(client)
  return client
}

/**
 * Teardown test database (cleans data and closes connection)
 */
export async function teardownTestDatabase(client: Client): Promise<void> {
  await cleanTestData(client)
  await client.end()
}

/**
 * Clean all test data from database
 */
export async function cleanTestData(client: Client): Promise<void> {
  await client.query('DELETE FROM maproom.chunks')
  await client.query('DELETE FROM maproom.files')
  await client.query('DELETE FROM maproom.worktrees')
  await client.query('DELETE FROM maproom.repos')
}

/**
 * Ensure test-corpus is indexed for SEMRANK tests
 * This makes tests self-contained and resilient to parallel execution
 */
export async function ensureTestCorpusIndexed(client: Client): Promise<void> {
  // Check if test-corpus exists
  const { rows } = await client.query(
    "SELECT COUNT(*) as count FROM maproom.repos WHERE name = 'test-corpus'"
  )
  const repoExists = parseInt(rows[0].count) > 0

  if (!repoExists) {
    console.log('📦 Test corpus not found, indexing /tmp/semrank-test-corpus...')

    // Use the Rust binary to index test corpus
    const binaryPath = path.join(__dirname, '..', '..', '..', 'cli', 'bin', 'crewchief-maproom')

    try {
      execSync(
        `"${binaryPath}" scan --repo test-corpus --worktree main --path /tmp/semrank-test-corpus --commit HEAD --force --generate-embeddings false`,
        {
          stdio: 'inherit',
          env: {
            ...process.env,
            MAPROOM_DATABASE_URL: getDatabaseUrl(),
          },
        }
      )

      // Verify indexing succeeded
      const { rows: verifyRows } = await client.query(
        "SELECT COUNT(*) as count FROM maproom.chunks c JOIN maproom.files f ON c.file_id = f.id JOIN maproom.worktrees w ON f.worktree_id = w.id JOIN maproom.repos r ON w.repo_id = r.id WHERE r.name = 'test-corpus'"
      )
      const chunkCount = parseInt(verifyRows[0].count)

      if (chunkCount === 0) {
        throw new Error('Test corpus indexed but no chunks found')
      }

      console.log(`✅ Test corpus indexed successfully (${chunkCount} chunks)`)
    } catch (error: any) {
      throw new Error(
        `Failed to index test corpus: ${error.message}\n` +
        'Ensure /tmp/semrank-test-corpus exists and contains TypeScript test fixtures.'
      )
    }
  }
}

/**
 * Create a test repository
 */
export async function createTestRepo(
  client: Client,
  name: string,
  rootPath: string = '/test'
): Promise<number> {
  const { rows } = await client.query(
    'INSERT INTO maproom.repos (name, root_path) VALUES ($1, $2) RETURNING id',
    [name, rootPath]
  )
  return rows[0].id as number
}

/**
 * Create a test worktree
 */
export async function createTestWorktree(
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

/**
 * Create a test file
 *
 * @deprecated Use createTestFileWithCommit instead for schema-compliant file creation
 */
export async function createTestFile(
  client: Client,
  worktreeId: number,
  relpath: string,
  lastModified: Date = new Date()
): Promise<number> {
  const { rows } = await client.query(
    'INSERT INTO maproom.files (worktree_id, relpath, last_modified) VALUES ($1, $2, $3) RETURNING id',
    [worktreeId, relpath, lastModified]
  )
  return rows[0].id as number
}

/**
 * Create a test file with proper commit tracking
 * This matches the current database schema which requires repo_id, commit_id, and content_hash
 */
export async function createTestFileWithCommit(
  client: Client,
  repoId: number,
  worktreeId: number,
  relpath: string
): Promise<number> {
  // Create a commit for this repo
  const commitResult = await client.query(
    'INSERT INTO maproom.commits (repo_id, sha) VALUES ($1, $2) RETURNING id',
    [repoId, 'test-commit-' + Math.random()]
  )
  const commitId = commitResult.rows[0].id

  // Create the file
  const { rows } = await client.query(
    'INSERT INTO maproom.files (repo_id, worktree_id, commit_id, relpath, content_hash) VALUES ($1, $2, $3, $4, $5) RETURNING id',
    [repoId, worktreeId, commitId, relpath, 'test-hash-' + Math.random()]
  )
  return rows[0].id as number
}

/**
 * Create a test chunk
 */
export async function createTestChunk(
  client: Client,
  fileId: number,
  options: {
    symbol_name?: string
    kind: string
    start_line: number
    end_line: number
    content?: string
    metadata?: any
  }
): Promise<number> {
  const { symbol_name, kind, start_line, end_line, content, metadata } = options

  // Create ts_doc for full-text search
  const tsDoc = (content || symbol_name || kind)
    .split(/\s+/)
    .map((t) => t.replace(/[^\w]/g, ''))
    .filter(Boolean)
    .join(' ')

  const { rows } = await client.query(
    `INSERT INTO maproom.chunks (
      file_id, symbol_name, kind, start_line, end_line,
      ts_doc, metadata, recency_score, churn_score
    ) VALUES ($1, $2, $3::maproom.chunk_kind, $4, $5, to_tsvector('simple', $6), $7, 1.0, 0.0)
    RETURNING id`,
    [fileId, symbol_name || null, kind, start_line, end_line, tsDoc, metadata || {}]
  )
  return rows[0].id as number
}

/**
 * Index test fixtures using the Rust indexer
 * This is a more realistic E2E approach
 */
export async function indexTestFixtures(
  fixturesPath: string,
  repo: string,
  worktree: string,
  commit: string = 'HEAD'
): Promise<void> {
  try {
    // Use the maproom binary to index fixtures
    const maproomBin = path.join(__dirname, '..', '..', 'bin', 'cli.js')

    execSync(
      `node "${maproomBin}" upsert --paths "${fixturesPath}" --commit ${commit} --repo ${repo} --worktree ${worktree} --root "${fixturesPath}"`,
      {
        stdio: 'pipe',
        env: {
          ...process.env,
          DATABASE_URL: getDatabaseUrl(),
        },
      }
    )
  } catch (error: any) {
    throw new Error(`Failed to index test fixtures: ${error.message}`)
  }
}

/**
 * Get all chunks for a file
 */
export async function getFileChunks(
  client: Client,
  fileId: number
): Promise<TestChunk[]> {
  const { rows } = await client.query(
    'SELECT id, file_id, symbol_name, kind::text, start_line, end_line FROM maproom.chunks WHERE file_id = $1 ORDER BY start_line',
    [fileId]
  )
  return rows as TestChunk[]
}

/**
 * Search chunks by query (for test verification)
 */
export async function searchChunks(
  client: Client,
  repoId: number,
  query: string,
  limit: number = 10
): Promise<any[]> {
  const tsQuery = query
    .split(/\s+/)
    .filter(Boolean)
    .map((t) => `${t.replace(/'/g, '')}:*`)
    .join(' & ')

  const { rows } = await client.query(
    `SELECT c.id, f.relpath, c.symbol_name, c.kind::text, c.start_line, c.end_line,
      ts_rank_cd(c.ts_doc, to_tsquery('simple', $2)) AS score
    FROM maproom.chunks c
    JOIN maproom.files f ON f.id = c.file_id
    WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
    ORDER BY score DESC
    LIMIT $3`,
    [repoId, tsQuery, limit]
  )
  return rows
}

/**
 * Wait for async operations with timeout
 */
export async function waitFor(
  condition: () => Promise<boolean>,
  options: { timeout?: number; interval?: number } = {}
): Promise<void> {
  const timeout = options.timeout || 5000
  const interval = options.interval || 100
  const startTime = Date.now()

  while (Date.now() - startTime < timeout) {
    if (await condition()) {
      return
    }
    await new Promise((resolve) => setTimeout(resolve, interval))
  }

  throw new Error(`Timeout waiting for condition after ${timeout}ms`)
}

/**
 * Get the count of chunks in the test corpus
 */
export async function getTestCorpusChunkCount(client: Client): Promise<number> {
  const { rows } = await client.query(
    "SELECT COUNT(*) as count FROM maproom.chunks c JOIN maproom.files f ON c.file_id = f.id JOIN maproom.repos r ON f.repo_id = r.id WHERE r.name = 'test-corpus'"
  )
  return parseInt(rows[0].count, 10)
}

/**
 * Check if test corpus fixtures are loaded
 */
export async function isTestCorpusLoaded(client: Client): Promise<boolean> {
  const { rows } = await client.query(
    "SELECT COUNT(*) as count FROM maproom.repos WHERE name = 'test-corpus'"
  )
  return parseInt(rows[0].count, 10) > 0
}

/**
 * Reload test fixtures from SQL file
 * Useful for tests that need a fresh state
 */
export async function reloadTestFixtures(client: Client): Promise<void> {
  const fixtureFile = path.resolve(__dirname, '../setup/test-fixtures.sql')
  const fixtureSQL = await import('node:fs').then(fs => fs.readFileSync(fixtureFile, 'utf-8'))

  // Execute the fixture SQL (it handles cleanup internally)
  await client.query(fixtureSQL)
}
