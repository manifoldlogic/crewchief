/**
 * Tests for ProcessOrchestrator
 *
 * Note: These tests use mocking to avoid requiring actual binaries.
 * Integration tests with real binaries should be performed separately.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { ProcessOrchestrator, ProcessError, type OrchestratorConfig } from './orchestrator'
import type { OutputChannel } from 'vscode'
import * as platform from '../utils/platform'
import { EventEmitter } from 'node:events'
import { Readable } from 'node:stream'
import type { ChildProcess } from 'node:child_process'

// Mock vscode module
vi.mock('vscode', () => ({
  window: {
    showErrorMessage: vi.fn(),
  },
  commands: {
    executeCommand: vi.fn(),
  },
}))

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
}))

// Mock OutputChannel for testing
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

// Mock ChildProcess
class MockChildProcess extends EventEmitter implements Partial<ChildProcess> {
  pid = 12345
  exitCode: number | null = null
  killed = false
  stdout: Readable
  stderr: Readable

  constructor() {
    super()
    // Create proper Readable streams for stdout/stderr
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

  simulateSuccess(): void {
    this.stdout.push('{"type":"status","state":"idle"}\n')
  }

  simulateCrash(code: number = 1): void {
    this.exitCode = code
    this.stderr.push('Fatal error\n')
    this.stderr.push(null)
    this.emit('exit', code, null)
  }
}

describe('ProcessOrchestrator', () => {
  let outputChannel: MockOutputChannel
  let config: OrchestratorConfig

  beforeEach(async () => {
    outputChannel = new MockOutputChannel()
    config = {
      extensionRoot: '/test/extension',
      workspaceRoot: '/test/workspace',
      databaseUrl: 'sqlite://~/.maproom/maproom.db',
    }

    vi.spyOn(platform, 'detectPlatform').mockReturnValue('linux-x64')
    vi.spyOn(platform, 'getBinaryExtension').mockReturnValue('')
    vi.spyOn(platform, 'isWindows').mockReturnValue(false)

    const fs = await import('node:fs/promises')
    const childProcess = await import('node:child_process')

    vi.mocked(fs.access).mockResolvedValue(undefined)
    vi.mocked(childProcess.spawn).mockClear()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('constructor', () => {
    it('should initialize successfully with valid config', () => {
      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)

      expect(orchestrator).toBeDefined()
      expect(outputChannel.hasLine('Process orchestrator initialized')).toBe(true)
      expect(outputChannel.hasLine('Platform: linux-x64')).toBe(true)
      expect(outputChannel.hasLine('/test/extension/bin/linux-x64/crewchief-maproom')).toBe(true)
    })

    it('should log workspace root', () => {
      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      expect(outputChannel.hasLine('Workspace root: /test/workspace')).toBe(true)
    })

    it('should throw ProcessError if platform detection fails', () => {
      vi.spyOn(platform, 'detectPlatform').mockImplementation(() => {
        throw new Error('Unsupported platform')
      })

      expect(() => new ProcessOrchestrator(outputChannel as any, config)).toThrow(ProcessError)
    })

    it('should handle Windows platform correctly', () => {
      vi.spyOn(platform, 'detectPlatform').mockReturnValue('win32-x64')
      vi.spyOn(platform, 'getBinaryExtension').mockReturnValue('.exe')

      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      expect(outputChannel.hasLine('crewchief-maproom.exe')).toBe(true)
    })
  })

  describe('startWatching', () => {
    it('should verify binary exists before spawning', async () => {
      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      const { spawn } = await import('node:child_process')
      const { access } = await import('node:fs/promises')

      const mockChild = new MockChildProcess()

      vi.mocked(spawn).mockImplementation((() => {
        setTimeout(() => mockChild.simulateSuccess(), 10)
        return mockChild as any
      }) as any)

      await orchestrator.startWatching()
      expect(access).toHaveBeenCalled()
    })

    it('should spawn single watch process with correct arguments', async () => {
      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      const { spawn } = await import('node:child_process')

      const mockChild = new MockChildProcess()

      vi.mocked(spawn).mockImplementation((() => {
        setTimeout(() => mockChild.simulateSuccess(), 10)
        return mockChild as any
      }) as any)

      await orchestrator.startWatching()

      const calls = vi.mocked(spawn).mock.calls
      expect(calls).toHaveLength(1) // Only one process spawned
      expect(calls[0][1]).toEqual([
        'watch',
        '--repo',
        'test-owner/test-repo',
        '--path',
        '/test/workspace',
        '--throttle',
        '3s',
      ])
    })

    it('should pass SQLite database URL in environment variables', async () => {
      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      const { spawn } = await import('node:child_process')

      const mockChild = new MockChildProcess()

      vi.mocked(spawn).mockImplementation(((_cmd: string, _args: string[], options: any) => {
        expect(options.env.DATABASE_URL).toBe('sqlite://~/.maproom/maproom.db')
        expect(options.env.MAPROOM_DATABASE_URL).toBe('sqlite://~/.maproom/maproom.db')

        setTimeout(() => mockChild.simulateSuccess(), 10)
        return mockChild as any
      }) as any)

      await orchestrator.startWatching()
    })

    it('should log stdout from process', async () => {
      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      const { spawn } = await import('node:child_process')

      const mockChild = new MockChildProcess()

      vi.mocked(spawn).mockImplementation((() => {
        setTimeout(() => {
          // Push valid NDJSON event
          mockChild.stdout.push('{"type":"progress","files":100,"complete":50}\n')
          mockChild.simulateSuccess()
        }, 10)
        return mockChild as any
      }) as any)

      await orchestrator.startWatching()
      await new Promise(resolve => setTimeout(resolve, 50))

      // Check for logged event (parser logs the event as JSON)
      expect(outputChannel.hasLine('[watch] Event:')).toBe(true)
    })

    it('should throw ProcessError if binary not found', async () => {
      const { access } = await import('node:fs/promises')
      vi.mocked(access).mockRejectedValue(Object.assign(new Error('File not found'), { code: 'ENOENT' }))

      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      await expect(orchestrator.startWatching()).rejects.toThrow(/Binary not found/)
    })

    it('should throw ProcessError if binary not executable', async () => {
      const { access } = await import('node:fs/promises')
      vi.mocked(access).mockRejectedValue(Object.assign(new Error('Permission denied'), { code: 'EACCES' }))

      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      await expect(orchestrator.startWatching()).rejects.toThrow(/Binary not executable/)
    })

    it('should throw ProcessError if process crashes immediately', async () => {
      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      const { spawn } = await import('node:child_process')

      const mockChild = new MockChildProcess()

      vi.mocked(spawn).mockImplementation((() => {
        setTimeout(() => mockChild.simulateCrash(1), 5)
        return mockChild as any
      }) as any)

      await expect(orchestrator.startWatching()).rejects.toThrow(/crashed immediately/)
    })
  })

  describe('stopWatching', () => {
    it('should stop the running process', async () => {
      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      const { spawn } = await import('node:child_process')

      const mockChild = new MockChildProcess()

      vi.mocked(spawn).mockImplementation((() => {
        setTimeout(() => mockChild.simulateSuccess(), 10)
        return mockChild as any
      }) as any)

      await orchestrator.startWatching()
      await orchestrator.stopWatching()

      expect(mockChild.killed).toBe(true)
    })

    it('should be idempotent', async () => {
      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      const { spawn } = await import('node:child_process')

      const mockChild = new MockChildProcess()

      vi.mocked(spawn).mockImplementation((() => {
        setTimeout(() => mockChild.simulateSuccess(), 10)
        return mockChild as any
      }) as any)

      await orchestrator.startWatching()
      await orchestrator.stopWatching()
      await expect(orchestrator.stopWatching()).resolves.toBeUndefined()
    })
  })

  describe('isRunning', () => {
    it('should return false initially', () => {
      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      expect(orchestrator.isRunning()).toBe(false)
    })

    it('should return true after starting process', async () => {
      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      const { spawn } = await import('node:child_process')

      const mockChild = new MockChildProcess()

      vi.mocked(spawn).mockImplementation((() => {
        setTimeout(() => mockChild.simulateSuccess(), 10)
        return mockChild as any
      }) as any)

      await orchestrator.startWatching()
      expect(orchestrator.isRunning()).toBe(true)
    })

    it('should return false after stopping process', async () => {
      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      const { spawn } = await import('node:child_process')

      const mockChild = new MockChildProcess()

      vi.mocked(spawn).mockImplementation((() => {
        setTimeout(() => mockChild.simulateSuccess(), 10)
        return mockChild as any
      }) as any)

      await orchestrator.startWatching()
      await orchestrator.stopWatching()
      expect(orchestrator.isRunning()).toBe(false)
    })
  })

  describe('getStatus', () => {
    it('should return status of watch process', async () => {
      const orchestrator = new ProcessOrchestrator(outputChannel as any, config)
      const { spawn } = await import('node:child_process')

      const mockChild = new MockChildProcess()

      vi.mocked(spawn).mockImplementation((() => {
        setTimeout(() => mockChild.simulateSuccess(), 10)
        return mockChild as any
      }) as any)

      await orchestrator.startWatching()

      const status = orchestrator.getStatus()
      expect(status.size).toBe(1) // Only one process
      expect(status.get('watch')).toEqual({
        running: true,
        crashed: false,
        exitCode: undefined,
      })
    })
  })
})

describe('ProcessError', () => {
  it('should create error with code and message', () => {
    const error = new ProcessError('Test error', 'TEST_CODE')
    expect(error.message).toBe('Test error')
    expect(error.code).toBe('TEST_CODE')
    expect(error.name).toBe('ProcessError')
  })

  it('should include process name when provided', () => {
    const error = new ProcessError('Test error', 'TEST_CODE', 'watch')
    expect(error.processName).toBe('watch')
  })

  it('should include exit code and stderr when provided', () => {
    const error = new ProcessError('Test error', 'TEST_CODE', 'watch', 1, 'stderr output')
    expect(error.exitCode).toBe(1)
    expect(error.stderr).toBe('stderr output')
  })

  it('should be instanceof Error', () => {
    const error = new ProcessError('Test error', 'TEST_CODE')
    expect(error).toBeInstanceOf(Error)
  })
})
