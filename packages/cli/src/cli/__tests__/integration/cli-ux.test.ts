import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { resolve } from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'

// Path to the built CLI
const CLI_PATH = resolve(__dirname, '../../../../dist/cli/index.js')

// Use the project workspace which has crewchief.config.js
const WORKSPACE_PATH = resolve(__dirname, '../../../../../..')

// Worktree names used by tests in this file
const TEST_WORKTREES = ['test-worktree-use', 'test-worktree-stdout']

function cleanupWorktree(name: string) {
  // Remove the git worktree and branch, ignoring errors if they don't exist
  spawnSync('git', ['worktree', 'remove', name, '--force'], {
    cwd: WORKSPACE_PATH,
    encoding: 'utf-8',
  })
  spawnSync('git', ['branch', '-D', name], {
    cwd: WORKSPACE_PATH,
    encoding: 'utf-8',
  })
}

describe('CLI UX Integration', () => {
  beforeAll(() => {
    // Verify CLI is built
    if (!existsSync(CLI_PATH)) {
      throw new Error(`CLI not built. Run 'pnpm build' first. Expected: ${CLI_PATH}`)
    }
    // Clean up any leftover worktrees from previous test runs
    for (const name of TEST_WORKTREES) {
      cleanupWorktree(name)
    }
  })

  afterAll(() => {
    // Clean up worktrees created during tests
    for (const name of TEST_WORKTREES) {
      cleanupWorktree(name)
    }
  })

  describe('worktree use with existing worktree', () => {
    it('returns path when worktree exists', () => {
      // First create a test worktree
      const createResult = spawnSync('node', [CLI_PATH, 'worktree', 'create', 'test-worktree-use'], {
        cwd: WORKSPACE_PATH,
        encoding: 'utf-8',
      })
      expect(createResult.status).toBe(0)
      // Extract only the path (last line) from create output, ignoring logger messages
      const createdPath = createResult.stdout.trim().split('\n').pop() || ''

      // Now use it
      const result = spawnSync('node', [CLI_PATH, 'worktree', 'use', 'test-worktree-use'], {
        cwd: WORKSPACE_PATH,
        encoding: 'utf-8',
      })

      expect(result.status).toBe(0)
      // stdout should be the same absolute path
      const outputPath = result.stdout.trim()
      expect(outputPath).toBe(createdPath)
      expect(outputPath).toMatch(/^\//) // Starts with / (absolute path)
    })

    it('outputs only path to stdout (no logger messages)', () => {
      // Create a test worktree
      const createResult = spawnSync('node', [CLI_PATH, 'worktree', 'create', 'test-worktree-stdout'], {
        cwd: WORKSPACE_PATH,
        encoding: 'utf-8',
      })
      expect(createResult.status).toBe(0)
      // Extract only the path (last line), ignoring any logger messages during creation
      const createdPath = createResult.stdout.trim().split('\n').pop() || ''

      // Use it
      const result = spawnSync('node', [CLI_PATH, 'worktree', 'use', 'test-worktree-stdout'], {
        cwd: WORKSPACE_PATH,
        encoding: 'utf-8',
      })

      // stdout should be just the path, one line
      const lines = result.stdout.trim().split('\n')
      expect(lines).toHaveLength(1)
      expect(result.stdout).not.toContain('[') // No logger prefixes
      expect(result.stdout).not.toContain('ok')
      expect(result.stdout).not.toContain('info')
      // Verify it matches the path from create
      expect(result.stdout.trim()).toBe(createdPath)
    })
  })

  describe('worktree use error handling', () => {
    it('fails with exit code 1 for nonexistent worktree', () => {
      const result = spawnSync('node', [CLI_PATH, 'worktree', 'use', 'nonexistent-worktree-xyz'], {
        cwd: WORKSPACE_PATH,
        encoding: 'utf-8',
      })

      expect(result.status).toBe(1)
      // Error message goes to stderr (logger.error)
      expect(result.stderr).toContain('not found')
    })

    it('shows suggestion to use worktree create', () => {
      const result = spawnSync('node', [CLI_PATH, 'worktree', 'use', 'nonexistent-worktree-xyz'], {
        cwd: WORKSPACE_PATH,
        encoding: 'utf-8',
      })

      // Info message (suggestion) may go to stdout in non-TTY mode
      const combined = result.stdout + result.stderr
      expect(combined).toContain('worktree create')
    })
  })

  describe('agent spawn accessibility', () => {
    it('agent spawn command is accessible', () => {
      const result = spawnSync('node', [CLI_PATH, 'agent', 'spawn', '--help'], {
        encoding: 'utf-8',
      })

      expect(result.status).toBe(0)
      expect(result.stdout).toContain('spawn')
      expect(result.stdout).toContain('--worktree')
    })

    it('agent --help shows spawn subcommand', () => {
      const result = spawnSync('node', [CLI_PATH, 'agent', '--help'], {
        encoding: 'utf-8',
      })

      expect(result.status).toBe(0)
      expect(result.stdout).toContain('spawn')
    })
  })

  describe('help text accuracy', () => {
    it('worktree use --help shows --shell flag and examples', () => {
      const result = spawnSync('node', [CLI_PATH, 'worktree', 'use', '--help'], {
        encoding: 'utf-8',
      })

      expect(result.stdout).toContain('--shell')
      expect(result.stdout).toContain('cd $(crewchief worktree use')
      expect(result.stdout).not.toContain('--branch') // Removed option
      expect(result.stdout).not.toContain('--base-path') // Removed option
    })

    it('worktree create --help shows --shell flag and examples', () => {
      const result = spawnSync('node', [CLI_PATH, 'worktree', 'create', '--help'], {
        encoding: 'utf-8',
      })

      expect(result.stdout).toContain('--shell')
      expect(result.stdout).toContain('cd $(crewchief worktree create')
      expect(result.stdout).not.toContain('--no-cd') // Removed option
    })

    it('top-level --help does not show spawn command', () => {
      const result = spawnSync('node', [CLI_PATH, '--help'], {
        encoding: 'utf-8',
      })

      expect(result.status).toBe(0)
      // spawn should not be listed as a top-level command
      // It should only be under 'agent'
      const lines = result.stdout.split('\n')
      const commandLines = lines.filter((l) => l.match(/^\s{2}\w/)) // Lines with commands
      const hasTopLevelSpawn = commandLines.some((l) => l.trim().startsWith('spawn'))
      expect(hasTopLevelSpawn).toBe(false)
    })

    it('agent spawn --help shows all options', () => {
      const result = spawnSync('node', [CLI_PATH, 'agent', 'spawn', '--help'], {
        encoding: 'utf-8',
      })

      expect(result.stdout).toContain('--worktree')
      expect(result.stdout).toContain('--agent <name>')
      expect(result.stdout).toContain('--args <string>')
      expect(result.stdout).toContain('-v, --verbose')
    })
  })
})
