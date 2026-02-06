import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { getAgentType } from '../../agents/registry'
import { busManager } from '../../bus'
import { loadConfig } from '../../config/loader'
import { WorktreeService, buildDeterministicBranchName } from '../../git/worktrees'
import type { TerminalProvider } from '../../terminal/interface'
import { RunManager } from '../runManager'
import { Scheduler } from '../scheduler'

// ---------------------------------------------------------------------------
// Module mocks - must be hoisted before imports
// ---------------------------------------------------------------------------
vi.mock('../runManager', () => ({
  RunManager: vi.fn(),
}))

vi.mock('../../agents/registry', () => ({
  getAgentType: vi.fn(),
}))

vi.mock('../../config/loader', () => ({
  loadConfig: vi.fn(),
}))

vi.mock('../../git/worktrees', () => ({
  WorktreeService: vi.fn(),
  buildDeterministicBranchName: vi.fn(),
}))

// Mock busManager with vi.fn() calls that can be spied on
vi.mock('../../bus', () => ({
  busManager: {
    startFollowing: vi.fn(),
    stopFollowing: vi.fn(),
    stopAll: vi.fn(),
    isFollowing: vi.fn().mockReturnValue(false),
    activeRunIds: [],
  },
}))

// ---------------------------------------------------------------------------
// Test setup
// ---------------------------------------------------------------------------
let mockTerminal: TerminalProvider
let runCommandSpy: ReturnType<typeof vi.fn<[string, string], Promise<void>>>

beforeEach(() => {
  vi.clearAllMocks()

  // Re-setup mocked return values after clearAllMocks
  vi.mocked(loadConfig).mockResolvedValue({
    repository: {
      mainBranch: 'main',
      worktreeBasePath: '/worktrees',
    },
  } as Awaited<ReturnType<typeof loadConfig>>)

  vi.mocked(getAgentType).mockReturnValue({
    id: 'claude',
    name: 'Claude',
    platform: 'claude',
    capabilities: ['code-generation', 'code-review'],
    agentDefinitionPath: '/path/to/agent.yaml',
    executionCommand: 'claude --agent',
  })

  // Mock WorktreeService constructor
  vi.mocked(WorktreeService).mockImplementation(
    () =>
      ({
        createWorktree: vi.fn().mockResolvedValue('/path/to/worktree'),
      }) as unknown as InstanceType<typeof WorktreeService>,
  )

  vi.mocked(buildDeterministicBranchName).mockReturnValue('test-branch')

  // Mock RunManager constructor
  vi.mocked(RunManager).mockImplementation(
    () =>
      ({
        createRun: vi.fn().mockReturnValue({
          id: 'test-run-uuid',
          agentTypeId: 'claude',
          task: 'test task',
          paneId: 'pane-1',
          worktreePath: '/path/to/worktree',
          branchName: 'test-branch',
          status: 'running' as const,
          startedAt: new Date().toISOString(),
        }),
        getRunBusPath: vi.fn().mockImplementation((runId: string) => `/test/base/.crewchief/runs/${runId}/bus.jsonl`),
        getRunDir: vi.fn().mockImplementation((runId: string) => `/test/base/.crewchief/runs/${runId}`),
      }) as unknown as InstanceType<typeof RunManager>,
  )

  runCommandSpy = vi.fn<[string, string], Promise<void>>().mockResolvedValue(undefined)

  mockTerminal = {
    id: 'mock',
    initialize: vi.fn().mockResolvedValue(undefined),
    dispose: vi.fn().mockResolvedValue(undefined),
    createWindow: vi.fn().mockResolvedValue('pane-1'),
    createTab: vi.fn().mockResolvedValue('pane-1'),
    splitPane: vi.fn().mockResolvedValue('pane-2'),
    runCommand: runCommandSpy,
    focus: vi.fn().mockResolvedValue(undefined),
  }
})

afterEach(() => {
  vi.restoreAllMocks()
})

// ---------------------------------------------------------------------------
// Bus integration tests
// ---------------------------------------------------------------------------
describe('Scheduler', () => {
  describe('bus integration', () => {
    it('calls busManager.startFollowing before terminal.runCommand', async () => {
      const callOrder: string[] = []

      // Track call order using the mocked busManager
      vi.mocked(busManager.startFollowing).mockImplementation(() => {
        callOrder.push('startFollowing')
      })
      runCommandSpy.mockImplementation(async () => {
        callOrder.push('runCommand')
      })

      const scheduler = new Scheduler(mockTerminal)
      await scheduler.assignSingleAgent('test task', 'claude')

      expect(callOrder).toEqual(['startFollowing', 'runCommand'])
    })

    it('passes correct runId and busPath to startFollowing', async () => {
      const scheduler = new Scheduler(mockTerminal)
      await scheduler.assignSingleAgent('test task', 'claude')

      expect(busManager.startFollowing).toHaveBeenCalledTimes(1)
      expect(busManager.startFollowing).toHaveBeenCalledWith(
        'test-run-uuid',
        '/test/base/.crewchief/runs/test-run-uuid/bus.jsonl',
      )
    })

    it('includes CREWCHIEF_BUS_PATH in command string', async () => {
      const scheduler = new Scheduler(mockTerminal)
      await scheduler.assignSingleAgent('test task', 'claude')

      expect(runCommandSpy).toHaveBeenCalledTimes(1)
      const command = runCommandSpy.mock.calls[0][1] as string
      expect(command).toContain('CREWCHIEF_BUS_PATH=')
    })

    it('CREWCHIEF_BUS_PATH value matches getRunBusPath output', async () => {
      const scheduler = new Scheduler(mockTerminal)
      await scheduler.assignSingleAgent('test task', 'claude')

      const command = runCommandSpy.mock.calls[0][1] as string
      const expectedBusPath = '/test/base/.crewchief/runs/test-run-uuid/bus.jsonl'

      // The bus path should be JSON-escaped in the command
      expect(command).toContain(`CREWCHIEF_BUS_PATH=${JSON.stringify(expectedBusPath)}`)
    })

    it('command string properly quotes/escapes bus path', async () => {
      const scheduler = new Scheduler(mockTerminal)
      await scheduler.assignSingleAgent('test task', 'claude')

      const command = runCommandSpy.mock.calls[0][1] as string

      // Should have the format: cd "..." && CREWCHIEF_BUS_PATH="..." exec
      expect(command).toMatch(/cd\s+".+" && CREWCHIEF_BUS_PATH="[^"]+" .+/)
    })

    it('startFollowing is called after createRun', async () => {
      // Import RunManager to check createRun was called
      const { RunManager } = await import('../runManager')

      const scheduler = new Scheduler(mockTerminal)
      await scheduler.assignSingleAgent('test task', 'claude')

      // Both should have been called
      const rmInstance = (RunManager as ReturnType<typeof vi.fn>).mock.results[0].value
      expect(rmInstance.createRun).toHaveBeenCalledTimes(1)
      expect(busManager.startFollowing).toHaveBeenCalledTimes(1)

      // startFollowing uses the run ID from createRun
      expect(busManager.startFollowing).toHaveBeenCalledWith('test-run-uuid', expect.stringContaining('test-run-uuid'))
    })
  })

  describe('command construction', () => {
    it('command includes cd to worktree path', async () => {
      const scheduler = new Scheduler(mockTerminal)
      await scheduler.assignSingleAgent('test task', 'claude')

      const command = runCommandSpy.mock.calls[0][1] as string
      expect(command).toContain('cd "/path/to/worktree"')
    })

    it('command includes agent execution after bus path env var', async () => {
      const scheduler = new Scheduler(mockTerminal)
      await scheduler.assignSingleAgent('test task', 'claude')

      const command = runCommandSpy.mock.calls[0][1] as string

      // The command should end with the execution command
      expect(command).toContain('claude --agent')

      // Bus path should come before execution command
      const busPathIndex = command.indexOf('CREWCHIEF_BUS_PATH=')
      const execIndex = command.indexOf('claude --agent')
      expect(busPathIndex).toBeLessThan(execIndex)
    })

    it('uses && to chain commands', async () => {
      const scheduler = new Scheduler(mockTerminal)
      await scheduler.assignSingleAgent('test task', 'claude')

      const command = runCommandSpy.mock.calls[0][1] as string
      expect(command).toContain(' && ')
    })
  })

  describe('return value', () => {
    it('returns run id', async () => {
      const scheduler = new Scheduler(mockTerminal)
      const runId = await scheduler.assignSingleAgent('test task', 'claude')

      expect(runId).toBe('test-run-uuid')
    })
  })
})

// ---------------------------------------------------------------------------
// Integration tests for end-to-end bus flow
// ---------------------------------------------------------------------------
describe('Scheduler bus integration (E2E)', () => {
  let tmpDir: string

  beforeEach(async () => {
    const fs = await import('node:fs')
    const os = await import('node:os')
    const path = await import('node:path')
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'scheduler-e2e-'))
  })

  afterEach(async () => {
    const fs = await import('node:fs')
    fs.rmSync(tmpDir, { recursive: true, force: true })
  })

  it('end-to-end: messages written via helper are readable by BusManager', async () => {
    // This test simulates the full flow:
    // 1. Agent writes via CrossProcessBusWriter (what agentBusHelper uses)
    // 2. BusManager reads via CrossProcessBusReader
    // This validates the scheduler integration would work
    const { BusManager } = await import('../../bus/busManager')
    const { CrossProcessBusWriter } = await import('../../bus/crossProcessBusWriter')
    const path = await import('node:path')

    const busPath = path.join(tmpDir, 'bus.jsonl')
    const messages: Array<{ type: string; from: string; payload: unknown }> = []

    const manager = new BusManager({
      onMessage: (msg) => messages.push(msg),
      intervalMs: 25,
    })

    // Simulate scheduler starting to follow BEFORE agent runs
    manager.startFollowing('test-run', busPath)

    // Simulate agent writing immediately (as agentBusHelper would)
    const writer = new CrossProcessBusWriter(busPath)
    writer.write({
      type: 'status',
      from: 'test-agent',
      to: 'orchestrator',
      payload: { activity: 'starting up' },
      timestamp: new Date(),
    })

    // Wait for polling to pick up the message
    await new Promise((resolve) => setTimeout(resolve, 100))

    manager.stopAll()

    expect(messages).toHaveLength(1)
    expect(messages[0].type).toBe('status')
    expect(messages[0].from).toBe('test-agent')
    expect((messages[0].payload as { activity: string }).activity).toBe('starting up')
  })

  it('early messages not missed when following starts before write', async () => {
    // This tests the critical timing requirement:
    // busManager.startFollowing() is called BEFORE the agent writes
    // The LogFollower handles file-not-yet-existing gracefully
    const { BusManager } = await import('../../bus/busManager')
    const { CrossProcessBusWriter } = await import('../../bus/crossProcessBusWriter')
    const path = await import('node:path')

    const busPath = path.join(tmpDir, 'early-msg-bus.jsonl')
    const messages: Array<{ type: string; from: string; payload: unknown }> = []

    const manager = new BusManager({
      onMessage: (msg) => messages.push(msg),
      intervalMs: 25,
    })

    // Start following BEFORE file exists (mimics scheduler behavior)
    manager.startFollowing('early-run', busPath)

    // Small delay to ensure reader is polling
    await new Promise((resolve) => setTimeout(resolve, 50))

    // Now agent writes immediately (simulating immediate write on spawn)
    const writer = new CrossProcessBusWriter(busPath)
    writer.write({
      type: 'status',
      from: 'early-agent',
      to: 'orchestrator',
      payload: { activity: 'immediate write' },
      timestamp: new Date(),
    })

    // Wait for polling
    await new Promise((resolve) => setTimeout(resolve, 100))

    manager.stopAll()

    // Early message should have been captured
    expect(messages).toHaveLength(1)
    expect(messages[0].from).toBe('early-agent')
    expect((messages[0].payload as { activity: string }).activity).toBe('immediate write')
  })
})
