/**
 * Tests for resolveDatabase function
 *
 * Verifies the three-tier database URL resolution:
 * 1. Explicit MAPROOM_DATABASE_URL
 * 2. IN_DEVCONTAINER detection
 * 3. Default localhost:5433
 */

import { describe, test, expect, beforeEach, afterEach } from 'vitest'
import { resolveDatabase } from '../../src/utils/resolve-database'

describe('resolveDatabase', () => {
  const originalEnv = process.env

  beforeEach(() => {
    // Reset environment before each test
    process.env = { ...originalEnv }
    delete process.env.MAPROOM_DATABASE_URL
    delete process.env.IN_DEVCONTAINER
  })

  afterEach(() => {
    process.env = originalEnv
  })

  test('uses MAPROOM_DATABASE_URL when set', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://custom@host:5432/db'
    expect(resolveDatabase()).toBe('postgresql://custom@host:5432/db')
  })

  test('uses container hostname when IN_DEVCONTAINER=true', () => {
    process.env.IN_DEVCONTAINER = 'true'
    expect(resolveDatabase()).toBe('postgresql://maproom:maproom@maproom-postgres:5432/maproom')
  })

  test('defaults to localhost:5433 when no env vars set', () => {
    expect(resolveDatabase()).toBe('postgresql://maproom:maproom@localhost:5433/maproom')
  })

  test('MAPROOM_DATABASE_URL takes precedence over IN_DEVCONTAINER', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://explicit@host:5432/db'
    process.env.IN_DEVCONTAINER = 'true'
    expect(resolveDatabase()).toBe('postgresql://explicit@host:5432/db')
  })

  test('handles IN_DEVCONTAINER=false as not set', () => {
    process.env.IN_DEVCONTAINER = 'false'
    expect(resolveDatabase()).toBe('postgresql://maproom:maproom@localhost:5433/maproom')
  })

  test('handles empty MAPROOM_DATABASE_URL as not set', () => {
    process.env.MAPROOM_DATABASE_URL = ''
    expect(resolveDatabase()).toBe('postgresql://maproom:maproom@localhost:5433/maproom')
  })
})
