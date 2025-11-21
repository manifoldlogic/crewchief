/**
 * Integration tests for workspace path detection in setup flow
 *
 * These tests verify:
 * - resolveWorkspacePath() integrates correctly with runSetup()
 * - WORKSPACE_HOST_PATH is set before docker compose spawns
 * - Environment variables propagate correctly to spawned processes
 *
 * TDD Approach: These tests are written BEFORE implementation
 * Expected result: All tests should FAIL with "function not defined" errors
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import * as fs from 'fs'
import { execFileSync, spawn } from 'child_process'

// Mock modules
vi.mock('fs', () => ({
  existsSync: vi.fn(),
  readFileSync: vi.fn(),
  mkdirSync: vi.fn(),
  copyFileSync: vi.fn()
}))

vi.mock('child_process', () => ({
  execFileSync: vi.fn(),
  spawn: vi.fn(() => ({
    on: vi.fn(),
    stdout: { on: vi.fn() },
    stderr: { on: vi.fn() }
  }))
}))

// Import functions from bin/cli.cjs
// runSetup() exists but needs modification
// resolveWorkspacePath() doesn't exist yet
const { runSetup } = require('../../bin/cli.cjs')

describe('runSetup() Integration', () => {
  // Store original environment
  const originalEnv = { ...process.env }

  beforeEach(() => {
    // Clear all mocks before each test
    vi.clearAllMocks()

    // Reset environment variables
    process.env = { ...originalEnv }
    delete process.env.WORKSPACE_HOST_PATH
  })

  afterEach(() => {
    // Restore original environment
    process.env = originalEnv
  })

  it('should set WORKSPACE_HOST_PATH environment variable before spawning docker compose', async () => {
    // GIVEN: running in devcontainer environment
    const mockedExistsSync = vi.mocked(fs.existsSync)
    const mockedExecFileSync = vi.mocked(execFileSync)
    const mockedSpawn = vi.mocked(spawn)

    // Setup Docker detection
    mockedExistsSync.mockImplementation((path) => {
      if (path === '/.dockerenv') return true
      return false
    })

    // Setup docker inspect
    mockedExecFileSync
      .mockReturnValueOnce(Buffer.from('container-abc123')) // hostname
      .mockReturnValueOnce(Buffer.from('/host_mnt/Users/user/project')) // docker inspect

    // Mock spawn to capture environment
    let spawnEnv: any = null
    mockedSpawn.mockImplementation((command, args, options: any) => {
      if (command === 'docker' && args?.[0] === 'compose') {
        spawnEnv = options?.env
      }
      return {
        on: vi.fn((event, callback) => {
          if (event === 'close') {
            // Simulate successful docker compose up
            setTimeout(() => callback(0), 10)
          }
        }),
        stdout: { on: vi.fn() },
        stderr: { on: vi.fn() }
      } as any
    })

    // WHEN: running setup
    await runSetup()

    // THEN: WORKSPACE_HOST_PATH should be set in process.env
    expect(process.env.WORKSPACE_HOST_PATH).toBe('/host_mnt/Users/user/project')

    // AND: spawn should be called with environment containing WORKSPACE_HOST_PATH
    expect(mockedSpawn).toHaveBeenCalled()
    expect(spawnEnv).toBeDefined()
    expect(spawnEnv?.WORKSPACE_HOST_PATH).toBe('/host_mnt/Users/user/project')
  })

  it('should spawn docker compose with resolved workspace path', async () => {
    // GIVEN: running on host (not in Docker)
    const mockedExistsSync = vi.mocked(fs.existsSync)
    const mockedReadFileSync = vi.mocked(fs.readFileSync)
    const mockedSpawn = vi.mocked(spawn)

    // Setup: not in Docker
    mockedExistsSync.mockReturnValue(false)
    mockedReadFileSync.mockReturnValue('12:memory:/')

    const expectedPath = process.cwd()

    // Mock spawn to capture call
    let dockerComposeSpawnCalled = false
    mockedSpawn.mockImplementation((command, args) => {
      if (command === 'docker' && args?.[0] === 'compose') {
        dockerComposeSpawnCalled = true
      }
      return {
        on: vi.fn((event, callback) => {
          if (event === 'close') {
            setTimeout(() => callback(0), 10)
          }
        }),
        stdout: { on: vi.fn() },
        stderr: { on: vi.fn() }
      } as any
    })

    // WHEN: running setup
    await runSetup()

    // THEN: docker compose should be spawned
    expect(dockerComposeSpawnCalled).toBe(true)

    // AND: WORKSPACE_HOST_PATH should be set to current directory
    expect(process.env.WORKSPACE_HOST_PATH).toBe(expectedPath)
  })

  it('should handle failures gracefully without crashing', async () => {
    // GIVEN: detection fails but setup should continue
    const mockedExistsSync = vi.mocked(fs.existsSync)
    const mockedExecFileSync = vi.mocked(execFileSync)
    const mockedSpawn = vi.mocked(spawn)

    // Setup: inside Docker but discovery fails
    mockedExistsSync.mockImplementation((path) => path === '/.dockerenv')
    mockedExecFileSync.mockImplementation(() => {
      throw new Error('docker: command not found')
    })

    // Mock spawn
    mockedSpawn.mockImplementation(() => ({
      on: vi.fn((event, callback) => {
        if (event === 'close') {
          setTimeout(() => callback(0), 10)
        }
      }),
      stdout: { on: vi.fn() },
      stderr: { on: vi.fn() }
    } as any))

    // Spy on console.warn to suppress output during test
    const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

    // WHEN: running setup with detection failure
    await expect(async () => {
      await runSetup()
    }).not.toThrow()

    // THEN: should fall back to /workspace
    expect(process.env.WORKSPACE_HOST_PATH).toBe('/workspace')

    // AND: should warn user about detection failure
    expect(consoleWarnSpy).toHaveBeenCalledWith(
      expect.stringContaining('could not discover workspace host path')
    )

    // AND: should still show workspace path message
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      expect.stringContaining('Workspace path:')
    )

    consoleWarnSpy.mockRestore()
    consoleErrorSpy.mockRestore()
  })
})
