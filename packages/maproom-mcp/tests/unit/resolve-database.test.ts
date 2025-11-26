/**
 * Tests for database URL resolution functions
 *
 * Verifies the four-tier database URL resolution:
 * 1. Explicit MAPROOM_DATABASE_URL
 * 2. IN_DEVCONTAINER detection
 * 3. SQLite default (~/.maproom/maproom.db if exists)
 * 4. Default localhost:5433
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { homedir } from 'node:os'
import { resolve } from 'node:path'
import {
  resolveDatabase,
  resolveDatabaseConfig,
  isSqliteUrl,
} from '../../src/utils/resolve-database'

// Mock fs.existsSync for SQLite file detection tests
vi.mock('node:fs', async () => {
  const actual = await vi.importActual('node:fs')
  return {
    ...actual,
    existsSync: vi.fn().mockReturnValue(false),
  }
})

import { existsSync } from 'node:fs'
const mockExistsSync = vi.mocked(existsSync)

describe('resolveDatabase', () => {
  const originalEnv = process.env

  beforeEach(() => {
    // Reset environment before each test
    process.env = { ...originalEnv }
    delete process.env.MAPROOM_DATABASE_URL
    delete process.env.IN_DEVCONTAINER
    mockExistsSync.mockReturnValue(false)
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

describe('isSqliteUrl', () => {
  test('returns true for sqlite:// URLs', () => {
    expect(isSqliteUrl('sqlite:///path/to/db.sqlite')).toBe(true)
    expect(isSqliteUrl('sqlite://~/maproom.db')).toBe(true)
    expect(isSqliteUrl('sqlite://./relative/path.db')).toBe(true)
  })

  test('returns false for non-sqlite URLs', () => {
    expect(isSqliteUrl('postgresql://localhost/db')).toBe(false)
    expect(isSqliteUrl('postgres://localhost/db')).toBe(false)
    expect(isSqliteUrl('/path/to/db.sqlite')).toBe(false)
    expect(isSqliteUrl('')).toBe(false)
  })
})

describe('resolveDatabaseConfig', () => {
  const originalEnv = process.env

  beforeEach(() => {
    process.env = { ...originalEnv }
    delete process.env.MAPROOM_DATABASE_URL
    delete process.env.IN_DEVCONTAINER
    mockExistsSync.mockReturnValue(false)
  })

  afterEach(() => {
    process.env = originalEnv
  })

  describe('SQLite URL parsing', () => {
    test('parses absolute sqlite:// URL', () => {
      process.env.MAPROOM_DATABASE_URL = 'sqlite:///absolute/path/to/db.sqlite'
      const config = resolveDatabaseConfig()

      expect(config.type).toBe('sqlite')
      expect(config.url).toBe('sqlite:///absolute/path/to/db.sqlite')
      expect(config.path).toBe('/absolute/path/to/db.sqlite')
    })

    test('parses sqlite:// URL with tilde expansion', () => {
      process.env.MAPROOM_DATABASE_URL = 'sqlite://~/.maproom/maproom.db'
      const config = resolveDatabaseConfig()
      const expectedPath = `${homedir()}/.maproom/maproom.db`

      expect(config.type).toBe('sqlite')
      expect(config.url).toBe(`sqlite://${expectedPath}`)
      expect(config.path).toBe(expectedPath)
    })

    test('parses sqlite:// URL with relative path', () => {
      process.env.MAPROOM_DATABASE_URL = 'sqlite://./data/maproom.db'
      const config = resolveDatabaseConfig()
      const expectedPath = resolve(process.cwd(), './data/maproom.db')

      expect(config.type).toBe('sqlite')
      expect(config.url).toBe(`sqlite://${expectedPath}`)
      expect(config.path).toBe(expectedPath)
    })
  })

  describe('PostgreSQL URL parsing', () => {
    test('parses postgresql:// URL', () => {
      process.env.MAPROOM_DATABASE_URL = 'postgresql://user:pass@host:5432/db'
      const config = resolveDatabaseConfig()

      expect(config.type).toBe('postgresql')
      expect(config.url).toBe('postgresql://user:pass@host:5432/db')
      expect(config.path).toBeUndefined()
    })

    test('parses postgres:// URL', () => {
      process.env.MAPROOM_DATABASE_URL = 'postgres://user:pass@host:5432/db'
      const config = resolveDatabaseConfig()

      expect(config.type).toBe('postgresql')
      expect(config.url).toBe('postgres://user:pass@host:5432/db')
      expect(config.path).toBeUndefined()
    })
  })

  describe('resolution priority', () => {
    test('explicit SQLite URL takes precedence over everything', () => {
      process.env.MAPROOM_DATABASE_URL = 'sqlite:///custom/path.db'
      process.env.IN_DEVCONTAINER = 'true'
      mockExistsSync.mockReturnValue(true)

      const config = resolveDatabaseConfig()
      expect(config.type).toBe('sqlite')
      expect(config.path).toBe('/custom/path.db')
    })

    test('DevContainer takes precedence over SQLite auto-detection', () => {
      process.env.IN_DEVCONTAINER = 'true'
      mockExistsSync.mockReturnValue(true)

      const config = resolveDatabaseConfig()
      expect(config.type).toBe('postgresql')
      expect(config.url).toBe('postgresql://maproom:maproom@maproom-postgres:5432/maproom')
    })

    test('detects SQLite when ~/.maproom/maproom.db exists', () => {
      mockExistsSync.mockReturnValue(true)
      const expectedPath = `${homedir()}/.maproom/maproom.db`

      const config = resolveDatabaseConfig()

      expect(config.type).toBe('sqlite')
      expect(config.url).toBe(`sqlite://${expectedPath}`)
      expect(config.path).toBe(expectedPath)
      expect(mockExistsSync).toHaveBeenCalledWith(expectedPath)
    })

    test('falls back to PostgreSQL when SQLite default not found', () => {
      mockExistsSync.mockReturnValue(false)

      const config = resolveDatabaseConfig()
      expect(config.type).toBe('postgresql')
      expect(config.url).toBe('postgresql://maproom:maproom@localhost:5433/maproom')
    })
  })
})
