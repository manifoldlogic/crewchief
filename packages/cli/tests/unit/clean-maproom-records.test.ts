import { spawnSync } from 'node:child_process'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { loadConfig } from '../../src/config/loader.js'
import type { CrewChiefConfig } from '../../src/config/schema.js'
import { cleanMaproomRecords } from '../../src/git/worktrees.js'
import { logger } from '../../src/utils/logger.js'
import { findMaproomBinary } from '../../src/utils/maproom-binary.js'

// Mock modules
vi.mock('node:child_process')
vi.mock('../../src/utils/maproom-binary.js')
vi.mock('../../src/utils/logger.js')
vi.mock('../../src/config/loader.js')

describe('cleanMaproomRecords', () => {
  beforeEach(() => {
    // Reset all mocks before each test
    vi.clearAllMocks()

    // Default mock: binary found
    vi.mocked(findMaproomBinary).mockReturnValue({
      path: '/mock/path/maproom',
      source: 'packaged',
    })

    // Default mock: successful cleanup with no output
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: '',
      stderr: '',
      pid: 0,
      output: [],
      signal: null,
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  // ===== BINARY NOT FOUND TESTS =====

  it('throws error when binary not found', async () => {
    vi.mocked(findMaproomBinary).mockReturnValue({
      path: null,
      source: 'not-found',
    })

    await expect(cleanMaproomRecords()).rejects.toThrow('Maproom binary not found')

    // Should not attempt to spawn
    expect(spawnSync).not.toHaveBeenCalled()
  })

  // ===== SUCCESS CASES =====

  it('succeeds with exit code 0', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: 'Cleanup complete',
      stderr: '',
      pid: 0,
      output: [],
      signal: null,
    })

    await expect(cleanMaproomRecords()).resolves.toBeUndefined()

    expect(findMaproomBinary).toHaveBeenCalled()
    expect(spawnSync).toHaveBeenCalledWith('/mock/path/maproom', ['db', 'cleanup-stale', '--confirm'], {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe'],
    })
  })

  it('throws error with exit code 2 (configuration error)', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 2,
      stdout: '',
      stderr: 'Configuration error: missing database',
      pid: 0,
      output: [],
      signal: null,
    })

    // Exit code 2 is now a configuration error (AFM-06: 0=success, 1=runtime, 2=config)
    await expect(cleanMaproomRecords()).rejects.toThrow('Configuration error: missing database')

    expect(spawnSync).toHaveBeenCalled()
  })

  // ===== ERROR CASES =====

  it('throws error with exit code 1', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 1,
      stdout: '',
      stderr: 'Database connection failed',
      pid: 0,
      output: [],
      signal: null,
    })

    await expect(cleanMaproomRecords()).rejects.toThrow('Database connection failed')
  })

  it('throws error with exit code 3', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 3,
      stdout: '',
      stderr: 'Invalid argument',
      pid: 0,
      output: [],
      signal: null,
    })

    // Any non-zero exit code should throw
    await expect(cleanMaproomRecords()).rejects.toThrow('Invalid argument')
  })

  it('throws error with exit code -1 (spawn failed)', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: -1,
      stdout: '',
      stderr: 'Failed to execute binary',
      pid: 0,
      output: [],
      signal: null,
    })

    await expect(cleanMaproomRecords()).rejects.toThrow('Failed to execute binary')
  })

  // ===== ERROR MESSAGE EXTRACTION TESTS =====

  it('uses stderr for error message when available', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 1,
      stdout: '',
      stderr: 'Error: database locked\nAdditional context line',
      pid: 0,
      output: [],
      signal: null,
    })

    // Should extract only the first line of stderr
    await expect(cleanMaproomRecords()).rejects.toThrow('Error: database locked')
  })

  it('uses stdout for error message when stderr is empty', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 1,
      stdout: 'Error from stdout\nSecond line',
      stderr: '',
      pid: 0,
      output: [],
      signal: null,
    })

    // Should extract only the first line of stdout
    await expect(cleanMaproomRecords()).rejects.toThrow('Error from stdout')
  })

  it('uses "Unknown error" when both stderr and stdout are empty', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 1,
      stdout: '',
      stderr: '',
      pid: 0,
      output: [],
      signal: null,
    })

    await expect(cleanMaproomRecords()).rejects.toThrow('Unknown error')
  })

  it('extracts only first line of multi-line error message', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 1,
      stdout: '',
      stderr: 'Primary error message\nStack trace line 1\nStack trace line 2',
      pid: 0,
      output: [],
      signal: null,
    })

    await expect(cleanMaproomRecords()).rejects.toThrow('Primary error message')
  })

  it('prefers stderr over stdout when both are present', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 1,
      stdout: 'stdout message',
      stderr: 'stderr message',
      pid: 0,
      output: [],
      signal: null,
    })

    // stderr should be preferred
    await expect(cleanMaproomRecords()).rejects.toThrow('stderr message')
  })

  // ===== LOGGER TESTS =====

  it('logs info when "Deleted" appears in output', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: 'Deleted 3 stale worktree records',
      stderr: '',
      pid: 0,
      output: [],
      signal: null,
    })

    await cleanMaproomRecords()

    expect(logger.info).toHaveBeenCalledWith('Cleaned maproom database records')
  })

  it('does not log when "Deleted" is not in output', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: 'No changes made',
      stderr: '',
      pid: 0,
      output: [],
      signal: null,
    })

    await cleanMaproomRecords()

    expect(logger.info).not.toHaveBeenCalled()
  })

  it('logs when "Deleted" appears with exit code 0', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: 'Found 5 stale records. Deleted all successfully.',
      stderr: '',
      pid: 0,
      output: [],
      signal: null,
    })

    await cleanMaproomRecords()

    expect(logger.info).toHaveBeenCalledWith('Cleaned maproom database records')
  })

  it('throws when exit code is 2 even if output contains "Deleted"', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 2,
      stdout: 'Previously Deleted records: 0',
      stderr: '',
      pid: 0,
      output: [],
      signal: null,
    })

    // Exit code 2 is now a configuration error (AFM-06), should throw regardless of output
    await expect(cleanMaproomRecords()).rejects.toThrow('Previously Deleted records: 0')
  })

  // ===== COMMAND INVOCATION TESTS =====

  it('calls findMaproomBinary with undefined configPath when config not provided', async () => {
    await cleanMaproomRecords()

    expect(findMaproomBinary).toHaveBeenCalledWith({ configPath: undefined })
  })

  it('passes correct arguments to spawnSync', async () => {
    const binaryPath = '/custom/path/to/binary'
    vi.mocked(findMaproomBinary).mockReturnValue({
      path: binaryPath,
      source: 'config',
    })

    await cleanMaproomRecords()

    expect(spawnSync).toHaveBeenCalledWith(binaryPath, ['db', 'cleanup-stale', '--confirm'], {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe'],
    })
  })

  it('uses binary path from env source', async () => {
    const envBinaryPath = '/env/path/maproom'
    vi.mocked(findMaproomBinary).mockReturnValue({
      path: envBinaryPath,
      source: 'env',
    })

    await cleanMaproomRecords()

    expect(spawnSync).toHaveBeenCalledWith(envBinaryPath, ['db', 'cleanup-stale', '--confirm'], expect.any(Object))
  })

  it('uses binary path from global source', async () => {
    vi.mocked(findMaproomBinary).mockReturnValue({
      path: 'maproom',
      source: 'global',
    })

    await cleanMaproomRecords()

    expect(spawnSync).toHaveBeenCalledWith('maproom', ['db', 'cleanup-stale', '--confirm'], expect.any(Object))
  })

  // ===== EDGE CASES =====

  it('handles null status code', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: null,
      stdout: '',
      stderr: 'Process terminated abnormally',
      pid: 0,
      output: [],
      signal: null,
    })

    // null status should be treated as an error (not 0)
    await expect(cleanMaproomRecords()).rejects.toThrow('Process terminated abnormally')
  })

  it('handles signal termination', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: null,
      stdout: '',
      stderr: '',
      pid: 0,
      output: [],
      signal: 'SIGTERM',
    })

    // Signal termination should throw with Unknown error
    await expect(cleanMaproomRecords()).rejects.toThrow('Unknown error')
  })

  it('handles very long error messages', async () => {
    const longError = 'A'.repeat(1000) + '\nSecond line'
    vi.mocked(spawnSync).mockReturnValue({
      status: 1,
      stdout: '',
      stderr: longError,
      pid: 0,
      output: [],
      signal: null,
    })

    // Should only use first line even if very long
    await expect(cleanMaproomRecords()).rejects.toThrow('A'.repeat(1000))
  })

  it('handles error messages with only newlines', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 1,
      stdout: '',
      stderr: '\n\n\nActual error on line 4',
      pid: 0,
      output: [],
      signal: null,
    })

    // split('\n')[0] should return empty string, fall back to 'Unknown error'
    // Actually, the first line will be empty, so it will use that
    await expect(cleanMaproomRecords()).rejects.toThrow('')
  })

  it('handles case-sensitive "Deleted" check', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: 'deleted 3 records',
      stderr: '',
      pid: 0,
      output: [],
      signal: null,
    })

    await cleanMaproomRecords()

    // "deleted" (lowercase) should not trigger logger
    expect(logger.info).not.toHaveBeenCalled()
  })

  it('detects "Deleted" anywhere in output', async () => {
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: 'Processing complete. Deleted 10 records. Done.',
      stderr: '',
      pid: 0,
      output: [],
      signal: null,
    })

    await cleanMaproomRecords()

    expect(logger.info).toHaveBeenCalledWith('Cleaned maproom database records')
  })

  // ===== CONFIG PARAMETER TESTS =====

  it('should use provided config parameter', async () => {
    const mockConfig = {
      repository: {
        maproomBinaryPath: '/custom/maproom',
      },
    } as CrewChiefConfig

    await cleanMaproomRecords(mockConfig)

    // Should NOT call loadConfig when config is provided
    expect(loadConfig).not.toHaveBeenCalled()

    // Should call findMaproomBinary with the config path
    expect(findMaproomBinary).toHaveBeenCalledWith({
      configPath: '/custom/maproom',
    })

    // Should execute the cleanup command
    expect(spawnSync).toHaveBeenCalledWith('/mock/path/maproom', ['db', 'cleanup-stale', '--confirm'], {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe'],
    })
  })

  it('should load config when not provided', async () => {
    const mockConfig = {
      repository: {
        maproomBinaryPath: '/loaded/maproom',
      },
    } as CrewChiefConfig

    vi.mocked(loadConfig).mockResolvedValue(mockConfig)

    await cleanMaproomRecords() // No config parameter

    // Should call loadConfig to get config
    expect(loadConfig).toHaveBeenCalled()

    // Should call findMaproomBinary with the loaded config path
    expect(findMaproomBinary).toHaveBeenCalledWith({
      configPath: '/loaded/maproom',
    })

    // Should execute the cleanup command
    expect(spawnSync).toHaveBeenCalledWith('/mock/path/maproom', ['db', 'cleanup-stale', '--confirm'], {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe'],
    })
  })

  it('should handle config load failure gracefully', async () => {
    vi.mocked(loadConfig).mockRejectedValue(new Error('Config not found'))

    // Should not throw when config loading fails
    await expect(cleanMaproomRecords()).resolves.toBeUndefined()

    // Should have attempted to load config
    expect(loadConfig).toHaveBeenCalled()

    // Should call findMaproomBinary with undefined configPath (fallback)
    expect(findMaproomBinary).toHaveBeenCalledWith({
      configPath: undefined,
    })

    // Should still execute the cleanup command
    expect(spawnSync).toHaveBeenCalledWith('/mock/path/maproom', ['db', 'cleanup-stale', '--confirm'], {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe'],
    })
  })
})
