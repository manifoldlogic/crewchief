import { mkdtemp, rm, stat, readFile, mkdir, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { HeadlessProvider, checkLogDirectorySize } from '../headless'

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
      await provider.runCommand(paneId, 'echo "test"')
      // Give time for spawn
      await new Promise((r) => setTimeout(r, 100))

      const agents = await provider.listAgents()
      expect(agents.length).toBeGreaterThanOrEqual(1)
      const agent = agents.find((a) => a.name === paneId)
      expect(agent).toBeDefined()
    })

    it('parses agent type from pane ID', async () => {
      const paneId = 'my-task__gemini'
      await provider.runCommand(paneId, 'echo "test"')
      await new Promise((r) => setTimeout(r, 100))

      const agents = await provider.listAgents()
      const agent = agents.find((a) => a.name === paneId)
      expect(agent?.type).toBe('gemini')
    })

    it('sets type to unknown for non-standard pane IDs', async () => {
      const paneId = 'simple-pane'
      await provider.runCommand(paneId, 'echo "test"')
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

    it('returns false when process has exited', async () => {
      const paneId = 'exited-test__claude'
      // Spawn a process that exits immediately
      await provider.runCommand(paneId, 'echo done')
      // Wait for it to exit
      await new Promise((r) => setTimeout(r, 300))

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

    it('marks exited processes as stopped', async () => {
      const paneId = 'quick-task__claude'
      await provider.runCommand(paneId, 'echo done')
      // Wait for process to exit
      await new Promise((r) => setTimeout(r, 300))

      const agents = await provider.listAgents()
      const agent = agents.find((a) => a.name === paneId)
      expect(agent?.status).toBe('stopped')
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
      const runId = 'test-run-perm-600'

      await provider.runCommand(paneId, 'echo "permission test"', runId)
      await new Promise((r) => setTimeout(r, 200))

      const stdoutPath = provider.getLogPath(paneId, 'stdout')
      const stderrPath = provider.getLogPath(paneId, 'stderr')
      const combinedPath = provider.getLogPath(paneId, 'combined')

      expect(stdoutPath).toBeDefined()
      expect(stderrPath).toBeDefined()
      expect(combinedPath).toBeDefined()

      const stdoutStat = await stat(stdoutPath!)
      const stderrStat = await stat(stderrPath!)
      const combinedStat = await stat(combinedPath!)

      expect(stdoutStat.mode & 0o777).toBe(0o600)
      expect(stderrStat.mode & 0o777).toBe(0o600)
      expect(combinedStat.mode & 0o777).toBe(0o600)
    })

    it('creates log directory with 0o700 permissions', async () => {
      const paneId = 'dir-perm-test__claude'
      const runId = 'test-run-dir-700'

      await provider.runCommand(paneId, 'echo "dir test"', runId)
      await new Promise((r) => setTimeout(r, 200))

      const logDir = path.join(tmpDir, '.crewchief/runs', runId, 'logs')
      const dirStat = await stat(logDir)
      expect(dirStat.mode & 0o777).toBe(0o700)
    })

    it('creates all three log files (stdout, stderr, combined)', async () => {
      const paneId = 'files-test__claude'
      const runId = 'test-run-three-files'

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
      await new Promise((r) => setTimeout(r, 200))

      const logPath = provider.getLogPath(paneId)
      expect(logPath).toBeUndefined()
    })
  })

  describe('log streaming', () => {
    it('writes stdout to stdout.log and combined.log', async () => {
      const paneId = 'stdout-test__claude'
      const runId = 'test-run-stdout'

      await provider.runCommand(paneId, 'echo "hello stdout"', runId)
      await new Promise((r) => setTimeout(r, 300))

      const stdoutPath = provider.getLogPath(paneId, 'stdout')!
      const combinedPath = provider.getLogPath(paneId, 'combined')!

      const stdoutContent = await readFile(stdoutPath, 'utf-8')
      const combinedContent = await readFile(combinedPath, 'utf-8')

      expect(stdoutContent).toContain('hello stdout')
      expect(combinedContent).toContain('hello stdout')
    })

    it('writes stderr to stderr.log and combined.log', async () => {
      const paneId = 'stderr-test__claude'
      const runId = 'test-run-stderr'

      await provider.runCommand(paneId, 'echo "hello stderr" >&2', runId)
      await new Promise((r) => setTimeout(r, 300))

      const stderrPath = provider.getLogPath(paneId, 'stderr')!
      const combinedPath = provider.getLogPath(paneId, 'combined')!

      const stderrContent = await readFile(stderrPath, 'utf-8')
      const combinedContent = await readFile(combinedPath, 'utf-8')

      expect(stderrContent).toContain('hello stderr')
      expect(combinedContent).toContain('hello stderr')
    })

    it('does not write stderr to stdout.log', async () => {
      const paneId = 'no-cross-test__claude'
      const runId = 'test-run-no-cross'

      await provider.runCommand(paneId, 'echo "only stderr" >&2', runId)
      await new Promise((r) => setTimeout(r, 300))

      const stdoutPath = provider.getLogPath(paneId, 'stdout')!
      const stdoutContent = await readFile(stdoutPath, 'utf-8')

      // stdout.log should be empty (no stdout output was produced)
      expect(stdoutContent).toBe('')
    })

    it('handles mixed stdout and stderr output', async () => {
      const paneId = 'mixed-test__claude'
      const runId = 'test-run-mixed'

      // Command that writes to both stdout and stderr
      await provider.runCommand(paneId, 'echo "out line" && echo "err line" >&2', runId)
      await new Promise((r) => setTimeout(r, 300))

      const stdoutContent = await readFile(provider.getLogPath(paneId, 'stdout')!, 'utf-8')
      const stderrContent = await readFile(provider.getLogPath(paneId, 'stderr')!, 'utf-8')
      const combinedContent = await readFile(provider.getLogPath(paneId, 'combined')!, 'utf-8')

      expect(stdoutContent).toContain('out line')
      expect(stderrContent).toContain('err line')
      expect(combinedContent).toContain('out line')
      expect(combinedContent).toContain('err line')
    })

    it('logs persist after process exits', async () => {
      const paneId = 'persist-test__claude'
      const runId = 'test-run-persist'

      await provider.runCommand(paneId, 'echo "persisted output"', runId)
      // Wait for process to exit
      await new Promise((r) => setTimeout(r, 500))

      // Verify process has exited
      const agents = await provider.listAgents()
      const agent = agents.find((a) => a.name === paneId)
      expect(agent?.status).toBe('stopped')

      // Logs should still be readable
      const logs = await provider.getLogs(paneId)
      expect(logs).toContain('persisted output')
    })
  })

  describe('getLogPath', () => {
    it('returns correct path for stdout stream', async () => {
      const paneId = 'path-stdout__claude'
      const runId = 'test-run-path-stdout'

      await provider.runCommand(paneId, 'echo "test"', runId)
      await new Promise((r) => setTimeout(r, 100))

      const logPath = provider.getLogPath(paneId, 'stdout')
      expect(logPath).toBe(path.join(tmpDir, '.crewchief/runs', runId, 'logs/stdout.log'))
    })

    it('returns correct path for stderr stream', async () => {
      const paneId = 'path-stderr__claude'
      const runId = 'test-run-path-stderr'

      await provider.runCommand(paneId, 'echo "test"', runId)
      await new Promise((r) => setTimeout(r, 100))

      const logPath = provider.getLogPath(paneId, 'stderr')
      expect(logPath).toBe(path.join(tmpDir, '.crewchief/runs', runId, 'logs/stderr.log'))
    })

    it('defaults to combined stream when no stream specified', async () => {
      const paneId = 'path-default__claude'
      const runId = 'test-run-path-default'

      await provider.runCommand(paneId, 'echo "test"', runId)
      await new Promise((r) => setTimeout(r, 100))

      const logPath = provider.getLogPath(paneId)
      expect(logPath).toBe(path.join(tmpDir, '.crewchief/runs', runId, 'logs/combined.log'))
    })

    it('returns undefined for pane without runId', async () => {
      const paneId = 'no-run__claude'

      await provider.runCommand(paneId, 'echo "test"')
      await new Promise((r) => setTimeout(r, 100))

      expect(provider.getLogPath(paneId)).toBeUndefined()
    })

    it('returns undefined for nonexistent pane', () => {
      expect(provider.getLogPath('does-not-exist')).toBeUndefined()
    })
  })

  describe('getLogs', () => {
    it('returns full log content when lines is not specified', async () => {
      const paneId = 'getlogs-full__claude'
      const runId = 'test-run-getlogs-full'

      await provider.runCommand(paneId, 'printf "line1\\nline2\\nline3\\n"', runId)
      await new Promise((r) => setTimeout(r, 300))

      const logs = await provider.getLogs(paneId)
      expect(logs).toContain('line1')
      expect(logs).toContain('line2')
      expect(logs).toContain('line3')
    })

    it('returns last N lines when lines is specified', async () => {
      const paneId = 'getlogs-tail__claude'
      const runId = 'test-run-getlogs-tail'

      await provider.runCommand(paneId, 'printf "line1\\nline2\\nline3\\nline4\\nline5\\n"', runId)
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
      await provider.runCommand(paneId, 'echo "test"')
      await new Promise((r) => setTimeout(r, 100))

      await expect(provider.getLogs(paneId)).rejects.toThrow('No logs found for pane no-logs__claude')
    })

    it('throws error for nonexistent pane', async () => {
      await expect(provider.getLogs('nonexistent')).rejects.toThrow('No logs found for pane nonexistent')
    })
  })

  describe('error handling', () => {
    it('continues without file logging when log creation fails', async () => {
      const paneId = 'fail-log__claude'
      // Create a file where the runId directory should go, causing mkdir to fail
      const runsDir = path.join(tmpDir, '.crewchief/runs')
      await mkdir(runsDir, { recursive: true })
      const blockingPath = path.join(runsDir, 'blocked-run')
      await writeFile(blockingPath, 'blocking file', { mode: 0o444 })

      // runCommand should not throw even when log creation fails
      // (blocked-run is a file, so mkdir blocked-run/logs will fail)
      await expect(provider.runCommand(paneId, 'echo "still works"', 'blocked-run')).resolves.not.toThrow()

      await new Promise((r) => setTimeout(r, 200))

      // Agent should still be tracked
      const agents = await provider.listAgents()
      const agent = agents.find((a) => a.name === paneId)
      expect(agent).toBeDefined()
    })

    it('works exactly as before when runId is not provided (backward compat)', async () => {
      const paneId = 'compat-test__claude'

      // Call without runId - exactly matches old signature
      await provider.runCommand(paneId, 'echo "backward compatible"')
      await new Promise((r) => setTimeout(r, 200))

      const agents = await provider.listAgents()
      const agent = agents.find((a) => a.name === paneId)
      expect(agent).toBeDefined()
      expect(agent?.status).toBe('stopped')
      expect(provider.getLogPath(paneId)).toBeUndefined()
    })
  })

  describe('log directory size monitoring', () => {
    it('returns size of log directory', async () => {
      const paneId = 'size-test__claude'
      const runId = 'test-run-size'

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
      const runId = 'test-run-dispose-logs'

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
})
