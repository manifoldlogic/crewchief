/**
 * Tests for connection factory and mode detection
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { ConnectionMode } from '../connection.js'
import { detectConnectionMode, createConnection } from '../connection-factory.js'

describe('detectConnectionMode', () => {
  let originalEnv: string | undefined
  let originalPlatform: string

  beforeEach(() => {
    originalEnv = process.env.MAPROOM_CONNECTION_MODE
    originalPlatform = process.platform
  })

  afterEach(() => {
    // Restore original environment
    if (originalEnv === undefined) {
      delete process.env.MAPROOM_CONNECTION_MODE
    } else {
      process.env.MAPROOM_CONNECTION_MODE = originalEnv
    }
  })

  it('should return socket mode when env var is "socket"', () => {
    process.env.MAPROOM_CONNECTION_MODE = 'socket'
    expect(detectConnectionMode()).toBe(ConnectionMode.Socket)
  })

  it('should return stdio mode when env var is "stdio"', () => {
    process.env.MAPROOM_CONNECTION_MODE = 'stdio'
    expect(detectConnectionMode()).toBe(ConnectionMode.Stdio)
  })

  it('should return auto mode when env var is "auto"', () => {
    process.env.MAPROOM_CONNECTION_MODE = 'auto'
    expect(detectConnectionMode()).toBe(ConnectionMode.Auto)
  })

  it('should be case-insensitive for env var', () => {
    process.env.MAPROOM_CONNECTION_MODE = 'SOCKET'
    expect(detectConnectionMode()).toBe(ConnectionMode.Socket)

    process.env.MAPROOM_CONNECTION_MODE = 'Stdio'
    expect(detectConnectionMode()).toBe(ConnectionMode.Stdio)
  })

  it('should return stdio mode on Windows when no env var set', () => {
    delete process.env.MAPROOM_CONNECTION_MODE
    // Mock process.platform
    Object.defineProperty(process, 'platform', {
      value: 'win32',
      configurable: true,
    })

    expect(detectConnectionMode()).toBe(ConnectionMode.Stdio)

    // Restore platform
    Object.defineProperty(process, 'platform', {
      value: originalPlatform,
      configurable: true,
    })
  })

  it('should return auto mode on Unix when no env var set', () => {
    delete process.env.MAPROOM_CONNECTION_MODE
    // Mock process.platform
    Object.defineProperty(process, 'platform', {
      value: 'linux',
      configurable: true,
    })

    expect(detectConnectionMode()).toBe(ConnectionMode.Auto)

    // Restore platform
    Object.defineProperty(process, 'platform', {
      value: originalPlatform,
      configurable: true,
    })
  })

  it('should return auto mode on macOS when no env var set', () => {
    delete process.env.MAPROOM_CONNECTION_MODE
    // Mock process.platform
    Object.defineProperty(process, 'platform', {
      value: 'darwin',
      configurable: true,
    })

    expect(detectConnectionMode()).toBe(ConnectionMode.Auto)

    // Restore platform
    Object.defineProperty(process, 'platform', {
      value: originalPlatform,
      configurable: true,
    })
  })
})

describe('createConnection', () => {
  it('should throw error for unknown connection mode', async () => {
    // Force invalid mode by casting
    const config = { mode: 'invalid' as unknown as ConnectionMode }

    await expect(createConnection(config)).rejects.toThrow(
      'Unknown connection mode'
    )
  })

  // Note: Full integration tests for socket and stdio modes are in separate test files
  // These would require an actual daemon binary and are better suited for integration tests
})
