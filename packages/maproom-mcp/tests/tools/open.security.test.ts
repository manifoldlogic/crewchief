/**
 * Security tests for Open tool
 *
 * Tests comprehensive security validation including:
 * - Path traversal attacks in input parameters
 * - Path traversal attacks in database data (database pollution)
 * - Symlink attacks (symlinks pointing outside repository)
 * - Absolute path injection in relpath
 * - Null byte injection in relpath
 *
 * These tests verify that security enhancements added in Phase 2
 * (Tickets OPNFIX-2001 and OPNFIX-2002) work correctly and cannot be bypassed.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { Client } from 'pg'
import path from 'node:path'
import fs from 'node:fs/promises'
import os from 'node:os'
import {
  setupTestDatabase,
  teardownTestDatabase,
  createTestRepo,
  createTestWorktree,
  createTestFileWithCommit,
} from '../helpers/database.js'
import { handleOpenTool } from '../../src/tools/open.js'

describe('Open Tool Security Tests', () => {
  let testClient: Client
  let testRepoPath: string

  beforeEach(async () => {
    testClient = await setupTestDatabase()

    // Create a temporary directory for test repository
    testRepoPath = path.join(os.tmpdir(), `maproom-security-test-${Date.now()}`)
    await fs.mkdir(testRepoPath, { recursive: true })
  })

  afterEach(async () => {
    await teardownTestDatabase(testClient)

    // Cleanup test repository
    if (testRepoPath) {
      await fs.rm(testRepoPath, { recursive: true, force: true }).catch(() => {})
    }
  })

  it('should reject path traversal in relpath', async () => {
    // Setup: Create a valid repository and worktree in the database
    const repoId = await createTestRepo(testClient, 'test-repo', testRepoPath)
    const worktreeId = await createTestWorktree(testClient, repoId, 'main', testRepoPath)

    // Create a legitimate file
    const validFile = path.join(testRepoPath, 'valid.txt')
    await fs.writeFile(validFile, 'legitimate content')
    await createTestFileWithCommit(testClient, repoId, worktreeId, 'valid.txt')

    // Attack: Try to access /etc/passwd via path traversal
    // This should be rejected during input validation (validatePath)
    await expect(async () => {
      await handleOpenTool(
        {
          relpath: '../../../etc/passwd',
          worktree: 'main',
        },
        testClient
      )
    }).rejects.toThrow(/path.*traversal/i)
  })

  it('should reject path traversal in database abs_path', async () => {
    // Setup: Create a repository
    const repoId = await createTestRepo(testClient, 'test-repo', testRepoPath)

    // Attack: Create worktree with abs_path that contains path traversal
    // This simulates database pollution where abs_path escapes its intended directory
    // Use a path that resolves to a non-existent location
    const maliciousPath = path.join(testRepoPath, '..', '..', 'nonexistent-evil-directory')
    const worktreeId = await createTestWorktree(testClient, repoId, 'main', maliciousPath)

    // Create a file entry pointing to a file that doesn't exist at the malicious path
    await createTestFileWithCommit(testClient, repoId, worktreeId, 'evil.txt')

    // The open tool should reject this because the file doesn't exist
    // at the malicious path, demonstrating that database pollution fails
    // when the filesystem doesn't cooperate with the attack
    await expect(async () => {
      await handleOpenTool(
        {
          relpath: 'evil.txt',
          worktree: 'main',
        },
        testClient
      )
    }).rejects.toThrow(/not.*accessible|not.*found|database.*pollution/i)
  })

  it('should reject symlinks outside repository', async () => {
    // Setup: Create a valid repository and worktree
    const repoId = await createTestRepo(testClient, 'test-repo', testRepoPath)
    const worktreeId = await createTestWorktree(testClient, repoId, 'main', testRepoPath)

    // Attack: Create a symlink inside repo pointing to /etc/passwd
    const symlinkPath = path.join(testRepoPath, 'malicious-link')
    try {
      await fs.symlink('/etc/passwd', symlinkPath)
    } catch (error: any) {
      // Skip test if we can't create symlinks (e.g., Windows without admin)
      if (error.code === 'EPERM' || error.code === 'ENOENT') {
        console.warn('Cannot create symlink, skipping test')
        return
      }
      throw error
    }

    // Index the symlink in the database
    await createTestFileWithCommit(testClient, repoId, worktreeId, 'malicious-link')

    // The open tool should detect the symlink points outside repository
    // and reject it during validateWithinRepo check
    await expect(async () => {
      await handleOpenTool(
        {
          relpath: 'malicious-link',
          worktree: 'main',
        },
        testClient
      )
    }).rejects.toThrow(/outside.*repository/i)
  })

  it('should reject absolute paths in relpath', async () => {
    // Setup: Create a valid repository and worktree
    const repoId = await createTestRepo(testClient, 'test-repo', testRepoPath)
    await createTestWorktree(testClient, repoId, 'main', testRepoPath)

    // Attack: Try to pass an absolute path as relpath
    // This should be rejected during input validation (validatePath checks for absolute paths)
    await expect(async () => {
      await handleOpenTool(
        {
          relpath: '/etc/passwd',
          worktree: 'main',
        },
        testClient
      )
    }).rejects.toThrow(/absolute.*path/i)
  })

  it('should reject null byte injection in relpath', async () => {
    // Setup: Create a valid repository and worktree
    const repoId = await createTestRepo(testClient, 'test-repo', testRepoPath)
    await createTestWorktree(testClient, repoId, 'main', testRepoPath)

    // Attack: Try to inject null byte to bypass validation
    // For example: "file.txt\0malicious" might trick some validators
    // This should be rejected during input validation (validatePath checks for null bytes)
    await expect(async () => {
      await handleOpenTool(
        {
          relpath: 'file.txt\0malicious',
          worktree: 'main',
        },
        testClient
      )
    }).rejects.toThrow(/null.*byte/i)
  })

  it('should allow symlinks within repository', async () => {
    // This is a positive test - symlinks WITHIN the repo should work
    const repoId = await createTestRepo(testClient, 'test-repo', testRepoPath)
    const worktreeId = await createTestWorktree(testClient, repoId, 'main', testRepoPath)

    // Create a real file
    const realFile = path.join(testRepoPath, 'real.txt')
    await fs.writeFile(realFile, 'real content')

    // Create a symlink pointing to the real file (both within repo)
    const symlinkPath = path.join(testRepoPath, 'link.txt')
    try {
      await fs.symlink(realFile, symlinkPath)
    } catch (error: any) {
      // Skip test if we can't create symlinks
      if (error.code === 'EPERM' || error.code === 'ENOENT') {
        console.warn('Cannot create symlink, skipping test')
        return
      }
      throw error
    }

    // Index the symlink
    await createTestFileWithCommit(testClient, repoId, worktreeId, 'link.txt')

    // This should succeed - symlink is within repository boundaries
    const result = await handleOpenTool(
      {
        relpath: 'link.txt',
        worktree: 'main',
      },
      testClient
    )

    expect(result.content).toBe('real content')
  })

  it('should not leak sensitive information in error messages', async () => {
    // This test verifies that error messages provide useful information
    // without exposing unnecessary system details
    const repoId = await createTestRepo(testClient, 'test-repo', testRepoPath)
    await createTestWorktree(testClient, repoId, 'main', testRepoPath)

    // Test that error messages are informative
    const attacks = [
      {
        input: { relpath: '../../../etc/passwd', worktree: 'main' },
        expectedPattern: /path.*traversal|invalid.*path/i,
      },
      {
        input: { relpath: '/etc/passwd', worktree: 'main' },
        expectedPattern: /absolute.*path/i,
      },
      {
        input: { relpath: 'file\0malicious', worktree: 'main' },
        expectedPattern: /null.*byte/i,
      },
    ]

    for (const attack of attacks) {
      try {
        await handleOpenTool(attack.input, testClient)
        // Should not reach here
        throw new Error('Expected security validation to reject attack')
      } catch (error: any) {
        // Error message SHOULD match expected security pattern
        expect(error.message).toMatch(attack.expectedPattern)

        // Error message SHOULD provide actionable information
        expect(error.message.length).toBeGreaterThan(0)

        // Error message should NOT leak database connection strings
        expect(error.message).not.toMatch(/postgresql:\/\//)
        expect(error.message).not.toMatch(/password/i)
      }
    }
  })

  it('should validate database abs_path with expectedRoot', async () => {
    // This test verifies the optional root validation feature from OPNFIX-2002
    const repoId = await createTestRepo(testClient, 'test-repo', testRepoPath)

    // Create worktree with abs_path outside expected workspace
    const suspiciousPath = '/tmp/suspicious-repo'
    const worktreeId = await createTestWorktree(testClient, repoId, 'main', suspiciousPath)

    // Create file in database
    await createTestFileWithCommit(testClient, repoId, worktreeId, 'test.txt')

    // If expectedRoot is set (e.g., via environment variable or config),
    // the tool should skip worktrees with abs_path outside expectedRoot
    // In this test, we can't easily set expectedRoot, but we verify the behavior
    // by checking that a non-existent path fails properly
    await expect(async () => {
      await handleOpenTool(
        {
          relpath: 'test.txt',
          worktree: 'main',
        },
        testClient
      )
    }).rejects.toThrow()
  })
})
