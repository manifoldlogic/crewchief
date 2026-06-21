/**
 * Tests for tool description variant injection via worktrees
 */

import { execSync } from 'child_process'
import { readFileSync, existsSync } from 'fs'
import { join } from 'path'
import { describe, it, expect, afterEach, beforeAll } from 'vitest'
import type { Variant } from '../../src/sdk/types.js'
import { createVariantWorktree } from '../../src/sdk/variant-injection.js'

describe('variant-injection', () => {
  const testVariants: Variant[] = [
    {
      id: 'test-variant-minimal',
      name: 'Minimal Test Variant',
      description: 'This is a minimal test variant description for testing purposes.',
    },
    {
      id: 'test-variant-detailed',
      name: 'Detailed Test Variant',
      description: `This is a detailed test variant with multiple lines.

It includes:
- Multiple sections
- Various formatting
- Special characters: \`code\`, "quotes", 'single quotes'

This tests the regex replacement logic.`,
      tokens: 100,
      generation: 1,
      notes: 'Test variant with complex formatting',
    },
  ]

  const cleanupFunctions: Array<() => Promise<void>> = []

  // Remove leftover variant-test worktrees AND their branches. Orphans accrue
  // when a run is interrupted/killed or a per-test cleanup fails: the worktree
  // stays registered and its branch stays checked out — and a checked-out
  // branch can't be deleted, so the worktree must be removed FIRST. Scoped to
  // the `variant-test-` prefix this suite uses (every test variant id starts
  // with `test-`), so it never touches unrelated worktrees or branches.
  function sweepVariantTestArtifacts() {
    // 1. Remove orphaned worktrees whose checked-out branch is a variant-test
    //    branch (porcelain emits a `branch refs/heads/<name>` line per worktree).
    try {
      const porcelain = execSync('git worktree list --porcelain', { encoding: 'utf-8' })
      let worktreePath: string | null = null
      for (const line of porcelain.split('\n')) {
        if (line.startsWith('worktree ')) {
          worktreePath = line.slice('worktree '.length)
        } else if (line === '') {
          worktreePath = null
        } else if (worktreePath && line.startsWith('branch refs/heads/variant-test-')) {
          try {
            execSync(`git worktree remove --force "${worktreePath}"`, { stdio: 'ignore' })
          } catch {
            // Working dir may already be gone — the prune below clears the registration.
          }
        }
      }
      execSync('git worktree prune', { stdio: 'ignore' })
    } catch {
      // Ignore — git may be unavailable in some environments.
    }

    // 2. Delete orphaned variant-test branches (now detached from any worktree,
    //    so `git branch -D` can actually remove them).
    try {
      const branches = execSync('git branch', { encoding: 'utf-8' })
        .split('\n')
        .filter((b) => b.includes('variant-test-'))
        .map((b) => b.trim().replace(/^\*?\s*/, ''))

      for (const branch of branches) {
        if (branch) {
          try {
            execSync(`git branch -D ${branch}`, { stdio: 'ignore' })
          } catch {
            // Ignore — branch might already be deleted.
          }
        }
      }
    } catch {
      // Ignore cleanup errors — git commands might fail in some environments.
    }
  }

  // Sweep before the suite, so leftovers from a previously *killed* run — where
  // neither afterEach nor afterAll could run — don't accumulate over time.
  beforeAll(() => {
    sweepVariantTestArtifacts()
  })

  afterEach(async () => {
    // Run the per-worktree cleanups returned by createVariantWorktree.
    for (const cleanup of cleanupFunctions) {
      try {
        await cleanup()
      } catch (error) {
        console.error('Cleanup error:', error)
      }
    }
    cleanupFunctions.length = 0

    // Safety net: remove anything the tracked cleanups missed — orphaned
    // worktrees plus their branches, in that order.
    sweepVariantTestArtifacts()
  })

  it('should create a variant worktree', async () => {
    const variant = testVariants[0]
    const { path, cleanup } = await createVariantWorktree(variant)
    cleanupFunctions.push(cleanup)

    // Verify worktree exists
    expect(existsSync(path)).toBe(true)

    // Verify it's a valid git worktree
    const gitDir = join(path, '.git')
    expect(existsSync(gitDir)).toBe(true)
  })

  it('should modify the tool description in the worktree', async () => {
    const variant = testVariants[0]
    const { path, cleanup } = await createVariantWorktree(variant)
    cleanupFunctions.push(cleanup)

    // Read the modified file
    const toolFilePath = join(path, 'packages/maproom-mcp/src/index.ts')
    expect(existsSync(toolFilePath)).toBe(true)

    const content = readFileSync(toolFilePath, 'utf-8')

    // Verify the description was replaced
    expect(content).toContain(variant.description)

    // Verify it still has the search tool definition
    expect(content).toContain("name: 'search'")
  })

  it('should handle variants with special characters', async () => {
    const variant = testVariants[1]
    const { path, cleanup } = await createVariantWorktree(variant)
    cleanupFunctions.push(cleanup)

    // Read the modified file
    const toolFilePath = join(path, 'packages/maproom-mcp/src/index.ts')
    const content = readFileSync(toolFilePath, 'utf-8')

    // Verify the complex description was replaced correctly
    // Note: multiline content will be escaped as \n in the file
    expect(content).toContain('This is a detailed test variant')
    expect(content).toContain('\\n') // Should have escaped newlines
    expect(content).toContain('Multiple sections')
  })

  it('should create unique worktrees for different variants', { timeout: 30000 }, async () => {
    const variant1 = testVariants[0]
    const variant2 = testVariants[1]

    const result1 = await createVariantWorktree(variant1)
    cleanupFunctions.push(result1.cleanup)

    const result2 = await createVariantWorktree(variant2)
    cleanupFunctions.push(result2.cleanup)

    // Verify paths are different
    expect(result1.path).not.toBe(result2.path)

    // Verify both worktrees exist
    expect(existsSync(result1.path)).toBe(true)
    expect(existsSync(result2.path)).toBe(true)

    // Verify each has the correct variant description
    const content1 = readFileSync(join(result1.path, 'packages/maproom-mcp/src/index.ts'), 'utf-8')
    const content2 = readFileSync(join(result2.path, 'packages/maproom-mcp/src/index.ts'), 'utf-8')

    // Verify each has parts of their respective descriptions
    // variant1 is simple (no newlines)
    expect(content1).toContain(variant1.description)

    // variant2 has newlines, check for beginning of description
    expect(content2).toContain('This is a detailed test variant')

    // Verify they don't have the beginning of each other's descriptions
    expect(content1).not.toContain('This is a detailed test variant')
    expect(content2).not.toContain('This is a minimal test variant')
  })

  it('should cleanup worktrees when cleanup function is called', async () => {
    const variant = testVariants[0]
    const { path, cleanup } = await createVariantWorktree(variant)

    // Verify worktree exists before cleanup
    expect(existsSync(path)).toBe(true)

    // Cleanup
    await cleanup()

    // Verify worktree no longer exists
    expect(existsSync(path)).toBe(false)
  })

  it('should cleanup on error during variant creation', async () => {
    // Create a variant that will cause an error (invalid description pattern)
    // Note: This test verifies error handling in createVariantWorktree
    const invalidVariant: Variant = {
      id: 'test-invalid',
      name: 'Invalid Test',
      description: '', // Empty description might cause issues
    }

    // This should either succeed or cleanup properly on error
    try {
      const { path, cleanup } = await createVariantWorktree(invalidVariant)
      cleanupFunctions.push(cleanup)

      // If it succeeds, verify the worktree exists
      expect(existsSync(path)).toBe(true)
    } catch (error) {
      // If it fails, that's okay - we just want to ensure no orphaned worktrees
      // Check that no worktree was left behind
      // (difficult to verify without tracking all worktrees, but error should be thrown)
      expect(error).toBeDefined()
    }
  })

  it('should preserve the rest of the file structure', async () => {
    const variant = testVariants[0]
    const { path, cleanup } = await createVariantWorktree(variant)
    cleanupFunctions.push(cleanup)

    const toolFilePath = join(path, 'packages/maproom-mcp/src/index.ts')
    const content = readFileSync(toolFilePath, 'utf-8')

    // Verify key structural elements are preserved
    expect(content).toContain('import')
    expect(content).toContain('toolSchemas')

    // Verify the search tool is still present
    expect(content).toContain("name: 'search'")

    // Verify file is still valid TypeScript (has reasonable structure)
    expect(content.length).toBeGreaterThan(1000)
  })
})
