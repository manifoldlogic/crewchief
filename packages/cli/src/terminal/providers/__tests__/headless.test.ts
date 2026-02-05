import { mkdtemp, rm, stat, readFile, mkdir, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { HeadlessProvider, checkLogDirectorySize } from '../headless'

// ---------------------------------------------------------------------------
// Valid UUID fixtures for tests
// ---------------------------------------------------------------------------
const UUID_PERM = '00000000-0000-0000-0000-000000000001'
const UUID_DIR = '00000000-0000-0000-0000-000000000002'
const UUID_THREE = '00000000-0000-0000-0000-000000000003'
const UUID_STDOUT = '00000000-0000-0000-0000-000000000004'
const UUID_STDERR = '00000000-0000-0000-0000-000000000005'
const UUID_NOCROSS = '00000000-0000-0000-0000-000000000006'
const UUID_MIXED = '00000000-0000-0000-0000-000000000007'
const UUID_PERSIST = '00000000-0000-0000-0000-000000000008'
const UUID_PATH_STDOUT = '00000000-0000-0000-0000-000000000009'
const UUID_PATH_STDERR = '00000000-0000-0000-0000-00000000000a'
const UUID_PATH_DEFAULT = '00000000-0000-0000-0000-00000000000b'
const UUID_GETLOGS_FULL = '00000000-0000-0000-0000-00000000000c'
const UUID_GETLOGS_TAIL = '00000000-0000-0000-0000-00000000000d'
const UUID_BLOCKED = '11111111-1111-1111-1111-111111111111'
const UUID_SIZE = '00000000-0000-0000-0000-00000000000e'
const UUID_DISPOSE = '00000000-0000-0000-0000-00000000000f'
const UUID_INVALID_TRAVERSAL = '../../etc/passwd'
const UUID_STREAM_LARGE = '00000000-0000-0000-0000-000000000010'
const UUID_STREAM_EMPTY = '00000000-0000-0000-0000-000000000011'
const UUID_STREAM_SMALL = '00000000-0000-0000-0000-000000000012'
const UUID_STREAM_ORDER = '00000000-0000-0000-0000-000000000013'
const UUID_STREAM_SINGLE = '00000000-0000-0000-0000-000000000014'

describe('HeadlessProvider', () => {
  let provider: HeadlessProvider

  beforeEach(async () => {
    provider = new HeadlessProvider()
    await provider.initialize()
  })

  afterEach(async () => {
    await provider.dispose()
  })

  describe('initialization', () => {
    it('has correct provider id', () => {
      expect(provider.id).toBe('headless')
    })

    it('initializes without error', async () => {
      const newProvider = new HeadlessProvider()
      await expect(newProvider.initialize()).resolves.not.toThrow()
      await newProvider.dispose()
    })
  })

  describe('createWindow', () => {
    it('returns a window ID with timestamp pattern', async () => {
      const windowId = await provider.createWindow({ title: 'Test Window' })
      expect(windowId).toMatch(/^headless-window-\d+$/)
    })

    it('returns unique IDs for multiple windows', async () => {
      const id1 = await provider.createWindow()
      // Small delay to ensure different timestamp
      await new Promise((r) => setTimeout(r, 10))
      const id2 = await provider.createWindow()
      expect(id1).not.toBe(id2)
    })
  })

  describe('createTab', () => {
    it('returns a pane ID with counter pattern', async () => {
      const paneId = await provider.createTab('window-1')
      expect(paneId).toMatch(/^headless-pane-\d+$/)
    })

    it('increments counter for each tab', async () => {
      const id1 = await provider.createTab('window-1')
      const id2 = await provider.createTab('window-1')
      const num1 = parseInt(id1.split('-').pop()!, 10)
      const num2 = parseInt(id2.split('-').pop()!, 10)
      expect(num2).toBe(num1 + 1)
    })
  })

  describe('splitPane', () => {
    it('returns a pane ID for horizontal split', async () => {
      const paneId = await provider.splitPane('existing-pane', 'horizontal')
      expect(paneId).toMatch(/^headless-pane-\d+$/)
    })

    it('returns a pane ID for vertical split', async () => {
      const paneId = await provider.splitPane('existing-pane', 'vertical')
      expect(paneId).toMatch(/^headless-pane-\d+$/)
    })
  })

  describe('runCommand', () => {
    it('spawns a process and tracks it', async () => {
      const paneId = 'test-pane__claude'
      // Use sleep to keep agent in map for verification
      await provider.runCommand(paneId, 'sleep 5')
      await new Promise((r) => setTimeout(r, 100))

      const agents = await provider.listAgents()
      expect(agents.length).toBeGreaterThanOrEqual(1)
      const agent = agents.find((a) => a.name === paneId)
      expect(agent).toBeDefined()
    })

    it('parses agent type from pane ID', async () => {
      const paneId = 'my-task__gemini'
      // Use sleep to keep agent in map for verification
      await provider.runCommand(paneId, 'sleep 5')
      await new Promise((r) => setTimeout(r, 100))

      const agents = await provider.listAgents()
      const agent = agents.find((a) => a.name === paneId)
      expect(agent?.type).toBe('gemini')
    })

    it('sets type to unknown for non-standard pane IDs', async () => {
      const paneId = 'simple-pane'
      // Use sleep to keep agent in map for verification
      await provider.runCommand(paneId, 'sleep 5')
      await new Promise((r) => setTimeout(r, 100))

      const agents = await provider.listAgents()
      const agent = agents.find((a) => a.name === paneId)
      expect(agent?.type).toBe('unknown')
    })
  })

  describe('focus', () => {
    it('is a no-op but does not throw', async () => {
      await expect(provider.focus('any-pane')).resolves.toBeUndefined()
    })
  })

  describe('sendMessage', () => {
    it('returns false when agent not found', async () => {
      const result = await provider.sendMessage('nonexistent', 'hello')
      expect(result).toBe(false)
    })

    it('writes to stdin for tracked agent', async () => {
      const paneId = 'msg-test__claude'
      // Spawn a process that stays alive briefly
      await provider.runCommand(paneId, 'cat')
      await new Promise((r) => setTimeout(r, 100))

      const result = await provider.sendMessage(paneId, 'test message')
      expect(result).toBe(true)
    })

    it('returns false when process has exited (agent removed from map)', async () => {
      const paneId = 'exited-test__claude'
      // Spawn a process that exits immediately
      await provider.runCommand(paneId, 'echo done')
      // Wait for it to exit and be cleaned up from map
      await new Promise((r) => setTimeout(r, 500))

      const result = await provider.sendMessage(paneId, 'test message')
      expect(result).toBe(false)
    })
  })

  describe('listAgents', () => {
    it('returns empty array when no agents tracked', async () => {
      const agents = await provider.listAgents()
      expect(agents).toEqual([])
    })

    it('returns tracked agents with correct structure', async () => {
      const paneId = 'task__claude'
      await provider.runCommand(paneId, 'sleep 1')
      await new Promise((r) => setTimeout(r, 100))

      const agents = await provider.listAgents()
      expect(agents.length).toBe(1)
      expect(agents[0]).toMatchObject({
        id: paneId,
        name: paneId,
        type: 'claude',
        status: 'running',
      })
    })

    it('removes exited processes from agent map', async () => {
      const paneId = 'quick-task__claude'
      await provider.runCommand(paneId, 'echo done')
      // Wait for process to exit and cleanup
      await new Promise((r) => setTimeout(r, 500))

      const agents = await provider.listAgents()
      const agent = agents.find((a) => a.name === paneId)
      expect(agent).toBeUndefined()
    })

    it('tracks multiple agents independently', async () => {
      await provider.runCommand('agent1__claude', 'sleep 1')
      await provider.runCommand('agent2__gemini', 'sleep 1')
      await new Promise((r) => setTimeout(r, 100))

      const agents = await provider.listAgents()
      expect(agents.length).toBe(2)
      expect(agents.map((a) => a.type).sort()).toEqual(['claude', 'gemini'])
    })
  })

  describe('dispose', () => {
    it('kills all tracked agents', async () => {
      await provider.runCommand('dispose-test__claude', 'sleep 10')
      await new Promise((r) => setTimeout(r, 100))

      const beforeDispose = await provider.listAgents()
      expect(beforeDispose.length).toBe(1)

      await provider.dispose()

      // After dispose, the map should be cleared
      const afterDispose = await provider.listAgents()
      expect(afterDispose.length).toBe(0)
    })

    it('handles multiple agents during dispose', async () => {
      await provider.runCommand('multi1__claude', 'sleep 10')
      await provider.runCommand('multi2__gemini', 'sleep 10')
      await new Promise((r) => setTimeout(r, 100))

      // Should not throw with multiple agents
      await expect(provider.dispose()).resolves.not.toThrow()
    })

    it('handles empty agent list during dispose', async () => {
      // dispose with no agents should work fine
      await expect(provider.dispose()).resolves.not.toThrow()
    })
  })
})

describe('HeadlessProvider resource limits', () => {
  let provider: HeadlessProvider

  afterEach(async () => {
    await provider.dispose()
  })

  it('enforces max concurrent agents limit', async () => {
    provider = new HeadlessProvider({ maxConcurrentAgents: 2 })
    await provider.initialize()

    // Spawn 2 agents with long-running commands (should succeed)
    await provider.runCommand('agent1__claude', 'sleep 10')
    await provider.runCommand('agent2__claude', 'sleep 10')

    // Give time for processes to start
    await new Promise((r) => setTimeout(r, 100))

    // 3rd should fail
    await expect(provider.runCommand('agent3__claude', 'sleep 10')).rejects.toThrow(
      'Maximum concurrent agents (2) reached',
    )
  })

  it('includes current running count in error message', async () => {
    provider = new HeadlessProvider({ maxConcurrentAgents: 2 })
    await provider.initialize()

    await provider.runCommand('agent1__claude', 'sleep 10')
    await provider.runCommand('agent2__claude', 'sleep 10')
    await new Promise((r) => setTimeout(r, 100))

    await expect(provider.runCommand('agent3__claude', 'sleep 10')).rejects.toThrow('Currently running: 2')
  })

  it('includes cleanup suggestion in error message', async () => {
    provider = new HeadlessProvider({ maxConcurrentAgents: 1 })
    await provider.initialize()

    await provider.runCommand('agent1__claude', 'sleep 10')
    await new Promise((r) => setTimeout(r, 100))

    await expect(provider.runCommand('agent2__claude', 'sleep 10')).rejects.toThrow(
      'Stop or wait for agents to complete before spawning more.',
    )
  })

  it('allows spawning after agent exits (exited agents removed from map)', async () => {
    provider = new HeadlessProvider({ maxConcurrentAgents: 1 })
    await provider.initialize()

    // Spawn a quick-exit command
    await provider.runCommand('agent1__claude', 'true')
    // Wait for it to exit and be cleaned up
    await new Promise((r) => setTimeout(r, 500))

    // Verify agent has been removed from map after exit
    const agents = await provider.listAgents()
    const agent = agents.find((a) => a.name === 'agent1__claude')
    expect(agent).toBeUndefined()

    // Should succeed because the first agent was cleaned up
    await expect(provider.runCommand('agent2__claude', 'true')).resolves.not.toThrow()
  })

  it('uses default limit of 20 when not configured', () => {
    provider = new HeadlessProvider()
    // Access private field to verify default
    expect((provider as unknown as { maxConcurrentAgents: number }).maxConcurrentAgents).toBe(20)
  })

  it('respects custom limit from constructor', () => {
    provider = new HeadlessProvider({ maxConcurrentAgents: 50 })
    expect((provider as unknown as { maxConcurrentAgents: number }).maxConcurrentAgents).toBe(50)
  })

  it('allows spawning up to the exact limit', async () => {
    provider = new HeadlessProvider({ maxConcurrentAgents: 3 })
    await provider.initialize()

    // Spawn exactly 3 agents (the limit)
    await provider.runCommand('agent1__claude', 'sleep 10')
    await provider.runCommand('agent2__claude', 'sleep 10')
    await provider.runCommand('agent3__claude', 'sleep 10')
    await new Promise((r) => setTimeout(r, 100))

    // Verify all 3 are running
    const agents = await provider.listAgents()
    const running = agents.filter((a) => a.status === 'running')
    expect(running.length).toBe(3)

    // 4th should fail
    await expect(provider.runCommand('agent4__claude', 'sleep 10')).rejects.toThrow(
      'Maximum concurrent agents (3) reached',
    )
  })

  it('frees slot when agent exits allowing new spawn', async () => {
    provider = new HeadlessProvider({ maxConcurrentAgents: 2 })
    await provider.initialize()

    // Fill both slots: one quick-exit, one long-running
    await provider.runCommand('quick__claude', 'true')
    await provider.runCommand('long__claude', 'sleep 10')

    // Wait for quick agent to exit
    await new Promise((r) => setTimeout(r, 300))

    // quick agent exited, so only 1 running - should be able to spawn another
    await expect(provider.runCommand('new__claude', 'sleep 10')).resolves.not.toThrow()
  })
})

describe('HeadlessProvider agent cleanup', () => {
  let provider: HeadlessProvider

  beforeEach(async () => {
    provider = new HeadlessProvider()
    await provider.initialize()
  })

  afterEach(async () => {
    await provider.dispose()
  })

  it('removes agent from map after process exits', async () => {
    const paneId = 'cleanup-test-1__claude'

    await provider.runCommand(paneId, 'echo "done"')

    // Agent should be in map immediately after spawn
    expect(provider['agents'].has(paneId)).toBe(true)

    // Wait for process to exit and cleanup
    await new Promise((r) => setTimeout(r, 500))

    // Agent should be removed from map after exit
    expect(provider['agents'].has(paneId)).toBe(false)
  })

  it('map size decreases when agents stop', async () => {
    await provider.runCommand('cleanup-a1__claude', 'sleep 1')
    await provider.runCommand('cleanup-a2__claude', 'sleep 1')

    expect(provider['agents'].size).toBe(2)

    // Wait for both to exit
    await new Promise((r) => setTimeout(r, 1500))

    expect(provider['agents'].size).toBe(0)
  })

  it('cleanup does not affect other running agents', async () => {
    await provider.runCommand('cleanup-quick__claude', 'echo "done"') // Exits quickly
    await provider.runCommand('cleanup-long__claude', 'sleep 10') // Runs longer

    // Wait for quick agent to exit
    await new Promise((r) => setTimeout(r, 500))

    // Quick agent should be removed
    expect(provider['agents'].has('cleanup-quick__claude')).toBe(false)

    // Long-running agent should still be present
    expect(provider['agents'].has('cleanup-long__claude')).toBe(true)
  })
})

describe('HeadlessProvider log persistence', () => {
  let provider: HeadlessProvider
  let tmpDir: string

  beforeEach(async () => {
    // Create a temp directory to use as baseDir so log files are isolated
    tmpDir = await mkdtemp(path.join(os.tmpdir(), 'headless-log-test-'))

    provider = new HeadlessProvider({ baseDir: tmpDir })
    await provider.initialize()
  })

  afterEach(async () => {
    await provider.dispose()
    await rm(tmpDir, { recursive: true, force: true })
  })

  describe('log file creation', () => {
    it('creates log files with 0o600 permissions when runId is provided', async () => {
      const paneId = 'perm-test__claude'
      const runId = UUID_PERM

      await provider.runCommand(paneId, 'echo "permission test"', runId)
      // Capture log paths immediately (before process exits and agent is cleaned up)
      const stdoutPath = provider.getLogPath(paneId, 'stdout')
      const stderrPath = provider.getLogPath(paneId, 'stderr')
      const combinedPath = provider.getLogPath(paneId, 'combined')

      expect(stdoutPath).toBeDefined()
      expect(stderrPath).toBeDefined()
      expect(combinedPath).toBeDefined()

      // Wait for process to complete and files to be written
      await new Promise((r) => setTimeout(r, 300))

      const stdoutStat = await stat(stdoutPath!)
      const stderrStat = await stat(stderrPath!)
      const combinedStat = await stat(combinedPath!)

      expect(stdoutStat.mode & 0o777).toBe(0o600)
      expect(stderrStat.mode & 0o777).toBe(0o600)
      expect(combinedStat.mode & 0o777).toBe(0o600)
    })

    it('creates log directory with 0o700 permissions', async () => {
      const paneId = 'dir-perm-test__claude'
      const runId = UUID_DIR

      await provider.runCommand(paneId, 'echo "dir test"', runId)
      await new Promise((r) => setTimeout(r, 200))

      const logDir = path.join(tmpDir, '.crewchief/runs', runId, 'logs')
      const dirStat = await stat(logDir)
      expect(dirStat.mode & 0o777).toBe(0o700)
    })

    it('creates all three log files (stdout, stderr, combined)', async () => {
      const paneId = 'files-test__claude'
      const runId = UUID_THREE

      await provider.runCommand(paneId, 'echo "hello"', runId)
      await new Promise((r) => setTimeout(r, 200))

      const logDir = path.join(tmpDir, '.crewchief/runs', runId, 'logs')
      const stdoutStat = await stat(path.join(logDir, 'stdout.log'))
      const stderrStat = await stat(path.join(logDir, 'stderr.log'))
      const combinedStat = await stat(path.join(logDir, 'combined.log'))

      expect(stdoutStat.isFile()).toBe(true)
      expect(stderrStat.isFile()).toBe(true)
      expect(combinedStat.isFile()).toBe(true)
    })

    it('does not create log files when runId is not provided', async () => {
      const paneId = 'no-logs-test__claude'

      await provider.runCommand(paneId, 'echo "no logs"')
      // Check immediately (before process exits and agent is cleaned up)
      const logPath = provider.getLogPath(paneId)
      expect(logPath).toBeUndefined()
    })
  })

  describe('log streaming', () => {
    it('writes stdout to stdout.log and combined.log', async () => {
      const paneId = 'stdout-test__claude'
      const runId = UUID_STDOUT

      await provider.runCommand(paneId, 'echo "hello stdout"', runId)
      // Capture log paths immediately (before process exits and agent is cleaned up)
      const stdoutPath = provider.getLogPath(paneId, 'stdout')!
      const combinedPath = provider.getLogPath(paneId, 'combined')!

      await new Promise((r) => setTimeout(r, 300))

      const stdoutContent = await readFile(stdoutPath, 'utf-8')
      const combinedContent = await readFile(combinedPath, 'utf-8')

      expect(stdoutContent).toContain('hello stdout')
      expect(combinedContent).toContain('hello stdout')
    })

    it('writes stderr to stderr.log and combined.log', async () => {
      const paneId = 'stderr-test__claude'
      const runId = UUID_STDERR

      await provider.runCommand(paneId, 'echo "hello stderr" >&2', runId)
      // Capture log paths immediately (before process exits and agent is cleaned up)
      const stderrPath = provider.getLogPath(paneId, 'stderr')!
      const combinedPath = provider.getLogPath(paneId, 'combined')!

      await new Promise((r) => setTimeout(r, 300))

      const stderrContent = await readFile(stderrPath, 'utf-8')
      const combinedContent = await readFile(combinedPath, 'utf-8')

      expect(stderrContent).toContain('hello stderr')
      expect(combinedContent).toContain('hello stderr')
    })

    it('does not write stderr to stdout.log', async () => {
      const paneId = 'no-cross-test__claude'
      const runId = UUID_NOCROSS

      await provider.runCommand(paneId, 'echo "only stderr" >&2', runId)
      // Capture log path immediately (before process exits and agent is cleaned up)
      const stdoutPath = provider.getLogPath(paneId, 'stdout')!

      await new Promise((r) => setTimeout(r, 300))

      const stdoutContent = await readFile(stdoutPath, 'utf-8')

      // stdout.log should be empty (no stdout output was produced)
      expect(stdoutContent).toBe('')
    })

    it('handles mixed stdout and stderr output', async () => {
      const paneId = 'mixed-test__claude'
      const runId = UUID_MIXED

      // Command that writes to both stdout and stderr
      await provider.runCommand(paneId, 'echo "out line" && echo "err line" >&2', runId)
      // Capture log paths immediately (before process exits and agent is cleaned up)
      const stdoutLogPath = provider.getLogPath(paneId, 'stdout')!
      const stderrLogPath = provider.getLogPath(paneId, 'stderr')!
      const combinedLogPath = provider.getLogPath(paneId, 'combined')!

      await new Promise((r) => setTimeout(r, 300))

      const stdoutContent = await readFile(stdoutLogPath, 'utf-8')
      const stderrContent = await readFile(stderrLogPath, 'utf-8')
      const combinedContent = await readFile(combinedLogPath, 'utf-8')

      expect(stdoutContent).toContain('out line')
      expect(stderrContent).toContain('err line')
      expect(combinedContent).toContain('out line')
      expect(combinedContent).toContain('err line')
    })

    it('logs persist on disk after process exits and agent is cleaned up', async () => {
      const paneId = 'persist-test__claude'
      const runId = UUID_PERSIST

      // Capture log path immediately after spawn (before process exits)
      await provider.runCommand(paneId, 'echo "persisted output"', runId)
      const logPath = provider.getLogPath(paneId, 'combined')
      expect(logPath).toBeDefined()

      // Wait for process to exit and agent to be cleaned up
      await new Promise((r) => setTimeout(r, 500))

      // Agent should be removed from map
      const agents = await provider.listAgents()
      expect(agents.find((a) => a.name === paneId)).toBeUndefined()

      // Log files should still exist on disk
      const content = await readFile(logPath!, 'utf-8')
      expect(content).toContain('persisted output')
    })
  })

  describe('getLogPath', () => {
    it('returns correct path for stdout stream', async () => {
      const paneId = 'path-stdout__claude'
      const runId = UUID_PATH_STDOUT

      await provider.runCommand(paneId, 'echo "test"', runId)
      // Capture immediately before process exits and agent is cleaned up
      const logPath = provider.getLogPath(paneId, 'stdout')
      expect(logPath).toBe(path.join(tmpDir, '.crewchief/runs', runId, 'logs/stdout.log'))
    })

    it('returns correct path for stderr stream', async () => {
      const paneId = 'path-stderr__claude'
      const runId = UUID_PATH_STDERR

      await provider.runCommand(paneId, 'echo "test"', runId)
      // Capture immediately before process exits and agent is cleaned up
      const logPath = provider.getLogPath(paneId, 'stderr')
      expect(logPath).toBe(path.join(tmpDir, '.crewchief/runs', runId, 'logs/stderr.log'))
    })

    it('defaults to combined stream when no stream specified', async () => {
      const paneId = 'path-default__claude'
      const runId = UUID_PATH_DEFAULT

      await provider.runCommand(paneId, 'echo "test"', runId)
      // Capture immediately before process exits and agent is cleaned up
      const logPath = provider.getLogPath(paneId)
      expect(logPath).toBe(path.join(tmpDir, '.crewchief/runs', runId, 'logs/combined.log'))
    })

    it('returns undefined for pane without runId', async () => {
      const paneId = 'no-run__claude'

      await provider.runCommand(paneId, 'echo "test"')
      // Check immediately before process exits and agent is cleaned up
      expect(provider.getLogPath(paneId)).toBeUndefined()
    })

    it('returns undefined for nonexistent pane', () => {
      expect(provider.getLogPath('does-not-exist')).toBeUndefined()
    })
  })

  describe('getLogs', () => {
    it('returns full log content when lines is not specified', async () => {
      const paneId = 'getlogs-full__claude'
      const runId = UUID_GETLOGS_FULL

      // Use sleep to keep process alive so agent stays in map for getLogs()
      await provider.runCommand(paneId, 'printf "line1\\nline2\\nline3\\n" && sleep 5', runId)
      await new Promise((r) => setTimeout(r, 300))

      const logs = await provider.getLogs(paneId)
      expect(logs).toContain('line1')
      expect(logs).toContain('line2')
      expect(logs).toContain('line3')
    })

    it('returns last N lines when lines is specified', async () => {
      const paneId = 'getlogs-tail__claude'
      const runId = UUID_GETLOGS_TAIL

      // Use sleep to keep process alive so agent stays in map for getLogs()
      await provider.runCommand(paneId, 'printf "line1\\nline2\\nline3\\nline4\\nline5\\n" && sleep 5', runId)
      await new Promise((r) => setTimeout(r, 300))

      const logs = await provider.getLogs(paneId, 2)
      // Last 2 lines of "line1\nline2\nline3\nline4\nline5\n" split gives
      // ["line1","line2","line3","line4","line5",""] => last 2 = ["line5",""]
      expect(logs).toContain('line5')
      expect(logs).not.toContain('line1')
      expect(logs).not.toContain('line2')
      expect(logs).not.toContain('line3')
    })

    it('throws error for pane without logs', async () => {
      const paneId = 'no-logs__claude'
      // Use sleep to keep agent in map so we test the no-runId path, not the missing-agent path
      await provider.runCommand(paneId, 'sleep 5')
      await new Promise((r) => setTimeout(r, 100))

      await expect(provider.getLogs(paneId)).rejects.toThrow('No logs found for pane no-logs__claude')
    })

    it('throws error for nonexistent pane', async () => {
      await expect(provider.getLogs('nonexistent')).rejects.toThrow('No logs found for pane nonexistent')
    })
  })

  describe('run ID validation', () => {
    it('rejects path traversal in getLogPath when agent has invalid runId', async () => {
      const paneId = 'traversal-test__claude'
      // Directly spawn with an invalid runId to test validation in getLogPath
      // Use sleep to keep agent in map for getLogPath test
      await provider.runCommand(paneId, 'sleep 5', UUID_INVALID_TRAVERSAL)
      await new Promise((r) => setTimeout(r, 100))

      expect(() => provider.getLogPath(paneId)).toThrow('Invalid run ID format')
    })

    it('rejects path traversal in getLogs when agent has invalid runId', async () => {
      const paneId = 'traversal-logs-test__claude'
      // Use sleep to keep agent in map for getLogs test
      await provider.runCommand(paneId, 'sleep 5', UUID_INVALID_TRAVERSAL)
      await new Promise((r) => setTimeout(r, 100))

      await expect(provider.getLogs(paneId)).rejects.toThrow('Invalid run ID format')
    })

    it('accepts valid UUID runId in getLogPath', async () => {
      const paneId = 'valid-uuid-test__claude'
      const runId = UUID_PERM

      await provider.runCommand(paneId, 'echo "test"', runId)
      // Capture immediately before process exits and agent is cleaned up
      const logPath = provider.getLogPath(paneId)
      expect(logPath).toBeDefined()
      expect(logPath).toContain(runId)
    })

    it('accepts valid UUID runId in getLogs', async () => {
      const paneId = 'valid-uuid-logs__claude'
      const runId = UUID_PERSIST

      // Use sleep to keep process alive so agent stays in map for getLogs()
      await provider.runCommand(paneId, 'echo "uuid test output" && sleep 5', runId)
      await new Promise((r) => setTimeout(r, 300))

      const logs = await provider.getLogs(paneId)
      expect(logs).toContain('uuid test output')
    })
  })

  describe('error handling', () => {
    it('continues without file logging when log creation fails', async () => {
      const paneId = 'fail-log__claude'
      // Create a file where the runId directory should go, causing mkdir to fail
      const runsDir = path.join(tmpDir, '.crewchief/runs')
      await mkdir(runsDir, { recursive: true })
      const blockingPath = path.join(runsDir, UUID_BLOCKED)
      await writeFile(blockingPath, 'blocking file', { mode: 0o444 })

      // runCommand should not throw even when log creation fails
      // (UUID_BLOCKED is a file, so mkdir UUID_BLOCKED/logs will fail)
      // Use sleep to keep agent in map for verification
      await expect(provider.runCommand(paneId, 'sleep 5', UUID_BLOCKED)).resolves.not.toThrow()

      await new Promise((r) => setTimeout(r, 200))

      // Agent should still be tracked (process still running)
      const agents = await provider.listAgents()
      const agent = agents.find((a) => a.name === paneId)
      expect(agent).toBeDefined()
    })

    it('works when runId is not provided (backward compat, agent cleaned up after exit)', async () => {
      const paneId = 'compat-test__claude'

      // Call without runId - exactly matches old signature
      await provider.runCommand(paneId, 'echo "backward compatible"')
      // Agent should be in map initially
      await new Promise((r) => setTimeout(r, 50))
      expect(provider.getLogPath(paneId)).toBeUndefined()

      // Wait for exit and cleanup
      await new Promise((r) => setTimeout(r, 500))

      // Agent should be removed from map after exit
      const agents = await provider.listAgents()
      const agent = agents.find((a) => a.name === paneId)
      expect(agent).toBeUndefined()
    })
  })

  describe('log directory size monitoring', () => {
    it('returns size of log directory', async () => {
      const paneId = 'size-test__claude'
      const runId = UUID_SIZE

      await provider.runCommand(paneId, 'echo "some output for size check"', runId)
      await new Promise((r) => setTimeout(r, 300))

      const runsDir = path.join(tmpDir, '.crewchief/runs')
      const size = await checkLogDirectorySize(runsDir)
      expect(size).toBeGreaterThan(0)
    })

    it('does not warn when directory is under 1GB', async () => {
      const { logger: loggerModule } = await import('../../../utils/logger')
      const warnSpy = vi.spyOn(loggerModule, 'warn')

      const runsDir = path.join(tmpDir, '.crewchief/runs')
      const size = await checkLogDirectorySize(runsDir)
      expect(size).toBeLessThan(1024 * 1024 * 1024)

      // The warn should NOT have been called for size (small dir)
      const sizeWarnings = warnSpy.mock.calls.filter(
        (call) => typeof call[0] === 'string' && call[0].includes('Log directory exceeds 1GB'),
      )
      expect(sizeWarnings.length).toBe(0)

      warnSpy.mockRestore()
    })

    it('returns 0 when runs directory does not exist', async () => {
      const nonexistentDir = path.join(tmpDir, 'does-not-exist')
      const size = await checkLogDirectorySize(nonexistentDir)
      expect(size).toBe(0)
    })
  })

  describe('dispose closes log streams', () => {
    it('closes log streams on dispose', async () => {
      const paneId = 'dispose-logs__claude'
      const runId = UUID_DISPOSE

      await provider.runCommand(paneId, 'sleep 5', runId)
      await new Promise((r) => setTimeout(r, 200))

      // Verify log files exist before dispose
      const logPath = provider.getLogPath(paneId, 'combined')
      expect(logPath).toBeDefined()
      const statBefore = await stat(logPath!)
      expect(statBefore.isFile()).toBe(true)

      // Dispose should close streams and not throw
      await expect(provider.dispose()).resolves.not.toThrow()

      // Log files should still exist on disk after dispose
      const statAfter = await stat(logPath!)
      expect(statAfter.isFile()).toBe(true)
    })
  })

  describe('getLogs streaming behavior', () => {
    it('streams large log files and returns correct last N lines', async () => {
      const paneId = 'stream-large__claude'
      const runId = UUID_STREAM_LARGE

      // Generate a file with 500 lines to simulate a large log
      const lineNumbers = Array.from({ length: 500 }, (_, i) => `log-line-${i + 1}`)
      const printfArg = lineNumbers.join('\\n') + '\\n'

      // Use sleep to keep process alive so agent stays in map for getLogs()
      await provider.runCommand(paneId, `printf "${printfArg}" && sleep 5`, runId)
      await new Promise((r) => setTimeout(r, 500))

      // Request only the last 5 lines via streaming
      const logs = await provider.getLogs(paneId, 5)
      const resultLines = logs.split('\n')

      expect(resultLines).toContain('log-line-496')
      expect(resultLines).toContain('log-line-497')
      expect(resultLines).toContain('log-line-498')
      expect(resultLines).toContain('log-line-499')
      expect(resultLines).toContain('log-line-500')
      expect(logs).not.toContain('log-line-1\n')
      expect(logs).not.toContain('log-line-495\n')
    })

    it('handles empty log file', async () => {
      const paneId = 'stream-empty__claude'
      const runId = UUID_STREAM_EMPTY

      // Use sleep to keep process alive (no output produced, but agent stays in map)
      await provider.runCommand(paneId, 'sleep 5', runId)
      await new Promise((r) => setTimeout(r, 300))

      // Request last 10 lines from empty file
      const logs = await provider.getLogs(paneId, 10)
      expect(logs).toBe('')
    })

    it('handles file with fewer lines than requested', async () => {
      const paneId = 'stream-small__claude'
      const runId = UUID_STREAM_SMALL

      // Use sleep to keep process alive so agent stays in map for getLogs()
      await provider.runCommand(paneId, 'printf "alpha\\nbeta\\ngamma\\n" && sleep 5', runId)
      await new Promise((r) => setTimeout(r, 300))

      // Request last 100 lines (more than available)
      const logs = await provider.getLogs(paneId, 100)
      expect(logs).toContain('alpha')
      expect(logs).toContain('beta')
      expect(logs).toContain('gamma')
    })

    it('preserves correct line order in streaming output', async () => {
      const paneId = 'stream-order__claude'
      const runId = UUID_STREAM_ORDER

      // Use sleep to keep process alive so agent stays in map for getLogs()
      await provider.runCommand(paneId, 'printf "first\\nsecond\\nthird\\nfourth\\nfifth\\n" && sleep 5', runId)
      await new Promise((r) => setTimeout(r, 300))

      // Request last 3 lines
      const logs = await provider.getLogs(paneId, 3)
      const resultLines = logs.split('\n').filter((l) => l.length > 0)

      // Lines must appear in chronological order
      expect(resultLines.indexOf('third')).toBeLessThan(resultLines.indexOf('fourth'))
      expect(resultLines.indexOf('fourth')).toBeLessThan(resultLines.indexOf('fifth'))
    })

    it('handles single-line file', async () => {
      const paneId = 'stream-single__claude'
      const runId = UUID_STREAM_SINGLE

      // Use sleep to keep process alive so agent stays in map for getLogs()
      await provider.runCommand(paneId, 'printf "only-one-line\\n" && sleep 5', runId)
      await new Promise((r) => setTimeout(r, 300))

      // Request last 5 lines from a single-line file
      const logs = await provider.getLogs(paneId, 5)
      expect(logs).toContain('only-one-line')
    })
  })
})
