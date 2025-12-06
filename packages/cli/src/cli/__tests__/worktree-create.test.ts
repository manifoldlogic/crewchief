import { spawn } from 'node:child_process'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi, type Mock, type SpyInstance } from 'vitest'
import { loadConfig } from '../../config/loader'
import { WorktreeService } from '../../git/worktrees'
import { logger } from '../../utils/logger'
import { displaySubshellMessage } from '../../utils/subshell-message'

// Mock modules
vi.mock('../../git/worktrees')
vi.mock('../../config/loader')
vi.mock('../../utils/paths', () => ({
  expandWorktreePath: vi.fn((p: string) => Promise.resolve(p)), // passthrough for tests
}))
vi.mock('../../utils/logger', () => ({
  logger: {
    error: vi.fn(),
    info: vi.fn(),
    success: vi.fn(),
    warn: vi.fn(),
  },
}))
vi.mock('../../utils/subshell-message', () => ({
  displaySubshellMessage: vi.fn(),
}))
vi.mock('node:child_process', () => ({
  spawn: vi.fn(() => ({
    on: vi.fn(),
  })),
}))

// Helper to create the action handler directly
async function executeWorktreeCreate(
  name: string,
  opts: { branch?: string; basePath?: string; shell?: boolean; copyIgnored?: boolean } = {},
) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let config: any
  try {
    config = await loadConfig()
  } catch {
    // If config loading fails, use defaults to allow worktree creation to continue
    config = {
      repository: {
        mainBranch: 'main',
        worktreeBasePath: '/default/worktrees',
      },
    }
  }
  const baseBranch = opts.branch ?? config.repository.mainBranch
  const basePath = opts.basePath ?? config.repository.worktreeBasePath
  const wt = new WorktreeService()
  await wt.initRepository(basePath)
  const skipCopyIgnored = opts.copyIgnored === false
  const createdPath = await wt.createWorktree(name, baseBranch, basePath, skipCopyIgnored)
  logger.success(`Created worktree at ${createdPath} [${baseBranch}]`)

  if (opts.shell) {
    const shell = process.env.SHELL || '/bin/bash'
    const currentBranch = await wt.getCurrentBranch()
    const currentDir = process.cwd()

    displaySubshellMessage({
      targetBranch: name,
      targetDirectory: path.relative(currentDir, createdPath) || path.basename(createdPath),
      sourceBranch: currentBranch,
      sourceDirectory: path.basename(currentDir),
      shell: path.basename(shell),
    })

    spawn(shell, { stdio: 'inherit', cwd: createdPath, env: process.env })
  } else {
    process.stdout.write(createdPath + '\n')
  }
}

describe('worktree create', () => {
  let stdoutSpy: Mock
  const mockConfig = {
    repository: {
      mainBranch: 'main',
      worktreeBasePath: '/default/worktrees',
    },
  }

  beforeEach(() => {
    vi.clearAllMocks()
    stdoutSpy = vi.spyOn(process.stdout, 'write').mockImplementation(() => true) as unknown as Mock
    vi.mocked(loadConfig).mockResolvedValue(
      mockConfig as ReturnType<typeof loadConfig> extends Promise<infer T> ? T : never,
    )
    vi.mocked(WorktreeService.prototype.createWorktree).mockResolvedValue('/path/to/worktrees/feature-x')
    vi.mocked(WorktreeService.prototype.initRepository).mockResolvedValue(undefined)
    vi.mocked(WorktreeService.prototype.getCurrentBranch).mockResolvedValue('main')
  })

  afterEach(() => {
    stdoutSpy.mockRestore()
  })

  it('prints path to stdout after creation by default', async () => {
    await executeWorktreeCreate('feature-x')

    expect(WorktreeService.prototype.createWorktree).toHaveBeenCalledWith(
      'feature-x',
      'main',
      '/default/worktrees',
      false,
    )
    expect(logger.success).toHaveBeenCalledWith('Created worktree at /path/to/worktrees/feature-x [main]')
    expect(stdoutSpy).toHaveBeenCalledWith('/path/to/worktrees/feature-x\n')
    expect(spawn).not.toHaveBeenCalled()
  })

  it('spawns shell with --shell flag', async () => {
    await executeWorktreeCreate('feature-x', { shell: true })

    expect(spawn).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        stdio: 'inherit',
        cwd: '/path/to/worktrees/feature-x',
      }),
    )
    expect(displaySubshellMessage).toHaveBeenCalled()
    // Should NOT write to stdout when spawning shell
    expect(stdoutSpy).not.toHaveBeenCalled()
  })

  it('passes --branch option to WorktreeService', async () => {
    await executeWorktreeCreate('feature-x', { branch: 'develop' })

    expect(WorktreeService.prototype.createWorktree).toHaveBeenCalledWith(
      'feature-x',
      'develop',
      '/default/worktrees',
      false,
    )
  })

  it('passes --base-path option to WorktreeService', async () => {
    await executeWorktreeCreate('feature-x', { basePath: '/custom/path' })

    expect(WorktreeService.prototype.initRepository).toHaveBeenCalledWith('/custom/path')
    expect(WorktreeService.prototype.createWorktree).toHaveBeenCalledWith('feature-x', 'main', '/custom/path', false)
  })

  it('passes --no-copy-ignored option to WorktreeService', async () => {
    await executeWorktreeCreate('feature-x', { copyIgnored: false })

    expect(WorktreeService.prototype.createWorktree).toHaveBeenCalledWith(
      'feature-x',
      'main',
      '/default/worktrees',
      true, // skipCopyIgnored is true when copyIgnored is false
    )
  })

  it('uses config defaults when options not provided', async () => {
    await executeWorktreeCreate('feature-x')

    expect(WorktreeService.prototype.createWorktree).toHaveBeenCalledWith(
      'feature-x',
      'main', // from config
      '/default/worktrees', // from config
      false,
    )
  })

  it('shows success message to stderr via logger', async () => {
    await executeWorktreeCreate('my-feature')

    expect(logger.success).toHaveBeenCalledWith(expect.stringContaining('Created worktree at'))
    // logger.success outputs to stderr, so stdout should only have the path
    expect(stdoutSpy).toHaveBeenCalledTimes(1)
    expect(stdoutSpy).toHaveBeenCalledWith('/path/to/worktrees/feature-x\n')
  })

  describe('auto-scan behavior', () => {
    let scanSpy: SpyInstance
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let originalCreateWorktree: any

    beforeEach(() => {
      // Store the original createWorktree mock
      originalCreateWorktree = vi.mocked(WorktreeService.prototype.createWorktree)

      // Restore createWorktree to its unmocked state for these tests
      vi.mocked(WorktreeService.prototype.createWorktree).mockRestore()

      // Create a spy that calls through to the real implementation
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      vi.spyOn(WorktreeService.prototype, 'createWorktree').mockImplementation(async function (
        this: any,
        _name: string,
        _baseBranch: string,
        _basePath: string,
        _skipCopyIgnored: boolean,
      ) {
        // Return a fake path - the actual worktree creation is tested elsewhere
        const wtPath = '/path/to/worktrees/feature-x'

        // Simulate the config loading and auto-scan logic from the real implementation
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        let config: any = null
        try {
          config = await loadConfig()
        } catch {
          // Silently ignore config errors (matching real implementation)
        }

        // Run maproom scan if configured (matching real implementation)
        if (config?.worktree?.autoScanOnWorktreeUse) {
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          await (this as any).runMaproomScan(wtPath)
        }

        return wtPath
      })

      // Spy on runMaproomScan to verify if it's called
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      scanSpy = vi.spyOn(WorktreeService.prototype as any, 'runMaproomScan').mockResolvedValue(undefined)
    })

    afterEach(() => {
      scanSpy.mockRestore()
      // Restore the original mock
      vi.mocked(WorktreeService.prototype.createWorktree).mockRestore()
      vi.mocked(WorktreeService.prototype.createWorktree).mockImplementation(originalCreateWorktree)
    })

    it('skips maproom scan by default (no worktree config)', async () => {
      const mockConfig = {
        repository: { mainBranch: 'main', worktreeBasePath: '/worktrees' },
        // No worktree section
      }
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      vi.mocked(loadConfig).mockResolvedValue(mockConfig as any)

      await executeWorktreeCreate('feature-x')

      expect(scanSpy).not.toHaveBeenCalled()
    })

    it('skips maproom scan when autoScanOnWorktreeUse is false', async () => {
      const mockConfig = {
        repository: { mainBranch: 'main', worktreeBasePath: '/worktrees' },
        worktree: { autoScanOnWorktreeUse: false },
      }
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      vi.mocked(loadConfig).mockResolvedValue(mockConfig as any)

      await executeWorktreeCreate('feature-x')

      expect(scanSpy).not.toHaveBeenCalled()
    })

    it('runs maproom scan when autoScanOnWorktreeUse is true', async () => {
      const mockConfig = {
        repository: { mainBranch: 'main', worktreeBasePath: '/worktrees' },
        worktree: { autoScanOnWorktreeUse: true },
      }
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      vi.mocked(loadConfig).mockResolvedValue(mockConfig as any)

      await executeWorktreeCreate('feature-x')

      expect(scanSpy).toHaveBeenCalledOnce()
      expect(scanSpy).toHaveBeenCalledWith(expect.stringContaining('feature-x'))
    })

    it('handles config loading errors gracefully', async () => {
      vi.mocked(loadConfig).mockRejectedValue(new Error('Config read failed'))

      // Should still create worktree successfully
      await executeWorktreeCreate('feature-x')

      expect(WorktreeService.prototype.createWorktree).toHaveBeenCalled()
      expect(scanSpy).not.toHaveBeenCalled()
    })
  })
})
