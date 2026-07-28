import type { SpawnSyncReturns } from 'node:child_process'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { runMaproomSearch } from '../maproom.js'

// ---- Mocks ----

// Mock node:child_process
const mockSpawnSync = vi.fn<[string, string[], unknown], SpawnSyncReturns<Buffer>>()
vi.mock('node:child_process', () => ({
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  spawnSync: (...args: any[]) => mockSpawnSync(...(args as [string, string[], unknown])),
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

describe('runMaproomSearch (passthrough)', () => {
  beforeEach(() => {
    mockSpawnSync.mockReset()
    process.exitCode = undefined
  })

  afterEach(() => {
    process.exitCode = undefined
  })

  it('spawns exactly one process (search) for a successful query', async () => {
    mockSpawnSync.mockReturnValue(spawnResult(0))

    await runMaproomSearch(['--repo', 'myrepo', '--query', 'auth'])

    expect(mockSpawnSync).toHaveBeenCalledTimes(1)
    expect(mockSpawnSync).toHaveBeenCalledWith(
      '/usr/local/bin/maproom',
      ['search', '--repo', 'myrepo', '--query', 'auth'],
      { stdio: 'inherit' },
    )
  })

  // Regression: the old auto-index path would spawn maproom scan before or after search.
  // Under the shared-PG backend there is no local SQLite file, so that path always
  // fired a spurious scan. Verify it is gone: search NEVER spawns scan regardless of
  // exit code.
  it('regression: never spawns a scan process, even on non-zero exit', async () => {
    // Simulate various failure exit codes (1, 2 = the old post-flight trigger, 127, etc.)
    for (const code of [1, 2, 127]) {
      mockSpawnSync.mockReset()
      process.exitCode = undefined
      mockSpawnSync.mockReturnValue(spawnResult(code))

      await runMaproomSearch(['auth'])

      // Exactly one spawn, and its args must start with 'search', never 'scan'
      expect(mockSpawnSync).toHaveBeenCalledTimes(1)
      const [, spawnArgs] = mockSpawnSync.mock.calls[0] as unknown as [string, string[], unknown]
      expect(spawnArgs[0]).toBe('search')
      expect(spawnArgs).not.toContain('scan')
    }
  })

  it('propagates non-zero exit code from search', async () => {
    mockSpawnSync.mockReturnValue(spawnResult(1))

    await runMaproomSearch(['auth'])

    expect(process.exitCode).toBe(1)
    expect(mockSpawnSync).toHaveBeenCalledTimes(1)
  })

  it('does not set exitCode on success', async () => {
    mockSpawnSync.mockReturnValue(spawnResult(0))

    await runMaproomSearch(['auth'])

    expect(process.exitCode).toBeUndefined()
  })

  it('passes through arbitrary extra flags to maproom search', async () => {
    mockSpawnSync.mockReturnValue(spawnResult(0))

    await runMaproomSearch(['auth flow', '--limit', '10', '--format', 'agent'])

    expect(mockSpawnSync).toHaveBeenCalledWith(
      '/usr/local/bin/maproom',
      ['search', 'auth flow', '--limit', '10', '--format', 'agent'],
      { stdio: 'inherit' },
    )
  })

  it('propagates exit code 2 without triggering a scan (PG error passthrough)', async () => {
    mockSpawnSync.mockReturnValue(spawnResult(2))

    await runMaproomSearch(['auth'])

    // Only one spawn (search), exit code 2 propagated
    expect(mockSpawnSync).toHaveBeenCalledTimes(1)
    expect(process.exitCode).toBe(2)
  })
})
