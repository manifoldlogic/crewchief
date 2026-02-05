import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { TerminalSchema } from '../../config/schema'
import { Scheduler } from '../../orchestrator/scheduler'
import { TerminalFactory } from '../../terminal/factory'
import { validateBackend, VALID_BACKENDS } from '../agent'

// Mock modules
vi.mock('../../orchestrator/scheduler')

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
 * Helper to simulate the spawn action handler logic.
 * Mirrors the backend selection logic from agent.ts.
 */
async function executeAgentSpawn(
  agents: string,
  task: string | undefined,
  options: {
    name?: string
    vertical?: boolean
    args?: string
    noLabel?: boolean
    backend?: string
    headless?: boolean
  } = {},
) {
  // Validate --backend option if provided
  if (options.backend && !validateBackend(options.backend)) {
    throw new Error(`Invalid backend: ${options.backend}\nValid options: ${VALID_BACKENDS.join(', ')}`)
  }

  // Determine terminal provider: --headless > --backend > auto-detect
  let terminal
  if (options.headless) {
    terminal = TerminalFactory.getProvider('headless')
  } else if (options.backend && options.backend !== 'auto') {
    terminal = TerminalFactory.getProvider(options.backend as 'iterm' | 'tmux' | 'headless')
  } else {
    terminal = TerminalFactory.autoDetect()
  }

  await terminal.initialize()

  const scheduler = new Scheduler(terminal)

  const agentTypes = agents
    .split(',')
    .map((a) => a.trim())
    .filter((a) => a.length > 0)

  if (agentTypes.length === 0) {
    throw new Error('No valid agent types specified')
  }

  const effectiveTask = task || options.name || `agent-${Date.now()}`

  if (agentTypes.length === 1) {
    const runId = await scheduler.assignSingleAgent(effectiveTask, agentTypes[0])
    return { runId, agentTypes, effectiveTask }
  } else {
    const results = await Promise.allSettled(agentTypes.map((type) => scheduler.assignSingleAgent(effectiveTask, type)))
    return { results, agentTypes, effectiveTask }
  }
}

describe('agent spawn', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(Scheduler.prototype.assignSingleAgent).mockResolvedValue('run-123')
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('spawns single agent via scheduler', async () => {
    const result = await executeAgentSpawn('claude', 'fix login bug')

    expect(TerminalFactory.autoDetect).toHaveBeenCalled()
    expect(Scheduler.prototype.assignSingleAgent).toHaveBeenCalledWith('fix login bug', 'claude')
    expect(result.runId).toBe('run-123')
  })

  it('spawns multiple agents when comma-separated', async () => {
    await executeAgentSpawn('claude,gemini', 'review code')

    expect(Scheduler.prototype.assignSingleAgent).toHaveBeenCalledTimes(2)
    expect(Scheduler.prototype.assignSingleAgent).toHaveBeenCalledWith('review code', 'claude')
    expect(Scheduler.prototype.assignSingleAgent).toHaveBeenCalledWith('review code', 'gemini')
  })

  it('uses custom name as task when provided', async () => {
    await executeAgentSpawn('claude', undefined, { name: 'my-custom-task' })

    expect(Scheduler.prototype.assignSingleAgent).toHaveBeenCalledWith('my-custom-task', 'claude')
  })

  it('generates default task name when none provided', async () => {
    const before = Date.now()
    await executeAgentSpawn('claude', undefined)
    const after = Date.now()

    const call = vi.mocked(Scheduler.prototype.assignSingleAgent).mock.calls[0]
    const taskName = call[0]

    expect(taskName).toMatch(/^agent-\d+$/)
    const timestamp = parseInt(taskName.split('-')[1])
    expect(timestamp).toBeGreaterThanOrEqual(before)
    expect(timestamp).toBeLessThanOrEqual(after)
  })

  it('throws error when no valid agent types specified', async () => {
    await expect(executeAgentSpawn('', undefined)).rejects.toThrow('No valid agent types specified')
  })

  it('trims whitespace from agent types', async () => {
    await executeAgentSpawn('  claude  ,  gemini  ', 'test task')

    expect(Scheduler.prototype.assignSingleAgent).toHaveBeenCalledWith('test task', 'claude')
    expect(Scheduler.prototype.assignSingleAgent).toHaveBeenCalledWith('test task', 'gemini')
  })

  it('filters empty agent types', async () => {
    await executeAgentSpawn('claude,,gemini,', 'test task')

    expect(Scheduler.prototype.assignSingleAgent).toHaveBeenCalledTimes(2)
    expect(Scheduler.prototype.assignSingleAgent).toHaveBeenCalledWith('test task', 'claude')
    expect(Scheduler.prototype.assignSingleAgent).toHaveBeenCalledWith('test task', 'gemini')
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

  describe('spawn with invalid backend', () => {
    it('rejects unknown backend in executeAgentSpawn', async () => {
      await expect(executeAgentSpawn('claude', 'task', { backend: 'screen' })).rejects.toThrow(
        'Invalid backend: screen',
      )
    })

    it('rejects wezterm backend', async () => {
      await expect(executeAgentSpawn('claude', 'task', { backend: 'wezterm' })).rejects.toThrow(
        'Invalid backend: wezterm',
      )
    })

    it('error message includes valid options', async () => {
      await expect(executeAgentSpawn('claude', 'task', { backend: 'bad' })).rejects.toThrow(
        'Valid options: iterm, tmux, headless, auto',
      )
    })
  })
})

describe('explicit backend selection', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(Scheduler.prototype.assignSingleAgent).mockResolvedValue('run-123')
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('uses getProvider when --backend tmux specified', async () => {
    await executeAgentSpawn('claude', 'task', { backend: 'tmux' })

    expect(TerminalFactory.getProvider).toHaveBeenCalledWith('tmux')
    expect(TerminalFactory.autoDetect).not.toHaveBeenCalled()
  })

  it('uses getProvider when --backend iterm specified', async () => {
    await executeAgentSpawn('claude', 'task', { backend: 'iterm' })

    expect(TerminalFactory.getProvider).toHaveBeenCalledWith('iterm')
    expect(TerminalFactory.autoDetect).not.toHaveBeenCalled()
  })

  it('uses getProvider when --backend headless specified', async () => {
    await executeAgentSpawn('claude', 'task', { backend: 'headless' })

    expect(TerminalFactory.getProvider).toHaveBeenCalledWith('headless')
    expect(TerminalFactory.autoDetect).not.toHaveBeenCalled()
  })

  it('uses autoDetect when --backend auto specified', async () => {
    await executeAgentSpawn('claude', 'task', { backend: 'auto' })

    expect(TerminalFactory.autoDetect).toHaveBeenCalled()
    expect(TerminalFactory.getProvider).not.toHaveBeenCalled()
  })

  it('uses autoDetect when no --backend specified', async () => {
    await executeAgentSpawn('claude', 'task')

    expect(TerminalFactory.autoDetect).toHaveBeenCalled()
    expect(TerminalFactory.getProvider).not.toHaveBeenCalled()
  })

  it('--headless flag forces headless provider', async () => {
    await executeAgentSpawn('claude', 'task', { headless: true })

    expect(TerminalFactory.getProvider).toHaveBeenCalledWith('headless')
    expect(TerminalFactory.autoDetect).not.toHaveBeenCalled()
  })

  it('--headless takes precedence over --backend tmux', async () => {
    await executeAgentSpawn('claude', 'task', { headless: true, backend: 'tmux' })

    expect(TerminalFactory.getProvider).toHaveBeenCalledWith('headless')
    // getProvider should only have been called once (for headless, not tmux)
    expect(TerminalFactory.getProvider).toHaveBeenCalledTimes(1)
  })

  it('--headless takes precedence over --backend iterm', async () => {
    await executeAgentSpawn('claude', 'task', { headless: true, backend: 'iterm' })

    expect(TerminalFactory.getProvider).toHaveBeenCalledWith('headless')
    expect(TerminalFactory.getProvider).toHaveBeenCalledTimes(1)
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
