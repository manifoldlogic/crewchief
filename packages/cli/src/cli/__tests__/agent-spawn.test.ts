import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { TerminalSchema } from '../../config/schema'
import { RunManager } from '../../orchestrator/runManager'
import { Scheduler, SpawnOptions } from '../../orchestrator/scheduler'
import { TerminalFactory } from '../../terminal/factory'
import { validateBackend, VALID_BACKENDS } from '../agent'

// Mock modules
vi.mock('../../orchestrator/scheduler')
vi.mock('../../orchestrator/runManager')

vi.mock('../../terminal/factory', () => ({
  TerminalFactory: {
    autoDetect: vi.fn(() => ({
      id: 'mock',
      initialize: vi.fn().mockResolvedValue(undefined),
    })),
    getProvider: vi.fn((id: string) => ({
      id,
      initialize: vi.fn().mockResolvedValue(undefined),
    })),
  },
}))

/**
 * Helper to simulate the new spawn action handler logic.
 * Mirrors the platform-based spawning logic from agent.ts.
 */
async function executeAgentSpawn(
  platformsStr: string,
  task: string | undefined,
  options: {
    worktree?: boolean
    agent?: string
    args?: string
    verbose?: boolean
  } = {},
) {
  // 1. Parse platforms
  const platforms = platformsStr
    .split(',')
    .map((p) => p.trim())
    .filter((p) => p.length > 0)

  if (platforms.length === 0) {
    throw new Error('No valid platforms specified')
  }

  // 2. Generate task name if not provided
  const effectiveTask = task || `task-${Date.now()}`

  // 3. Create scheduler
  const terminal = TerminalFactory.autoDetect()
  const runManager = new RunManager()
  const scheduler = new Scheduler(terminal, runManager)

  // 4. Spawn each platform sequentially
  const results: Array<{ platform: string; runId?: string; error?: string }> = []
  for (const platform of platforms) {
    try {
      const spawnOptions: SpawnOptions = {
        useWorktree: options.worktree ?? false,
        agentName: options.agent,
        verbose: options.verbose,
        extraArgs: options.args,
      }
      const runId = await scheduler.spawnAgent(effectiveTask, platform, spawnOptions)
      results.push({ platform, runId })
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error)
      results.push({ platform, error: message })
    }
  }

  return { results, platforms, effectiveTask }
}

describe('agent spawn', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(Scheduler.prototype.spawnAgent).mockResolvedValue('run-123')
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('spawns single platform via scheduler.spawnAgent()', async () => {
    const result = await executeAgentSpawn('claude', 'fix login bug')

    expect(TerminalFactory.autoDetect).toHaveBeenCalled()
    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledWith('fix login bug', 'claude', {
      useWorktree: false,
      agentName: undefined,
      verbose: undefined,
      extraArgs: undefined,
    })
    expect(result.results[0].runId).toBe('run-123')
  })

  it('spawns multiple platforms sequentially', async () => {
    const result = await executeAgentSpawn('claude,gemini', 'review code')

    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledTimes(2)
    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledWith(
      'review code',
      'claude',
      expect.objectContaining({ useWorktree: false }),
    )
    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledWith(
      'review code',
      'gemini',
      expect.objectContaining({ useWorktree: false }),
    )
    expect(result.results).toHaveLength(2)
    expect(result.results[0].runId).toBe('run-123')
    expect(result.results[1].runId).toBe('run-123')
  })

  it('generates default task name with task- prefix when none provided', async () => {
    const before = Date.now()
    await executeAgentSpawn('claude', undefined)
    const after = Date.now()

    const call = vi.mocked(Scheduler.prototype.spawnAgent).mock.calls[0]
    const taskName = call[0]

    expect(taskName).toMatch(/^task-\d+$/)
    const timestamp = parseInt(taskName.split('-')[1])
    expect(timestamp).toBeGreaterThanOrEqual(before)
    expect(timestamp).toBeLessThanOrEqual(after)
  })

  it('throws error when no valid platforms specified', async () => {
    await expect(executeAgentSpawn('', undefined)).rejects.toThrow('No valid platforms specified')
  })

  it('trims whitespace from platform names', async () => {
    await executeAgentSpawn('  claude  ,  gemini  ', 'test task')

    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledWith('test task', 'claude', expect.any(Object))
    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledWith('test task', 'gemini', expect.any(Object))
  })

  it('filters empty platform names', async () => {
    await executeAgentSpawn('claude,,gemini,', 'test task')

    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledTimes(2)
    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledWith('test task', 'claude', expect.any(Object))
    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledWith('test task', 'gemini', expect.any(Object))
  })

  it('passes --worktree flag as useWorktree option', async () => {
    await executeAgentSpawn('claude', 'fix bug', { worktree: true })

    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledWith(
      'fix bug',
      'claude',
      expect.objectContaining({ useWorktree: true }),
    )
  })

  it('passes --agent flag as agentName option', async () => {
    await executeAgentSpawn('claude', 'fix bug', { agent: 'backend-developer' })

    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledWith(
      'fix bug',
      'claude',
      expect.objectContaining({ agentName: 'backend-developer' }),
    )
  })

  it('passes --args flag as extraArgs option', async () => {
    await executeAgentSpawn('claude', 'fix bug', { args: '--dangerously-skip-permissions' })

    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledWith(
      'fix bug',
      'claude',
      expect.objectContaining({ extraArgs: '--dangerously-skip-permissions' }),
    )
  })

  it('passes --verbose flag', async () => {
    await executeAgentSpawn('claude', 'fix bug', { verbose: true })

    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledWith(
      'fix bug',
      'claude',
      expect.objectContaining({ verbose: true }),
    )
  })

  it('continues spawning remaining platforms when one fails', async () => {
    vi.mocked(Scheduler.prototype.spawnAgent)
      .mockRejectedValueOnce(new Error('platform not found'))
      .mockResolvedValueOnce('run-456')

    const result = await executeAgentSpawn('badplatform,claude', 'test task')

    expect(Scheduler.prototype.spawnAgent).toHaveBeenCalledTimes(2)
    expect(result.results[0].error).toBe('platform not found')
    expect(result.results[1].runId).toBe('run-456')
  })

  it('always uses TerminalFactory.autoDetect()', async () => {
    await executeAgentSpawn('claude', 'task')

    expect(TerminalFactory.autoDetect).toHaveBeenCalled()
    // No longer calls getProvider since --backend flag is removed
    expect(TerminalFactory.getProvider).not.toHaveBeenCalled()
  })
})

describe('backend validation', () => {
  describe('validateBackend()', () => {
    it('validates known backend values', () => {
      expect(validateBackend('iterm')).toBe(true)
      expect(validateBackend('tmux')).toBe(true)
      expect(validateBackend('headless')).toBe(true)
      expect(validateBackend('auto')).toBe(true)
    })

    it('rejects unknown backend values', () => {
      expect(validateBackend('screen')).toBe(false)
      expect(validateBackend('wezterm')).toBe(false)
      expect(validateBackend('kitty')).toBe(false)
      expect(validateBackend('')).toBe(false)
      expect(validateBackend('ITERM')).toBe(false)
    })
  })

  describe('VALID_BACKENDS constant', () => {
    it('contains expected backends', () => {
      expect(VALID_BACKENDS).toContain('iterm')
      expect(VALID_BACKENDS).toContain('tmux')
      expect(VALID_BACKENDS).toContain('headless')
      expect(VALID_BACKENDS).toContain('auto')
    })

    it('has exactly 4 entries', () => {
      expect(VALID_BACKENDS).toHaveLength(4)
    })
  })
})

describe('configuration migration', () => {
  it('existing configs with backend: iterm still work', () => {
    const config = {
      backend: 'iterm' as const,
      iterm: { sessionName: 'my-session' },
    }

    const parsed = TerminalSchema.parse(config)
    expect(parsed.backend).toBe('iterm')
    expect(parsed.iterm?.sessionName).toBe('my-session')
  })

  it('existing configs with backend: auto still work', () => {
    const config = {
      backend: 'auto' as const,
    }

    const parsed = TerminalSchema.parse(config)
    expect(parsed.backend).toBe('auto')
  })

  it('empty config defaults to auto', () => {
    const parsed = TerminalSchema.parse({})
    expect(parsed.backend).toBe('auto')
  })

  it('new config with backend: tmux works', () => {
    const config = {
      backend: 'tmux' as const,
      tmux: { sessionName: 'custom' },
    }

    const parsed = TerminalSchema.parse(config)
    expect(parsed.backend).toBe('tmux')
    expect(parsed.tmux?.sessionName).toBe('custom')
  })

  it('new config with backend: headless works', () => {
    const config = {
      backend: 'headless' as const,
    }

    const parsed = TerminalSchema.parse(config)
    expect(parsed.backend).toBe('headless')
  })
})
