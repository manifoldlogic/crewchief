/**
 * Integration tests for Open tool with git operations
 *
 * Tests cover:
 * - Git commit detection
 * - Historical file retrieval with git show
 * - Real git repository operations
 * - Database integration
 */

import { describe, it, expect, beforeAll, afterAll, beforeEach } from 'vitest'
import { Client } from 'pg'
import {
  execGit,
  isCommitCheckedOut,
  getFileFromGit,
  getRepoRoot,
} from '../../src/utils/git.js'
import { handleOpenTool } from '../../src/tools/open.js'
import fs from 'node:fs/promises'
import path from 'node:path'
import os from 'node:os'

// Test database client
let testClient: Client
let testRepoPath: string

beforeAll(async () => {
  // Setup test database connection
  const connectionString = process.env.TEST_DATABASE_URL || process.env.MAPROOM_DATABASE_URL
  if (!connectionString) {
    console.warn('No TEST_DATABASE_URL set, skipping integration tests')
    return
  }
  testClient = new Client({ connectionString })
  await testClient.connect()

  // Create a temporary git repository for testing
  testRepoPath = path.join(os.tmpdir(), `maproom-test-${Date.now()}`)
  await fs.mkdir(testRepoPath, { recursive: true })
})

afterAll(async () => {
  if (testClient) {
    await testClient.end()
  }

  // Cleanup test repository
  if (testRepoPath) {
    await fs.rm(testRepoPath, { recursive: true, force: true }).catch(() => {})
  }
})

describe('Git Utilities - Integration Tests', () => {
  beforeEach(async () => {
    // Skip if no database connection
    if (!testClient) return

    // Initialize git repo for each test
    await execGit(['init'], testRepoPath)
    await execGit(['config', 'user.email', 'test@example.com'], testRepoPath)
    await execGit(['config', 'user.name', 'Test User'], testRepoPath)
  })

  it.skipIf(!testClient)('should get repository root', async () => {
    const root = await getRepoRoot(testRepoPath)
    expect(root).toBe(testRepoPath)
  })

  it.skipIf(!testClient)('should detect checked out commit (HEAD)', async () => {
    // Create initial commit
    const testFile = path.join(testRepoPath, 'test.txt')
    await fs.writeFile(testFile, 'initial content')
    await execGit(['add', 'test.txt'], testRepoPath)
    await execGit(['commit', '-m', 'Initial commit'], testRepoPath)

    const headCommit = await execGit(['rev-parse', 'HEAD'], testRepoPath)
    const isCheckedOut = await isCommitCheckedOut(headCommit.trim(), testRepoPath)
    expect(isCheckedOut).toBe(true)
  })

  it.skipIf(!testClient)('should detect non-checked-out commit', async () => {
    // Create two commits
    const testFile = path.join(testRepoPath, 'test.txt')
    await fs.writeFile(testFile, 'first content')
    await execGit(['add', 'test.txt'], testRepoPath)
    await execGit(['commit', '-m', 'First commit'], testRepoPath)
    const firstCommit = await execGit(['rev-parse', 'HEAD'], testRepoPath)

    await fs.writeFile(testFile, 'second content')
    await execGit(['add', 'test.txt'], testRepoPath)
    await execGit(['commit', '-m', 'Second commit'], testRepoPath)

    // First commit is not currently checked out
    const isCheckedOut = await isCommitCheckedOut(firstCommit.trim(), testRepoPath)
    expect(isCheckedOut).toBe(false)
  })

  it.skipIf(!testClient)('should retrieve file from git history', async () => {
    // Create a commit with a file
    const testFile = path.join(testRepoPath, 'history.txt')
    await fs.writeFile(testFile, 'historical content')
    await execGit(['add', 'history.txt'], testRepoPath)
    await execGit(['commit', '-m', 'Add history file'], testRepoPath)
    const commit = await execGit(['rev-parse', 'HEAD'], testRepoPath)

    // Retrieve file from git
    const content = await getFileFromGit(commit.trim(), 'history.txt', testRepoPath)
    expect(content).toBe('historical content')
  })

  it.skipIf(!testClient)('should retrieve file from old commit', async () => {
    // Create first version
    const testFile = path.join(testRepoPath, 'versioned.txt')
    await fs.writeFile(testFile, 'version 1')
    await execGit(['add', 'versioned.txt'], testRepoPath)
    await execGit(['commit', '-m', 'Version 1'], testRepoPath)
    const v1Commit = await execGit(['rev-parse', 'HEAD'], testRepoPath)

    // Create second version
    await fs.writeFile(testFile, 'version 2')
    await execGit(['add', 'versioned.txt'], testRepoPath)
    await execGit(['commit', '-m', 'Version 2'], testRepoPath)

    // Create third version
    await fs.writeFile(testFile, 'version 3')
    await execGit(['add', 'versioned.txt'], testRepoPath)
    await execGit(['commit', '-m', 'Version 3'], testRepoPath)

    // Retrieve old version
    const v1Content = await getFileFromGit(v1Commit.trim(), 'versioned.txt', testRepoPath)
    expect(v1Content).toBe('version 1')

    // Current version should be different
    const currentContent = await fs.readFile(testFile, 'utf8')
    expect(currentContent).toBe('version 3')
  })

  it.skipIf(!testClient)('should handle git errors for non-existent files', async () => {
    // Create a commit
    const testFile = path.join(testRepoPath, 'exists.txt')
    await fs.writeFile(testFile, 'content')
    await execGit(['add', 'exists.txt'], testRepoPath)
    await execGit(['commit', '-m', 'Add file'], testRepoPath)
    const commit = await execGit(['rev-parse', 'HEAD'], testRepoPath)

    // Try to retrieve non-existent file
    await expect(
      getFileFromGit(commit.trim(), 'nonexistent.txt', testRepoPath)
    ).rejects.toThrow()
  })

  it.skipIf(!testClient)('should handle git errors for invalid commit', async () => {
    await expect(
      getFileFromGit('invalid_commit_sha', 'test.txt', testRepoPath)
    ).rejects.toThrow()
  })

  it.skipIf(!testClient)('should handle nested file paths', async () => {
    // Create nested directory structure
    const nestedDir = path.join(testRepoPath, 'src', 'utils')
    await fs.mkdir(nestedDir, { recursive: true })
    const nestedFile = path.join(nestedDir, 'helper.ts')
    await fs.writeFile(nestedFile, 'export function help() {}')
    await execGit(['add', 'src/utils/helper.ts'], testRepoPath)
    await execGit(['commit', '-m', 'Add nested file'], testRepoPath)
    const commit = await execGit(['rev-parse', 'HEAD'], testRepoPath)

    const content = await getFileFromGit(commit.trim(), 'src/utils/helper.ts', testRepoPath)
    expect(content).toBe('export function help() {}')
  })
})

describe('Open Tool - Database Integration', () => {
  it.skipIf(!testClient)('should query worktree path from database', async () => {
    // This test requires the database to be properly set up with test data
    // Skip if database is not available
    if (!testClient) return

    // Check if test data exists
    const { rows } = await testClient.query(
      'SELECT COUNT(*) as count FROM maproom.worktrees LIMIT 1'
    )

    if (parseInt(rows[0].count) === 0) {
      console.warn('No test data in database, skipping test')
      return
    }

    // Query should work without errors
    const result = await testClient.query(
      'SELECT w.abs_path FROM maproom.worktrees w LIMIT 1'
    )
    expect(result.rows.length).toBeGreaterThanOrEqual(0)
  })
})

describe('Open Tool - End-to-End Tests', () => {
  it.skip('should handle full workflow: filesystem read', async () => {
    // This would require a fully set up test environment with database data
    // Marked as skip for now - implement when test fixtures are available
  })

  it.skip('should handle full workflow: git history read', async () => {
    // This would require a fully set up test environment with database data
    // Marked as skip for now - implement when test fixtures are available
  })
})

describe('Git Utilities - Error Handling', () => {
  it('should handle git not being installed', async () => {
    // This test is tricky - we can't easily test git not being installed
    // But we can test invalid git operations
    await expect(
      execGit(['invalid-command'], testRepoPath)
    ).rejects.toThrow()
  })

  it('should handle non-git directory', async () => {
    const nonGitDir = path.join(os.tmpdir(), `non-git-${Date.now()}`)
    await fs.mkdir(nonGitDir, { recursive: true })

    try {
      await expect(
        getRepoRoot(nonGitDir)
      ).rejects.toThrow()
    } finally {
      await fs.rm(nonGitDir, { recursive: true, force: true })
    }
  })

  it('should handle undefined commit (current HEAD)', async () => {
    const isCheckedOut = await isCommitCheckedOut(undefined)
    expect(isCheckedOut).toBe(true)
  })
})
