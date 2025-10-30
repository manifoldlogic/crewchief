/**
 * Integration tests for Context Tool
 *
 * Tests end-to-end functionality:
 * - Database chunk retrieval
 * - File loading
 * - Token counting
 * - Budget management
 * - Relationship expansion
 * - Error handling
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { Client } from 'pg'
import fs from 'node:fs/promises'
import path from 'node:path'
import os from 'node:os'
import { handleContextTool } from '../../src/tools/context.js'
import { setupTestDatabase, teardownTestDatabase, createTestRepo, createTestWorktree, createTestFile, createTestChunk } from '../helpers/database.js'

describe('Context Tool Integration', () => {
  let client: Client
  let testDir: string
  let repoId: number
  let worktreeId: number
  let fileId: number
  let chunkId: number

  beforeAll(async () => {
    // Set up test database
    client = await setupTestDatabase()

    // Create temporary test directory
    testDir = await fs.mkdtemp(path.join(os.tmpdir(), 'maproom-context-test-'))

    // Create test data in database
    repoId = await createTestRepo(client, 'test-repo')
    worktreeId = await createTestWorktree(client, repoId, 'main', testDir)

    // Create a test TypeScript file
    const testFilePath = path.join(testDir, 'src', 'calculator.ts')
    await fs.mkdir(path.dirname(testFilePath), { recursive: true })
    const testFileContent = `// Calculator module
export class Calculator {
  /**
   * Add two numbers
   */
  add(a: number, b: number): number {
    return a + b
  }

  /**
   * Subtract two numbers
   */
  subtract(a: number, b: number): number {
    return a - b
  }

  /**
   * Multiply two numbers
   */
  multiply(a: number, b: number): number {
    return a * b
  }
}

export function createCalculator(): Calculator {
  return new Calculator()
}
`
    await fs.writeFile(testFilePath, testFileContent, 'utf8')

    // Index the file in database
    fileId = await createTestFile(client, worktreeId, 'src/calculator.ts')

    // Create a chunk for the add method
    chunkId = await createTestChunk(client, fileId, {
      symbol_name: 'add',
      kind: 'method_definition',
      start_line: 6,
      end_line: 8,
    })
  })

  afterAll(async () => {
    // Clean up test directory
    await fs.rm(testDir, { recursive: true, force: true })
    await teardownTestDatabase(client)
  })

  it('should retrieve primary chunk with valid chunk_id', async () => {
    const result = await handleContextTool(
      {
        chunk_id: String(chunkId),
        budget_tokens: 6000,
      },
      client
    )

    expect(result).toBeDefined()
    expect(result.items).toHaveLength(1)
    expect(result.items[0].role).toBe('primary')
    expect(result.items[0].symbol_name).toBe('add')
    expect(result.items[0].content).toContain('add(a: number, b: number)')
    expect(result.total_tokens).toBeGreaterThan(0)
    expect(result.budget_remaining).toBeLessThan(6000)
    expect(result.truncated).toBe(false)
  })

  it('should include metadata in bundle', async () => {
    const result = await handleContextTool(
      {
        chunk_id: String(chunkId),
        budget_tokens: 6000,
        expand: {
          callers: false,
          callees: false,
          tests: false,
        },
      },
      client
    )

    expect(result.metadata).toBeDefined()
    expect(result.metadata.chunk_id).toBe(chunkId)
    expect(result.metadata.worktree).toBe('main')
    expect(result.metadata.expand_options).toBeDefined()
    expect(result.metadata.expand_options.callers).toBe(false)
  })

  it('should handle budget constraints', async () => {
    const result = await handleContextTool(
      {
        chunk_id: String(chunkId),
        budget_tokens: 50, // Very small budget
      },
      client
    )

    expect(result).toBeDefined()
    expect(result.items).toHaveLength(1) // Only primary chunk
    expect(result.truncated).toBe(true)
    expect(result.warnings).toBeDefined()
    expect(result.warnings!.length).toBeGreaterThan(0)
  })

  it('should validate chunk_id parameter', async () => {
    await expect(
      handleContextTool(
        {
          chunk_id: 'invalid',
          budget_tokens: 6000,
        },
        client
      )
    ).rejects.toThrow()
  })

  it('should validate budget_tokens range', async () => {
    // Budget too low
    await expect(
      handleContextTool(
        {
          chunk_id: String(chunkId),
          budget_tokens: 900, // Below minimum of 1000
        },
        client
      )
    ).rejects.toThrow()

    // Budget too high
    await expect(
      handleContextTool(
        {
          chunk_id: String(chunkId),
          budget_tokens: 25000, // Above maximum of 20000
        },
        client
      )
    ).rejects.toThrow()
  })

  it('should handle missing chunk gracefully', async () => {
    await expect(
      handleContextTool(
        {
          chunk_id: '999999',
          budget_tokens: 6000,
        },
        client
      )
    ).rejects.toThrow(/Chunk not found/)
  })

  it('should use default budget_tokens when not specified', async () => {
    const result = await handleContextTool(
      {
        chunk_id: String(chunkId),
      },
      client
    )

    expect(result.budget_tokens).toBe(6000)
  })

  it('should use default expand options when not specified', async () => {
    const result = await handleContextTool(
      {
        chunk_id: String(chunkId),
        budget_tokens: 6000,
      },
      client
    )

    expect(result.metadata.expand_options).toBeDefined()
    expect(result.metadata.expand_options.callers).toBe(true)
    expect(result.metadata.expand_options.callees).toBe(true)
    expect(result.metadata.expand_options.tests).toBe(true)
    expect(result.metadata.expand_options.max_depth).toBe(2)
  })

  it('should respect custom expand options', async () => {
    const result = await handleContextTool(
      {
        chunk_id: String(chunkId),
        budget_tokens: 6000,
        expand: {
          callers: false,
          callees: false,
          tests: false,
          docs: true,
          config: true,
          max_depth: 3,
        },
      },
      client
    )

    expect(result.metadata.expand_options.callers).toBe(false)
    expect(result.metadata.expand_options.callees).toBe(false)
    expect(result.metadata.expand_options.tests).toBe(false)
    expect(result.metadata.expand_options.docs).toBe(true)
    expect(result.metadata.expand_options.config).toBe(true)
    expect(result.metadata.expand_options.max_depth).toBe(3)
  })

  it('should extract correct line range', async () => {
    const result = await handleContextTool(
      {
        chunk_id: String(chunkId),
        budget_tokens: 6000,
      },
      client
    )

    const primaryItem = result.items[0]
    expect(primaryItem.range.start).toBe(6)
    expect(primaryItem.range.end).toBe(8)
    expect(primaryItem.relpath).toBe('src/calculator.ts')
  })

  it('should count tokens approximately', async () => {
    const result = await handleContextTool(
      {
        chunk_id: String(chunkId),
        budget_tokens: 6000,
      },
      client
    )

    const primaryItem = result.items[0]
    // Token count should be roughly content.length / 4
    const expectedTokens = Math.ceil(primaryItem.content.length / 4)
    expect(primaryItem.tokens).toBe(expectedTokens)
    expect(result.total_tokens).toBe(primaryItem.tokens)
  })

  it('should handle file read errors gracefully', async () => {
    // Create a chunk for a non-existent file
    const missingFileId = await createTestFile(client, worktreeId, 'src/missing.ts')
    const missingChunkId = await createTestChunk(client, missingFileId, {
      symbol_name: 'missing',
      kind: 'function_declaration',
      start_line: 1,
      end_line: 5,
    })

    await expect(
      handleContextTool(
        {
          chunk_id: String(missingChunkId),
          budget_tokens: 6000,
        },
        client
      )
    ).rejects.toThrow(/File not found/)
  })

  it('should handle relationships table not existing gracefully', async () => {
    // This test verifies that the tool doesn't crash when the relationships
    // table doesn't exist (which may happen in some environments)
    const result = await handleContextTool(
      {
        chunk_id: String(chunkId),
        budget_tokens: 6000,
        expand: {
          callers: true,
          callees: true,
          tests: true,
        },
      },
      client
    )

    // Should succeed with just the primary chunk
    expect(result.items).toHaveLength(1)
    expect(result.items[0].role).toBe('primary')
  })
})

describe('Context Tool - Budget Management', () => {
  let client: Client
  let testDir: string
  let repoId: number
  let worktreeId: number
  let largeChunkId: number

  beforeAll(async () => {
    client = await setupTestDatabase()
    testDir = await fs.mkdtemp(path.join(os.tmpdir(), 'maproom-budget-test-'))

    repoId = await createTestRepo(client, 'budget-test')
    worktreeId = await createTestWorktree(client, repoId, 'main', testDir)

    // Create a large file
    const largeFilePath = path.join(testDir, 'large.ts')
    const largeContent = `// Large file for budget testing\n${'export function foo() {}\n'.repeat(200)}`
    await fs.writeFile(largeFilePath, largeContent, 'utf8')

    const fileId = await createTestFile(client, worktreeId, 'large.ts')
    largeChunkId = await createTestChunk(client, fileId, {
      symbol_name: 'large',
      kind: 'module',
      start_line: 1,
      end_line: 200,
    })
  })

  afterAll(async () => {
    await fs.rm(testDir, { recursive: true, force: true })
    await teardownTestDatabase(client)
  })

  it('should mark bundle as truncated when primary chunk exceeds budget', async () => {
    const result = await handleContextTool(
      {
        chunk_id: String(largeChunkId),
        budget_tokens: 100,
      },
      client
    )

    expect(result.truncated).toBe(true)
    expect(result.warnings).toBeDefined()
    expect(result.warnings!.some((w) => w.includes('exceeds budget'))).toBe(true)
  })

  it('should calculate budget_remaining correctly', async () => {
    const result = await handleContextTool(
      {
        chunk_id: String(largeChunkId),
        budget_tokens: 10000,
      },
      client
    )

    expect(result.budget_remaining).toBe(result.budget_tokens - result.total_tokens)
    expect(result.budget_remaining).toBeGreaterThanOrEqual(0)
  })
})
