/**
 * Tests for database URL resolution functions
 *
 * Verifies SQLite-only database URL resolution:
 * 1. Explicit MAPROOM_DATABASE_URL (must be sqlite://)
 * 2. SQLite default (~/.maproom/maproom.db)
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
    mockExistsSync.mockReturnValue(false)
  })

  afterEach(() => {
    process.env = originalEnv
  })

  test('uses MAPROOM_DATABASE_URL when set', () => {
    process.env.MAPROOM_DATABASE_URL = 'sqlite:///custom/path/db.sqlite'
    expect(resolveDatabase()).toBe('sqlite:///custom/path/db.sqlite')
  })

  test('defaults to ~/.maproom/maproom.db when no env vars set', () => {
    const expectedPath = `${homedir()}/.maproom/maproom.db`
    expect(resolveDatabase()).toBe(`sqlite://${expectedPath}`)
  })

  test('handles empty MAPROOM_DATABASE_URL as not set', () => {
    process.env.MAPROOM_DATABASE_URL = ''
    const expectedPath = `${homedir()}/.maproom/maproom.db`
    expect(resolveDatabase()).toBe(`sqlite://${expectedPath}`)
  })

  test('throws error for postgresql:// URL', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://user:pass@host:5432/db'
    expect(() => resolveDatabase()).toThrow('Only SQLite is supported')
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

  describe('PostgreSQL rejection', () => {
    test('throws error for postgresql:// URL', () => {
      process.env.MAPROOM_DATABASE_URL = 'postgresql://user:pass@host:5432/db'
      expect(() => resolveDatabaseConfig()).toThrow('Only SQLite is supported')
    })

    test('throws error for postgres:// URL', () => {
      process.env.MAPROOM_DATABASE_URL = 'postgres://user:pass@host:5432/db'
      expect(() => resolveDatabaseConfig()).toThrow('Only SQLite is supported')
    })
  })

  describe('resolution priority', () => {
    test('explicit SQLite URL takes precedence', () => {
      process.env.MAPROOM_DATABASE_URL = 'sqlite:///custom/path.db'
      mockExistsSync.mockReturnValue(true)

      const config = resolveDatabaseConfig()
      expect(config.type).toBe('sqlite')
      expect(config.path).toBe('/custom/path.db')
    })

    test('defaults to ~/.maproom/maproom.db when no URL set', () => {
      const expectedPath = `${homedir()}/.maproom/maproom.db`

      const config = resolveDatabaseConfig()

      expect(config.type).toBe('sqlite')
      expect(config.url).toBe(`sqlite://${expectedPath}`)
      expect(config.path).toBe(expectedPath)
    })
  })
})
