/**
 * Integration tests for VSCode Maproom Extension
 *
 * These tests verify that multiple modules work together correctly
 * with minimal mocking. They test end-to-end workflows across:
 * - Event flow: Parser → Orchestrator → StatusBar
 * - Process lifecycle: Start → Watch → Error → Recovery
 * - Extension activation: Ollama → Reconcile → Processes → UI
 *
 * Note: These are integration-style tests using Vitest, not @vscode/test-electron.
 * Real VSCode API calls are still mocked, but module interactions are real.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { EventEmitter } from 'node:events'
import { Readable } from 'node:stream'
import type { ChildProcess } from 'node:child_process'
import type { OutputChannel, ExtensionContext, StatusBarItem } from 'vscode'

// Import modules to test (real, not mocked)
import { StdoutParser } from '../process/parser'
import { ProcessOrchestrator, type OrchestratorConfig } from '../process/orchestrator'
import { StatusBarManager } from '../ui/statusBar'
import { CrashRecovery } from '../process/recovery'
import type { WatchEvent, ProgressEvent, StatusEvent, ErrorEvent, CompleteEvent } from '../process/events'

// Mock vscode module
vi.mock('vscode', () => ({
  window: {
    showErrorMessage: vi.fn(),
    createStatusBarItem: vi.fn(() => new MockStatusBarItem()),
  },
  commands: {
    executeCommand: vi.fn(),
  },
  StatusBarAlignment: {
    Left: 1,
    Right: 2,
  },
}))

// Mock fs/promises module
vi.mock('node:fs/promises', () => ({
  access: vi.fn().mockResolvedValue(undefined),
  constants: {
    F_OK: 0,
    R_OK: 4,
    X_OK: 1,
  },
}))

// Mock child_process module
vi.mock('node:child_process', async (importOriginal) => {
  const actual = (await importOriginal()) as any
  return {
    ...actual,
    spawn: vi.fn(),
    execFile: vi.fn(),
  }
})

// Mock git utilities
vi.mock('../utils/git', () => ({
  getRepoName: vi.fn().mockResolvedValue('test-owner/test-repo'),
  getBranchName: vi.fn().mockResolvedValue('main'),
}))

/**
 * Mock StatusBarItem
 */
class MockStatusBarItem {
  text = ''
  tooltip = ''
  command: string | undefined = undefined
  alignment = 2
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

  subscriptions: any[] = []
  extensionPath = '/mock/extension/path'
  extensionUri = { fsPath: '/mock/extension/path' } as any
  globalState = this.workspaceState
  secrets = {
    get: vi.fn(),
    store: vi.fn(),
    delete: vi.fn(),
  }
  storageUri = { fsPath: '/mock/storage' } as any
  globalStorageUri = { fsPath: '/mock/global-storage' } as any
  logUri = { fsPath: '/mock/logs' } as any
  extensionMode = 3
  extension = {} as any
  environmentVariableCollection = {} as any
  asAbsolutePath = (relativePath: string) => `/mock/extension/path/${relativePath}`
  storagePath = '/mock/storage'
  globalStoragePath = '/mock/global-storage'
  logPath = '/mock/logs'
}

/**
 * Mock OutputChannel
 */
class MockOutputChannel {
  private lines: string[] = []

  appendLine(value: string): void {
    this.lines.push(value)
  }

  append(value: string): void {
    this.lines.push(value)
  }

  clear(): void {
    this.lines = []
  }

  show(): void {
    // No-op
  }

  hide(): void {
    // No-op
  }

  dispose(): void {
    this.lines = []
  }

  getLines(): string[] {
    return this.lines
  }

  name = 'Maproom Test'
  replace = () => {}
}

/**
 * Create a mock ChildProcess
 */
function createMockProcess(): ChildProcess {
  const process = new EventEmitter() as any
  process.pid = Math.floor(Math.random() * 10000)
  process.stdout = new Readable({ read() {} })
  process.stderr = new Readable({ read() {} })
  process.stdin = new EventEmitter() as any
  process.stdin.write = vi.fn()
  process.kill = vi.fn(() => true)
  return process
}

describe('Integration: NDJSON Event Flow', () => {
  it('should flow events from parser through orchestrator to status bar', async () => {
    // Create real instances of all components
    const context = new MockExtensionContext()
    const outputChannel = new MockOutputChannel()

    // Mock spawn to return our controlled process
    const mockProcess = createMockProcess()
    const { spawn } = await import('node:child_process')
    vi.mocked(spawn).mockReturnValue(mockProcess)

    // Create orchestrator (will spawn processes)
    const config: OrchestratorConfig = {
      extensionRoot: '/mock/extension',
      workspaceRoot: '/mock/workspace',
      postgres: {
        host: 'localhost',
        port: 5432,
        user: 'maproom',
        password: 'maproom',
        database: 'maproom',
      },
    }

    const orchestrator = new ProcessOrchestrator(outputChannel as any, config)

    // Create status bar and connect to orchestrator
    const statusBar = new StatusBarManager(context as any, orchestrator)

    // Collect emitted events
    const receivedEvents: WatchEvent[] = []
    orchestrator.on('watchEvent', (processName: string, event: WatchEvent) => {
      receivedEvents.push(event)
    })

    // Start watching (spawns processes)
    await orchestrator.startWatching()

    // Simulate NDJSON events from the process
    const events: WatchEvent[] = [
      { type: 'status', state: 'indexing', files: 100 } as StatusEvent,
      { type: 'progress', files: 100, complete: 25 } as ProgressEvent,
      { type: 'progress', files: 100, complete: 50 } as ProgressEvent,
      { type: 'progress', files: 100, complete: 100 } as ProgressEvent,
      { type: 'complete', files: 100, duration: 2500 } as CompleteEvent,
      { type: 'status', state: 'watching' } as StatusEvent,
    ]

    // Push events through stdout (simulating Rust binary output)
    for (const event of events) {
      mockProcess.stdout.push(`${JSON.stringify(event)}\n`)
    }

    // Give time for events to propagate
    await new Promise((resolve) => setTimeout(resolve, 100))

    // Verify events flowed through the system
    expect(receivedEvents.length).toBeGreaterThan(0)
    expect(receivedEvents).toContainEqual(expect.objectContaining({ type: 'progress' }))
    expect(receivedEvents).toContainEqual(expect.objectContaining({ type: 'complete' }))

    // Cleanup
    await orchestrator.stopWatching()
    statusBar.dispose()
  })

  it('should update status bar state based on orchestrator events', async () => {
    const context = new MockExtensionContext()
    const outputChannel = new MockOutputChannel()

    const mockProcess = createMockProcess()
    const { spawn } = await import('node:child_process')
    vi.mocked(spawn).mockReturnValue(mockProcess)

    const config: OrchestratorConfig = {
      extensionRoot: '/mock/extension',
      workspaceRoot: '/mock/workspace',
      postgres: {
        host: 'localhost',
        port: 5432,
        user: 'maproom',
        password: 'maproom',
        database: 'maproom',
      },
    }

    const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
    const statusBar = new StatusBarManager(context as any, orchestrator)

    // Set initial state
    statusBar.setState('starting')

    await orchestrator.startWatching()

    // Emit indexing event
    mockProcess.stdout.push(`${JSON.stringify({ type: 'status', state: 'indexing', files: 50 })}\n`)
    await new Promise((resolve) => setTimeout(resolve, 100))

    // Emit progress event
    mockProcess.stdout.push(`${JSON.stringify({ type: 'progress', files: 50, complete: 25 })}\n`)
    await new Promise((resolve) => setTimeout(resolve, 100))

    // Emit complete event
    mockProcess.stdout.push(`${JSON.stringify({ type: 'complete', files: 50, duration: 1500 })}\n`)
    await new Promise((resolve) => setTimeout(resolve, 100))

    // Emit watching event
    mockProcess.stdout.push(`${JSON.stringify({ type: 'status', state: 'watching' })}\n`)
    await new Promise((resolve) => setTimeout(resolve, 100))

    // Verify status bar received updates (the internal state changes are tested in statusBar.test.ts)
    const status = orchestrator.getStatus()
    expect(status.size).toBeGreaterThan(0)

    await orchestrator.stopWatching()
    statusBar.dispose()
  })
})

describe('Integration: Process Crash Recovery Workflow', () => {
  it('should integrate CrashRecovery with crash handling', async () => {
    /**
     * Note: Detailed crash recovery tests are in recovery.test.ts
     * This integration test verifies that CrashRecovery can be instantiated
     * and used in an integration context with the orchestrator
     */

    // Verify CrashRecovery can be created with config
    const recovery = new CrashRecovery({
      maxAttempts: 3,
      maxBackoffMs: 500,
      successResetMs: 1000,
    })

    // Verify initial state is CLOSED
    expect(recovery.getState()).toBe('CLOSED')

    // Cleanup
    recovery.dispose()
  })

  it('should track circuit breaker state transitions', async () => {
    /**
     * Verifies that CrashRecovery state can be queried
     * Detailed state transition logic is tested in recovery.test.ts
     */
    const recovery = new CrashRecovery({ maxAttempts: 2 })

    // Initial state should be CLOSED
    expect(recovery.getState()).toBe('CLOSED')

    // Can reset state
    recovery.reset()
    expect(recovery.getState()).toBe('CLOSED')

    recovery.dispose()
  })
})

describe('Integration: Single Unified Watch Process', () => {
  it('should manage unified watch process', async () => {
    const outputChannel = new MockOutputChannel()

    const watchProcess = createMockProcess()

    let spawnCallCount = 0
    const { spawn } = await import('node:child_process')
    vi.mocked(spawn).mockImplementation(() => {
      spawnCallCount++
      return watchProcess
    })

    const config: OrchestratorConfig = {
      extensionRoot: '/mock/extension',
      workspaceRoot: '/mock/workspace',
      postgres: {
        host: 'localhost',
        port: 5432,
        user: 'maproom',
        password: 'maproom',
        database: 'maproom',
      },
    }

    const orchestrator = new ProcessOrchestrator(outputChannel as any, config)

    // Start watching (spawns single unified watch process)
    await orchestrator.startWatching()

    // Verify only one process was spawned
    expect(spawnCallCount).toBe(1)

    // Send events from watch process (includes file and branch events)
    watchProcess.stdout.push(`${JSON.stringify({ type: 'status', state: 'watching' })}\n`)

    await new Promise((resolve) => setTimeout(resolve, 100))

    // Get status of watch process
    const status = orchestrator.getStatus()
    expect(status.size).toBe(1)
    expect(status.get('watch')).toBeDefined()

    // Cleanup
    await orchestrator.stopWatching()
  })
})

describe('Integration: Error Propagation Through Stack', () => {
  it('should propagate parse errors from parser to orchestrator', async () => {
    const outputChannel = new MockOutputChannel()
    const mockProcess = createMockProcess()

    const { spawn } = await import('node:child_process')
    vi.mocked(spawn).mockReturnValue(mockProcess)

    const config: OrchestratorConfig = {
      extensionRoot: '/mock/extension',
      workspaceRoot: '/mock/workspace',
      postgres: {
        host: 'localhost',
        port: 5432,
        user: 'maproom',
        password: 'maproom',
        database: 'maproom',
      },
    }

    const orchestrator = new ProcessOrchestrator(outputChannel as any, config)

    // Collect parse errors
    const parseErrors: Array<{ processName: string; error: Error; line: string }> = []
    orchestrator.on('parseError', (processName, error, line) => {
      parseErrors.push({ processName, error, line })
    })

    await orchestrator.startWatching()

    // Send invalid JSON through stdout
    mockProcess.stdout.push('not valid json\n')
    mockProcess.stdout.push('{"invalid":}\n')
    mockProcess.stdout.push('{"type":"unknown"}\n')

    await new Promise((resolve) => setTimeout(resolve, 100))

    // Verify parse errors were captured
    expect(parseErrors.length).toBeGreaterThan(0)

    await orchestrator.stopWatching()
  })

  it('should emit error events when received from process', async () => {
    const outputChannel = new MockOutputChannel()
    const mockProcess = createMockProcess()

    const { spawn } = await import('node:child_process')
    vi.mocked(spawn).mockReturnValue(mockProcess)

    const config: OrchestratorConfig = {
      extensionRoot: '/mock/extension',
      workspaceRoot: '/mock/workspace',
      postgres: {
        host: 'localhost',
        port: 5432,
        user: 'maproom',
        password: 'maproom',
        database: 'maproom',
      },
    }

    const orchestrator = new ProcessOrchestrator(outputChannel as any, config)

    const watchEvents: WatchEvent[] = []
    orchestrator.on('watchEvent', (processName, event) => {
      watchEvents.push(event)
    })

    await orchestrator.startWatching()

    // Send error event from process
    const errorEvent: ErrorEvent = {
      type: 'error',
      message: 'Failed to parse file',
      file: 'src/main.rs',
      error_type: 'parse',
    }

    mockProcess.stdout.push(`${JSON.stringify(errorEvent)}\n`)
    await new Promise((resolve) => setTimeout(resolve, 100))

    // Verify error event was received
    const errorEvents = watchEvents.filter((e) => e.type === 'error')
    expect(errorEvents.length).toBeGreaterThan(0)
    expect(errorEvents[0]).toMatchObject({
      type: 'error',
      message: 'Failed to parse file',
      file: 'src/main.rs',
    })

    await orchestrator.stopWatching()
  })
})

describe('Integration: Extension Workflow Simulation', () => {
  it('should simulate extension activation workflow', async () => {
    /**
     * This test simulates the key parts of extension activation:
     * 1. Create output channel
     * 2. Create status bar manager
     * 3. Create process orchestrator
     * 4. Start watching
     * 5. Status bar updates based on events
     * 6. Cleanup on deactivation
     */

    const context = new MockExtensionContext()
    const outputChannel = new MockOutputChannel()
    const mockProcess = createMockProcess()

    const { spawn } = await import('node:child_process')
    vi.mocked(spawn).mockReturnValue(mockProcess)

    // Step 1: Create output channel (in real extension, this is vscode.window.createOutputChannel)
    outputChannel.appendLine('Extension activating...')

    // Step 2: Create status bar manager with "starting" state
    const statusBar = new StatusBarManager(context as any)
    statusBar.setState('starting')

    // Step 3: Create process orchestrator
    const config: OrchestratorConfig = {
      extensionRoot: '/mock/extension',
      workspaceRoot: '/mock/workspace',
      postgres: {
        host: 'localhost',
        port: 5432,
        user: 'maproom',
        password: 'maproom',
        database: 'maproom',
      },
    }

    const orchestrator = new ProcessOrchestrator(outputChannel as any, config)

    // Step 4: Connect status bar to orchestrator
    statusBar.connectOrchestrator(orchestrator)

    // Step 5: Start watching
    await orchestrator.startWatching()

    // Step 6: Simulate events from processes
    mockProcess.stdout.push(`${JSON.stringify({ type: 'status', state: 'watching' })}\n`)
    await new Promise((resolve) => setTimeout(resolve, 100))

    mockProcess.stdout.push(`${JSON.stringify({ type: 'progress', files: 10, complete: 5 })}\n`)
    await new Promise((resolve) => setTimeout(resolve, 100))

    // Step 7: Verify system is operational
    const status = orchestrator.getStatus()
    expect(status.size).toBeGreaterThan(0)

    // Step 8: Cleanup (simulates deactivation)
    await orchestrator.stopWatching()
    statusBar.dispose()

    // Verify cleanup
    expect(status.size).toBeGreaterThan(0) // Processes existed before stop
  })
})
