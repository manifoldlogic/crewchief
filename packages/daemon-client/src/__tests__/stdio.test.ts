/**
 * Tests for StdioConnection
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { StdioConnection } from '../stdio.js'
import { DaemonCommunicationError, DaemonStartupError } from '../errors.js'

describe('StdioConnection', () => {
  it('should create instance with binary path', () => {
    const conn = new StdioConnection('maproom')
    expect(conn).toBeDefined()
    expect(conn.isConnected()).toBe(false)
  })

  it('should report not connected initially', () => {
    const conn = new StdioConnection('maproom')
    expect(conn.isConnected()).toBe(false)
  })

  it('should reject requests when not connected', async () => {
    const conn = new StdioConnection('maproom')

    await expect(conn.sendRequest('ping')).rejects.toThrow(
      DaemonCommunicationError
    )
  })

  it('should register error handlers', () => {
    const conn = new StdioConnection('maproom')
    const handler = vi.fn()

    conn.on('error', handler)
    // Handler registration should not throw
    expect(handler).not.toHaveBeenCalled()
  })

  it('should register close handlers', () => {
    const conn = new StdioConnection('maproom')
    const handler = vi.fn()

    conn.on('close', handler)
    // Handler registration should not throw
    expect(handler).not.toHaveBeenCalled()
  })

  // Note: Full integration tests that spawn actual daemon processes are in
  // integration test suites. These unit tests focus on the interface contract.
})

describe('StdioConnection - Connection Interface', () => {
  it('should implement all Connection interface methods', () => {
    const conn = new StdioConnection('maproom')

    // Check all required methods exist
    expect(typeof conn.sendRequest).toBe('function')
    expect(typeof conn.close).toBe('function')
    expect(typeof conn.isConnected).toBe('function')
    expect(typeof conn.on).toBe('function')
  })

  it('should handle multiple close calls gracefully', async () => {
    const conn = new StdioConnection('maproom')

    // First close should work
    await expect(conn.close()).resolves.not.toThrow()

    // Second close should also work (idempotent)
    await expect(conn.close()).resolves.not.toThrow()
  })
})
