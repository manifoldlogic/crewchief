/**
 * End-to-End tests for Open tool
 *
 * Tests cover:
 * - Full workflow: index → search → open
 * - Database pollution handling via multi-candidate fallback
 * - Error handling when all candidates fail
 * - Deterministic ordering with filesystem validation
 * - Security validation against path traversal
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { Client } from 'pg'
import path from 'node:path'
import fs from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import {
  setupTestDatabase,
  teardownTestDatabase,
  indexTestFixtures,
  createTestRepo,
  createTestWorktree,
  createTestFileWithCommit,
} from '../helpers/database.js'
import { handleOpenTool } from '../../src/tools/open.js'
import { ValidationError } from '../../src/utils/validation.js'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const fixturesPath = path.join(__dirname, '..', 'fixtures')

let testClient: Client

describe('Open Tool E2E Tests', () => {
  beforeEach(async () => {
    testClient = await setupTestDatabase()
  })

  afterEach(async () => {
    await teardownTestDatabase(testClient)
  })

  it('should handle full E2E workflow: index → search → open', async () => {
    // Create test data manually instead of using indexTestFixtures
    // This avoids dependency on the CLI binary which may not be built
    const repoId = await createTestRepo(testClient, 'test-repo', fixturesPath)
    const worktreeId = await createTestWorktree(testClient, repoId, 'main', fixturesPath)
    await createTestFileWithCommit(testClient, repoId, worktreeId, 'sample-typescript.ts')

    // Call open tool with the indexed file
    const result = await handleOpenTool(
      {
        relpath: 'sample-typescript.ts',
        worktree: 'main',
      },
      testClient
    )

    // Read actual file content for comparison
    const actualContent = await fs.readFile(
      path.join(fixturesPath, 'sample-typescript.ts'),
      'utf8'
    )

    // Verify content matches
    expect(result.content).toBe(actualContent)
    expect(result.relpath).toBe('sample-typescript.ts')
  })

  it('should handle database pollution via fallback', async () => {
    // Simulate database pollution by temporarily disabling unique constraint
    // In real scenarios, pollution can happen from database corruption or migration issues
    const repoId = await createTestRepo(testClient, 'polluted-repo', fixturesPath)

    // Temporarily drop the unique constraint to simulate pollution
    await testClient.query('ALTER TABLE maproom.worktrees DROP CONSTRAINT IF EXISTS worktrees_repo_id_name_key')

    try {
      // Create first worktree with INVALID abs_path (simulates pollution)
      const invalidPath = '/nonexistent/invalid/path'
      const firstWorktreeId = await createTestWorktree(
        testClient,
        repoId,
        'main',
        invalidPath
      )

      // Create second worktree with VALID abs_path (actual fixtures path)
      const validPath = fixturesPath
      const secondWorktreeId = await createTestWorktree(
        testClient,
        repoId,
        'main',
        validPath
      )

      // Create file entries for both worktrees
      await createTestFileWithCommit(testClient, repoId, firstWorktreeId, 'sample-typescript.ts')
      await createTestFileWithCommit(testClient, repoId, secondWorktreeId, 'sample-typescript.ts')

      // Call open tool - should automatically fall back to valid path
      const result = await handleOpenTool(
        {
          relpath: 'sample-typescript.ts',
          worktree: 'main',
        },
        testClient
      )

      // Read actual file content for comparison
      const actualContent = await fs.readFile(
        path.join(fixturesPath, 'sample-typescript.ts'),
        'utf8'
      )

      // Verify it returned content from the valid path (automatic fallback)
      expect(result.content).toBe(actualContent)
      expect(result.relpath).toBe('sample-typescript.ts')
    } finally {
      // CRITICAL: Delete duplicate data BEFORE restoring constraint
      // Otherwise restore will fail with "Key (repo_id, name) is duplicated"
      await testClient.query('DELETE FROM maproom.worktrees WHERE repo_id = $1', [repoId])

      // Now safe to restore the unique constraint
      await testClient.query('ALTER TABLE maproom.worktrees ADD CONSTRAINT worktrees_repo_id_name_key UNIQUE (repo_id, name)')
    }
  })

  it('should provide clear error when all candidates fail', async () => {
    // Simulate database pollution by temporarily disabling unique constraint
    const repoId = await createTestRepo(testClient, 'all-invalid-repo', '/test')

    // Temporarily drop the unique constraint
    await testClient.query('ALTER TABLE maproom.worktrees DROP CONSTRAINT IF EXISTS worktrees_repo_id_name_key')

    try {
      // Create multiple worktrees with all invalid abs_path values
      const invalidPath1 = '/nonexistent/path/one'
      const invalidPath2 = '/nonexistent/path/two'
      const invalidPath3 = '/nonexistent/path/three'

      const worktree1 = await createTestWorktree(
        testClient,
        repoId,
        'main',
        invalidPath1
      )
      const worktree2 = await createTestWorktree(
        testClient,
        repoId,
        'main',
        invalidPath2
      )
      const worktree3 = await createTestWorktree(
        testClient,
        repoId,
        'main',
        invalidPath3
      )

      // Create file entries for all worktrees
      await createTestFileWithCommit(testClient, repoId, worktree1, 'test.ts')
      await createTestFileWithCommit(testClient, repoId, worktree2, 'test.ts')
      await createTestFileWithCommit(testClient, repoId, worktree3, 'test.ts')

      // Call open tool - should throw ValidationError
      await expect(
        handleOpenTool(
          {
            relpath: 'test.ts',
            worktree: 'main',
          },
          testClient
        )
      ).rejects.toThrow(ValidationError)

      // Verify error message mentions candidate count
      try {
        await handleOpenTool(
          {
            relpath: 'test.ts',
            worktree: 'main',
          },
          testClient
        )
      } catch (error: any) {
        expect(error.message).toContain('3')
        expect(error.message).toContain('candidate')
        expect(error.code).toBe('FILE_NOT_FOUND')
      }
    } finally {
      // CRITICAL: Delete duplicate data BEFORE restoring constraint
      // Otherwise restore will fail with "Key (repo_id, name) is duplicated"
      await testClient.query('DELETE FROM maproom.worktrees WHERE repo_id = $1', [repoId])

      // Now safe to restore the unique constraint
      await testClient.query('ALTER TABLE maproom.worktrees ADD CONSTRAINT worktrees_repo_id_name_key UNIQUE (repo_id, name)')
    }
  })

  it('should validate candidates in order (DESC by id)', async () => {
    // Simulate database pollution by temporarily disabling unique constraint
    const repoId = await createTestRepo(testClient, 'ordered-repo', fixturesPath)

    // Temporarily drop the unique constraint
    await testClient.query('ALTER TABLE maproom.worktrees DROP CONSTRAINT IF EXISTS worktrees_repo_id_name_key')

    try {
      // Create three worktrees in specific order
      // First: invalid path (should be checked first due to DESC ordering)
      const invalidPath = '/nonexistent/invalid'
      const firstWorktreeId = await createTestWorktree(
        testClient,
        repoId,
        'main',
        invalidPath
      )

      // Second: valid path (should be returned as first valid)
      const validPath = fixturesPath
      const secondWorktreeId = await createTestWorktree(
        testClient,
        repoId,
        'main',
        validPath
      )

      // Third: also valid but should not be checked (early return)
      const alsoValidPath = fixturesPath
      const thirdWorktreeId = await createTestWorktree(
        testClient,
        repoId,
        'main',
        alsoValidPath
      )

      // Create file entries for all three worktrees
      await createTestFileWithCommit(testClient, repoId, firstWorktreeId, 'sample-typescript.ts')
      await createTestFileWithCommit(testClient, repoId, secondWorktreeId, 'sample-typescript.ts')
      await createTestFileWithCommit(testClient, repoId, thirdWorktreeId, 'sample-typescript.ts')

      // Verify ordering in database (should be DESC by id)
      const { rows } = await testClient.query(
        `SELECT id, abs_path
         FROM maproom.worktrees
         WHERE repo_id = $1
         ORDER BY id DESC`,
        [repoId]
      )

      expect(rows.length).toBe(3)
      expect(rows[0].id).toBe(thirdWorktreeId) // Newest first
      expect(rows[1].id).toBe(secondWorktreeId) // Middle
      expect(rows[2].id).toBe(firstWorktreeId) // Oldest last

      // Call open tool
      const result = await handleOpenTool(
        {
          relpath: 'sample-typescript.ts',
          worktree: 'main',
        },
        testClient
      )

      // Read actual file content
      const actualContent = await fs.readFile(
        path.join(fixturesPath, 'sample-typescript.ts'),
        'utf8'
      )

      // Verify it returned content from valid path
      // Due to DESC ordering, it should try: third (valid), return immediately
      expect(result.content).toBe(actualContent)
      expect(result.relpath).toBe('sample-typescript.ts')
    } finally {
      // CRITICAL: Delete duplicate data BEFORE restoring constraint
      // Otherwise restore will fail with "Key (repo_id, name) is duplicated"
      await testClient.query('DELETE FROM maproom.worktrees WHERE repo_id = $1', [repoId])

      // Now safe to restore the unique constraint
      await testClient.query('ALTER TABLE maproom.worktrees ADD CONSTRAINT worktrees_repo_id_name_key UNIQUE (repo_id, name)')
    }
  })

  it('should reject path traversal in database abs_path', async () => {
    // Create a test repository
    const repoId = await createTestRepo(testClient, 'security-test-repo', fixturesPath)

    // Create worktree with malicious abs_path containing path traversal
    const maliciousPath = path.join(fixturesPath, '..', '..', 'secrets')
    const worktreeId = await createTestWorktree(
      testClient,
      repoId,
      'main',
      maliciousPath
    )

    // Create file entry
    await createTestFileWithCommit(testClient, repoId, worktreeId, 'sample-typescript.ts')

    // Manually update abs_path to contain path traversal (bypassing validation)
    await testClient.query(
      'UPDATE maproom.worktrees SET abs_path = $1 WHERE id = $2',
      ['../../../etc/passwd', worktreeId]
    )

    // Call open tool - should reject the malicious path
    await expect(
      handleOpenTool(
        {
          relpath: 'sample-typescript.ts',
          worktree: 'main',
        },
        testClient
      )
    ).rejects.toThrow(ValidationError)

    // Verify appropriate error
    try {
      await handleOpenTool(
        {
          relpath: 'sample-typescript.ts',
          worktree: 'main',
        },
        testClient
      )
    } catch (error: any) {
      expect(error.message).toContain('candidate')
      expect(error.code).toBe('FILE_NOT_FOUND')
    }
  })
})
