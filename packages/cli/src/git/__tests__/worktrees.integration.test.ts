import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import simpleGit from 'simple-git'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { WorktreeService } from '../worktrees'

/**
 * Integration tests for WorktreeService with path expansion
 * These tests create real worktrees and verify they exist at expanded locations
 */
describe('WorktreeService integration with path expansion', () => {
  let testRepoDir: string
  let worktreeService: WorktreeService
  let createdWorktrees: string[] = []

  beforeEach(async () => {
    // Create a temporary git repository for testing
    testRepoDir = fs.mkdtempSync(path.join(os.tmpdir(), 'wtpath-test-repo-'))
    const git = simpleGit(testRepoDir)

    // Initialize git repo
    await git.init()
    await git.addConfig('user.name', 'Test User')
    await git.addConfig('user.email', 'test@example.com')

    // Create initial commit
    fs.writeFileSync(path.join(testRepoDir, 'README.md'), '# Test Repo')
    await git.add('README.md')
    await git.commit('Initial commit')

    worktreeService = new WorktreeService(testRepoDir)
  })

  afterEach(async () => {
    // Clean up worktrees - use robust cleanup with force and error handling
    for (const wtPath of createdWorktrees) {
      try {
        // First try to remove the worktree via git
        await worktreeService.removeWorktree(wtPath).catch(() => {
          // Ignore errors - worktree might not exist or be invalid
        })

        // Then remove the directory
        if (fs.existsSync(wtPath)) {
          fs.rmSync(wtPath, { recursive: true, force: true })
        }
      } catch (error) {
        // Log but don't fail - cleanup failures shouldn't break test suite
        console.warn(`Cleanup warning for ${wtPath}:`, error)
      }
    }
    createdWorktrees = []

    // Clean up test repo
    try {
      if (fs.existsSync(testRepoDir)) {
        fs.rmSync(testRepoDir, { recursive: true, force: true })
      }
    } catch (error) {
      console.warn('Cleanup warning for test repo:', error)
    }
  })

  it('creates worktree with tilde expansion', async () => {
    // Use a path in home directory
    const basePath = '~/crewchief-test-worktrees'
    const wtName = 'test-tilde-wt'
    const expectedPath = path.join(os.homedir(), 'crewchief-test-worktrees', wtName)

    // Get current branch
    const git = simpleGit(testRepoDir)
    const currentBranch = (await git.branch()).current

    // Create worktree
    const createdPath = await worktreeService.createWorktree(wtName, currentBranch, basePath, true, 'manual')
    createdWorktrees.push(createdPath)

    // Verify worktree was created at expanded location
    expect(createdPath).toBe(expectedPath)
    expect(fs.existsSync(createdPath)).toBe(true)

    // Verify it's a valid git worktree
    const wtGit = simpleGit(createdPath)
    const wtBranch = (await wtGit.branch()).current
    expect(wtBranch).toBe(wtName)

    // Verify README exists
    expect(fs.existsSync(path.join(createdPath, 'README.md'))).toBe(true)
  })

  it('creates worktree with repo placeholder expansion', async () => {
    // Use repo-name placeholder
    const basePath = path.join(os.tmpdir(), '<repo-name>-worktrees')
    const wtName = 'test-placeholder-wt'

    // Expected: /tmp/wtpath-test-repo-XXXXXX-worktrees/test-placeholder-wt
    const repoName = path.basename(testRepoDir)
    const expectedPath = path.join(os.tmpdir(), `${repoName}-worktrees`, wtName)

    // Get current branch
    const git = simpleGit(testRepoDir)
    const currentBranch = (await git.branch()).current

    // Create worktree
    const createdPath = await worktreeService.createWorktree(wtName, currentBranch, basePath, true, 'manual')
    createdWorktrees.push(createdPath)

    // Verify worktree was created at expanded location
    expect(createdPath).toBe(expectedPath)
    expect(fs.existsSync(createdPath)).toBe(true)

    // Verify it's a valid git worktree
    const wtGit = simpleGit(createdPath)
    const wtBranch = (await wtGit.branch()).current
    expect(wtBranch).toBe(wtName)
  })

  it('creates worktree with backward compatibility for relative paths', async () => {
    // Relative path should still work
    const basePath = '.crewchief/worktrees'
    const wtName = 'test-relative-wt'
    const expectedPath = path.join(testRepoDir, '.crewchief', 'worktrees', wtName)

    // Get current branch
    const git = simpleGit(testRepoDir)
    const currentBranch = (await git.branch()).current

    // Create worktree
    const createdPath = await worktreeService.createWorktree(wtName, currentBranch, basePath, true, 'manual')
    createdWorktrees.push(createdPath)

    // Verify worktree was created at expected location (relative to repo root)
    expect(createdPath).toBe(expectedPath)
    expect(fs.existsSync(createdPath)).toBe(true)

    // Verify it's a valid git worktree
    const wtGit = simpleGit(createdPath)
    const wtBranch = (await wtGit.branch()).current
    expect(wtBranch).toBe(wtName)
  })

  it('creates worktree with absolute path', async () => {
    // Absolute path
    const basePath = path.join(os.tmpdir(), 'absolute-test-worktrees')
    const wtName = 'test-absolute-wt'
    const expectedPath = path.join(basePath, wtName)

    // Get current branch
    const git = simpleGit(testRepoDir)
    const currentBranch = (await git.branch()).current

    // Create worktree
    const createdPath = await worktreeService.createWorktree(wtName, currentBranch, basePath, true, 'manual')
    createdWorktrees.push(createdPath)

    // Verify worktree was created at absolute location
    expect(createdPath).toBe(expectedPath)
    expect(fs.existsSync(createdPath)).toBe(true)

    // Verify it's a valid git worktree
    const wtGit = simpleGit(createdPath)
    const wtBranch = (await wtGit.branch()).current
    expect(wtBranch).toBe(wtName)
  })

  it('throws error with context when path expansion fails', async () => {
    // Try to create worktree in a system directory (should be rejected)
    const basePath = '/etc/worktrees'
    const wtName = 'test-error-wt'

    // Get current branch
    const git = simpleGit(testRepoDir)
    const currentBranch = (await git.branch()).current

    // Should throw with helpful error message
    await expect(worktreeService.createWorktree(wtName, currentBranch, basePath, true, 'manual')).rejects.toThrow(
      /Invalid worktree path "\/etc\/worktrees".*Rejected system directory/,
    )
  })

  it('handles combined tilde and repo placeholder expansion', async () => {
    // Combined expansion: ~/crewchief/<repo-name>
    const basePath = '~/crewchief-test/<repo-name>-wt'
    const wtName = 'test-combined-wt'

    const repoName = path.basename(testRepoDir)
    const expectedPath = path.join(os.homedir(), 'crewchief-test', `${repoName}-wt`, wtName)

    // Get current branch
    const git = simpleGit(testRepoDir)
    const currentBranch = (await git.branch()).current

    // Create worktree
    const createdPath = await worktreeService.createWorktree(wtName, currentBranch, basePath, true, 'manual')
    createdWorktrees.push(createdPath)

    // Verify worktree was created at expanded location
    expect(createdPath).toBe(expectedPath)
    expect(fs.existsSync(createdPath)).toBe(true)

    // Verify it's a valid git worktree
    const wtGit = simpleGit(createdPath)
    const wtBranch = (await wtGit.branch()).current
    expect(wtBranch).toBe(wtName)
  })
})
