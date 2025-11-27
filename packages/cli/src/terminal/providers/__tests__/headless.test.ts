import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { HeadlessProvider } from '../headless'

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
