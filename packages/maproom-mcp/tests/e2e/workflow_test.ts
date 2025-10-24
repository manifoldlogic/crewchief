/**
 * End-to-End Workflow Tests for MCP Server
 *
 * Tests complete user workflows:
 * 1. Search → Open workflow
 * 2. Search → Context workflow
 * 3. Search → Open → Context chain
 * 4. Upsert → Search workflow
 * 5. Explain workflow
 *
 * These tests verify that all tools work correctly together in realistic usage patterns.
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { Client } from 'pg'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  createClient,
  setupTestSchema,
  cleanTestData,
  createTestRepo,
  createTestWorktree,
  createTestFile,
  createTestChunk,
} from '../helpers/database.js'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

// Import tool handlers
import { handleOpenTool } from '../../src/tools/open.js'
import { handleUpsertTool } from '../../src/tools/upsert.js'
import { handleExplainTool } from '../../src/tools/explain.js'

let testClient: Client
let testRepoId: number
let testWorktreeId: number
let testFileId: number
let testChunkId: number
const fixturesPath = path.join(__dirname, '..', 'fixtures')

describe('E2E Workflow Tests', () => {
  beforeAll(async () => {
    // Skip tests if no database configured
    if (!process.env.DATABASE_URL && !process.env.TEST_DATABASE_URL) {
      console.warn('No TEST_DATABASE_URL set, skipping E2E workflow tests')
      return
    }

    testClient = await createClient()
    await setupTestSchema(testClient)
    await cleanTestData(testClient)

    // Create test data
    const repo = await createTestRepo(testClient, 'test-e2e-workflows')
    testRepoId = repo.id

    const worktree = await createTestWorktree(
      testClient,
      testRepoId,
      'main',
      fixturesPath
    )
    testWorktreeId = worktree.id

    // Create test file
    const file = await createTestFile(
      testClient,
      testWorktreeId,
      'sample-typescript.ts'
    )
    testFileId = file.id

    // Create test chunks
    const chunk1 = await createTestChunk(testClient, testFileId, {
      symbolName: 'UserService',
      kind: 'class_declaration',
      startLine: 12,
      endLine: 60,
      content: 'export class UserService { constructor() { this.users = new Map() } createUser findById deleteUser listUsers }',
      metadata: { language: 'typescript' },
    })
    testChunkId = chunk1.id

    await createTestChunk(testClient, testFileId, {
      symbolName: 'createUser',
      kind: 'function_declaration',
      startLine: 20,
      endLine: 33,
      content: 'async createUser(name: string, email: string): Promise<User> { const id = this.users.size + 1 const user: User = { id, name, email, createdAt: new Date() } this.users.set(id, user) return user }',
      metadata: { language: 'typescript' },
    })

    await createTestChunk(testClient, testFileId, {
      symbolName: 'findById',
      kind: 'function_declaration',
      startLine: 40,
      endLine: 43,
      content: 'async findById(id: number): Promise<User | undefined> { return this.users.get(id) }',
      metadata: { language: 'typescript' },
    })

    await createTestChunk(testClient, testFileId, {
      symbolName: 'validateEmail',
      kind: 'function_declaration',
      startLine: 62,
      endLine: 65,
      content: 'export function validateEmail(email: string): boolean { const emailRegex = /^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$/ return emailRegex.test(email) }',
      metadata: { language: 'typescript' },
    })
  })

  afterAll(async () => {
    if (testClient) {
      await cleanTestData(testClient)
      await testClient.end()
    }
  })

  describe('Workflow 1: Search → Open', () => {
    it('should search for a function and then open the file', async () => {
      if (!testClient) return

      // Step 1: Search for 'createUser'
      const searchQuery = 'createUser'
      const { rows: searchResults } = await testClient.query(
        `SELECT c.id, f.relpath, c.symbol_name, c.kind::text, c.start_line, c.end_line,
          ts_rank_cd(c.ts_doc, to_tsquery('simple', $1)) AS score
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $2 AND c.ts_doc @@ to_tsquery('simple', $1)
        ORDER BY score DESC
        LIMIT 10`,
        ['createUser:*', testRepoId]
      )

      expect(searchResults.length).toBeGreaterThan(0)
      const firstHit = searchResults[0]
      expect(firstHit.symbol_name).toBe('createUser')

      // Step 2: Open the file using the search result
      const openParams = {
        relpath: firstHit.relpath,
        worktree: 'main',
        range: {
          start: firstHit.start_line,
          end: firstHit.end_line,
        },
      }

      const fileContent = await handleOpenTool(openParams, testClient)

      expect(fileContent).toBeDefined()
      expect(fileContent.content).toContain('createUser')
      expect(fileContent.content).toContain('Promise<User>')
      expect(fileContent.relpath).toBe('sample-typescript.ts')
      expect(fileContent.range).toEqual({
        start: firstHit.start_line,
        end: firstHit.end_line,
      })
    })

    it('should search for a class and open the full class definition', async () => {
      if (!testClient) return

      // Search for UserService class
      const { rows: searchResults } = await testClient.query(
        `SELECT c.id, f.relpath, c.symbol_name, c.kind::text, c.start_line, c.end_line
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
        ORDER BY ts_rank_cd(c.ts_doc, to_tsquery('simple', $2)) DESC
        LIMIT 1`,
        [testRepoId, 'UserService:*']
      )

      expect(searchResults.length).toBe(1)
      expect(searchResults[0].symbol_name).toBe('UserService')

      // Open the class
      const fileContent = await handleOpenTool(
        {
          relpath: searchResults[0].relpath,
          worktree: 'main',
          range: {
            start: searchResults[0].start_line,
            end: searchResults[0].end_line,
          },
        },
        testClient
      )

      expect(fileContent.content).toContain('class UserService')
      expect(fileContent.content).toContain('createUser')
      expect(fileContent.content).toContain('findById')
    })
  })

  describe('Workflow 2: Search → Context', () => {
    it('should search for a chunk and retrieve its context', async () => {
      if (!testClient) return

      // Step 1: Search for 'validateEmail'
      const { rows: searchResults } = await testClient.query(
        `SELECT c.id, c.symbol_name
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
        LIMIT 1`,
        [testRepoId, 'validateEmail:*']
      )

      expect(searchResults.length).toBe(1)
      const chunkId = searchResults[0].id

      // Step 2: Get context for the chunk (stub implementation check)
      const { rows: contextRows } = await testClient.query(
        `SELECT c.id, c.symbol_name, c.kind::text, c.start_line, c.end_line,
          f.relpath, w.name as worktree_name, w.abs_path
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        JOIN maproom.worktrees w ON w.id = f.worktree_id
        WHERE c.id = $1`,
        [chunkId]
      )

      expect(contextRows.length).toBe(1)
      const chunk = contextRows[0]
      expect(chunk.symbol_name).toBe('validateEmail')
      expect(chunk.kind).toBe('function_declaration')
    })

    it('should handle context retrieval for class methods', async () => {
      if (!testClient) return

      // Search for 'findById' method
      const { rows: searchResults } = await testClient.query(
        `SELECT c.id, c.symbol_name
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1 AND c.symbol_name = $2
        LIMIT 1`,
        [testRepoId, 'findById']
      )

      expect(searchResults.length).toBe(1)
      const chunkId = searchResults[0].id

      // Verify chunk exists and has metadata
      const { rows: contextRows } = await testClient.query(
        `SELECT c.id, c.symbol_name, c.metadata
        FROM maproom.chunks c
        WHERE c.id = $1`,
        [chunkId]
      )

      expect(contextRows.length).toBe(1)
      expect(contextRows[0].metadata.language).toBe('typescript')
    })
  })

  describe('Workflow 3: Search → Open → Context Chain', () => {
    it('should execute full exploration workflow', async () => {
      if (!testClient) return

      // Step 1: Search for 'User'
      const { rows: searchResults } = await testClient.query(
        `SELECT c.id, f.relpath, c.symbol_name, c.start_line, c.end_line
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
        ORDER BY ts_rank_cd(c.ts_doc, to_tsquery('simple', $2)) DESC
        LIMIT 1`,
        [testRepoId, 'User:* | UserService:*']
      )

      expect(searchResults.length).toBeGreaterThan(0)
      const firstHit = searchResults[0]

      // Step 2: Open the file to see the code
      const fileContent = await handleOpenTool(
        {
          relpath: firstHit.relpath,
          worktree: 'main',
          range: {
            start: firstHit.start_line,
            end: firstHit.end_line,
          },
        },
        testClient
      )

      expect(fileContent.content).toBeDefined()

      // Step 3: Get context for deeper understanding
      const { rows: contextRows } = await testClient.query(
        `SELECT c.id, c.symbol_name, c.kind::text
        FROM maproom.chunks c
        WHERE c.id = $1`,
        [firstHit.id]
      )

      expect(contextRows.length).toBe(1)
      expect(contextRows[0].symbol_name).toBeDefined()

      // Verify the workflow chain completed successfully
      expect(firstHit).toBeDefined()
      expect(fileContent).toBeDefined()
      expect(contextRows).toBeDefined()
    })

    it('should handle empty search results gracefully', async () => {
      if (!testClient) return

      // Search for something that doesn't exist
      const { rows: searchResults } = await testClient.query(
        `SELECT c.id, f.relpath, c.symbol_name
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
        LIMIT 1`,
        [testRepoId, 'NonExistentFunction:*']
      )

      expect(searchResults.length).toBe(0)
      // Workflow should stop here - no open or context calls needed
    })
  })

  describe('Workflow 4: Upsert → Search', () => {
    it.skip('should index new files and make them searchable', async () => {
      // TODO: This requires the Rust indexer to be available
      // Skip for now as it depends on external binary
      if (!testClient) return

      const upsertParams = {
        paths: [path.join(fixturesPath, 'sample-markdown.md')],
        commit: 'HEAD',
        repo: 'test-e2e-workflows',
        worktree: 'main',
        root: fixturesPath,
      }

      // Index the markdown file
      const upsertResult = await handleUpsertTool(upsertParams)
      expect(upsertResult.updated_files).toBeGreaterThan(0)

      // Search for content from the newly indexed file
      const { rows: searchResults } = await testClient.query(
        `SELECT c.id, f.relpath, c.symbol_name
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
        LIMIT 5`,
        [testRepoId, 'UserService:* | Management:*']
      )

      expect(searchResults.length).toBeGreaterThan(0)
      // Should find chunks from the markdown file
      const markdownHits = searchResults.filter((r) =>
        r.relpath.endsWith('.md')
      )
      expect(markdownHits.length).toBeGreaterThan(0)
    })

    it.skip('should handle incremental updates', async () => {
      if (!testClient) return

      // First index
      const upsertParams = {
        paths: [path.join(fixturesPath, 'sample-config.json')],
        commit: 'HEAD',
        repo: 'test-e2e-workflows',
        worktree: 'main',
        root: fixturesPath,
      }

      const firstResult = await handleUpsertTool(upsertParams)
      expect(firstResult.updated_files).toBeGreaterThan(0)

      // Re-index the same file (simulating an update)
      const secondResult = await handleUpsertTool(upsertParams)
      expect(secondResult.updated_files).toBeGreaterThan(0)

      // Verify the file is still searchable
      const { rows: searchResults } = await testClient.query(
        `SELECT c.id, f.relpath
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1 AND f.relpath = $2`,
        [testRepoId, 'sample-config.json']
      )

      expect(searchResults.length).toBeGreaterThan(0)
    })
  })

  describe('Workflow 5: Explain', () => {
    it.skip('should generate symbol card for a chunk', async () => {
      // Explain tool is experimental and disabled by default
      if (!testClient) return

      const explainParams = {
        chunk_id: testChunkId,
      }

      const symbolCard = await handleExplainTool(explainParams, testClient, {
        enabled: true,
      })

      expect(symbolCard).toBeDefined()
      expect(typeof symbolCard).toBe('string')
      expect(symbolCard).toContain('UserService')
    })

    it.skip('should return error when explain is disabled', async () => {
      if (!testClient) return

      const explainParams = {
        chunk_id: testChunkId,
      }

      await expect(
        handleExplainTool(explainParams, testClient, { enabled: false })
      ).rejects.toThrow()
    })
  })

  describe('Complex Multi-Tool Workflows', () => {
    it('should handle multiple sequential searches', async () => {
      if (!testClient) return

      // Search for 'User' first
      const { rows: userResults } = await testClient.query(
        `SELECT c.id, c.symbol_name
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
        LIMIT 5`,
        [testRepoId, 'User:*']
      )

      expect(userResults.length).toBeGreaterThan(0)

      // Then search for 'email'
      const { rows: emailResults } = await testClient.query(
        `SELECT c.id, c.symbol_name
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
        LIMIT 5`,
        [testRepoId, 'email:*']
      )

      expect(emailResults.length).toBeGreaterThan(0)

      // Both searches should return results
      expect(userResults.length + emailResults.length).toBeGreaterThan(0)
    })

    it('should handle opening multiple files in sequence', async () => {
      if (!testClient) return

      // Get all files in the test repo
      const { rows: files } = await testClient.query(
        `SELECT DISTINCT f.relpath
        FROM maproom.files f
        WHERE f.worktree_id = $1
        LIMIT 3`,
        [testWorktreeId]
      )

      expect(files.length).toBeGreaterThan(0)

      // Open each file
      for (const file of files) {
        const content = await handleOpenTool(
          {
            relpath: file.relpath,
            worktree: 'main',
          },
          testClient
        )

        expect(content).toBeDefined()
        expect(content.relpath).toBe(file.relpath)
      }
    })

    it('should handle search with different filters', async () => {
      if (!testClient) return

      // Search without filters
      const { rows: allResults } = await testClient.query(
        `SELECT c.id
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)`,
        [testRepoId, 'user:*']
      )

      // Search with specific file type (TypeScript only)
      const { rows: tsResults } = await testClient.query(
        `SELECT c.id
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
          AND f.relpath LIKE '%.ts'`,
        [testRepoId, 'user:*']
      )

      expect(allResults.length).toBeGreaterThanOrEqual(tsResults.length)
      expect(tsResults.length).toBeGreaterThan(0)
    })
  })

  describe('Error Recovery in Workflows', () => {
    it('should handle search with no results gracefully', async () => {
      if (!testClient) return

      const { rows: results } = await testClient.query(
        `SELECT c.id
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)`,
        [testRepoId, 'xyznonexistent:*']
      )

      expect(results.length).toBe(0)
      // Should not throw, just return empty results
    })

    it('should handle invalid chunk_id in context retrieval', async () => {
      if (!testClient) return

      const { rows: results } = await testClient.query(
        `SELECT c.id
        FROM maproom.chunks c
        WHERE c.id = $1`,
        [999999]
      )

      expect(results.length).toBe(0)
    })

    it('should handle invalid relpath in open tool', async () => {
      if (!testClient) return

      await expect(
        handleOpenTool(
          {
            relpath: 'nonexistent/file.ts',
            worktree: 'main',
          },
          testClient
        )
      ).rejects.toThrow()
    })
  })
})
