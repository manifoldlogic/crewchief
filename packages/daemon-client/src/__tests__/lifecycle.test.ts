/**
 * Unit tests for daemon process lifecycle management
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { DaemonLifecycle } from '../lifecycle.js'
import { DaemonStartError, DaemonCrashError } from '../errors.js'
import type { DaemonConfig } from '../types.js'
import { EventEmitter } from 'node:events'
import type { ChildProcess } from 'node:child_process'
import { Readable, Writable } from 'node:stream'

// Mock child_process
vi.mock('node:child_process', () => ({
  spawn: vi.fn(),
}))

describe('DaemonLifecycle', () => {
  let mockProcess: Partial<ChildProcess> & EventEmitter
  let mockStdin: Writable
  let mockStdout: Readable
  let mockStderr: Readable
  let spawnMock: ReturnType<typeof vi.fn>

  beforeEach(async () => {
    const { spawn } = await import('node:child_process')
    spawnMock = spawn as ReturnType<typeof vi.fn>

    // Create mock streams
    mockStdin = new Writable({
      write() { return true },
    })
    mockStdout = new Readable({
      read() {},
    })
    mockStderr = new Readable({
      read() {},
    })

    // Create mock process
    mockProcess = new EventEmitter() as Partial<ChildProcess> & EventEmitter
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    mockProcess.stdin = mockStdin as any
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    mockProcess.stdout = mockStdout as any
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    mockProcess.stderr = mockStderr as any
    Object.defineProperty(mockProcess, 'exitCode', { value: null, writable: true, configurable: true })
    Object.defineProperty(mockProcess, 'killed', { value: false, writable: true, configurable: true })
    mockProcess.kill = vi.fn(() => true)

    spawnMock.mockReturnValue(mockProcess)
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('start', () => {
    it('should spawn daemon with correct binary path and args', async () => {
      const config: DaemonConfig = {
        binaryPath: '/path/to/daemon',
      }
      const lifecycle = new DaemonLifecycle(config)

      // Start daemon (will wait for stabilization)
      const startPromise = lifecycle.start()

      // Let stabilization complete
      await new Promise((resolve) => setTimeout(resolve, 100))

      const daemon = await startPromise

      expect(spawnMock).toHaveBeenCalledWith(
        '/path/to/daemon',
        ['serve'],
        expect.objectContaining({
          stdio: ['pipe', 'pipe', 'pipe'],
          windowsHide: true,
        })
      )
      expect(daemon.process).toBe(mockProcess)
    })

    it('should pass environment variables to daemon', async () => {
      const config: DaemonConfig = {
        binaryPath: '/path/to/daemon',
        env: {
          MAPROOM_DATABASE_URL: 'postgresql://...',
          OPENAI_API_KEY: 'sk-...',
        },
      }
      const lifecycle = new DaemonLifecycle(config)

      const startPromise = lifecycle.start()
      await new Promise((resolve) => setTimeout(resolve, 100))
      await startPromise

      expect(spawnMock).toHaveBeenCalledWith(
        '/path/to/daemon',
        ['serve'],
        expect.objectContaining({
          env: expect.objectContaining({
            MAPROOM_DATABASE_URL: 'postgresql://...',
            OPENAI_API_KEY: 'sk-...',
          }),
        })
      )
    })

    it('should throw DaemonStartError if streams not available', async () => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      mockProcess.stdin = null as any

      const config: DaemonConfig = { binaryPath: '/path/to/daemon' }
      const lifecycle = new DaemonLifecycle(config)

      await expect(lifecycle.start()).rejects.toThrow(DaemonStartError)
    })

    it('should throw DaemonCrashError if daemon exits immediately', async () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon' }
      const lifecycle = new DaemonLifecycle(config)

      const startPromise = lifecycle.start()

      // Simulate immediate crash
      setTimeout(() => {
        mockProcess.emit('exit', 1, null)
      }, 50)

      await expect(startPromise).rejects.toThrow(DaemonCrashError)
    })

    it('should throw DaemonStartError on process error', async () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon' }
      const lifecycle = new DaemonLifecycle(config)

      const startPromise = lifecycle.start()

      // Simulate error
      setTimeout(() => {
        const error = new Error('ENOENT')
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        ;(error as any).code = 'ENOENT'
        mockProcess.emit('error', error)
      }, 50)

      await expect(startPromise).rejects.toThrow(DaemonStartError)
    })
  })

  describe('stop', () => {
    it('should send SIGTERM for graceful shutdown', async () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon' }
      const lifecycle = new DaemonLifecycle(config)

      const startPromise = lifecycle.start()
      await new Promise((resolve) => setTimeout(resolve, 100))
      const daemon = await startPromise

      const stopPromise = lifecycle.stop(daemon)

      // Simulate process exit
      setTimeout(() => {
        mockProcess.emit('exit', 0, null)
      }, 50)

      await stopPromise

      expect(mockProcess.kill).toHaveBeenCalledWith('SIGTERM')
    })

    // Removed 3 empty stubs (SIGKILL timeout, already-exited, cleanup) — TESTCI.2004
    // Basic stop() covered by "should send SIGTERM for graceful shutdown" above.
    // Advanced stop() scenarios covered by integration tests.
  })

  describe('shouldRestart', () => {
    it('should return true when restart attempts below max', () => {
      const config: DaemonConfig = {
        binaryPath: '/path/to/daemon',
        maxRestartAttempts: 5,
        autoRestart: true,
      }
      const lifecycle = new DaemonLifecycle(config)

      expect(lifecycle.shouldRestart()).toBe(true)
    })

    it('should return false when autoRestart disabled', () => {
      const config: DaemonConfig = {
        binaryPath: '/path/to/daemon',
        autoRestart: false,
      }
      const lifecycle = new DaemonLifecycle(config)

      expect(lifecycle.shouldRestart()).toBe(false)
    })

    it('should return false after max restart attempts (circuit breaker)', () => {
      const config: DaemonConfig = {
        binaryPath: '/path/to/daemon',
        maxRestartAttempts: 3,
        autoRestart: true,
      }
      const lifecycle = new DaemonLifecycle(config)

      // Consume restart attempts
      lifecycle.getBackoffDelay() // attempt 1
      lifecycle.getBackoffDelay() // attempt 2
      lifecycle.getBackoffDelay() // attempt 3

      // Should now be at max
      expect(lifecycle.shouldRestart()).toBe(false)
    })

    it('should reset attempt counter after success window', () => {
      vi.useFakeTimers()

      const config: DaemonConfig = {
        binaryPath: '/path/to/daemon',
        maxRestartAttempts: 5,
        autoRestart: true,
      }
      const lifecycle = new DaemonLifecycle(config)

      // Use up some attempts
      lifecycle.getBackoffDelay()
      lifecycle.getBackoffDelay()

      // Advance time past reset window (60s)
      vi.advanceTimersByTime(61000)

      // Should reset and return true
      expect(lifecycle.shouldRestart()).toBe(true)

      vi.useRealTimers()
    })
  })

  describe('getBackoffDelay', () => {
    it('should calculate exponential backoff (1s, 2s, 4s, 8s, 16s)', () => {
      const config: DaemonConfig = {
        binaryPath: '/path/to/daemon',
        restartBackoffMs: 1000,
      }
      const lifecycle = new DaemonLifecycle(config)

      expect(lifecycle.getBackoffDelay()).toBe(1000) // 2^0 = 1
      expect(lifecycle.getBackoffDelay()).toBe(2000) // 2^1 = 2
      expect(lifecycle.getBackoffDelay()).toBe(4000) // 2^2 = 4
      expect(lifecycle.getBackoffDelay()).toBe(8000) // 2^3 = 8
      expect(lifecycle.getBackoffDelay()).toBe(16000) // 2^4 = 16
    })

    it('should use custom base delay', () => {
      const config: DaemonConfig = {
        binaryPath: '/path/to/daemon',
        restartBackoffMs: 500,
      }
      const lifecycle = new DaemonLifecycle(config)

      expect(lifecycle.getBackoffDelay()).toBe(500)
      expect(lifecycle.getBackoffDelay()).toBe(1000)
      expect(lifecycle.getBackoffDelay()).toBe(2000)
    })

    it('should increment restart attempts', () => {
      const config: DaemonConfig = {
        binaryPath: '/path/to/daemon',
        maxRestartAttempts: 3,
      }
      const lifecycle = new DaemonLifecycle(config)

      lifecycle.getBackoffDelay()
      lifecycle.getBackoffDelay()
      lifecycle.getBackoffDelay()

      // After 3 attempts, should hit circuit breaker
      expect(lifecycle.shouldRestart()).toBe(false)
    })
  })

  describe('resetRestartAttempts', () => {
    it('should reset attempt counter to zero', () => {
      const config: DaemonConfig = {
        binaryPath: '/path/to/daemon',
        maxRestartAttempts: 3,
      }
      const lifecycle = new DaemonLifecycle(config)

      lifecycle.getBackoffDelay()
      lifecycle.getBackoffDelay()
      lifecycle.getBackoffDelay()

      expect(lifecycle.shouldRestart()).toBe(false)

      lifecycle.resetRestartAttempts()

      expect(lifecycle.shouldRestart()).toBe(true)
    })
  })
})
