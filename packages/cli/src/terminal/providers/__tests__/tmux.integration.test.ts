import { spawnSync } from 'node:child_process'
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { TmuxProvider } from '../tmux'

// Check tmux availability using POSIX-compatible detection
const tmuxCheck = spawnSync('command', ['-v', 'tmux'], { shell: true })
const tmuxAvailable = tmuxCheck.status === 0

describe.skipIf(!tmuxAvailable)('TmuxProvider Integration', () => {
  let provider: TmuxProvider
  const testSession = 'crewchief_test'

  beforeAll(async () => {
    // Kill any existing test session to start clean
    spawnSync('tmux', ['kill-session', '-t', testSession])

    // Create test session
    const result = spawnSync('tmux', ['new-session', '-d', '-s', testSession])
    if (result.status !== 0) {
      throw new Error(
        `Failed to create test session: ${result.stderr.toString().trim()}. ` +
          'Ensure tmux server can start (may need a terminal).',
      )
    }

    provider = new TmuxProvider({ sessionName: testSession })
    await provider.initialize()
  })

  afterAll(async () => {
    // Clean up test session
    spawnSync('tmux', ['kill-session', '-t', testSession])
  })

  it('initializes without error', async () => {
    // Provider was already initialized in beforeAll; verify it worked
    expect(provider.id).toBe('tmux')
  })

  it('creates window and returns pane ID', async () => {
    const paneId = await provider.createWindow({ title: 'integration-test' })
    expect(paneId).toMatch(/^%\d+$/)
  })

  it('creates window with working directory', async () => {
    const paneId = await provider.createWindow({
      title: 'dir-test',
      workingDirectory: '/tmp',
    })
    expect(paneId).toMatch(/^%\d+$/)
  })

  it('creates tab and returns pane ID', async () => {
    const paneId = await provider.createTab('window-1')
    expect(paneId).toMatch(/^%\d+$/)
  })

  it('splits pane horizontally', async () => {
    const windowPaneId = await provider.createWindow({ title: 'split-h-test' })
    const splitPaneId = await provider.splitPane(windowPaneId, 'horizontal')
    expect(splitPaneId).toMatch(/^%\d+$/)
    expect(splitPaneId).not.toBe(windowPaneId)
  })

  it('splits pane vertically', async () => {
    const windowPaneId = await provider.createWindow({ title: 'split-v-test' })
    const splitPaneId = await provider.splitPane(windowPaneId, 'vertical')
    expect(splitPaneId).toMatch(/^%\d+$/)
    expect(splitPaneId).not.toBe(windowPaneId)
  })

  it('sends message to pane', async () => {
    const paneId = await provider.createWindow({ title: 'msg-test__claude' })
    const result = await provider.sendMessage(paneId, 'test message')
    expect(result).toBe(true)
  })

  it('runs command in pane', async () => {
    const paneId = await provider.createWindow({ title: 'cmd-test' })
    await expect(provider.runCommand(paneId, 'echo hello')).resolves.not.toThrow()
  })

  it('focuses a pane without error', async () => {
    const paneId = await provider.createWindow({ title: 'focus-test' })
    await expect(provider.focus(paneId)).resolves.not.toThrow()
  })

  it('lists agents after creating agent-named windows', async () => {
    await provider.createWindow({ title: 'list-test__claude' })
    await provider.createWindow({ title: 'list-test2__gemini' })

    const agents = await provider.listAgents()
    const claudeAgent = agents.find((a) => a.name === 'list-test__claude')
    const geminiAgent = agents.find((a) => a.name === 'list-test2__gemini')

    expect(claudeAgent).toBeDefined()
    expect(claudeAgent?.platform).toBe('claude')
    expect(claudeAgent?.status).toBe('running')

    expect(geminiAgent).toBeDefined()
    expect(geminiAgent?.platform).toBe('gemini')
  })

  it('filters out non-agent windows from listAgents', async () => {
    await provider.createWindow({ title: 'plain-window' })

    const agents = await provider.listAgents()
    const plainAgent = agents.find((a) => a.name === 'plain-window')
    expect(plainAgent).toBeUndefined()
  })

  it('sends message with special characters', async () => {
    const paneId = await provider.createWindow({ title: 'escape-test__claude' })
    const result = await provider.sendMessage(paneId, 'echo "$HOME" \'test\' `date`')
    expect(result).toBe(true)
  })

  it('disposes without error', async () => {
    await expect(provider.dispose()).resolves.not.toThrow()
  })
})
