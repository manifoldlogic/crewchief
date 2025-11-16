/**
 * Tests for workspace scan orchestration
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { runInitialScan, ScanError, type ScanConfig } from './scan.js'
import type { StatusBarManager } from '../ui/statusBar.js'
import { EventEmitter } from 'node:events'
import { Readable } from 'node:stream'
import type { ChildProcess } from 'node:child_process'

// Mock fs/promises module
vi.mock('node:fs/promises', () => ({
  access: vi.fn(),
  constants: {
    F_OK: 0,
    R_OK: 4,
    X_OK: 1,
  },
}))

// Mock child_process module
vi.mock('node:child_process', () => ({
  spawn: vi.fn(),
}))

// Mock vscode module
vi.mock('vscode', () => {
  const mockProgress = {
    report: vi.fn(),
  }

  const mockWithProgress = vi.fn()

  return {
    default: {
      window: {
        createOutputChannel: vi.fn(),
        withProgress: mockWithProgress,
        showErrorMessage: vi.fn(),
      },
      ProgressLocation: {
        Notification: 15,
      },
    },
    window: {
      createOutputChannel: vi.fn(),
      withProgress: mockWithProgress,
      showErrorMessage: vi.fn(),
    },
    ProgressLocation: {
      Notification: 15,
    },
  }
})

// Mock OutputChannel
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

  show(): void {}
  hide(): void {}
  dispose(): void {
    this.lines = []
  }

  getLines(): string[] {
    return this.lines
  }

  hasLine(pattern: string | RegExp): boolean {
    return this.lines.some(line => {
      if (typeof pattern === 'string') {
        return line.includes(pattern)
      }
      return pattern.test(line)
    })
  }

  name = 'Maproom'
  replace = () => {}
}

// Mock StatusBarManager
class MockStatusBarManager {
  private state: string = 'idle'
  private message?: string

  setState(state: string, message?: string): void {
    this.state = state
    this.message = message
  }

  getState(): string {
    return this.state
  }

  getMessage(): string | undefined {
    return this.message
  }
}

// Mock ChildProcess
class MockChildProcess extends EventEmitter implements Partial<ChildProcess> {
  pid = 12345
  exitCode: number | null = null
  killed = false
  stdout: Readable
  stderr: Readable

  constructor() {
    super()
    this.stdout = new Readable({ read() {} })
    this.stderr = new Readable({ read() {} })
  }

  kill(signal?: NodeJS.Signals | number): boolean {
    this.killed = true
    setTimeout(() => {
      this.exitCode = 0
      this.emit('exit', 0, signal)
    }, 10)
    return true
  }

  simulateProgressEvents(totalFiles: number): void {
    // Emit progress events
    for (let i = 0; i <= totalFiles; i += Math.ceil(totalFiles / 10)) {
      const progress = Math.min(i, totalFiles)
      this.stdout.push(`{"type":"progress","files":${totalFiles},"complete":${progress}}\n`)
    }
    // Emit complete event
    this.stdout.push(`{"type":"complete","files":${totalFiles},"duration":5000}\n`)
    this.stdout.push(null)

    // Exit successfully
    setTimeout(() => {
      this.exitCode = 0
      this.emit('exit', 0, null)
    }, 50)
  }

  simulateError(code: number, errorMessage: string): void {
    this.stderr.push(`error: ${errorMessage}\n`)
    this.stderr.push(null)

    setTimeout(() => {
      this.exitCode = code
      this.emit('exit', code, null)
    }, 50)
  }

  simulateSpawnError(error: Error): void {
    setTimeout(() => {
      this.emit('error', error)
    }, 10)
  }
}

describe('runInitialScan', () => {
  let mockChild: MockChildProcess
  let outputChannel: MockOutputChannel
  let statusBarManager: MockStatusBarManager
  let config: ScanConfig
  let mockProgress: any
  let mockWithProgress: any

  beforeEach(async () => {
    // Reset mocks
    vi.clearAllMocks()

    // Create mock instances
    mockChild = new MockChildProcess()
    outputChannel = new MockOutputChannel()
    statusBarManager = new MockStatusBarManager()

    // Create fresh progress mock
    mockProgress = {
      report: vi.fn(),
    }

    config = {
      extensionRoot: '/test/extension',
      workspaceRoot: '/test/workspace',
      databaseUrl: 'postgresql://test:test@localhost:5432/test',
      outputChannel: outputChannel as any,
      statusBarManager: statusBarManager as any,
      env: { TEST_ENV: 'test' },
    }

    // Mock fs.access to succeed
    const { access } = await import('node:fs/promises')
    vi.mocked(access).mockResolvedValue(undefined)

    // Mock spawn to return our mock child process
    const { spawn } = await import('node:child_process')
    vi.mocked(spawn).mockReturnValue(mockChild as any)

    // Get the mocked vscode module and set up withProgress
    const vscode = await import('vscode')
    mockWithProgress = vscode.window.withProgress
    vi.mocked(mockWithProgress).mockImplementation(async (options, callback) => {
      return callback(mockProgress)
    })
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('successful scan', () => {
    it('should complete scan and return file count', async () => {
      // Simulate successful scan with progress events
      setTimeout(() => {
        mockChild.simulateProgressEvents(100)
      }, 10)

      const result = await runInitialScan(config)

      expect(result).toBe(100)
      expect(outputChannel.hasLine(/Starting initial scan/)).toBe(true)
      expect(outputChannel.hasLine(/Scan completed successfully/)).toBe(true)
    })

    it('should update progress notification with file counts', async () => {
      setTimeout(() => {
        mockChild.simulateProgressEvents(50)
      }, 10)

      await runInitialScan(config)

      // Verify progress.report was called
      expect(mockProgress.report).toHaveBeenCalled()

      // Check that at least one report included a message with file count
      const calls = mockProgress.report.mock.calls
      const hasFileCountMessage = calls.some(call =>
        call[0]?.message?.includes('files')
      )
      expect(hasFileCountMessage).toBe(true)
    })

    it('should update status bar on completion', async () => {
      setTimeout(() => {
        mockChild.simulateProgressEvents(250)
      }, 10)

      await runInitialScan(config)

      expect(statusBarManager.getState()).toBe('watching')
      expect(statusBarManager.getMessage()).toContain('Indexed')
      expect(statusBarManager.getMessage()).toContain('250')
    })

    it('should log progress events to output channel', async () => {
      setTimeout(() => {
        mockChild.simulateProgressEvents(100)
      }, 10)

      await runInitialScan(config)

      expect(outputChannel.hasLine(/Progress:/)).toBe(true)
      expect(outputChannel.hasLine(/Scan complete:/)).toBe(true)
    })

    it('should pass environment variables to child process', async () => {
      const { spawn } = await import('node:child_process')

      setTimeout(() => {
        mockChild.simulateProgressEvents(10)
      }, 10)

      await runInitialScan(config)

      // Verify spawn was called with correct env vars
      expect(spawn).toHaveBeenCalledWith(
        expect.any(String),
        expect.arrayContaining(['scan', '--path', config.workspaceRoot]),
        expect.objectContaining({
          env: expect.objectContaining({
            DATABASE_URL: config.databaseUrl,
            TEST_ENV: 'test',
          }),
        })
      )
    })
  })

  describe('error handling', () => {
    it('should throw ScanError on non-zero exit code', async () => {
      setTimeout(() => {
        mockChild.simulateError(1, 'Database connection failed')
      }, 10)

      await expect(runInitialScan(config)).rejects.toThrow(ScanError)
    })

    it('should include stderr in error', async () => {
      setTimeout(() => {
        mockChild.simulateError(2, 'Permission denied')
      }, 10)

      try {
        await runInitialScan(config)
        expect.fail('Should have thrown')
      } catch (error) {
        expect(error).toBeInstanceOf(ScanError)
        if (error instanceof ScanError) {
          expect(error.stderr).toContain('Permission denied')
          expect(error.exitCode).toBe(2)
        }
      }
    })

    it('should show user-friendly error notification', async () => {
      const vscode = await import('vscode')

      setTimeout(() => {
        mockChild.simulateError(1, 'Failed to connect to database')
      }, 10)

      await expect(runInitialScan(config)).rejects.toThrow()

      // Verify error notification was shown
      expect(vscode.window.showErrorMessage).toHaveBeenCalled()
    })

    it('should handle spawn errors', async () => {
      const spawnError = new Error('ENOENT: command not found') as NodeJS.ErrnoException
      spawnError.code = 'ENOENT'

      setTimeout(() => {
        mockChild.simulateSpawnError(spawnError)
      }, 10)

      await expect(runInitialScan(config)).rejects.toThrow(ScanError)
    })

    it('should log stderr to output channel', async () => {
      setTimeout(() => {
        mockChild.stderr.push('Warning: slow connection\n')
        mockChild.stderr.push('Error: timeout\n')
        mockChild.simulateError(1, 'Fatal error')
      }, 10)

      await expect(runInitialScan(config)).rejects.toThrow()

      expect(outputChannel.hasLine(/STDERR.*Warning/)).toBe(true)
      expect(outputChannel.hasLine(/STDERR.*Error: timeout/)).toBe(true)
    })
  })

  describe('binary verification', () => {
    it('should throw ScanError if binary not found', async () => {
      const { access } = await import('node:fs/promises')
      const enoentError = new Error('ENOENT') as NodeJS.ErrnoException
      enoentError.code = 'ENOENT'
      vi.mocked(access).mockRejectedValue(enoentError)

      await expect(runInitialScan(config)).rejects.toThrow(ScanError)
      await expect(runInitialScan(config)).rejects.toThrow(/Binary not found/)
    })

    it('should throw ScanError if binary not executable', async () => {
      const { access } = await import('node:fs/promises')
      const eaccesError = new Error('EACCES') as NodeJS.ErrnoException
      eaccesError.code = 'EACCES'
      vi.mocked(access).mockRejectedValue(eaccesError)

      await expect(runInitialScan(config)).rejects.toThrow(ScanError)
      await expect(runInitialScan(config)).rejects.toThrow(/not executable/)
    })

    it('should verify binary exists before spawning', async () => {
      const { access } = await import('node:fs/promises')

      setTimeout(() => {
        mockChild.simulateProgressEvents(10)
      }, 10)

      await runInitialScan(config)

      // Verify access was called to check binary
      expect(access).toHaveBeenCalled()
    })
  })

  describe('progress tracking', () => {
    it('should track progress percentage correctly', async () => {
      setTimeout(() => {
        // Emit progress events: 0%, 25%, 50%, 75%, 100%
        mockChild.stdout.push('{"type":"progress","files":100,"complete":0}\n')
        mockChild.stdout.push('{"type":"progress","files":100,"complete":25}\n')
        mockChild.stdout.push('{"type":"progress","files":100,"complete":50}\n')
        mockChild.stdout.push('{"type":"progress","files":100,"complete":75}\n')
        mockChild.stdout.push('{"type":"progress","files":100,"complete":100}\n')
        mockChild.stdout.push('{"type":"complete","files":100,"duration":3000}\n')
        mockChild.stdout.push(null)

        setTimeout(() => {
          mockChild.exitCode = 0
          mockChild.emit('exit', 0, null)
        }, 50)
      }, 10)

      await runInitialScan(config)

      // Verify progress.report was called with incremental updates
      const calls = mockProgress.report.mock.calls
      expect(calls.length).toBeGreaterThan(0)

      // Check that increments were provided
      const hasIncrements = calls.some(call => call[0]?.increment !== undefined)
      expect(hasIncrements).toBe(true)
    })

    it('should format file counts with locale formatting', async () => {
      setTimeout(() => {
        mockChild.stdout.push('{"type":"progress","files":1500,"complete":750}\n')
        mockChild.stdout.push('{"type":"complete","files":1500,"duration":2000}\n')
        mockChild.stdout.push(null)

        setTimeout(() => {
          mockChild.exitCode = 0
          mockChild.emit('exit', 0, null)
        }, 50)
      }, 10)

      await runInitialScan(config)

      expect(statusBarManager.getMessage()).toContain('1,500')
    })
  })

  describe('parse errors', () => {
    it('should handle malformed JSON gracefully', async () => {
      setTimeout(() => {
        mockChild.stdout.push('{"type":"progress","files":10,"complete":5}\n')
        mockChild.stdout.push('{invalid json}\n')
        mockChild.stdout.push('{"type":"complete","files":10,"duration":1000}\n')
        mockChild.stdout.push(null)

        setTimeout(() => {
          mockChild.exitCode = 0
          mockChild.emit('exit', 0, null)
        }, 50)
      }, 10)

      // Should complete successfully despite parse error
      const result = await runInitialScan(config)
      expect(result).toBe(10)

      // Parse error should be logged
      expect(outputChannel.hasLine(/Parse error/)).toBe(true)
    })

    it('should log invalid event schemas', async () => {
      setTimeout(() => {
        mockChild.stdout.push('{"type":"progress","files":10}\n') // Missing complete field
        mockChild.stdout.push('{"type":"complete","files":10,"duration":1000}\n')
        mockChild.stdout.push(null)

        setTimeout(() => {
          mockChild.exitCode = 0
          mockChild.emit('exit', 0, null)
        }, 50)
      }, 10)

      await runInitialScan(config)

      expect(outputChannel.hasLine(/Parse error/)).toBe(true)
    })
  })

  describe('event handling', () => {
    it('should handle status events', async () => {
      setTimeout(() => {
        mockChild.stdout.push('{"type":"status","state":"indexing"}\n')
        mockChild.stdout.push('{"type":"progress","files":20,"complete":10}\n')
        mockChild.stdout.push('{"type":"complete","files":20,"duration":1500}\n')
        mockChild.stdout.push(null)

        setTimeout(() => {
          mockChild.exitCode = 0
          mockChild.emit('exit', 0, null)
        }, 50)
      }, 10)

      await runInitialScan(config)

      expect(outputChannel.hasLine(/Status: indexing/)).toBe(true)
    })

    it('should handle error events during scan', async () => {
      setTimeout(() => {
        mockChild.stdout.push('{"type":"progress","files":50,"complete":10}\n')
        mockChild.stdout.push('{"type":"error","message":"Failed to parse file","file":"test.ts"}\n')
        mockChild.stdout.push('{"type":"progress","files":50,"complete":11}\n')
        mockChild.stdout.push('{"type":"complete","files":50,"duration":2000}\n')
        mockChild.stdout.push(null)

        setTimeout(() => {
          mockChild.exitCode = 0
          mockChild.emit('exit', 0, null)
        }, 50)
      }, 10)

      await runInitialScan(config)

      // Error event should be logged but not stop scan
      expect(outputChannel.hasLine(/ERROR during scan.*Failed to parse file/)).toBe(true)
    })
  })

  describe('withProgress integration', () => {
    it('should use Notification location for progress', async () => {
      const vscode = await import('vscode')

      setTimeout(() => {
        mockChild.simulateProgressEvents(10)
      }, 10)

      await runInitialScan(config)

      expect(mockWithProgress).toHaveBeenCalledWith(
        expect.objectContaining({
          location: vscode.ProgressLocation.Notification,
          title: 'Indexing workspace',
          cancellable: false,
        }),
        expect.any(Function)
      )
    })
  })
})
