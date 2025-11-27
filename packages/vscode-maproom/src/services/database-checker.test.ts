/**
 * Tests for database-checker.ts
 *
 * Verifies database configuration resolution and availability checking
 * for SQLite backend.
 */

import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import {
  resolveDatabaseConfig,
  checkDatabaseAvailable,
  getDatabaseUrl,
  getDatabaseUnavailableMessage,
  expandPath,
  type DatabaseConfig,
} from './database-checker'

// Mock vscode
const mockSettings: Record<string, any> = {}
vi.mock('vscode', () => ({
  workspace: {
    getConfiguration: vi.fn(() => ({
      get: vi.fn((key: string) => mockSettings[key]),
    })),
  },
}))

// Mock node:fs
const mockFiles: string[] = []
vi.mock('node:fs', () => ({
  existsSync: vi.fn((path: string) => mockFiles.includes(path)),
}))

// Mock node:os
vi.mock('node:os', () => ({
  homedir: vi.fn(() => '/mock/home'),
}))

describe('expandPath', () => {
  it('expands ~ to home directory', () => {
    const result = expandPath('~/.maproom/maproom.db')
    expect(result).toBe('/mock/home/.maproom/maproom.db')
  })

  it('returns absolute paths unchanged', () => {
    const result = expandPath('/absolute/path/to/file.db')
    expect(result).toBe('/absolute/path/to/file.db')
  })

  it('handles paths without tilde', () => {
    const result = expandPath('relative/path/file.db')
    expect(result).toBe('relative/path/file.db')
  })

  it('only expands tilde at the beginning', () => {
    const result = expandPath('/path/with/~/inside')
    expect(result).toBe('/path/with/~/inside')
  })
})

describe('resolveDatabaseConfig', () => {
  beforeEach(() => {
    // Reset mock settings
    Object.keys(mockSettings).forEach((key) => delete mockSettings[key])
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('returns sqlite config by default', () => {
    mockSettings['sqlitePath'] = ''

    const config = resolveDatabaseConfig()

    expect(config.type).toBe('sqlite')
    expect(config.url).toContain('sqlite://')
    expect(config.path).toBeDefined()
  })

  it('expands tilde in sqlite path', () => {
    mockSettings['sqlitePath'] = '~/.maproom/custom.db'

    const config = resolveDatabaseConfig()

    expect(config.path).toBe('/mock/home/.maproom/custom.db')
    expect(config.url).toBe('sqlite:///mock/home/.maproom/custom.db')
  })

  it('uses default path when sqlitePath is empty', () => {
    mockSettings['sqlitePath'] = ''

    const config = resolveDatabaseConfig()

    expect(config.path).toBe('/mock/home/.maproom/maproom.db')
  })

  it('uses custom path when sqlitePath is set', () => {
    mockSettings['sqlitePath'] = '/custom/path/mydb.db'

    const config = resolveDatabaseConfig()

    expect(config.path).toBe('/custom/path/mydb.db')
    expect(config.url).toBe('sqlite:///custom/path/mydb.db')
  })

  it('resolves relative paths to absolute for sqlite', () => {
    mockSettings['sqlitePath'] = 'relative/path/db.sqlite'

    const config = resolveDatabaseConfig()

    // Should be resolved to absolute path
    expect(config.path).toContain('relative/path/db.sqlite')
    expect(config.path?.startsWith('/')).toBe(true)
  })
})

describe('checkDatabaseAvailable', () => {
  beforeEach(() => {
    mockFiles.length = 0 // Clear mock files array
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('returns true when sqlite file exists', async () => {
    const config: DatabaseConfig = {
      type: 'sqlite',
      url: 'sqlite:///path/to/db.sqlite',
      path: '/path/to/db.sqlite',
    }
    mockFiles.push('/path/to/db.sqlite')

    const result = await checkDatabaseAvailable(config)

    expect(result).toBe(true)
  })

  it('returns false when sqlite file missing', async () => {
    const config: DatabaseConfig = {
      type: 'sqlite',
      url: 'sqlite:///path/to/missing.db',
      path: '/path/to/missing.db',
    }
    // mockFiles is empty, so file doesn't exist

    const result = await checkDatabaseAvailable(config)

    expect(result).toBe(false)
  })
})

describe('getDatabaseUrl', () => {
  it('returns url from config', () => {
    const config: DatabaseConfig = {
      type: 'sqlite',
      url: 'sqlite:///path/to/db.sqlite',
      path: '/path/to/db.sqlite',
    }

    const url = getDatabaseUrl(config)

    expect(url).toBe('sqlite:///path/to/db.sqlite')
  })
})

describe('getDatabaseUnavailableMessage', () => {
  it('returns sqlite message with file path', () => {
    const config: DatabaseConfig = {
      type: 'sqlite',
      url: 'sqlite:///path/to/db.sqlite',
      path: '/path/to/db.sqlite',
    }

    const message = getDatabaseUnavailableMessage(config)

    expect(message).toContain('SQLite database not found')
    expect(message).toContain('/path/to/db.sqlite')
    expect(message).toContain('crewchief-maproom scan')
  })
})

describe('DatabaseConfig Interface', () => {
  it('sqlite config has required fields', () => {
    const config: DatabaseConfig = {
      type: 'sqlite',
      url: 'sqlite:///test.db',
      path: '/test.db',
    }

    expect(config.type).toBe('sqlite')
    expect(config.url).toBeDefined()
    expect(config.path).toBeDefined()
  })
})
