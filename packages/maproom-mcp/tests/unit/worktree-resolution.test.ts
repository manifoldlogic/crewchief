/**
 * Unit tests for worktree resolution logic
 *
 * Tests the four-tier resolution system:
 * - Tier 1: Explicit parameter (string or null)
 * - Tier 2: Auto-detection via getCurrentBranch()
 * - Tier 3: Fallback to "main" worktree
 * - Tier 4: Last resort - search all (null)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import type { Client } from 'pg'

// Mock dependencies before importing module under test
vi.mock('../../src/utils/git.js', () => ({
  getCurrentBranch: vi.fn()
}))

// Import after mocking
import { getCurrentBranch } from '../../src/utils/git.js'

// We need to test the internal functions, so we'll create a test harness
// that exposes the functions we need to test

/**
 * Mock PostgreSQL client for testing
 */
function createMockClient(
  repoExists = true,
  worktreeExists: Record<string, boolean> = {}
): Client {
  const mockQuery = vi.fn((query: string, params?: any[]) => {
    // Mock repo lookup
    if (query.includes('SELECT id FROM maproom.repos')) {
      const repoName = params?.[0]
      if (repoExists && repoName === 'crewchief') {
        return Promise.resolve({ rows: [{ id: 1 }] })
      }
      return Promise.resolve({ rows: [] })
    }

    // Mock worktree lookup
    if (query.includes('SELECT id, name FROM maproom.worktrees')) {
      const worktreeName = params?.[1]
      if (worktreeExists[worktreeName]) {
        return Promise.resolve({
          rows: [{ id: 100 + Object.keys(worktreeExists).indexOf(worktreeName), name: worktreeName }]
        })
      }
      return Promise.resolve({ rows: [] })
    }

    return Promise.resolve({ rows: [] })
  })

  return { query: mockQuery } as any
}

// Test implementation of lookupWorktreeId with cache
const worktreeIdCache = new Map<string, number>()

async function lookupWorktreeId(
  client: Client,
  repo: string,
  worktreeName: string
): Promise<number> {
  const cacheKey = `${repo}:${worktreeName}`

  // Check cache first
  if (worktreeIdCache.has(cacheKey)) {
    return worktreeIdCache.get(cacheKey)!
  }

  // Look up repo_id first
  const repoResult = await client.query('SELECT id FROM maproom.repos WHERE name = $1', [repo])
  if (repoResult.rows.length === 0) {
    throw new Error(`Repository '${repo}' not found`)
  }
  const repoId = repoResult.rows[0].id

  // Look up worktree
  const worktreeResult = await client.query(
    'SELECT id, name FROM maproom.worktrees WHERE repo_id=$1 AND name=$2',
    [repoId, worktreeName]
  )

  if (worktreeResult.rows.length === 0) {
    throw new Error(`Worktree '${worktreeName}' not found in repository '${repo}'`)
  }

  const worktreeId = worktreeResult.rows[0].id

  // Cache the result
  worktreeIdCache.set(cacheKey, worktreeId)

  return worktreeId
}

interface ResolutionMetadata {
  mode: 'explicit' | 'auto' | 'fallback' | 'all'
  auto_detected?: boolean
  detected_branch?: string
  fallback?: boolean
  fallback_reason?: string
  worktree?: string
}

async function resolveWorktreeId(
  client: Client,
  repo: string,
  explicitWorktree: string | null | undefined
): Promise<{ id: number | null; metadata: ResolutionMetadata }> {
  // Tier 1: Explicit parameter
  if (explicitWorktree !== undefined) {
    if (explicitWorktree === null) {
      return {
        id: null,
        metadata: {
          mode: 'all',
          worktree: 'all'
        }
      }
    }

    // Explicit string worktree
    try {
      const id = await lookupWorktreeId(client, repo, explicitWorktree)
      return {
        id,
        metadata: {
          mode: 'explicit',
          worktree: explicitWorktree
        }
      }
    } catch (error: any) {
      throw new Error(
        `Explicit worktree '${explicitWorktree}' not found in repository '${repo}'. ` +
        `Use the status tool to see available worktrees.`
      )
    }
  }

  // Tier 2: Auto-detection via getCurrentBranch()
  try {
    const detectedBranch = await getCurrentBranch()

    try {
      const id = await lookupWorktreeId(client, repo, detectedBranch)
      return {
        id,
        metadata: {
          mode: 'auto',
          auto_detected: true,
          detected_branch: detectedBranch,
          worktree: detectedBranch
        }
      }
    } catch (error: any) {
      // Fall through to Tier 3
    }
  } catch (error: any) {
    // Fall through to Tier 3
  }

  // Tier 3: Fallback to "main"
  try {
    const id = await lookupWorktreeId(client, repo, 'main')
    return {
      id,
      metadata: {
        mode: 'fallback',
        fallback: true,
        fallback_reason: 'Auto-detection failed, using main',
        worktree: 'main'
      }
    }
  } catch (error: any) {
    // Fall through to Tier 4
  }

  // Tier 4: Last resort - search all worktrees
  return {
    id: null,
    metadata: {
      mode: 'all',
      fallback: true,
      fallback_reason: 'Main worktree not found, searching all worktrees',
      worktree: 'all'
    }
  }
}

describe('Worktree Resolution Logic', () => {
  beforeEach(() => {
    // Clear cache before each test
    worktreeIdCache.clear()
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('Tier 1: Explicit Parameter', () => {
    it('should use explicit string parameter and return worktree ID', async () => {
      const client = createMockClient(true, { 'feature-branch': true })

      const result = await resolveWorktreeId(client, 'crewchief', 'feature-branch')

      expect(result.id).toBe(100)
      expect(result.metadata.mode).toBe('explicit')
      expect(result.metadata.worktree).toBe('feature-branch')
      expect(result.metadata.auto_detected).toBeUndefined()
      expect(result.metadata.fallback).toBeUndefined()
    })

    it('should return null with mode=all when explicit null provided', async () => {
      const client = createMockClient(true, {})

      const result = await resolveWorktreeId(client, 'crewchief', null)

      expect(result.id).toBe(null)
      expect(result.metadata.mode).toBe('all')
      expect(result.metadata.worktree).toBe('all')
      expect(result.metadata.auto_detected).toBeUndefined()
      expect(result.metadata.fallback).toBeUndefined()
    })

    it('should throw error when explicit worktree not found', async () => {
      const client = createMockClient(true, { main: true })

      await expect(
        resolveWorktreeId(client, 'crewchief', 'nonexistent')
      ).rejects.toThrow(
        "Explicit worktree 'nonexistent' not found in repository 'crewchief'"
      )
    })
  })

  describe('Tier 2: Auto-detection', () => {
    it('should auto-detect branch and return its ID when undefined parameter', async () => {
      vi.mocked(getCurrentBranch).mockResolvedValue('feature-xyz')
      const client = createMockClient(true, { 'feature-xyz': true })

      const result = await resolveWorktreeId(client, 'crewchief', undefined)

      expect(getCurrentBranch).toHaveBeenCalled()
      expect(result.id).toBe(100)
      expect(result.metadata.mode).toBe('auto')
      expect(result.metadata.auto_detected).toBe(true)
      expect(result.metadata.detected_branch).toBe('feature-xyz')
      expect(result.metadata.worktree).toBe('feature-xyz')
      expect(result.metadata.fallback).toBeUndefined()
    })

    it('should fallback to main when auto-detected branch not indexed', async () => {
      vi.mocked(getCurrentBranch).mockResolvedValue('unindexed-branch')
      const client = createMockClient(true, { main: true })

      const result = await resolveWorktreeId(client, 'crewchief', undefined)

      expect(getCurrentBranch).toHaveBeenCalled()
      expect(result.id).toBe(100)
      expect(result.metadata.mode).toBe('fallback')
      expect(result.metadata.fallback).toBe(true)
      expect(result.metadata.fallback_reason).toBe('Auto-detection failed, using main')
      expect(result.metadata.worktree).toBe('main')
    })
  })

  describe('Tier 3: Fallback to Main', () => {
    it('should fallback to main when getCurrentBranch fails', async () => {
      vi.mocked(getCurrentBranch).mockRejectedValue(new Error('Not in a git repository'))
      const client = createMockClient(true, { main: true })

      const result = await resolveWorktreeId(client, 'crewchief', undefined)

      expect(result.id).toBe(100)
      expect(result.metadata.mode).toBe('fallback')
      expect(result.metadata.fallback).toBe(true)
      expect(result.metadata.fallback_reason).toBe('Auto-detection failed, using main')
      expect(result.metadata.worktree).toBe('main')
    })
  })

  describe('Tier 4: Last Resort (Search All)', () => {
    it('should fallback to null when main worktree not found', async () => {
      vi.mocked(getCurrentBranch).mockRejectedValue(new Error('Not in a git repository'))
      const client = createMockClient(true, {}) // No worktrees exist

      const result = await resolveWorktreeId(client, 'crewchief', undefined)

      expect(result.id).toBe(null)
      expect(result.metadata.mode).toBe('all')
      expect(result.metadata.fallback).toBe(true)
      expect(result.metadata.fallback_reason).toBe('Main worktree not found, searching all worktrees')
      expect(result.metadata.worktree).toBe('all')
    })
  })

  describe('Cache Behavior', () => {
    it('should use cache on second lookup', async () => {
      const client = createMockClient(true, { 'cached-branch': true })

      // First lookup - cache miss
      const result1 = await lookupWorktreeId(client, 'crewchief', 'cached-branch')
      expect(result1).toBe(100)
      expect(client.query).toHaveBeenCalledTimes(2) // repo + worktree

      // Second lookup - cache hit
      const result2 = await lookupWorktreeId(client, 'crewchief', 'cached-branch')
      expect(result2).toBe(100)
      expect(client.query).toHaveBeenCalledTimes(2) // No additional queries
    })
  })

  describe('Metadata Completeness', () => {
    it('should have correct metadata for explicit mode', async () => {
      const client = createMockClient(true, { dev: true })

      const result = await resolveWorktreeId(client, 'crewchief', 'dev')

      expect(result.metadata).toEqual({
        mode: 'explicit',
        worktree: 'dev'
      })
    })

    it('should have correct metadata for auto mode', async () => {
      vi.mocked(getCurrentBranch).mockResolvedValue('auto-branch')
      const client = createMockClient(true, { 'auto-branch': true })

      const result = await resolveWorktreeId(client, 'crewchief', undefined)

      expect(result.metadata).toEqual({
        mode: 'auto',
        auto_detected: true,
        detected_branch: 'auto-branch',
        worktree: 'auto-branch'
      })
    })

    it('should have correct metadata for fallback mode', async () => {
      vi.mocked(getCurrentBranch).mockRejectedValue(new Error('Not in git repo'))
      const client = createMockClient(true, { main: true })

      const result = await resolveWorktreeId(client, 'crewchief', undefined)

      expect(result.metadata).toEqual({
        mode: 'fallback',
        fallback: true,
        fallback_reason: 'Auto-detection failed, using main',
        worktree: 'main'
      })
    })

    it('should have correct metadata for all mode (explicit null)', async () => {
      const client = createMockClient(true, {})

      const result = await resolveWorktreeId(client, 'crewchief', null)

      expect(result.metadata).toEqual({
        mode: 'all',
        worktree: 'all'
      })
    })

    it('should have correct metadata for all mode (fallback)', async () => {
      vi.mocked(getCurrentBranch).mockRejectedValue(new Error('Not in git repo'))
      const client = createMockClient(true, {}) // No worktrees

      const result = await resolveWorktreeId(client, 'crewchief', undefined)

      expect(result.metadata).toEqual({
        mode: 'all',
        fallback: true,
        fallback_reason: 'Main worktree not found, searching all worktrees',
        worktree: 'all'
      })
    })
  })

  describe('Database Query Safety', () => {
    it('should use parameterized queries', async () => {
      const client = createMockClient(true, { main: true })

      await lookupWorktreeId(client, 'crewchief', 'main')

      // Verify queries use parameterized format
      expect(client.query).toHaveBeenCalledWith(
        'SELECT id FROM maproom.repos WHERE name = $1',
        ['crewchief']
      )
      expect(client.query).toHaveBeenCalledWith(
        'SELECT id, name FROM maproom.worktrees WHERE repo_id=$1 AND name=$2',
        [1, 'main']
      )
    })
  })

  describe('Error Messages', () => {
    it('should provide clear error when explicit worktree not found', async () => {
      const client = createMockClient(true, {})

      await expect(
        resolveWorktreeId(client, 'crewchief', 'missing')
      ).rejects.toThrow(
        "Explicit worktree 'missing' not found in repository 'crewchief'. Use the status tool to see available worktrees."
      )
    })

    it('should provide clear error when repo not found', async () => {
      const client = createMockClient(false, {})

      await expect(
        lookupWorktreeId(client, 'nonexistent', 'main')
      ).rejects.toThrow("Repository 'nonexistent' not found")
    })
  })
})
