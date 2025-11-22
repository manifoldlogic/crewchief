/**
 * Unit tests for DaemonClient
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { DaemonClient } from '../client.js'
import {
  DaemonError,
  DaemonTimeoutError,
  DaemonCrashError,
} from '../errors.js'
import type { DaemonConfig } from '../types.js'
import { EventEmitter } from 'node:events'
import type { ChildProcess } from 'node:child_process'
import { Readable, Writable } from 'node:stream'

// Mock child_process
vi.mock('node:child_process', () => ({
  spawn: vi.fn(),
}))

describe('DaemonClient', () => {
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
    Object.defineProperty(mockProcess, 'exitCode', { value: null, writable: true })
    Object.defineProperty(mockProcess, 'killed', { value: false, writable: true })
    mockProcess.kill = vi.fn(() => true)

    spawnMock.mockReturnValue(mockProcess)
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('constructor', () => {
    it('should create client without starting daemon (lazy init)', () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon' }
      const client = new DaemonClient(config)

      expect(client).toBeInstanceOf(DaemonClient)
      expect(spawnMock).not.toHaveBeenCalled()
    })
  })

  describe('search', () => {
    // TODO: Remaining client tests have complex async/mock timing issues
    // Core functionality is tested by passing tests. Integration tests will cover these scenarios.
    it('should start daemon on first search request', async () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon', timeout: 1000 }
      const client = new DaemonClient(config)

      const searchPromise = client.search({ query: 'test', repo: 'crewchief' })

      // Let daemon start
      await new Promise((resolve) => setTimeout(resolve, 100))

      // Simulate daemon response
      const response = JSON.stringify({
        jsonrpc: '2.0',
        result: { hits: [], total: 0 },
        id: 1,
      })
      mockStdout.push(response + '\n')
      mockStdout.push(null) // End stream

      const result = await searchPromise

      expect(spawnMock).toHaveBeenCalled()
      expect(result).toEqual({ hits: [], total: 0 })
    })

    it('should reuse existing daemon for subsequent requests', async () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon', timeout: 1000 }
      const client = new DaemonClient(config)

      // First request
      const search1 = client.search({ query: 'test1', repo: 'crewchief' })
      await new Promise((resolve) => setTimeout(resolve, 100))

      const response1 = JSON.stringify({
        jsonrpc: '2.0',
        result: { hits: [], total: 0 },
        id: 1,
      })
      mockStdout.push(response1 + '\n')

      await search1

      // Clear spawn mock
      spawnMock.mockClear()

      // Second request
      const search2 = client.search({ query: 'test2', repo: 'crewchief' })

      const response2 = JSON.stringify({
        jsonrpc: '2.0',
        result: { hits: [], total: 1 },
        id: 2,
      })
      mockStdout.push(response2 + '\n')

      await search2

      // Should NOT spawn again
      expect(spawnMock).not.toHaveBeenCalled()
    })

    it('should use sequential request IDs', async () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon', timeout: 1000 }
      const client = new DaemonClient(config)

      const writeSpy = vi.spyOn(mockStdin, 'write')

      // Start daemon
      const search1 = client.search({ query: 'test1', repo: 'crewchief' })
      await new Promise((resolve) => setTimeout(resolve, 100))

      // Check first request ID
      expect(writeSpy).toHaveBeenCalledWith(
        expect.stringContaining('"id":1')
      )

      mockStdout.push(JSON.stringify({ jsonrpc: '2.0', result: {}, id: 1 }) + '\n')
      await search1

      // Second request
      writeSpy.mockClear()
      const search2 = client.search({ query: 'test2', repo: 'crewchief' })

      expect(writeSpy).toHaveBeenCalledWith(
        expect.stringContaining('"id":2')
      )

      mockStdout.push(JSON.stringify({ jsonrpc: '2.0', result: {}, id: 2 }) + '\n')
      await search2
    })

    it('should timeout if daemon does not respond', async () => {
      vi.useFakeTimers()

      const config: DaemonConfig = { binaryPath: '/path/to/daemon', timeout: 1000 }
      const client = new DaemonClient(config)

      const searchPromise = client.search({ query: 'test', repo: 'crewchief' })

      // Let daemon start
      await vi.advanceTimersByTimeAsync(100)

      // Advance time past timeout
      await vi.advanceTimersByTimeAsync(1000)

      await expect(searchPromise).rejects.toThrow(DaemonTimeoutError)

      vi.useRealTimers()
    })

    it('should reject request if daemon is shutting down', async () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon' }
      const client = new DaemonClient(config)

      // Start daemon
      const search1 = client.search({ query: 'test1', repo: 'crewchief' })
      await new Promise((resolve) => setTimeout(resolve, 100))

      mockStdout.push(JSON.stringify({ jsonrpc: '2.0', result: {}, id: 1 }) + '\n')
      await search1

      // Start shutdown
      const stopPromise = client.stop()

      // Try to search during shutdown
      const search2 = client.search({ query: 'test2', repo: 'crewchief' })

      await expect(search2).rejects.toThrow(DaemonError)
      await expect(search2).rejects.toThrow('shutting down')

      mockProcess.emit('exit', 0, null)
      await stopPromise
    })
  })

  describe('ping', () => {
    it('should send ping request to daemon', async () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon', timeout: 1000 }
      const client = new DaemonClient(config)

      const pingPromise = client.ping()
      await new Promise((resolve) => setTimeout(resolve, 100))

      const response = JSON.stringify({
        jsonrpc: '2.0',
        result: 'pong',
        id: 1,
      })
      mockStdout.push(response + '\n')

      const result = await pingPromise

      expect(result).toBe('pong')
    })
  })

  describe('stop', () => {
    it('should stop daemon gracefully', async () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon' }
      const client = new DaemonClient(config)

      // Start daemon
      const search = client.search({ query: 'test', repo: 'crewchief' })
      await new Promise((resolve) => setTimeout(resolve, 100))

      mockStdout.push(JSON.stringify({ jsonrpc: '2.0', result: {}, id: 1 }) + '\n')
      await search

      // Stop daemon
      const stopPromise = client.stop()

      // Simulate process exit
      mockProcess.emit('exit', 0, null)

      await stopPromise

      expect(mockProcess.kill).toHaveBeenCalledWith('SIGTERM')
    })

    it('should be idempotent (safe to call multiple times)', async () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon' }
      const client = new DaemonClient(config)

      // Start daemon
      const search = client.search({ query: 'test', repo: 'crewchief' })
      await new Promise((resolve) => setTimeout(resolve, 100))

      mockStdout.push(JSON.stringify({ jsonrpc: '2.0', result: {}, id: 1 }) + '\n')
      await search

      // Stop multiple times
      const stop1 = client.stop()
      mockProcess.emit('exit', 0, null)
      await stop1

      await client.stop() // Should not throw
      await client.stop() // Should not throw
    })

    it('should wait for in-flight requests with timeout', async () => {
      vi.useFakeTimers()

      const config: DaemonConfig = {
        binaryPath: '/path/to/daemon',
        shutdownTimeout: 5000,
        timeout: 30000,
      }
      const client = new DaemonClient(config)

      // Start daemon and make request
      const search = client.search({ query: 'test', repo: 'crewchief' })
      await vi.advanceTimersByTimeAsync(100)

      // Start shutdown while request is pending
      const stopPromise = client.stop()

      // Advance time (request completes before shutdown timeout)
      await vi.advanceTimersByTimeAsync(1000)

      // Complete the request
      mockStdout.push(JSON.stringify({ jsonrpc: '2.0', result: {}, id: 1 }) + '\n')
      await search

      // Finish shutdown
      mockProcess.emit('exit', 0, null)
      await vi.advanceTimersByTimeAsync(100)
      await stopPromise

      vi.useRealTimers()
    })
  })

  describe('isHealthy', () => {
    it('should return true if ping succeeds', async () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon', timeout: 1000 }
      const client = new DaemonClient(config)

      const healthPromise = client.isHealthy()
      await new Promise((resolve) => setTimeout(resolve, 100))

      mockStdout.push(JSON.stringify({ jsonrpc: '2.0', result: 'pong', id: 1 }) + '\n')

      const healthy = await healthPromise

      expect(healthy).toBe(true)
    })

    it('should return false if ping fails', async () => {
      vi.useFakeTimers()

      const config: DaemonConfig = { binaryPath: '/path/to/daemon', timeout: 1000 }
      const client = new DaemonClient(config)

      const healthPromise = client.isHealthy()
      await vi.advanceTimersByTimeAsync(100)

      // Simulate timeout
      await vi.advanceTimersByTimeAsync(1000)

      const healthy = await healthPromise

      expect(healthy).toBe(false)

      vi.useRealTimers()
    })
  })

  describe('restart', () => {
    it('should stop and start daemon', async () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon' }
      const client = new DaemonClient(config)

      // Start daemon
      const search = client.search({ query: 'test', repo: 'crewchief' })
      await new Promise((resolve) => setTimeout(resolve, 100))

      mockStdout.push(JSON.stringify({ jsonrpc: '2.0', result: {}, id: 1 }) + '\n')
      await search

      // Restart
      const restartPromise = client.restart()

      // Simulate old process exit
      mockProcess.emit('exit', 0, null)
      await new Promise((resolve) => setTimeout(resolve, 100))

      await restartPromise

      expect(mockProcess.kill).toHaveBeenCalled()
      expect(spawnMock).toHaveBeenCalledTimes(2) // Original + restart
    })
  })

  describe('crash recovery', () => {
    it('should reject pending requests on daemon crash', async () => {
      const config: DaemonConfig = { binaryPath: '/path/to/daemon', timeout: 30000 }
      const client = new DaemonClient(config)

      // Start daemon
      const search = client.search({ query: 'test', repo: 'crewchief' })
      await new Promise((resolve) => setTimeout(resolve, 100))

      // Simulate crash
      mockProcess.emit('exit', 1, null)

      await expect(search).rejects.toThrow(DaemonCrashError)
    })

    it('should auto-restart daemon after crash', async () => {
      vi.useFakeTimers()

      const config: DaemonConfig = {
        binaryPath: '/path/to/daemon',
        autoRestart: true,
        restartBackoffMs: 1000,
      }
      const client = new DaemonClient(config)

      // Start daemon
      const search = client.search({ query: 'test', repo: 'crewchief' })
      await vi.advanceTimersByTimeAsync(100)

      mockStdout.push(JSON.stringify({ jsonrpc: '2.0', result: {}, id: 1 }) + '\n')
      await search

      // Clear spawn count
      spawnMock.mockClear()

      // Simulate crash
      mockProcess.emit('exit', 1, null)

      // Advance time past backoff delay
      await vi.advanceTimersByTimeAsync(1000)

      // Should have spawned again
      expect(spawnMock).toHaveBeenCalled()

      vi.useRealTimers()
    })
  })
})
