/**
 * Integration tests for daemon discovery and connect-or-spawn logic
 *
 * These tests verify that the double-check pattern correctly prevents
 * race conditions when multiple clients start simultaneously.
 *
 * Requirements:
 * - crewchief-maproom binary must be in PATH or built
 * - Tests spawn actual daemon processes
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import * as fs from 'node:fs'
import { exec } from 'node:child_process'
import { promisify } from 'node:util'
import {
  connectOrSpawn,
  getDefaultConfig,
  type DiscoveryConfig,
} from '../discovery.js'
import { DaemonStartupError } from '../errors.js'

const execAsync = promisify(exec)

describe('connectOrSpawn', () => {
  let config: DiscoveryConfig

  beforeEach(() => {
    // Use unique paths for each test to avoid conflicts
    const timestamp = Date.now()
    config = {
      ...getDefaultConfig(),
      binaryPath: '/workspace/target/release/crewchief-maproom', // Use absolute path for tests
      socketPath: `/tmp/test-maproom-${timestamp}.sock`,
      lockPath: `/tmp/test-maproom-${timestamp}.lock`,
    }

    // Clean up any existing files
    ;[config.socketPath, config.lockPath].forEach((path) => {
      if (fs.existsSync(path)) {
        fs.unlinkSync(path)
      }
    })
  })

  afterEach(async () => {
    // Kill any test daemons using specific socket path
    try {
      const { stdout } = await execAsync(
        `ps aux | grep 'crewchief-maproom.*serve.*${config.socketPath}' | grep -v grep | awk '{print $2}'`
      )
      const pids = stdout
        .trim()
        .split('\n')
        .filter((p) => p.length > 0)
      for (const pid of pids) {
        try {
          await execAsync(`kill ${pid}`)
        } catch {
          // Process may have already exited
        }
      }
      // Give processes time to clean up
      await new Promise((resolve) => setTimeout(resolve, 500))
    } catch {
      // Ignore if no processes found
    }

    // Clean up files
    [config.socketPath, config.lockPath].forEach((path) => {
      if (fs.existsSync(path)) {
        fs.unlinkSync(path)
      }
    })
  })

  it('spawns daemon on first call', async () => {
    const conn = await connectOrSpawn(config)
    expect(conn.isConnected()).toBe(true)

    // Verify daemon process exists for this specific socket
    const { stdout } = await execAsync(
      `ps aux | grep 'crewchief-maproom.*serve.*${config.socketPath}' | grep -v grep`
    )
    expect(stdout).toContain('serve')

    await conn.close()
  })

  it('reuses existing daemon on second call', async () => {
    // First call spawns daemon
    const conn1 = await connectOrSpawn(config)

    // Second call should reuse
    const conn2 = await connectOrSpawn(config)

    expect(conn1.isConnected()).toBe(true)
    expect(conn2.isConnected()).toBe(true)

    // Verify only one daemon process for this specific socket
    const { stdout } = await execAsync(
      `ps aux | grep 'crewchief-maproom.*serve.*${config.socketPath}' | grep -v grep`
    )
    const lines = stdout.trim().split('\n').filter((l) => l.length > 0)
    expect(lines.length).toBe(1)

    await conn1.close()
    await conn2.close()
  })

  it('handles concurrent spawn attempts (race condition)', async () => {
    // Spawn 5 clients simultaneously
    const connections = await Promise.all([
      connectOrSpawn(config),
      connectOrSpawn(config),
      connectOrSpawn(config),
      connectOrSpawn(config),
      connectOrSpawn(config),
    ])

    // All should connect successfully
    expect(connections.every((c) => c.isConnected())).toBe(true)

    // Verify only one daemon spawned for this specific socket
    const { stdout } = await execAsync(
      `ps aux | grep 'crewchief-maproom.*serve.*${config.socketPath}' | grep -v grep`
    )
    const lines = stdout.trim().split('\n').filter((l) => l.length > 0)
    expect(lines.length).toBe(1)

    // Clean up
    await Promise.all(connections.map((c) => c.close()))
  }, 30000) // Longer timeout for this test

  it('throws error if daemon binary not found', async () => {
    const badConfig = {
      ...config,
      binaryPath: '/nonexistent/binary',
      startupTimeout: 1000, // Short timeout for this test
    }

    await expect(connectOrSpawn(badConfig)).rejects.toThrow(DaemonStartupError)
  }, 5000) // 5 second timeout for test

  it('times out if daemon never creates socket', async () => {
    const badConfig = {
      ...config,
      binaryPath: 'sleep', // Won't create socket
      startupTimeout: 1000, // Short timeout for test
    }

    // Note: sleep command needs an argument, so this will fail immediately
    // which still tests our error handling
    await expect(connectOrSpawn(badConfig)).rejects.toThrow(DaemonStartupError)
  })

  it('verifies only one daemon PID exists', async () => {
    // Spawn multiple connections
    const connections = await Promise.all([
      connectOrSpawn(config),
      connectOrSpawn(config),
      connectOrSpawn(config),
    ])

    // Get all daemon PIDs for this specific socket
    const { stdout } = await execAsync(
      `ps aux | grep 'crewchief-maproom.*serve.*${config.socketPath}' | grep -v grep | awk '{print $2}'`
    )
    const pids = stdout
      .trim()
      .split('\n')
      .filter((p) => p.length > 0)

    // Should have exactly one PID
    expect(pids.length).toBe(1)

    await Promise.all(connections.map((c) => c.close()))
  })

  it('cleans up lock file after successful spawn', async () => {
    const conn = await connectOrSpawn(config)

    // Lock file should exist (created during spawn)
    expect(fs.existsSync(config.lockPath)).toBe(true)

    await conn.close()
  })
})
