import { spawn } from 'node:child_process'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from 'vitest'
import { WorktreeService } from '../../git/worktrees'
import { logger } from '../../utils/logger'
import { displaySubshellMessage } from '../../utils/subshell-message'

// Mock modules
vi.mock('../../git/worktrees')
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
async function executeWorktreeUse(name: string, opts: { shell?: boolean; print?: boolean } = {}) {
  const wt = new WorktreeService()
  const list = await wt.listWorktrees()

  const matches = list.filter((item) => {
    const sel = name.trim()
    const byBranch = item.branch && item.branch === sel
    const byBaseName = path.basename(item.path) === sel
    let byPath = false
    try {
      const resolvedSel = path.resolve(sel)
      byPath = path.resolve(item.path) === resolvedSel || path.resolve(item.path).includes(resolvedSel)
    } catch {}
    return Boolean(byBranch || byBaseName || byPath)
  })

  if (matches.length === 0) {
    logger.error(`Worktree '${name}' not found.`)
    logger.info(`Create it with: crewchief worktree create ${name}`)
    process.exitCode = 1
    return
  }

  if (matches.length > 1) {
    logger.error(`Ambiguous selector '${name}'. Candidates:`)
    for (const m of matches) logger.info(`${m.path}${m.branch ? ` [${m.branch}]` : ''}`)
    process.exitCode = 1
    return
  }

  const targetPath = path.resolve(matches[0].path)
  const targetBranch = matches[0].branch

  if (opts.shell) {
    const shell = process.env.SHELL || '/bin/bash'
    const currentBranch = await wt.getCurrentBranch()
    const currentDir = process.cwd()

    if (targetBranch) {
      displaySubshellMessage({
        targetBranch: targetBranch,
        targetDirectory: path.relative(currentDir, targetPath) || path.basename(targetPath),
        sourceBranch: currentBranch,
        sourceDirectory: path.basename(currentDir),
        shell: path.basename(shell),
      })
    }

    spawn(shell, { stdio: 'inherit', cwd: targetPath, env: process.env })
  } else {
    process.stdout.write(targetPath + '\n')
  }
}

describe('worktree use', () => {
  let stdoutSpy: Mock
  let originalExitCode: number | undefined

  beforeEach(() => {
    vi.clearAllMocks()
    stdoutSpy = vi.spyOn(process.stdout, 'write').mockImplementation(() => true) as unknown as Mock
    originalExitCode = process.exitCode
    process.exitCode = undefined
  })

  afterEach(() => {
    stdoutSpy.mockRestore()
    process.exitCode = originalExitCode
  })

  it('prints path to stdout when worktree exists', async () => {
    vi.mocked(WorktreeService.prototype.listWorktrees).mockResolvedValue([
      { path: '/path/to/worktrees/feature-x', branch: 'feature-x' },
    ])

    await executeWorktreeUse('feature-x')

    expect(stdoutSpy).toHaveBeenCalledWith(path.resolve('/path/to/worktrees/feature-x') + '\n')
    expect(process.exitCode).toBeUndefined()
  })

  it('exits with code 1 when worktree does not exist', async () => {
    vi.mocked(WorktreeService.prototype.listWorktrees).mockResolvedValue([])

    await executeWorktreeUse('nonexistent')

    expect(logger.error).toHaveBeenCalledWith("Worktree 'nonexistent' not found.")
    expect(logger.info).toHaveBeenCalledWith('Create it with: crewchief worktree create nonexistent')
    expect(process.exitCode).toBe(1)
  })

  it('spawns shell with --shell flag', async () => {
    vi.mocked(WorktreeService.prototype.listWorktrees).mockResolvedValue([
      { path: '/path/to/worktrees/feature-x', branch: 'feature-x' },
    ])
    vi.mocked(WorktreeService.prototype.getCurrentBranch).mockResolvedValue('main')

    await executeWorktreeUse('feature-x', { shell: true })

    expect(spawn).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        stdio: 'inherit',
        cwd: path.resolve('/path/to/worktrees/feature-x'),
      }),
    )
    expect(displaySubshellMessage).toHaveBeenCalled()
    // Should NOT write to stdout when spawning shell
    expect(stdoutSpy).not.toHaveBeenCalled()
  })

  it('lists candidates when selector is ambiguous', async () => {
    // Both worktrees have the same directory basename
    vi.mocked(WorktreeService.prototype.listWorktrees).mockResolvedValue([
      { path: '/path/to/worktrees-1/feature', branch: 'feature-1' },
      { path: '/path/to/worktrees-2/feature', branch: 'feature-2' },
    ])

    await executeWorktreeUse('feature')

    expect(logger.error).toHaveBeenCalledWith("Ambiguous selector 'feature'. Candidates:")
    expect(logger.info).toHaveBeenCalledTimes(2)
    expect(process.exitCode).toBe(1)
  })

  it('--print flag is accepted (no-op alias)', async () => {
    vi.mocked(WorktreeService.prototype.listWorktrees).mockResolvedValue([
      { path: '/path/to/worktrees/feature-x', branch: 'feature-x' },
    ])

    // --print should behave the same as default (print path)
    await executeWorktreeUse('feature-x', { print: true })

    expect(stdoutSpy).toHaveBeenCalledWith(path.resolve('/path/to/worktrees/feature-x') + '\n')
    expect(process.exitCode).toBeUndefined()
  })

  it('matches worktree by branch name', async () => {
    vi.mocked(WorktreeService.prototype.listWorktrees).mockResolvedValue([
      { path: '/path/to/worktrees/my-feature', branch: 'my-feature-branch' },
    ])

    await executeWorktreeUse('my-feature-branch')

    expect(stdoutSpy).toHaveBeenCalledWith(path.resolve('/path/to/worktrees/my-feature') + '\n')
  })

  it('matches worktree by directory basename', async () => {
    vi.mocked(WorktreeService.prototype.listWorktrees).mockResolvedValue([
      { path: '/path/to/worktrees/my-worktree', branch: 'different-branch' },
    ])

    await executeWorktreeUse('my-worktree')

    expect(stdoutSpy).toHaveBeenCalledWith(path.resolve('/path/to/worktrees/my-worktree') + '\n')
  })
})
