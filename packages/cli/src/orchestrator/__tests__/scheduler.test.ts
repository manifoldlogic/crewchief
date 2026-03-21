import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { resolveAgent, resolvePlatform } from '../../agents/platforms'
import { busManager } from '../../bus'
import { loadConfig } from '../../config/loader'
import { WorktreeService, buildDeterministicBranchName } from '../../git/worktrees'
import type { TerminalProvider } from '../../terminal/interface'
import { deriveMaproomSocketPath } from '../../utils/worktree-metadata.js'
import { RunManager } from '../runManager'
import { Scheduler } from '../scheduler'
import type { SpawnOptions } from '../scheduler'

// ---------------------------------------------------------------------------
// Module mocks - must be hoisted before imports
// ---------------------------------------------------------------------------
vi.mock('../runManager', () => ({
  RunManager: vi.fn(),
}))

vi.mock('../../agents/platforms', () => ({
  resolvePlatform: vi.fn(),
  resolveAgent: vi.fn(),
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

vi.mock('../../utils/worktree-metadata.js', () => ({
  deriveMaproomSocketPath: vi.fn(),
}))

// ---------------------------------------------------------------------------
// Test setup
// ---------------------------------------------------------------------------
let mockTerminal: TerminalProvider
let mockRunManager: {
  createRun: ReturnType<typeof vi.fn>
  getRunBusPath: ReturnType<typeof vi.fn>
  getRunDir: ReturnType<typeof vi.fn>
}
let runCommandSpy: ReturnType<typeof vi.fn<[string, string], Promise<void>>>

beforeEach(() => {
  vi.clearAllMocks()

  // Re-setup mocked return values after clearAllMocks
  vi.mocked(deriveMaproomSocketPath).mockImplementation(
    (dir: string) => `/tmp/maproom-1000-${dir.split('/').pop()}.sock`,
  )

  vi.mocked(loadConfig).mockResolvedValue({
    repository: {
      mainBranch: 'main',
      worktreeBasePath: '/worktrees',
    },
  } as Awaited<ReturnType<typeof loadConfig>>)

  // Mock resolveAgent - default returns bare claude command
  vi.mocked(resolveAgent).mockReturnValue({
    platform: { name: 'claude', command: 'claude', agentDir: '.claude/agents', agentExtensions: ['.md'] },
    agentName: null,
    agentPath: null,
    command: 'claude',
  })

  // Mock resolvePlatform
  vi.mocked(resolvePlatform).mockReturnValue({
    name: 'claude',
    command: 'claude',
    agentDir: '.claude/agents',
    agentExtensions: ['.md'],
  })

  // Mock WorktreeService constructor
  vi.mocked(WorktreeService).mockImplementation(
    () =>
      ({
        createWorktree: vi.fn().mockResolvedValue('/path/to/worktree'),
      }) as unknown as InstanceType<typeof WorktreeService>,
  )

  vi.mocked(buildDeterministicBranchName).mockReturnValue('test-branch')

  // Create a mock RunManager instance directly (not via constructor)
  mockRunManager = {
    createRun: vi.fn().mockReturnValue({
      id: 'test-run-uuid',
      platform: 'claude',
      agentName: null,
      label: 'test-task__claude',
      task: 'test task',
      paneId: 'pane-1',
      workingDirectory: '/current/dir',
      branchName: null,
      status: 'running' as const,
      startedAt: new Date().toISOString(),
    }),
    getRunBusPath: vi.fn().mockImplementation((runId: string) => `/test/base/.crewchief/runs/${runId}/bus.jsonl`),
    getRunDir: vi.fn().mockImplementation((runId: string) => `/test/base/.crewchief/runs/${runId}`),
  }

  vi.mocked(RunManager).mockImplementation(() => mockRunManager as unknown as InstanceType<typeof RunManager>)

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
// spawnAgent tests
// ---------------------------------------------------------------------------
describe('Scheduler.spawnAgent', () => {
  describe('without worktree (default path)', () => {
    const defaultOptions: SpawnOptions = { useWorktree: false }

    it('uses process.cwd() as working directory', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('fix bug', 'claude', defaultOptions)

      // createWindow should receive cwd
      expect(mockTerminal.createWindow).toHaveBeenCalledWith(
        expect.objectContaining({
          workingDirectory: process.cwd(),
        }),
      )
    })

    it('does NOT call loadConfig', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('fix bug', 'claude', defaultOptions)

      expect(loadConfig).not.toHaveBeenCalled()
    })

    it('does NOT create a WorktreeService', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('fix bug', 'claude', defaultOptions)

      expect(WorktreeService).not.toHaveBeenCalled()
    })

    it('passes branchName: null to RunManager', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('fix bug', 'claude', defaultOptions)

      expect(mockRunManager.createRun).toHaveBeenCalledWith(
        'claude', // platform
        'fix bug', // task
        'pane-1', // paneId
        process.cwd(), // workingDirectory
        null, // branchName
        null, // agentName
        'fix-bug__claude', // label
      )
    })

    it('calls resolveAgent with cwd as projectDir', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('fix bug', 'claude', defaultOptions)

      expect(resolveAgent).toHaveBeenCalledWith('claude', undefined, process.cwd())
    })

    it('returns the run ID', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      const runId = await scheduler.spawnAgent('fix bug', 'claude', defaultOptions)

      expect(runId).toBe('test-run-uuid')
    })
  })

  describe('with worktree', () => {
    const worktreeOptions: SpawnOptions = { useWorktree: true }

    it('calls loadConfig', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('fix bug', 'claude', worktreeOptions)

      expect(loadConfig).toHaveBeenCalledTimes(1)
    })

    it('creates a WorktreeService and calls createWorktree', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('fix bug', 'claude', worktreeOptions)

      expect(WorktreeService).toHaveBeenCalled()
      const wtInstance = vi.mocked(WorktreeService).mock.results[0].value
      expect(wtInstance.createWorktree).toHaveBeenCalledWith('test-branch', 'main', '/worktrees')
    })

    it('passes branch name to RunManager', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('fix bug', 'claude', worktreeOptions)

      expect(mockRunManager.createRun).toHaveBeenCalledWith(
        'claude',
        'fix bug',
        'pane-1',
        '/path/to/worktree',
        'test-branch', // branchName from buildDeterministicBranchName
        null,
        'fix-bug__claude',
      )
    })

    it('uses worktree path as workingDirectory for terminal', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('fix bug', 'claude', worktreeOptions)

      expect(mockTerminal.createWindow).toHaveBeenCalledWith(
        expect.objectContaining({
          workingDirectory: '/path/to/worktree',
        }),
      )
    })

    it('calls resolveAgent without projectDir (worktree not yet created)', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('fix bug', 'claude', worktreeOptions)

      expect(resolveAgent).toHaveBeenCalledWith('claude', undefined, undefined)
    })

    it('throws when loadConfig fails (missing config)', async () => {
      vi.mocked(loadConfig).mockRejectedValue(new Error('Config file not found'))

      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await expect(scheduler.spawnAgent('fix bug', 'claude', worktreeOptions)).rejects.toThrow('Config file not found')
    })
  })

  describe('agent resolution', () => {
    it('passes agentName to resolveAgent when provided', async () => {
      vi.mocked(resolveAgent).mockReturnValue({
        platform: { name: 'claude', command: 'claude', agentDir: '.claude/agents', agentExtensions: ['.md'] },
        agentName: 'code-review',
        agentPath: '/project/.claude/agents/code-review.md',
        command: 'claude --agent /project/.claude/agents/code-review.md',
      })

      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('review PR', 'claude', { useWorktree: false, agentName: 'code-review' })

      expect(resolveAgent).toHaveBeenCalledWith('claude', 'code-review', process.cwd())
    })

    it('records agentName in RunManager when provided', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('review PR', 'claude', { useWorktree: false, agentName: 'code-review' })

      expect(mockRunManager.createRun).toHaveBeenCalledWith(
        'claude',
        'review PR',
        'pane-1',
        process.cwd(),
        null,
        'code-review', // agentName recorded
        'review-pr__claude', // label
      )
    })

    it('uses resolved command for execution', async () => {
      vi.mocked(resolveAgent).mockReturnValue({
        platform: { name: 'claude', command: 'claude', agentDir: '.claude/agents', agentExtensions: ['.md'] },
        agentName: 'code-review',
        agentPath: '/project/.claude/agents/code-review.md',
        command: 'claude --agent /project/.claude/agents/code-review.md',
      })

      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('review PR', 'claude', { useWorktree: false, agentName: 'code-review' })

      const command = runCommandSpy.mock.calls[0][1] as string
      expect(command).toContain('claude --agent /project/.claude/agents/code-review.md')
    })
  })

  describe('unknown platform (custom fallback)', () => {
    it('resolveAgent handles unknown platforms gracefully', async () => {
      vi.mocked(resolveAgent).mockReturnValue({
        platform: { name: 'mycustomtool', command: 'mycustomtool', agentDir: null, agentExtensions: [] },
        agentName: null,
        agentPath: null,
        command: 'mycustomtool',
      })

      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      const runId = await scheduler.spawnAgent('do stuff', 'mycustomtool', { useWorktree: false })

      expect(runId).toBe('test-run-uuid')
      expect(resolveAgent).toHaveBeenCalledWith('mycustomtool', undefined, process.cwd())

      const command = runCommandSpy.mock.calls[0][1] as string
      expect(command).toContain('mycustomtool')
    })
  })

  describe('label generation', () => {
    it('generates label as slugify(task) + "__" + platformName', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('Fix Login Bug!', 'claude', { useWorktree: false })

      // slugify("Fix Login Bug!") => "fix-login-bug"
      expect(mockTerminal.createWindow).toHaveBeenCalledWith(
        expect.objectContaining({
          title: 'fix-login-bug__claude',
        }),
      )

      expect(mockRunManager.createRun).toHaveBeenCalledWith(
        'claude',
        'Fix Login Bug!',
        'pane-1',
        process.cwd(),
        null,
        null,
        'fix-login-bug__claude',
      )
    })
  })

  describe('terminal provider WindowOptions', () => {
    it('passes platform in WindowOptions', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('task', 'gemini', { useWorktree: false })

      expect(mockTerminal.createWindow).toHaveBeenCalledWith(
        expect.objectContaining({
          platform: 'gemini',
        }),
      )
    })

    it('passes title (label) in WindowOptions', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('my task', 'claude', { useWorktree: false })

      expect(mockTerminal.createWindow).toHaveBeenCalledWith(
        expect.objectContaining({
          title: 'my-task__claude',
        }),
      )
    })
  })

  describe('RunManager 7-param call', () => {
    it('calls createRun with all 7 parameters', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('my task', 'claude', { useWorktree: false, agentName: 'reviewer' })

      expect(mockRunManager.createRun).toHaveBeenCalledTimes(1)
      const args = mockRunManager.createRun.mock.calls[0]
      expect(args).toHaveLength(7)
      expect(args[0]).toBe('claude') // platform
      expect(args[1]).toBe('my task') // task
      expect(args[2]).toBe('pane-1') // paneId
      expect(args[3]).toBe(process.cwd()) // workingDirectory
      expect(args[4]).toBeNull() // branchName (no worktree)
      expect(args[5]).toBe('reviewer') // agentName
      expect(args[6]).toBe('my-task__claude') // label
    })
  })

  describe('bus integration', () => {
    it('calls busManager.startFollowing before terminal.runCommand', async () => {
      const callOrder: string[] = []

      vi.mocked(busManager.startFollowing).mockImplementation(() => {
        callOrder.push('startFollowing')
      })
      runCommandSpy.mockImplementation(async () => {
        callOrder.push('runCommand')
      })

      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('test task', 'claude', { useWorktree: false })

      expect(callOrder).toEqual(['startFollowing', 'runCommand'])
    })

    it('passes correct runId and busPath to startFollowing', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('test task', 'claude', { useWorktree: false })

      expect(busManager.startFollowing).toHaveBeenCalledTimes(1)
      expect(busManager.startFollowing).toHaveBeenCalledWith(
        'test-run-uuid',
        '/test/base/.crewchief/runs/test-run-uuid/bus.jsonl',
      )
    })

    it('includes CREWCHIEF_BUS_PATH in command string', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('test task', 'claude', { useWorktree: false })

      const command = runCommandSpy.mock.calls[0][1] as string
      expect(command).toContain('CREWCHIEF_BUS_PATH=')
    })

    it('includes MAPROOM_MCP_SOCKET in command string (Connection E)', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('test task', 'claude', { useWorktree: false })

      const command = runCommandSpy.mock.calls[0][1] as string
      expect(command).toContain('MAPROOM_MCP_SOCKET=')
    })

    it('derives MAPROOM_MCP_SOCKET from the effective working directory', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('test task', 'claude', { useWorktree: false })

      // deriveMaproomSocketPath should have been called with process.cwd()
      expect(deriveMaproomSocketPath).toHaveBeenCalledWith(process.cwd())
    })

    it('derives MAPROOM_MCP_SOCKET from worktree path when useWorktree is true', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('test task', 'claude', { useWorktree: true })

      // When using worktree, the effective working dir is the created worktree path
      expect(deriveMaproomSocketPath).toHaveBeenCalledWith('/path/to/worktree')
    })
  })

  describe('extraArgs', () => {
    it('appends extra args to the command', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('task', 'claude', { useWorktree: false, extraArgs: '--verbose --timeout 30' })

      const command = runCommandSpy.mock.calls[0][1] as string
      expect(command).toContain('claude --verbose --timeout 30')
    })

    it('does not append extra args when not provided', async () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      await scheduler.spawnAgent('task', 'claude', { useWorktree: false })

      const command = runCommandSpy.mock.calls[0][1] as string
      // Command should end with 'claude' without trailing args
      // (preceded by env vars CREWCHIEF_BUS_PATH and MAPROOM_MCP_SOCKET)
      expect(command).toMatch(/MAPROOM_MCP_SOCKET="[^"]*" claude$/)
    })
  })

  describe('constructor', () => {
    it('requires terminal provider', () => {
      // Scheduler constructor requires terminal provider as first argument
      const scheduler = new Scheduler(mockTerminal)
      expect(scheduler).toBeDefined()
    })

    it('accepts optional RunManager', () => {
      const scheduler = new Scheduler(mockTerminal, mockRunManager as unknown as RunManager)
      expect(scheduler).toBeDefined()
    })

    it('falls back to new RunManager() when not provided', async () => {
      const scheduler = new Scheduler(mockTerminal)
      await scheduler.spawnAgent('task', 'claude', { useWorktree: false })

      // RunManager constructor should have been called (fallback)
      expect(RunManager).toHaveBeenCalled()
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
    const { BusManager } = await import('../../bus/busManager')
    const { CrossProcessBusWriter } = await import('../../bus/crossProcessBusWriter')
    const path = await import('node:path')

    const busPath = path.join(tmpDir, 'bus.jsonl')
    const messages: Array<{ type: string; from: string; payload: unknown }> = []

    const manager = new BusManager({
      onMessage: (msg) => messages.push(msg),
      intervalMs: 25,
    })

    manager.startFollowing('test-run', busPath)

    const writer = new CrossProcessBusWriter(busPath)
    writer.write({
      type: 'status',
      from: 'test-agent',
      to: 'orchestrator',
      payload: { activity: 'starting up' },
      timestamp: new Date(),
    })

    await new Promise((resolve) => setTimeout(resolve, 100))

    manager.stopAll()

    expect(messages).toHaveLength(1)
    expect(messages[0].type).toBe('status')
    expect(messages[0].from).toBe('test-agent')
    expect((messages[0].payload as { activity: string }).activity).toBe('starting up')
  })

  it('early messages not missed when following starts before write', async () => {
    const { BusManager } = await import('../../bus/busManager')
    const { CrossProcessBusWriter } = await import('../../bus/crossProcessBusWriter')
    const path = await import('node:path')

    const busPath = path.join(tmpDir, 'early-msg-bus.jsonl')
    const messages: Array<{ type: string; from: string; payload: unknown }> = []

    const manager = new BusManager({
      onMessage: (msg) => messages.push(msg),
      intervalMs: 25,
    })

    manager.startFollowing('early-run', busPath)

    await new Promise((resolve) => setTimeout(resolve, 50))

    const writer = new CrossProcessBusWriter(busPath)
    writer.write({
      type: 'status',
      from: 'early-agent',
      to: 'orchestrator',
      payload: { activity: 'immediate write' },
      timestamp: new Date(),
    })

    await new Promise((resolve) => setTimeout(resolve, 100))

    manager.stopAll()

    expect(messages).toHaveLength(1)
    expect(messages[0].from).toBe('early-agent')
    expect((messages[0].payload as { activity: string }).activity).toBe('immediate write')
  })
})
