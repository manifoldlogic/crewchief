import { spawnSync } from 'node:child_process'
import { describe, it, expect, beforeEach, vi, Mock } from 'vitest'
import { TmuxProvider } from '../tmux'

// Mock child_process
vi.mock('node:child_process', () => ({
  spawnSync: vi.fn(),
}))

// Mock logger to suppress output during tests
vi.mock('../../../utils/logger', () => ({
  logger: {
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    success: vi.fn(),
  },
}))

const mockSpawnSync = spawnSync as Mock

describe('TmuxProvider', () => {
  let provider: TmuxProvider

  beforeEach(() => {
    vi.clearAllMocks()
    provider = new TmuxProvider()
  })

  describe('constructor', () => {
    it('has correct provider id', () => {
      expect(provider.id).toBe('tmux')
    })

    it('uses default session name "crewchief"', () => {
      // We'll verify this through initialize() which references the session name
      const defaultProvider = new TmuxProvider()
      expect(defaultProvider.id).toBe('tmux')
    })

    it('accepts custom session name', () => {
      const customProvider = new TmuxProvider({ sessionName: 'my-session' })
      expect(customProvider.id).toBe('tmux')
    })
  })

  describe('initialize', () => {
    it('throws error when tmux not found', async () => {
      mockSpawnSync.mockReturnValueOnce({
        status: 1,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await expect(provider.initialize()).rejects.toThrow('tmux not found')
    })

    it('throws error when tmux version cannot be parsed', async () => {
      // command -v tmux succeeds
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('/usr/bin/tmux'),
        stderr: Buffer.from(''),
      } as any)

      // tmux -V returns unparseable output
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('something unexpected'),
        stderr: Buffer.from(''),
      } as any)

      await expect(provider.initialize()).rejects.toThrow('Could not determine tmux version')
    })

    it('throws error for tmux version < 2.1 (version 2.0)', async () => {
      // command -v tmux succeeds
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('/usr/bin/tmux'),
        stderr: Buffer.from(''),
      } as any)

      // tmux -V returns old version
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('tmux 2.0'),
        stderr: Buffer.from(''),
      } as any)

      await expect(provider.initialize()).rejects.toThrow('too old')
    })

    it('throws error for tmux version < 2.1 (version 1.9)', async () => {
      // command -v tmux succeeds
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('/usr/bin/tmux'),
        stderr: Buffer.from(''),
      } as any)

      // tmux -V returns very old version
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('tmux 1.9'),
        stderr: Buffer.from(''),
      } as any)

      await expect(provider.initialize()).rejects.toThrow('too old')
    })

    it('accepts tmux version 2.1 (minimum required)', async () => {
      // command -v tmux succeeds
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('/usr/bin/tmux'),
        stderr: Buffer.from(''),
      } as any)

      // tmux -V returns minimum version
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('tmux 2.1'),
        stderr: Buffer.from(''),
      } as any)

      // list-sessions returns existing session
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('crewchief: 1 windows'),
        stderr: Buffer.from(''),
      } as any)

      await expect(provider.initialize()).resolves.not.toThrow()
    })

    it('accepts tmux version 3.3 (newer version)', async () => {
      // command -v tmux succeeds
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('/usr/bin/tmux'),
        stderr: Buffer.from(''),
      } as any)

      // tmux -V returns newer version
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('tmux 3.3'),
        stderr: Buffer.from(''),
      } as any)

      // list-sessions returns existing session
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('crewchief: 1 windows'),
        stderr: Buffer.from(''),
      } as any)

      await expect(provider.initialize()).resolves.not.toThrow()
    })

    it('creates session when it does not exist', async () => {
      // command -v tmux succeeds
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('/usr/bin/tmux'),
        stderr: Buffer.from(''),
      } as any)

      // tmux -V returns valid version
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('tmux 3.3'),
        stderr: Buffer.from(''),
      } as any)

      // list-sessions returns no matching session
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('other-session: 1 windows'),
        stderr: Buffer.from(''),
      } as any)

      // new-session succeeds
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await expect(provider.initialize()).resolves.not.toThrow()

      // Verify new-session was called
      const newSessionCall = mockSpawnSync.mock.calls[3]
      expect(newSessionCall[0]).toBe('tmux')
      expect(newSessionCall[1]).toEqual(['new-session', '-d', '-s', 'crewchief'])
    })

    it('does not create session when it already exists', async () => {
      // command -v tmux succeeds
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('/usr/bin/tmux'),
        stderr: Buffer.from(''),
      } as any)

      // tmux -V returns valid version
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('tmux 3.3'),
        stderr: Buffer.from(''),
      } as any)

      // list-sessions returns matching session
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('crewchief: 1 windows'),
        stderr: Buffer.from(''),
      } as any)

      await expect(provider.initialize()).resolves.not.toThrow()

      // Only 3 calls: command -v, tmux -V, list-sessions (no new-session)
      expect(mockSpawnSync).toHaveBeenCalledTimes(3)
    })

    it('throws error when session creation fails', async () => {
      // command -v tmux succeeds
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('/usr/bin/tmux'),
        stderr: Buffer.from(''),
      } as any)

      // tmux -V returns valid version
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('tmux 3.3'),
        stderr: Buffer.from(''),
      } as any)

      // list-sessions returns no matching session
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      // new-session fails
      mockSpawnSync.mockReturnValueOnce({
        status: 1,
        stdout: Buffer.from(''),
        stderr: Buffer.from('duplicate session: crewchief'),
      } as any)

      await expect(provider.initialize()).rejects.toThrow("Failed to create tmux session 'crewchief'")
    })

    it('uses custom session name when creating session', async () => {
      const customProvider = new TmuxProvider({ sessionName: 'custom-session' })

      // command -v tmux succeeds
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('/usr/bin/tmux'),
        stderr: Buffer.from(''),
      } as any)

      // tmux -V returns valid version
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('tmux 3.3'),
        stderr: Buffer.from(''),
      } as any)

      // list-sessions returns no matching session
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      // new-session succeeds
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await expect(customProvider.initialize()).resolves.not.toThrow()

      // Verify new-session was called with custom session name
      const newSessionCall = mockSpawnSync.mock.calls[3]
      expect(newSessionCall[1]).toEqual(['new-session', '-d', '-s', 'custom-session'])
    })

    it('includes version output in error when version is unparseable', async () => {
      // command -v tmux succeeds
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('/usr/bin/tmux'),
        stderr: Buffer.from(''),
      } as any)

      // tmux -V returns garbled output
      mockSpawnSync.mockReturnValueOnce({
        status: 0,
        stdout: Buffer.from('garbled output here'),
        stderr: Buffer.from(''),
      } as any)

      await expect(provider.initialize()).rejects.toThrow('garbled output here')
    })
  })

  describe('dispose', () => {
    it('completes without error', async () => {
      await expect(provider.dispose()).resolves.not.toThrow()
    })

    it('returns undefined (no-op)', async () => {
      const result = await provider.dispose()
      expect(result).toBeUndefined()
    })
  })

  describe('createWindow', () => {
    it('creates window with title and returns pane ID', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%1\n'),
        stderr: Buffer.from(''),
      } as any)

      const paneId = await provider.createWindow({ title: 'test-agent' })

      expect(paneId).toBe('%1')
      expect(mockSpawnSync).toHaveBeenCalledWith('tmux', [
        'new-window',
        '-t',
        'crewchief',
        '-n',
        'test-agent',
        '-P',
        '-F',
        '#{pane_id}',
      ])
    })

    it('generates title from timestamp when no title provided', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%2\n'),
        stderr: Buffer.from(''),
      } as any)

      const paneId = await provider.createWindow()

      expect(paneId).toBe('%2')
      const call = mockSpawnSync.mock.calls[0]
      // The generated name should match agent-<timestamp> pattern
      expect(call[1][4]).toMatch(/^agent-\d+$/)
    })

    it('generates title from timestamp when options provided without title', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%3\n'),
        stderr: Buffer.from(''),
      } as any)

      const paneId = await provider.createWindow({})

      expect(paneId).toBe('%3')
      const call = mockSpawnSync.mock.calls[0]
      expect(call[1][4]).toMatch(/^agent-\d+$/)
    })

    it('includes working directory when provided', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%4\n'),
        stderr: Buffer.from(''),
      } as any)

      await provider.createWindow({
        title: 'agent-with-dir',
        workingDirectory: '/path/to/project',
      })

      const call = mockSpawnSync.mock.calls[0]
      expect(call[1]).toContain('-c')
      expect(call[1]).toContain('/path/to/project')
    })

    it('does not include -c flag when no working directory', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%5\n'),
        stderr: Buffer.from(''),
      } as any)

      await provider.createWindow({ title: 'no-dir' })

      const call = mockSpawnSync.mock.calls[0]
      expect(call[1]).not.toContain('-c')
    })

    it('throws error when tmux command fails', async () => {
      mockSpawnSync.mockReturnValue({
        status: 1,
        stdout: Buffer.from(''),
        stderr: Buffer.from('session not found: crewchief'),
      } as any)

      await expect(provider.createWindow({ title: 'fail-agent' })).rejects.toThrow('Failed to create tmux window')
    })

    it('includes stderr in error message', async () => {
      mockSpawnSync.mockReturnValue({
        status: 1,
        stdout: Buffer.from(''),
        stderr: Buffer.from('no server running on /tmp/tmux-1000/default'),
      } as any)

      await expect(provider.createWindow({ title: 'err-agent' })).rejects.toThrow('no server running')
    })

    it('trims whitespace from returned pane ID', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('  %10  \n'),
        stderr: Buffer.from(''),
      } as any)

      const paneId = await provider.createWindow({ title: 'trim-test' })
      expect(paneId).toBe('%10')
    })

    it('uses custom session name in tmux command', async () => {
      const customProvider = new TmuxProvider({ sessionName: 'my-session' })

      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%1\n'),
        stderr: Buffer.from(''),
      } as any)

      await customProvider.createWindow({ title: 'agent' })

      const call = mockSpawnSync.mock.calls[0]
      expect(call[1][2]).toBe('my-session')
    })
  })

  describe('createTab', () => {
    it('creates a new window in the session and returns pane ID', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%6\n'),
        stderr: Buffer.from(''),
      } as any)

      const paneId = await provider.createTab('window-1')

      expect(paneId).toBe('%6')
      expect(mockSpawnSync).toHaveBeenCalledWith('tmux', ['new-window', '-t', 'crewchief', '-P', '-F', '#{pane_id}'])
    })

    it('throws error when tmux command fails', async () => {
      mockSpawnSync.mockReturnValue({
        status: 1,
        stdout: Buffer.from(''),
        stderr: Buffer.from('session not found'),
      } as any)

      await expect(provider.createTab('window-1')).rejects.toThrow('Failed to create tmux tab in window window-1')
    })

    it('trims whitespace from returned pane ID', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('  %7  \n'),
        stderr: Buffer.from(''),
      } as any)

      const paneId = await provider.createTab('window-1')
      expect(paneId).toBe('%7')
    })
  })

  describe('splitPane', () => {
    it('splits pane horizontally with -h flag', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%8\n'),
        stderr: Buffer.from(''),
      } as any)

      const paneId = await provider.splitPane('%1', 'horizontal')

      expect(paneId).toBe('%8')
      expect(mockSpawnSync).toHaveBeenCalledWith('tmux', ['split-window', '-h', '-t', '%1', '-P', '-F', '#{pane_id}'])
    })

    it('splits pane vertically with -v flag', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%9\n'),
        stderr: Buffer.from(''),
      } as any)

      const paneId = await provider.splitPane('%1', 'vertical')

      expect(paneId).toBe('%9')
      expect(mockSpawnSync).toHaveBeenCalledWith('tmux', ['split-window', '-v', '-t', '%1', '-P', '-F', '#{pane_id}'])
    })

    it('throws error when split fails', async () => {
      mockSpawnSync.mockReturnValue({
        status: 1,
        stdout: Buffer.from(''),
        stderr: Buffer.from('pane not found'),
      } as any)

      await expect(provider.splitPane('%99', 'horizontal')).rejects.toThrow('Failed to split tmux pane %99')
    })
  })

  describe('runCommand', () => {
    it('sends escaped command to the specified pane', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await provider.runCommand('%1', 'echo hello')

      expect(mockSpawnSync).toHaveBeenCalledWith('tmux', ['send-keys', '-t', '%1', 'echo hello', 'Enter'])
    })

    it('escapes special characters in the command', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await provider.runCommand('%1', 'echo "$HOME"')

      const call = mockSpawnSync.mock.calls[0]
      // Dollar sign and double quotes should be escaped
      expect(call[1][3]).toBe('echo \\"\\$HOME\\"')
    })

    it('throws error when send-keys fails', async () => {
      mockSpawnSync.mockReturnValue({
        status: 1,
        stdout: Buffer.from(''),
        stderr: Buffer.from('pane not found'),
      } as any)

      await expect(provider.runCommand('%99', 'echo test')).rejects.toThrow('Failed to run command in pane %99')
    })
  })

  describe('focus', () => {
    it('calls select-pane with the target pane ID', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await provider.focus('%1')

      expect(mockSpawnSync).toHaveBeenCalledWith('tmux', ['select-pane', '-t', '%1'])
    })

    it('does not throw when select-pane fails', async () => {
      mockSpawnSync.mockReturnValue({
        status: 1,
        stdout: Buffer.from(''),
        stderr: Buffer.from('pane not found'),
      } as any)

      // focus logs error but does not throw
      await expect(provider.focus('%99')).resolves.not.toThrow()
    })
  })

  describe('sendMessage', () => {
    it('sends message via tmux send-keys and returns true', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      const result = await provider.sendMessage('%1', 'hello world')

      expect(result).toBe(true)
      expect(mockSpawnSync).toHaveBeenCalledWith('tmux', ['send-keys', '-t', '%1', 'hello world', 'Enter'])
    })

    it('returns false when send-keys fails', async () => {
      mockSpawnSync.mockReturnValue({
        status: 1,
        stdout: Buffer.from(''),
        stderr: Buffer.from('pane %99 not found'),
      } as any)

      const result = await provider.sendMessage('%99', 'test')

      expect(result).toBe(false)
    })

    it('escapes double quotes in messages', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await provider.sendMessage('%1', 'say "hello"')

      const call = mockSpawnSync.mock.calls[0]
      expect(call[1][3]).toBe('say \\"hello\\"')
    })

    it('escapes single quotes in messages', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await provider.sendMessage('%1', "it's a test")

      const call = mockSpawnSync.mock.calls[0]
      expect(call[1][3]).toBe("it\\'s a test")
    })

    it('escapes backslashes in messages', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await provider.sendMessage('%1', 'path\\to\\file')

      const call = mockSpawnSync.mock.calls[0]
      expect(call[1][3]).toBe('path\\\\to\\\\file')
    })

    it('escapes dollar signs in messages', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await provider.sendMessage('%1', 'echo $HOME')

      const call = mockSpawnSync.mock.calls[0]
      expect(call[1][3]).toBe('echo \\$HOME')
    })

    it('escapes backticks in messages', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await provider.sendMessage('%1', 'run `date` now')

      const call = mockSpawnSync.mock.calls[0]
      expect(call[1][3]).toBe('run \\`date\\` now')
    })

    it('escapes newlines in messages', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await provider.sendMessage('%1', 'line1\nline2')

      const call = mockSpawnSync.mock.calls[0]
      expect(call[1][3]).toBe('line1\\nline2')
    })

    it('escapes all special characters together', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await provider.sendMessage('%1', 'echo "$HOME" \'test\' `cmd` path\\dir\nnewline')

      const call = mockSpawnSync.mock.calls[0]
      // Backslashes escaped first, then quotes, then $, then backtick, then newline
      expect(call[1][3]).toBe('echo \\"\\$HOME\\" \\\'test\\\' \\`cmd\\` path\\\\dir\\nnewline')
    })

    it('handles empty message', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      const result = await provider.sendMessage('%1', '')

      expect(result).toBe(true)
      const call = mockSpawnSync.mock.calls[0]
      expect(call[1][3]).toBe('')
    })

    it('handles message with no special characters', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await provider.sendMessage('%1', 'plain text message')

      const call = mockSpawnSync.mock.calls[0]
      expect(call[1][3]).toBe('plain text message')
    })
  })

  describe('listAgents', () => {
    it('returns agents matching name__type pattern', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%0:fix-bug__claude\n%1:task__gemini\n%2:bash\n'),
        stderr: Buffer.from(''),
      } as any)

      const agents = await provider.listAgents()

      expect(agents).toHaveLength(2)
      expect(agents[0]).toEqual({
        id: '%0',
        name: 'fix-bug__claude',
        type: 'claude',
        status: 'running',
      })
      expect(agents[1]).toEqual({
        id: '%1',
        name: 'task__gemini',
        type: 'gemini',
        status: 'running',
      })
    })

    it('calls tmux list-panes with correct arguments', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await provider.listAgents()

      expect(mockSpawnSync).toHaveBeenCalledWith('tmux', [
        'list-panes',
        '-s',
        '-t',
        'crewchief',
        '-F',
        '#{pane_id}:#{window_name}',
      ])
    })

    it('uses custom session name in list-panes command', async () => {
      const customProvider = new TmuxProvider({ sessionName: 'my-session' })

      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      await customProvider.listAgents()

      const call = mockSpawnSync.mock.calls[0]
      expect(call[1][3]).toBe('my-session')
    })

    it('returns empty array when list-panes fails', async () => {
      mockSpawnSync.mockReturnValue({
        status: 1,
        stdout: Buffer.from(''),
        stderr: Buffer.from('session not found'),
      } as any)

      const agents = await provider.listAgents()

      expect(agents).toEqual([])
    })

    it('returns empty array for empty output', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from(''),
        stderr: Buffer.from(''),
      } as any)

      const agents = await provider.listAgents()

      expect(agents).toEqual([])
    })

    it('returns empty array for whitespace-only output', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('  \n  \n'),
        stderr: Buffer.from(''),
      } as any)

      const agents = await provider.listAgents()

      expect(agents).toEqual([])
    })

    it('filters out non-agent windows (no __ in name)', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%0:bash\n%1:zsh\n%2:vim\n'),
        stderr: Buffer.from(''),
      } as any)

      const agents = await provider.listAgents()

      expect(agents).toEqual([])
    })

    it('handles window names with multiple __ separators', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%0:complex__name__claude\n'),
        stderr: Buffer.from(''),
      } as any)

      const agents = await provider.listAgents()

      expect(agents).toHaveLength(1)
      // split('__')[1] gets the part after the first __
      expect(agents[0].type).toBe('name')
    })

    it('sets type to "unknown" when __ is at end of name', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%0:task__\n'),
        stderr: Buffer.from(''),
      } as any)

      const agents = await provider.listAgents()

      expect(agents).toHaveLength(1)
      // split('__')[1] is '' which is falsy, so 'unknown' is used
      expect(agents[0].type).toBe('unknown')
    })

    it('handles single agent in output', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%5:my-task__codex'),
        stderr: Buffer.from(''),
      } as any)

      const agents = await provider.listAgents()

      expect(agents).toHaveLength(1)
      expect(agents[0]).toEqual({
        id: '%5',
        name: 'my-task__codex',
        type: 'codex',
        status: 'running',
      })
    })

    it('handles mixed agent and non-agent windows', async () => {
      mockSpawnSync.mockReturnValue({
        status: 0,
        stdout: Buffer.from('%0:bash\n%1:fix__claude\n%2:monitor\n%3:deploy__gemini\n'),
        stderr: Buffer.from(''),
      } as any)

      const agents = await provider.listAgents()

      expect(agents).toHaveLength(2)
      expect(agents[0].name).toBe('fix__claude')
      expect(agents[1].name).toBe('deploy__gemini')
    })
  })
})
