/**
 * Tests for database URL resolution functions
 *
 * Verifies database URL resolution:
 * 1. Explicit MAPROOM_DATABASE_URL — sqlite:// and postgres:// / postgresql://
 * 2. SQLite default (~/.maproom/maproom.db) when no URL is set
 *
 * Requirements covered:
 * - R1: postgres:// and postgresql:// accepted; sqlite tier-2 default kept
 * - R4: redactedUrl never contains credentials
 * - A5: vitest fallback handles both backends
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { homedir } from 'node:os'
import { resolve } from 'node:path'
import {
  resolveDatabase,
  resolveDatabaseConfig,
  isSqliteUrl,
  isPostgresUrl,
  redactPostgresUrl,
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

// ---------------------------------------------------------------------------
// resolveDatabase (deprecated helper — regression-pinned)
// ---------------------------------------------------------------------------
describe('resolveDatabase', () => {
  const originalEnv = process.env

  beforeEach(() => {
    process.env = { ...originalEnv }
    delete process.env.MAPROOM_DATABASE_URL
    mockExistsSync.mockReturnValue(false)
  })

  afterEach(() => {
    process.env = originalEnv
  })

  test('uses sqlite MAPROOM_DATABASE_URL when set', () => {
    process.env.MAPROOM_DATABASE_URL = 'sqlite:///custom/path/db.sqlite'
    expect(resolveDatabase()).toBe('sqlite:///custom/path/db.sqlite')
  })

  test('returns the full postgres URL (with credentials) for resolveDatabase()', () => {
    // resolveDatabase() is a thin wrapper — it returns dbConfig.url unchanged
    const pgUrl = 'postgres://user:pass@host:5432/mydb'
    process.env.MAPROOM_DATABASE_URL = pgUrl
    expect(resolveDatabase()).toBe(pgUrl)
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

  test('throws error for unrecognised URL scheme', () => {
    process.env.MAPROOM_DATABASE_URL = 'mysql://user:pass@host:3306/db'
    expect(() => resolveDatabase()).toThrow('Unsupported database URL scheme')
  })
})

// ---------------------------------------------------------------------------
// isSqliteUrl
// ---------------------------------------------------------------------------
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

// ---------------------------------------------------------------------------
// isPostgresUrl (new)
// ---------------------------------------------------------------------------
describe('isPostgresUrl', () => {
  test('returns true for postgres:// and postgresql:// URLs', () => {
    expect(isPostgresUrl('postgres://localhost/db')).toBe(true)
    expect(isPostgresUrl('postgresql://user:pass@host:5432/db')).toBe(true)
    expect(isPostgresUrl('postgres://host.docker.internal:5433/maproom')).toBe(true)
  })

  test('returns false for non-postgres URLs', () => {
    expect(isPostgresUrl('sqlite:///path/to/db.sqlite')).toBe(false)
    expect(isPostgresUrl('mysql://localhost/db')).toBe(false)
    expect(isPostgresUrl('')).toBe(false)
  })
})

// ---------------------------------------------------------------------------
// redactPostgresUrl (new) — D-2: never echo the password
// ---------------------------------------------------------------------------
describe('redactPostgresUrl', () => {
  test('removes username and password, keeps host+port+dbname', () => {
    expect(redactPostgresUrl('postgres://user:pass@host.docker.internal:5433/maproom'))
      .toBe('postgres://host.docker.internal:5433/maproom')
  })

  test('handles postgres URL with username but no password', () => {
    expect(redactPostgresUrl('postgres://user@localhost:5432/mydb'))
      .toBe('postgres://localhost:5432/mydb')
  })

  test('handles postgresql:// scheme — normalises to postgres://', () => {
    expect(redactPostgresUrl('postgresql://admin:secret@db.example.com:5432/app'))
      .toBe('postgres://db.example.com:5432/app')
  })

  test('handles URL without port', () => {
    expect(redactPostgresUrl('postgres://user:pass@localhost/mydb'))
      .toBe('postgres://localhost/mydb')
  })

  test('handles URL without credentials', () => {
    expect(redactPostgresUrl('postgres://localhost:5433/maproom'))
      .toBe('postgres://localhost:5433/maproom')
  })

  test('returns safe placeholder for malformed URL', () => {
    expect(redactPostgresUrl('not-a-url')).toBe('postgres://[redacted]')
  })

  test('redacted URL never contains the original password', () => {
    const fullUrl = 'postgres://maproom:supersecret@host.docker.internal:5433/maproom'
    const redacted = redactPostgresUrl(fullUrl)
    expect(redacted).not.toContain('supersecret')
    expect(redacted).not.toContain('maproom:')  // no user:pass@ form
  })
})

// ---------------------------------------------------------------------------
// resolveDatabaseConfig
// ---------------------------------------------------------------------------
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

  // --- SQLite (regression-pinned) -------------------------------------------
  describe('SQLite URL parsing', () => {
    test('parses absolute sqlite:// URL', () => {
      process.env.MAPROOM_DATABASE_URL = 'sqlite:///absolute/path/to/db.sqlite'
      const config = resolveDatabaseConfig()

      expect(config.type).toBe('sqlite')
      expect(config.url).toBe('sqlite:///absolute/path/to/db.sqlite')
      if (config.type === 'sqlite') {
        expect(config.path).toBe('/absolute/path/to/db.sqlite')
      }
    })

    test('parses sqlite:// URL with tilde expansion', () => {
      process.env.MAPROOM_DATABASE_URL = 'sqlite://~/.maproom/maproom.db'
      const config = resolveDatabaseConfig()
      const expectedPath = `${homedir()}/.maproom/maproom.db`

      expect(config.type).toBe('sqlite')
      expect(config.url).toBe(`sqlite://${expectedPath}`)
      if (config.type === 'sqlite') {
        expect(config.path).toBe(expectedPath)
      }
    })

    test('parses sqlite:// URL with relative path', () => {
      process.env.MAPROOM_DATABASE_URL = 'sqlite://./data/maproom.db'
      const config = resolveDatabaseConfig()
      const expectedPath = resolve(process.cwd(), './data/maproom.db')

      expect(config.type).toBe('sqlite')
      expect(config.url).toBe(`sqlite://${expectedPath}`)
      if (config.type === 'sqlite') {
        expect(config.path).toBe(expectedPath)
      }
    })
  })

  // --- PostgreSQL (new) -------------------------------------------------------
  describe('PostgreSQL URL acceptance (R1)', () => {
    test('accepts postgres:// URL and returns postgres config', () => {
      const pgUrl = 'postgres://maproom:maproom@host.docker.internal:5433/maproom'
      process.env.MAPROOM_DATABASE_URL = pgUrl
      const config = resolveDatabaseConfig()

      expect(config.type).toBe('postgres')
      expect(config.url).toBe(pgUrl)
    })

    test('accepts postgresql:// URL and returns postgres config', () => {
      const pgUrl = 'postgresql://admin:secret@db.example.com:5432/app'
      process.env.MAPROOM_DATABASE_URL = pgUrl
      const config = resolveDatabaseConfig()

      expect(config.type).toBe('postgres')
      expect(config.url).toBe(pgUrl)
    })

    test('postgres config has redactedUrl that omits credentials', () => {
      process.env.MAPROOM_DATABASE_URL =
        'postgres://maproom:maproom@host.docker.internal:5433/maproom'
      const config = resolveDatabaseConfig()

      expect(config.type).toBe('postgres')
      if (config.type === 'postgres') {
        expect(config.redactedUrl).toBe('postgres://host.docker.internal:5433/maproom')
        expect(config.redactedUrl).not.toContain('maproom:maproom@')
      }
    })

    test('postgres config redactedUrl never contains the password', () => {
      process.env.MAPROOM_DATABASE_URL =
        'postgres://user:supersecret@localhost:5432/mydb'
      const config = resolveDatabaseConfig()

      expect(config.type).toBe('postgres')
      if (config.type === 'postgres') {
        expect(config.redactedUrl).not.toContain('supersecret')
      }
    })
  })

  // --- Unrecognised scheme ---------------------------------------------------
  describe('Unsupported URL schemes', () => {
    test('throws for mysql:// URL', () => {
      process.env.MAPROOM_DATABASE_URL = 'mysql://user:pass@host:3306/db'
      expect(() => resolveDatabaseConfig()).toThrow('Unsupported database URL scheme')
    })

    test('throws for bare file path', () => {
      process.env.MAPROOM_DATABASE_URL = '/path/to/database.db'
      expect(() => resolveDatabaseConfig()).toThrow('Unsupported database URL scheme')
    })
  })

  // --- Resolution priority --------------------------------------------------
  describe('resolution priority', () => {
    test('explicit SQLite URL takes precedence over default', () => {
      process.env.MAPROOM_DATABASE_URL = 'sqlite:///custom/path.db'
      mockExistsSync.mockReturnValue(true)

      const config = resolveDatabaseConfig()
      expect(config.type).toBe('sqlite')
      if (config.type === 'sqlite') {
        expect(config.path).toBe('/custom/path.db')
      }
    })

    test('explicit postgres URL takes precedence over default', () => {
      process.env.MAPROOM_DATABASE_URL =
        'postgres://maproom:maproom@host.docker.internal:5433/maproom'
      const config = resolveDatabaseConfig()
      expect(config.type).toBe('postgres')
    })

    test('defaults to ~/.maproom/maproom.db (sqlite) when no URL set', () => {
      const expectedPath = `${homedir()}/.maproom/maproom.db`

      const config = resolveDatabaseConfig()

      expect(config.type).toBe('sqlite')
      expect(config.url).toBe(`sqlite://${expectedPath}`)
      if (config.type === 'sqlite') {
        expect(config.path).toBe(expectedPath)
      }
    })
  })
})
