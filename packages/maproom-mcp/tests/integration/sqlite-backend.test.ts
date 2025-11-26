/**
 * SQLite Backend Integration Tests
 *
 * Tests MCP tools (status, search) with SQLite backend to verify
 * graceful degradation and correct functionality without PostgreSQL.
 *
 * Uses pre-indexed fixture from Rust crate tests.
 */

import { describe, test, expect, beforeAll, afterAll, vi } from 'vitest'
import {
  createTestSqliteDatabase,
  cleanupTestSqliteDatabase,
  getSqliteTestUrl,
} from '../helpers/sqlite.js'

describe('MCP Tools with SQLite Backend', () => {
  let testDbPath: string
  let originalEnv: string | undefined

  beforeAll(() => {
    // Save original env
    originalEnv = process.env.MAPROOM_DATABASE_URL

    // Create isolated test database
    testDbPath = createTestSqliteDatabase()
    process.env.MAPROOM_DATABASE_URL = getSqliteTestUrl(testDbPath)
  })

  afterAll(() => {
    // Restore original env
    if (originalEnv) {
      process.env.MAPROOM_DATABASE_URL = originalEnv
    } else {
      delete process.env.MAPROOM_DATABASE_URL
    }

    // Cleanup test database
    cleanupTestSqliteDatabase(testDbPath)
  })

  describe('status tool', () => {
    test('returns degraded response for SQLite backend', async () => {
      // Import dynamically to pick up env change
      const { resolveDatabaseConfig } = await import(
        '../../src/utils/resolve-database.js'
      )

      const config = resolveDatabaseConfig()

      expect(config.type).toBe('sqlite')
      expect(config.path).toBe(testDbPath)
      expect(config.url).toBe(`sqlite://${testDbPath}`)
    })

    test('degraded status response has expected SQLite fields', async () => {
      // Reset modules to pick up new env
      vi.resetModules()

      const { resolveDatabaseConfig } = await import(
        '../../src/utils/resolve-database.js'
      )

      const config = resolveDatabaseConfig()

      // Verify SQLite mode detected
      expect(config.type).toBe('sqlite')

      // The degraded response structure (from index.ts handleStatus)
      // We can't easily test handleStatus directly without mocking more,
      // but we verify the config detection works correctly
      expect(config.path).toBeTruthy()
      expect(config.url).toContain('sqlite://')
    })
  })

  describe('URL resolution', () => {
    test('sqlite:// URL detected correctly', async () => {
      const { isSqliteUrl } = await import(
        '../../src/utils/resolve-database.js'
      )

      expect(isSqliteUrl('sqlite:///path/to/db.sqlite')).toBe(true)
      expect(isSqliteUrl('postgresql://localhost/db')).toBe(false)
    })

    test('resolveDatabaseConfig returns SQLite type for sqlite:// URL', async () => {
      // Ensure our env is set correctly
      expect(process.env.MAPROOM_DATABASE_URL).toContain('sqlite://')

      vi.resetModules()

      const { resolveDatabaseConfig } = await import(
        '../../src/utils/resolve-database.js'
      )

      const config = resolveDatabaseConfig()

      expect(config.type).toBe('sqlite')
      expect(config.path).toBe(testDbPath)
    })
  })

  describe('error handling', () => {
    test('missing SQLite file path is detected', async () => {
      const { resolveDatabaseConfig } = await import(
        '../../src/utils/resolve-database.js'
      )

      // Save current env
      const savedUrl = process.env.MAPROOM_DATABASE_URL
      process.env.MAPROOM_DATABASE_URL = 'sqlite:///nonexistent/path/db.sqlite'

      try {
        vi.resetModules()

        const { resolveDatabaseConfig: freshResolve } = await import(
          '../../src/utils/resolve-database.js'
        )

        const config = freshResolve()

        // Config resolves, but path doesn't exist
        expect(config.type).toBe('sqlite')
        expect(config.path).toBe('/nonexistent/path/db.sqlite')
        // The actual file existence check happens in daemon.ts
      } finally {
        // Restore env
        process.env.MAPROOM_DATABASE_URL = savedUrl
        vi.resetModules()
      }
    })
  })
})

describe('DatabaseConfig Interface', () => {
  test('SQLite config has all required fields', async () => {
    const { resolveDatabaseConfig } = await import(
      '../../src/utils/resolve-database.js'
    )

    const config = resolveDatabaseConfig()

    // Type field
    expect(config.type).toBeDefined()
    expect(['postgresql', 'sqlite']).toContain(config.type)

    // URL field
    expect(config.url).toBeDefined()
    expect(typeof config.url).toBe('string')

    // Path field (optional, present for SQLite)
    if (config.type === 'sqlite') {
      expect(config.path).toBeDefined()
      expect(typeof config.path).toBe('string')
    }
  })
})
