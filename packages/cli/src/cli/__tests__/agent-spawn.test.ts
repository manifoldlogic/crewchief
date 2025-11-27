import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { Scheduler } from '../../orchestrator/scheduler'
import { TerminalFactory } from '../../terminal/factory'

// Mock modules
vi.mock('../../orchestrator/scheduler')
vi.mock('../../terminal/factory', () => ({
  TerminalFactory: {
    autoDetect: vi.fn(() => ({
      id: 'mock',
      initialize: vi.fn().mockResolvedValue(undefined),
    })),
  },
}))

// Helper to simulate the spawn action handler logic
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
  const terminal = TerminalFactory.autoDetect()
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
