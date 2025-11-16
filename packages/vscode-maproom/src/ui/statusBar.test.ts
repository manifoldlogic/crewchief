/**
 * Tests for StatusBarManager
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { EventEmitter } from 'node:events'
import type { WatchEvent } from '../process/events'

/**
 * Mock StatusBarItem
 */
class MockStatusBarItem {
  text = ''
  tooltip = ''
  command: string | undefined = undefined
  alignment = 2 // StatusBarAlignment.Right
  priority = 100
  isShown = false

  show() {
    this.isShown = true
  }

  hide() {
    this.isShown = false
  }

  dispose() {
    this.isShown = false
  }
}

/**
 * Mock ExtensionContext
 */
class MockExtensionContext {
  private workspaceStateData = new Map<string, any>()

  workspaceState = {
    get: (key: string) => this.workspaceStateData.get(key),
    update: (key: string, value: any) => {
      this.workspaceStateData.set(key, value)
      return Promise.resolve()
    },
    keys: () => Array.from(this.workspaceStateData.keys()),
  }

  // Mock other required properties
  subscriptions: any[] = []
  extensionPath = '/mock/extension/path'
  extensionUri = { fsPath: '/mock/extension/path' } as any
  globalState: any = {
    get: () => undefined,
    update: () => Promise.resolve(),
    keys: () => [],
  }
  storagePath: string | undefined = '/mock/storage'
  globalStoragePath = '/mock/global/storage'
  logPath = '/mock/log'
  extensionMode = 3 // ExtensionMode.Test
}

/**
 * Mock ProcessOrchestrator (extends EventEmitter)
 */
class MockOrchestrator extends EventEmitter {
  emitWatchEvent(processName: string, event: WatchEvent) {
    this.emit('watchEvent', processName, event)
  }
}

/**
 * Global status bar item reference for testing
 */
let globalStatusBarItem: MockStatusBarItem | null = null

/**
 * Mock vscode module
 */
vi.mock('vscode', () => ({
  window: {
    createStatusBarItem: (alignment: number, priority: number) => {
      const item = new MockStatusBarItem()
      item.alignment = alignment
      item.priority = priority
      globalStatusBarItem = item
      return item
    },
  },
  StatusBarAlignment: {
    Left: 1,
    Right: 2,
  },
}))

// Import StatusBarManager after mock is set up
const { StatusBarManager } = await import('./statusBar.js')

describe('StatusBarManager', () => {
  let context: MockExtensionContext
  let orchestrator: MockOrchestrator
  let statusBar: StatusBarManager
  let statusBarItem: MockStatusBarItem

  beforeEach(() => {
    context = new MockExtensionContext()
    orchestrator = new MockOrchestrator()
    statusBar = new StatusBarManager(context as any, orchestrator as any)
    statusBarItem = globalStatusBarItem!
  })

  afterEach(() => {
    statusBar.dispose()
    vi.clearAllMocks()
  })

  describe('initialization', () => {
    it('should create status bar item with correct alignment and priority', () => {
      expect(statusBarItem.alignment).toBe(2) // Right
      expect(statusBarItem.priority).toBe(100)
    })

    it('should set command to maproom.showOutput', () => {
      expect(statusBarItem.command).toBe('maproom.showOutput')
    })

    it('should show status bar item after creation', () => {
      expect(statusBarItem.isShown).toBe(true)
    })

    it('should initialize with starting state', () => {
      expect(statusBarItem.text).toContain('Starting...')
      expect(statusBarItem.text).toContain('$(sync~spin)')
    })
  })

  describe('status event handling', () => {
    it('should update text to "Watching..." on watching state', async () => {
      orchestrator.emitWatchEvent('watch', { type: 'status', state: 'watching' })

      // Wait for debounce
      await sleep(1100)

      expect(statusBarItem.text).toContain('Watching...')
      expect(statusBarItem.text).toContain('$(eye)')
    })

    it('should update text to "Indexing" on indexing state', async () => {
      orchestrator.emitWatchEvent('watch', { type: 'status', state: 'indexing' })

      // Wait for debounce
      await sleep(1100)

      expect(statusBarItem.text).toContain('Indexing')
      expect(statusBarItem.text).toContain('$(sync~spin)')
    })

    it('should update text to "Maproom Ready" on idle state', async () => {
      // First set to watching
      orchestrator.emitWatchEvent('watch', { type: 'status', state: 'watching' })
      await sleep(1100)

      // Then back to idle
      orchestrator.emitWatchEvent('watch', { type: 'status', state: 'idle' })
      await sleep(1100)

      expect(statusBarItem.text).toContain('Maproom Ready')
      expect(statusBarItem.text).toContain('$(database)')
    })
  })

  describe('progress event handling', () => {
    it('should show file count during indexing', async () => {
      orchestrator.emitWatchEvent('watch', {
        type: 'progress',
        complete: 15,
        files: 100,
      })

      // Wait for debounce
      await sleep(1100)

      expect(statusBarItem.text).toContain('Indexing')
      expect(statusBarItem.text).toContain('100 files')
      expect(statusBarItem.text).toContain('$(sync~spin)')
    })

    it('should update progress count as indexing progresses', async () => {
      // First progress
      orchestrator.emitWatchEvent('watch', {
        type: 'progress',
        complete: 25,
        files: 100,
      })
      await sleep(1100)

      expect(statusBarItem.text).toContain('100 files')

      // Second progress with different total
      orchestrator.emitWatchEvent('watch', {
        type: 'progress',
        complete: 75,
        files: 150,
      })
      await sleep(1100)

      expect(statusBarItem.text).toContain('150 files')
    })
  })

  describe('complete event handling', () => {
    it('should transition to watching state after completion', async () => {
      orchestrator.emitWatchEvent('watch', {
        type: 'complete',
        files: 100,
        duration: 5000,
      })

      // Wait for debounce
      await sleep(1100)

      expect(statusBarItem.text).toContain('Indexed')
      expect(statusBarItem.text).toContain('100 files')
      expect(statusBarItem.text).toContain('$(eye)')
    })

    it('should store last indexed timestamp on completion', async () => {
      const beforeTime = Date.now()

      orchestrator.emitWatchEvent('watch', {
        type: 'complete',
        files: 100,
        duration: 5000,
      })

      await sleep(1100)

      const lastIndexed = context.workspaceState.get<number>('maproom.lastIndexed')
      expect(lastIndexed).toBeDefined()
      expect(lastIndexed).toBeGreaterThanOrEqual(beforeTime)
      expect(lastIndexed).toBeLessThanOrEqual(Date.now())
    })
  })

  describe('error event handling', () => {
    it('should show error icon and text on error', async () => {
      orchestrator.emitWatchEvent('watch', {
        type: 'error',
        message: 'Failed to index file',
        file: 'src/test.ts',
      })

      // Wait for debounce
      await sleep(1100)

      expect(statusBarItem.text).toContain('Maproom Error')
      expect(statusBarItem.text).toContain('$(error)')
    })

    it('should include error message in tooltip', async () => {
      orchestrator.emitWatchEvent('watch', {
        type: 'error',
        message: 'Connection refused',
      })

      // Wait for debounce
      await sleep(1100)

      expect(statusBarItem.tooltip).toContain('Connection refused')
    })

    it('should include file path in error message when provided', async () => {
      orchestrator.emitWatchEvent('watch', {
        type: 'error',
        message: 'Parse failed',
        file: 'src/broken.ts',
      })

      await sleep(1100)

      expect(statusBarItem.tooltip).toContain('Parse failed (in src/broken.ts)')
    })

    it('should include error type in error message when provided', async () => {
      orchestrator.emitWatchEvent('watch', {
        type: 'error',
        message: 'Database connection lost',
        error_type: 'database',
      })

      await sleep(1100)

      expect(statusBarItem.tooltip).toContain('[database] Database connection lost')
    })

    it('should include both file path and error type when both provided', async () => {
      orchestrator.emitWatchEvent('watch', {
        type: 'error',
        message: 'Failed to read',
        file: 'large.bin',
        error_type: 'io',
      })

      await sleep(1100)

      expect(statusBarItem.tooltip).toContain('[io] Failed to read (in large.bin)')
    })
  })

  describe('file_processed event handling', () => {
    it('should handle file_processed events', async () => {
      orchestrator.emitWatchEvent('watch', {
        type: 'file_processed',
        file_path: 'src/components/App.tsx',
        elapsed: 125,
      })

      await sleep(1100)

      // Should maintain indexing state
      expect(statusBarItem.text).toContain('Indexing')
    })
  })

  describe('tooltip updates', () => {
    it('should include current state in tooltip', async () => {
      orchestrator.emitWatchEvent('watch', { type: 'status', state: 'watching' })
      await sleep(1100)

      expect(statusBarItem.tooltip).toContain('Status: watching')
    })

    it('should include progress in tooltip during indexing', async () => {
      orchestrator.emitWatchEvent('watch', {
        type: 'progress',
        complete: 50,
        files: 200,
      })
      await sleep(1100)

      expect(statusBarItem.tooltip).toContain('Progress: 50/200 files')
    })

    it('should include last indexed time in tooltip', async () => {
      // Set a known timestamp (2 hours ago)
      const twoHoursAgo = Date.now() - 2 * 60 * 60 * 1000
      context.workspaceState.update('maproom.lastIndexed', twoHoursAgo)

      // Trigger an update
      orchestrator.emitWatchEvent('watch', { type: 'status', state: 'watching' })
      await sleep(1100)

      expect(statusBarItem.tooltip).toContain('Last indexed: 2 hours ago')
    })

    it('should include click instruction in tooltip', async () => {
      expect(statusBarItem.tooltip).toContain('Click to show output')
    })
  })

  describe('debouncing', () => {
    it('should debounce rapid updates', async () => {
      // Send multiple rapid updates
      for (let i = 0; i < 10; i++) {
        orchestrator.emitWatchEvent('watch', {
          type: 'progress',
          complete: i * 10,
          files: 100,
        })
        await sleep(50) // Much faster than debounce interval
      }

      // Should still be showing one of the intermediate states
      const textBeforeDebounce = statusBarItem.text

      // Wait for debounce
      await sleep(1100)

      // Should now show the final state (100 files total)
      expect(statusBarItem.text).toContain('100 files')
    })

    it('should not update more than once per second', async () => {
      let updateCount = 0
      let lastText = statusBarItem.text

      // Track updates by watching text changes
      const checkInterval = setInterval(() => {
        if (statusBarItem.text !== lastText) {
          updateCount++
          lastText = statusBarItem.text
        }
      }, 100)

      // Send many rapid updates over 2 seconds
      for (let i = 0; i < 20; i++) {
        orchestrator.emitWatchEvent('watch', {
          type: 'progress',
          complete: i * 5,
          files: 100,
        })
        await sleep(100)
      }

      // Wait a bit more for final debounce
      await sleep(1100)

      clearInterval(checkInterval)

      // Should have updated only a few times (3-4 max for 2 seconds of updates with 1s debounce)
      // With 20 events over 2s and 1s debounce, we expect ~2-3 updates
      expect(updateCount).toBeLessThan(6)
      expect(updateCount).toBeGreaterThan(0)
    })
  })

  describe('time formatting', () => {
    it('should format recent time as "just now"', async () => {
      const now = Date.now()
      context.workspaceState.update('maproom.lastIndexed', now - 30000) // 30 seconds ago

      orchestrator.emitWatchEvent('watch', { type: 'status', state: 'watching' })
      await sleep(1100)

      expect(statusBarItem.tooltip).toContain('Last indexed: just now')
    })

    it('should format minutes correctly', async () => {
      const fiveMinutesAgo = Date.now() - 5 * 60 * 1000
      context.workspaceState.update('maproom.lastIndexed', fiveMinutesAgo)

      orchestrator.emitWatchEvent('watch', { type: 'status', state: 'watching' })
      await sleep(1100)

      expect(statusBarItem.tooltip).toContain('Last indexed: 5 minutes ago')
    })

    it('should use singular for 1 minute', async () => {
      const oneMinuteAgo = Date.now() - 65 * 1000 // 65 seconds
      context.workspaceState.update('maproom.lastIndexed', oneMinuteAgo)

      orchestrator.emitWatchEvent('watch', { type: 'status', state: 'watching' })
      await sleep(1100)

      expect(statusBarItem.tooltip).toContain('Last indexed: 1 minute ago')
    })

    it('should format hours correctly', async () => {
      const threeHoursAgo = Date.now() - 3 * 60 * 60 * 1000
      context.workspaceState.update('maproom.lastIndexed', threeHoursAgo)

      orchestrator.emitWatchEvent('watch', { type: 'status', state: 'watching' })
      await sleep(1100)

      expect(statusBarItem.tooltip).toContain('Last indexed: 3 hours ago')
    })

    it('should format days correctly', async () => {
      const twoDaysAgo = Date.now() - 2 * 24 * 60 * 60 * 1000
      context.workspaceState.update('maproom.lastIndexed', twoDaysAgo)

      orchestrator.emitWatchEvent('watch', { type: 'status', state: 'watching' })
      await sleep(1100)

      expect(statusBarItem.tooltip).toContain('Last indexed: 2 days ago')
    })
  })

  describe('disposal', () => {
    it('should dispose status bar item', () => {
      statusBar.dispose()

      expect(statusBarItem.isShown).toBe(false)
    })

    it('should clear pending debounce timer', async () => {
      // Schedule an update
      orchestrator.emitWatchEvent('watch', { type: 'status', state: 'watching' })

      // Dispose before debounce completes
      statusBar.dispose()

      const textBeforeDispose = statusBarItem.text

      // Wait for what would have been the debounce
      await sleep(1100)

      // Text should not have changed after disposal
      expect(statusBarItem.text).toBe(textBeforeDispose)
    })

    it('should not update after disposal', async () => {
      statusBar.dispose()

      const textAfterDispose = statusBarItem.text

      // Try to emit event
      orchestrator.emitWatchEvent('watch', { type: 'status', state: 'watching' })
      await sleep(1100)

      // Text should not change
      expect(statusBarItem.text).toBe(textAfterDispose)
    })
  })
})

/**
 * Sleep utility for tests
 */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}
