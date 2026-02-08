import { spawnSync, SpawnSyncReturns } from 'node:child_process'
import { existsSync } from 'node:fs'
import { describe, it, expect, beforeEach, afterEach, vi, Mock } from 'vitest'
import { ITermProvider } from '../iterm'

// Mock child_process
vi.mock('node:child_process', () => ({
  spawnSync: vi.fn(),
}))

// Mock fs
vi.mock('node:fs', () => ({
  existsSync: vi.fn(),
}))

const mockSpawnSync = spawnSync as Mock
const mockExistsSync = existsSync as Mock

describe('ITermProvider', () => {
  let provider: ITermProvider

  beforeEach(() => {
    vi.clearAllMocks()
    // Default: scripts exist
    mockExistsSync.mockReturnValue(true)
  })

  afterEach(async () => {
    if (provider) {
      await provider.dispose()
    }
  })

  describe('constructor', () => {
    it('finds scripts directory when spawn_agent.py exists', () => {
      mockExistsSync.mockReturnValue(true)
      provider = new ITermProvider()
      expect(provider.id).toBe('iterm')
    })

    it('handles missing scripts directory', () => {
      mockExistsSync.mockReturnValue(false)
      provider = new ITermProvider()
      // Provider should still be created, just won't work
      expect(provider.id).toBe('iterm')
    })
  })

  describe('initialize', () => {
    it('throws when not running in iTerm.app', async () => {
      const originalTermProgram = process.env.TERM_PROGRAM
      process.env.TERM_PROGRAM = 'Terminal'

      provider = new ITermProvider()
      await expect(provider.initialize()).rejects.toThrow('requires running in iTerm.app')

      process.env.TERM_PROGRAM = originalTermProgram
    })

    it('throws when scripts not found', async () => {
      const originalTermProgram = process.env.TERM_PROGRAM
      process.env.TERM_PROGRAM = 'iTerm.app'
      mockExistsSync.mockReturnValue(false)

      provider = new ITermProvider()
      await expect(provider.initialize()).rejects.toThrow('scripts not found')

      process.env.TERM_PROGRAM = originalTermProgram
    })

    it('succeeds when in iTerm.app with scripts available', async () => {
      const originalTermProgram = process.env.TERM_PROGRAM
      process.env.TERM_PROGRAM = 'iTerm.app'
      mockExistsSync.mockReturnValue(true)

      provider = new ITermProvider()
      await expect(provider.initialize()).resolves.not.toThrow()

      process.env.TERM_PROGRAM = originalTermProgram
    })
  })

  describe('createWindow', () => {
    beforeEach(() => {
      mockExistsSync.mockReturnValue(true)
    })

    it('calls spawn_agent.py with agent type', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: 'Spawned session\n',
        stderr: '',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      const windowId = await provider.createWindow({ title: 'test-title' })

      expect(mockSpawnSync).toHaveBeenCalled()
      const call = mockSpawnSync.mock.calls[0]
      expect(call[0]).toBe('python3')
      expect(call[1][0]).toContain('spawn_agent.py')
      expect(call[1]).toContain('claude')
      expect(call[1]).toContain('--name')
      expect(call[1]).toContain('test-title')
      expect(windowId).toBe('test-title')
    })

    it('passes working directory when provided', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: 'Spawned\n',
        stderr: '',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      await provider.createWindow({
        title: 'agent',
        workingDirectory: '/path/to/project',
      })

      expect(mockSpawnSync).toHaveBeenCalled()
      const call = mockSpawnSync.mock.calls[0]
      expect(call[1]).toContain('--project-dir')
      expect(call[1]).toContain('/path/to/project')
    })

    it('throws error when spawn_agent.py fails', async () => {
      mockSpawnSync.mockReturnValue({
        status: 1,
        stdout: '',
        stderr: 'Python error',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      await expect(provider.createWindow()).rejects.toThrow('spawn_agent.py failed')
    })

    it('throws error when scripts directory not found', async () => {
      mockExistsSync.mockReturnValue(false)

      provider = new ITermProvider()
      await expect(provider.createWindow()).rejects.toThrow('scripts not found')
    })
  })

  describe('splitPane', () => {
    beforeEach(() => {
      mockExistsSync.mockReturnValue(true)
    })

    it('calls split_vertical.py for vertical split', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: '',
        stderr: '',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      await provider.splitPane('pane-1', 'vertical')

      expect(mockSpawnSync).toHaveBeenCalled()
      const call = mockSpawnSync.mock.calls[0]
      expect(call[0]).toBe('python3')
      expect(call[1][0]).toContain('split_vertical.py')
    })

    it('calls split_horizontal.py for horizontal split', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: '',
        stderr: '',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      await provider.splitPane('pane-1', 'horizontal')

      expect(mockSpawnSync).toHaveBeenCalled()
      const call = mockSpawnSync.mock.calls[0]
      expect(call[0]).toBe('python3')
      expect(call[1][0]).toContain('split_horizontal.py')
    })
  })

  describe('runCommand', () => {
    beforeEach(() => {
      mockExistsSync.mockReturnValue(true)
    })

    it('calls send_to_pane.py with pane ID and command', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: '',
        stderr: '',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      await provider.runCommand('pane-1', 'echo hello')

      expect(mockSpawnSync).toHaveBeenCalled()
      const call = mockSpawnSync.mock.calls[0]
      expect(call[0]).toBe('python3')
      expect(call[1][0]).toContain('send_to_pane.py')
      expect(call[1]).toContain('--to')
      expect(call[1]).toContain('pane-1')
      expect(call[1]).toContain('--text')
      expect(call[1]).toContain('echo hello')
    })

    it('throws error when send_to_pane.py fails', async () => {
      mockSpawnSync.mockReturnValue({
        status: 1,
        stdout: '',
        stderr: 'Failed to send',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      await expect(provider.runCommand('pane-1', 'echo test')).rejects.toThrow('send_to_pane.py failed')
    })
  })

  describe('sendMessage', () => {
    beforeEach(() => {
      mockExistsSync.mockReturnValue(true)
    })

    it('returns false when scripts not found', async () => {
      mockExistsSync.mockReturnValue(false)

      provider = new ITermProvider()
      const result = await provider.sendMessage('pane-1', 'hello')

      expect(result).toBe(false)
    })

    it('calls send_to_pane.py with correct arguments', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: '',
        stderr: '',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      await provider.sendMessage('task__claude', 'hello world')

      expect(mockSpawnSync).toHaveBeenCalled()
      const call = mockSpawnSync.mock.calls[0]
      expect(call[0]).toBe('python3')
      expect(call[1][0]).toContain('send_to_pane.py')
      expect(call[1]).toContain('--to')
      expect(call[1]).toContain('task__claude')
      expect(call[1]).toContain('--text')
      expect(call[1]).toContain('hello world')
      expect(call[1]).toContain('--agent')
      expect(call[1]).toContain('claude')
    })

    it('includes agent type when pane ID follows name__type format', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: '',
        stderr: '',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      await provider.sendMessage('my-task__gemini', 'test message')

      expect(mockSpawnSync).toHaveBeenCalled()
      const call = mockSpawnSync.mock.calls[0]
      expect(call[1]).toContain('--agent')
      expect(call[1]).toContain('gemini')
    })

    it('omits agent flag when pane ID is unknown type', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: '',
        stderr: '',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      await provider.sendMessage('simple-pane', 'test message')

      // Should not include --agent flag for unknown type
      const call = mockSpawnSync.mock.calls[0]
      expect(call[1]).not.toContain('--agent')
    })

    it('returns true on success', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: '',
        stderr: '',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      const result = await provider.sendMessage('pane__claude', 'test')

      expect(result).toBe(true)
    })

    it('returns false on failure', async () => {
      mockSpawnSync.mockReturnValue({
        status: 1,
        stdout: '',
        stderr: 'Error',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      const result = await provider.sendMessage('pane__claude', 'test')

      expect(result).toBe(false)
    })
  })

  describe('listAgents', () => {
    beforeEach(() => {
      mockExistsSync.mockReturnValue(true)
    })

    it('returns empty array when scripts not found', async () => {
      mockExistsSync.mockReturnValue(false)

      provider = new ITermProvider()
      const agents = await provider.listAgents()

      expect(agents).toEqual([])
    })

    it('returns empty array when list_panes.py fails', async () => {
      mockSpawnSync.mockReturnValue({
        status: 1,
        stdout: '',
        stderr: 'Error',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      const agents = await provider.listAgents()

      expect(agents).toEqual([])
    })

    it('parses list_panes.py output correctly', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: `  1. [fix-bug__claude]     Window:1 Tab:1 ID:3C31E1FF-1234
  2. [task__gemini]     Window:1 Tab:2 ID:AB12CD34-5678
  3. [regular-pane]     Window:1 Tab:3 ID:DEAD0000-BEEF`,
        stderr: '',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      const agents = await provider.listAgents()

      // Should only return panes with name__platform format
      expect(agents).toHaveLength(2)
      expect(agents[0]).toEqual({
        id: '3C31E1FF-1234',
        name: 'fix-bug__claude',
        platform: 'claude',
        status: 'running',
      })
      expect(agents[1]).toEqual({
        id: 'AB12CD34-5678',
        name: 'task__gemini',
        platform: 'gemini',
        status: 'running',
      })
    })

    it('filters out non-agent panes', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: `  1. [bash]     Window:1 Tab:1 ID:AAAAAA
  2. [my-agent__claude]     Window:1 Tab:2 ID:BBBBBB`,
        stderr: '',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      const agents = await provider.listAgents()

      expect(agents).toHaveLength(1)
      expect(agents[0].name).toBe('my-agent__claude')
    })

    it('returns empty array for empty output', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: '',
        stderr: '',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      const agents = await provider.listAgents()

      expect(agents).toEqual([])
    })
  })

  describe('focus', () => {
    it('calls osascript to activate iTerm', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: '',
        stderr: '',
      } as SpawnSyncReturns<string>)

      provider = new ITermProvider()
      await provider.focus('pane-1')

      expect(mockSpawnSync).toHaveBeenCalledWith('osascript', ['-e', 'tell application "iTerm2" to activate'])
    })
  })

  describe('dispose', () => {
    it('completes without error', async () => {
      provider = new ITermProvider()
      await expect(provider.dispose()).resolves.not.toThrow()
    })
  })

  describe('parseAgentType (via sendMessage)', () => {
    beforeEach(() => {
      mockExistsSync.mockReturnValue(true)
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: '',
        stderr: '',
      } as SpawnSyncReturns<string>)
    })

    it('extracts type from simple name__type format', async () => {
      provider = new ITermProvider()
      await provider.sendMessage('task__claude', 'test')

      const args = mockSpawnSync.mock.calls[0][1] as string[]
      expect(args.includes('claude')).toBe(true)
    })

    it('extracts type from complex name with underscores', async () => {
      provider = new ITermProvider()
      await provider.sendMessage('my_complex_task__gemini', 'test')

      const args = mockSpawnSync.mock.calls[0][1] as string[]
      expect(args.includes('gemini')).toBe(true)
    })

    it('handles multiple double underscores by taking last segment', async () => {
      provider = new ITermProvider()
      await provider.sendMessage('weird__name__codex', 'test')

      const args = mockSpawnSync.mock.calls[0][1] as string[]
      expect(args.includes('codex')).toBe(true)
    })
  })
})
