import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { TerminalFactory } from '../factory'
import { HeadlessProvider } from '../providers/headless'
import { MockProvider } from '../providers/mock'

describe('Terminal Provider Smoke Tests', () => {
  describe('TerminalFactory', () => {
    it('auto-detects headless in non-iTerm environment', () => {
      // In this test environment, TERM_PROGRAM is not iTerm.app
      const provider = TerminalFactory.autoDetect()
      expect(provider.id).toBe('headless')
    })

    it('returns mock provider when requested', () => {
      const provider = TerminalFactory.getProvider('mock')
      expect(provider.id).toBe('mock')
    })

    it('returns headless provider when requested', () => {
      const provider = TerminalFactory.getProvider('headless')
      expect(provider.id).toBe('headless')
    })
  })

  describe('MockProvider', () => {
    let mock: MockProvider

    beforeEach(async () => {
      mock = new MockProvider()
      await mock.initialize()
    })

    afterEach(async () => {
      await mock.dispose()
    })

    it('tracks created windows', async () => {
      const windowId = await mock.createWindow({ title: 'Test Window' })
      expect(windowId).toMatch(/^mock-window-\d+$/)
      expect(mock.windows).toContain(windowId)
    })

    it('tracks created panes', async () => {
      await mock.createWindow()
      const initialPaneId = Object.keys(mock.panes)[0]
      const newPaneId = await mock.splitPane(initialPaneId, 'vertical')
      expect(newPaneId).toMatch(/^mock-pane-\d+$/)
      expect(mock.panes[newPaneId]).toBeDefined()
    })

    it('records executed commands', async () => {
      await mock.createWindow()
      const paneId = Object.keys(mock.panes)[0]
      await mock.runCommand(paneId, 'echo hello')
      expect(mock.executedCommands).toHaveLength(1)
      expect(mock.executedCommands[0]).toEqual({
        paneId,
        command: 'echo hello',
      })
    })

    it('throws for invalid pane ID on runCommand', async () => {
      await expect(mock.runCommand('nonexistent', 'echo')).rejects.toThrow('does not exist')
    })

    it('resets state on dispose', async () => {
      await mock.createWindow()
      await mock.dispose()
      expect(mock.windows).toHaveLength(0)
      expect(mock.executedCommands).toHaveLength(0)
    })
  })

  describe('HeadlessProvider', () => {
    let headless: HeadlessProvider

    beforeEach(async () => {
      headless = new HeadlessProvider()
      await headless.initialize()
    })

    afterEach(async () => {
      await headless.dispose()
    })

    it('has correct provider id', () => {
      expect(headless.id).toBe('headless')
    })

    it('creates logical window IDs', async () => {
      const windowId = await headless.createWindow({ title: 'Test' })
      expect(windowId).toMatch(/^headless-window-\d+$/)
    })

    it('creates logical pane IDs via createTab', async () => {
      const windowId = await headless.createWindow()
      const paneId = await headless.createTab(windowId)
      expect(paneId).toMatch(/^headless-pane-\d+$/)
    })

    it('creates logical pane IDs via splitPane', async () => {
      const paneId = await headless.splitPane('dummy', 'horizontal')
      expect(paneId).toMatch(/^headless-pane-\d+$/)
    })

    it('spawns and cleans up processes', async () => {
      const paneId = await headless.createTab('window')
      // Spawn a quick process
      await headless.runCommand(paneId, 'echo smoke-test-success')
      // Give it a moment
      await new Promise((r) => setTimeout(r, 200))
      // Dispose should clean up
      await headless.dispose()
      // If we get here without hanging, cleanup worked
      expect(true).toBe(true)
    })

    it('focus is a no-op but does not throw', async () => {
      await expect(headless.focus('any-pane')).resolves.toBeUndefined()
    })
  })
})
