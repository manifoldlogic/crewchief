import type { SpawnSyncReturns } from 'node:child_process'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  resolveMaproomDbPath,
  maproomIndexExists,
  runAutoIndexScan,
  runMaproomSearchWithAutoIndex,
} from '../maproom.js'

// ---- Mocks ----

// Mock node:child_process
const mockSpawnSync = vi.fn<(...args: unknown[]) => SpawnSyncReturns<Buffer>>()
vi.mock('node:child_process', () => ({
  spawnSync: (...args: unknown[]) => mockSpawnSync(...args),
}))

// Mock node:fs for existsSync (used by maproomIndexExists and findMaproomBinary)
const mockExistsSync = vi.fn<(p: string) => boolean>()
vi.mock('node:fs', () => ({
  default: { existsSync: (p: string) => mockExistsSync(p) },
  existsSync: (p: string) => mockExistsSync(p),
}))

// Mock config loader
vi.mock('../../config/loader.js', () => ({
  loadConfig: vi.fn().mockResolvedValue({
    repository: { maproomBinaryPath: undefined },
  }),
}))

// Mock maproom binary resolver to always return a known path
vi.mock('../../utils/maproom-binary.js', () => ({
  findMaproomBinary: vi.fn(() => ({
    path: '/usr/local/bin/maproom',
    source: 'global',
  })),
}))

// Mock the validation module so it never blocks
vi.mock('../maproom-validation.js', () => ({
  validateMaproomEnvironment: vi.fn(() => ({
    valid: true,
    errors: [],
    warnings: [],
  })),
  displayValidationResult: vi.fn(),
}))

// ---- Helpers ----

/** Build a minimal SpawnSyncReturns with the given exit code */
function spawnResult(status: number): SpawnSyncReturns<Buffer> {
  return {
    status,
    signal: null,
    output: [],
    pid: 0,
    stdout: Buffer.alloc(0),
    stderr: Buffer.alloc(0),
  }
}

// ---- Tests ----

describe('resolveMaproomDbPath', () => {
  const origEnv = process.env.MAPROOM_DATABASE_URL

  afterEach(() => {
    if (origEnv !== undefined) {
      process.env.MAPROOM_DATABASE_URL = origEnv
    } else {
      delete process.env.MAPROOM_DATABASE_URL
    }
  })

  it('returns ~/.maproom/maproom.db by default', () => {
    delete process.env.MAPROOM_DATABASE_URL
    const dbPath = resolveMaproomDbPath()
    expect(dbPath).toMatch(/\.maproom\/maproom\.db$/)
  })

  it('strips sqlite:// prefix from MAPROOM_DATABASE_URL', () => {
    process.env.MAPROOM_DATABASE_URL = 'sqlite:///custom/path/maproom.db'
    expect(resolveMaproomDbPath()).toBe('/custom/path/maproom.db')
  })

  it('expands tilde in MAPROOM_DATABASE_URL', () => {
    process.env.MAPROOM_DATABASE_URL = 'sqlite://~/.maproom/maproom.db'
    const dbPath = resolveMaproomDbPath()
    expect(dbPath).not.toContain('~')
    expect(dbPath).toMatch(/\.maproom\/maproom\.db$/)
  })

  it('handles plain path in MAPROOM_DATABASE_URL (no sqlite:// prefix)', () => {
    process.env.MAPROOM_DATABASE_URL = '/custom/maproom.db'
    expect(resolveMaproomDbPath()).toBe('/custom/maproom.db')
  })
})

describe('maproomIndexExists', () => {
  afterEach(() => {
    delete process.env.MAPROOM_DATABASE_URL
  })

  it('returns true when the database file exists', () => {
    mockExistsSync.mockReturnValue(true)
    expect(maproomIndexExists()).toBe(true)
  })

  it('returns false when the database file does not exist', () => {
    mockExistsSync.mockReturnValue(false)
    expect(maproomIndexExists()).toBe(false)
  })
})

describe('runAutoIndexScan', () => {
  let consoleSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {})
  })

  afterEach(() => {
    consoleSpy.mockRestore()
    mockSpawnSync.mockReset()
  })

  it('prints the progress message', () => {
    mockSpawnSync.mockReturnValue(spawnResult(0))
    runAutoIndexScan('/usr/local/bin/maproom')
    expect(consoleSpy).toHaveBeenCalledWith(
      'No index found. Building FTS index for this repo (no embedding provider required).',
    )
  })

  it('invokes maproom scan with no extra flags (FTS-only)', () => {
    mockSpawnSync.mockReturnValue(spawnResult(0))
    runAutoIndexScan('/usr/local/bin/maproom')
    expect(mockSpawnSync).toHaveBeenCalledWith('/usr/local/bin/maproom', ['scan'], { stdio: 'inherit' })
  })

  it('returns 0 on success', () => {
    mockSpawnSync.mockReturnValue(spawnResult(0))
    expect(runAutoIndexScan('/usr/local/bin/maproom')).toBe(0)
  })

  it('returns non-zero on failure', () => {
    mockSpawnSync.mockReturnValue(spawnResult(1))
    expect(runAutoIndexScan('/usr/local/bin/maproom')).toBe(1)
  })
})

describe('runMaproomSearchWithAutoIndex', () => {
  let consoleLogSpy: ReturnType<typeof vi.spyOn>
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {})
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    mockSpawnSync.mockReset()
    mockExistsSync.mockReset()
    process.exitCode = undefined
  })

  afterEach(() => {
    consoleLogSpy.mockRestore()
    consoleErrorSpy.mockRestore()
    process.exitCode = undefined
  })

  it('runs search directly when index already exists (no auto-index)', async () => {
    // Database file exists
    mockExistsSync.mockReturnValue(true)
    // Search succeeds
    mockSpawnSync.mockReturnValue(spawnResult(0))

    await runMaproomSearchWithAutoIndex(['--repo', 'myrepo', '--query', 'auth'])

    // Should NOT have printed the auto-index message
    expect(consoleLogSpy).not.toHaveBeenCalledWith(expect.stringContaining('No index found'))
    // Should have called search only (no scan)
    expect(mockSpawnSync).toHaveBeenCalledTimes(1)
    expect(mockSpawnSync).toHaveBeenCalledWith(
      '/usr/local/bin/maproom',
      ['search', '--repo', 'myrepo', '--query', 'auth'],
      { stdio: 'inherit' },
    )
  })

  it('auto-indexes and then searches when no database file exists (pre-flight)', async () => {
    // Database file does NOT exist
    mockExistsSync.mockReturnValue(false)
    // First call: scan succeeds; second call: search succeeds
    mockSpawnSync
      .mockReturnValueOnce(spawnResult(0)) // scan
      .mockReturnValueOnce(spawnResult(0)) // search

    await runMaproomSearchWithAutoIndex(['--repo', 'myrepo', '--query', 'auth'])

    // Should have printed the progress message
    expect(consoleLogSpy).toHaveBeenCalledWith(
      'No index found. Building FTS index for this repo (no embedding provider required).',
    )
    // Should have called scan then search
    expect(mockSpawnSync).toHaveBeenCalledTimes(2)
    expect(mockSpawnSync).toHaveBeenNthCalledWith(1, '/usr/local/bin/maproom', ['scan'], { stdio: 'inherit' })
    expect(mockSpawnSync).toHaveBeenNthCalledWith(
      2,
      '/usr/local/bin/maproom',
      ['search', '--repo', 'myrepo', '--query', 'auth'],
      { stdio: 'inherit' },
    )
  })

  it('retries search after exit code 2 (post-flight fallback)', async () => {
    // Database file exists (pre-flight passes)
    mockExistsSync.mockReturnValue(true)
    // First search returns exit 2 (config error / repo not indexed)
    // Then scan succeeds, then retry search succeeds
    mockSpawnSync
      .mockReturnValueOnce(spawnResult(2)) // search -> exit 2
      .mockReturnValueOnce(spawnResult(0)) // scan
      .mockReturnValueOnce(spawnResult(0)) // retry search

    await runMaproomSearchWithAutoIndex(['--repo', 'myrepo', '--query', 'auth'])

    // Should have printed the auto-index message on the fallback path
    expect(consoleLogSpy).toHaveBeenCalledWith(
      'No index found. Building FTS index for this repo (no embedding provider required).',
    )
    // Three spawns: failed search, scan, retry search
    expect(mockSpawnSync).toHaveBeenCalledTimes(3)
  })

  it('reports error when pre-flight scan fails', async () => {
    mockExistsSync.mockReturnValue(false)
    // Scan fails
    mockSpawnSync.mockReturnValue(spawnResult(1))

    await runMaproomSearchWithAutoIndex(['--repo', 'myrepo', '--query', 'auth'])

    expect(consoleErrorSpy).toHaveBeenCalledWith(
      'Auto-index failed. Run "crewchief maproom scan" manually for details.',
    )
    expect(process.exitCode).toBe(1)
    // Should NOT have attempted search
    expect(mockSpawnSync).toHaveBeenCalledTimes(1) // only the scan
  })

  it('reports error when post-flight scan fails', async () => {
    mockExistsSync.mockReturnValue(true)
    // Search returns exit 2 (triggers fallback)
    mockSpawnSync
      .mockReturnValueOnce(spawnResult(2)) // search -> exit 2
      .mockReturnValueOnce(spawnResult(1)) // scan fails

    await runMaproomSearchWithAutoIndex(['--repo', 'myrepo', '--query', 'auth'])

    expect(consoleErrorSpy).toHaveBeenCalledWith(
      'Auto-index failed. Run "crewchief maproom scan" manually for details.',
    )
    expect(process.exitCode).toBe(1)
    // Should have search then scan but NOT a retry search
    expect(mockSpawnSync).toHaveBeenCalledTimes(2)
  })

  it('propagates non-zero non-2 exit code from search', async () => {
    mockExistsSync.mockReturnValue(true)
    // Search returns exit 1 (runtime error, not config error)
    mockSpawnSync.mockReturnValue(spawnResult(1))

    await runMaproomSearchWithAutoIndex(['--repo', 'myrepo', '--query', 'auth'])

    // Should NOT trigger auto-index (only exit 2 triggers it)
    expect(consoleLogSpy).not.toHaveBeenCalledWith(expect.stringContaining('No index found'))
    expect(process.exitCode).toBe(1)
    expect(mockSpawnSync).toHaveBeenCalledTimes(1)
  })
})
