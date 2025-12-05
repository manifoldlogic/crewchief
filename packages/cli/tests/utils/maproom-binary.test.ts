import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { findMaproomBinary } from '../../src/utils/maproom-binary.js'

// Mock modules
vi.mock('node:fs')
vi.mock('node:child_process')

describe('findMaproomBinary', () => {
  let originalPlatform: string
  let originalArch: string
  let originalEnv: NodeJS.ProcessEnv

  beforeEach(() => {
    // Save original values
    originalPlatform = process.platform
    originalArch = process.arch
    originalEnv = { ...process.env }

    // Reset mocks before each test
    vi.clearAllMocks()

    // Clean environment variables
    delete process.env.CREWCHIEF_MAPROOM_BIN

    // Mock fs.existsSync to return false by default
    vi.mocked(fs.existsSync).mockReturnValue(false)

    // Mock spawnSync to fail by default (no global binary)
    vi.mocked(spawnSync).mockReturnValue({
      status: 1,
      stdout: Buffer.from(''),
      stderr: Buffer.from(''),
      pid: 0,
      output: [],
      signal: null,
    })
  })

  afterEach(() => {
    // Restore original values
    Object.defineProperty(process, 'platform', {
      value: originalPlatform,
      writable: true,
      configurable: true,
    })
    Object.defineProperty(process, 'arch', {
      value: originalArch,
      writable: true,
      configurable: true,
    })
    process.env = originalEnv
  })

  // ===== PRECEDENCE TESTS =====

  it('prioritizes CREWCHIEF_MAPROOM_BIN env var over all others', () => {
    const envPath = '/custom/path/to/crewchief-maproom'
    process.env.CREWCHIEF_MAPROOM_BIN = envPath

    // Mock env path exists
    vi.mocked(fs.existsSync).mockImplementation((path) => {
      return path === envPath
    })

    // Mock config path and global also exist (but should be ignored)
    const configPath = '/config/path/crewchief-maproom'
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: Buffer.from(''),
      stderr: Buffer.from(''),
      pid: 0,
      output: [],
      signal: null,
    })

    const result = findMaproomBinary({ configPath })

    expect(result.path).toBe(envPath)
    expect(result.source).toBe('env')
  })

  it('uses config path when env var not set', () => {
    const configPath = '/config/path/crewchief-maproom'

    vi.mocked(fs.existsSync).mockImplementation((path) => {
      return path === configPath
    })

    const result = findMaproomBinary({ configPath })

    expect(result.path).toBe(configPath)
    expect(result.source).toBe('config')
  })

  it('uses global install when env var and config not set', () => {
    // Mock global binary found
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: Buffer.from('/usr/local/bin/crewchief-maproom'),
      stderr: Buffer.from(''),
      pid: 0,
      output: [],
      signal: null,
    })

    const result = findMaproomBinary()

    expect(result.path).toBe('crewchief-maproom')
    expect(result.source).toBe('global')
  })

  it('falls back to packaged binary when nothing else available', () => {
    // Set platform to linux-x64
    Object.defineProperty(process, 'platform', { value: 'linux', writable: true })
    Object.defineProperty(process, 'arch', { value: 'x64', writable: true })

    // Mock packaged binary exists
    vi.mocked(fs.existsSync).mockImplementation((path) => {
      // Match platform-specific path pattern
      return typeof path === 'string' && path.includes('bin/linux-x64/crewchief-maproom')
    })

    const result = findMaproomBinary()

    expect(result.path).toContain('bin/linux-x64/crewchief-maproom')
    expect(result.source).toBe('packaged')
  })

  it('returns not-found when no binary exists', () => {
    // All mocks return false/failure by default
    const result = findMaproomBinary()

    expect(result.path).toBeNull()
    expect(result.source).toBe('not-found')
  })

  it('returns correct source information for debugging', () => {
    const envPath = '/env/path/crewchief-maproom'
    process.env.CREWCHIEF_MAPROOM_BIN = envPath

    vi.mocked(fs.existsSync).mockImplementation((path) => {
      return path === envPath
    })

    const result = findMaproomBinary()

    expect(result).toEqual({
      path: envPath,
      source: 'env',
    })
    expect(result.source).toBe('env')
  })

  // ===== PLATFORM TESTS =====

  it('uses .exe suffix on Windows', () => {
    Object.defineProperty(process, 'platform', { value: 'win32', writable: true })
    Object.defineProperty(process, 'arch', { value: 'x64', writable: true })

    // Mock Windows packaged binary exists
    vi.mocked(fs.existsSync).mockImplementation((path) => {
      return typeof path === 'string' && path.includes('bin/win32-x64/crewchief-maproom.exe')
    })

    const result = findMaproomBinary()

    expect(result.path).toContain('crewchief-maproom.exe')
    expect(result.source).toBe('packaged')
  })

  it('uses no suffix on Unix', () => {
    Object.defineProperty(process, 'platform', { value: 'darwin', writable: true })
    Object.defineProperty(process, 'arch', { value: 'arm64', writable: true })

    // Mock macOS packaged binary exists
    vi.mocked(fs.existsSync).mockImplementation((path) => {
      return typeof path === 'string' && path.includes('bin/darwin-arm64/crewchief-maproom')
    })

    const result = findMaproomBinary()

    expect(result.path).toContain('crewchief-maproom')
    expect(result.path).not.toContain('.exe')
    expect(result.source).toBe('packaged')
  })

  // ===== PATH VALIDATION TESTS =====

  it('resolves relative config paths', () => {
    const configFileLocation = '/home/user/project/.crewchief/config.json'
    const configPath = './bin/crewchief-maproom'
    const expectedPath = '/home/user/project/.crewchief/bin/crewchief-maproom'

    vi.mocked(fs.existsSync).mockImplementation((path) => {
      return path === expectedPath
    })

    const result = findMaproomBinary({ configPath, configFileLocation })

    expect(result.path).toBe(expectedPath)
    expect(result.source).toBe('config')
  })

  it('handles absolute config paths', () => {
    const configPath = '/absolute/path/to/crewchief-maproom'

    vi.mocked(fs.existsSync).mockImplementation((path) => {
      return path === configPath
    })

    const result = findMaproomBinary({ configPath })

    expect(result.path).toBe(configPath)
    expect(result.source).toBe('config')
  })

  it('warns when config path does not exist', () => {
    const loggerWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})

    const configPath = '/nonexistent/path/crewchief-maproom'

    // Config path doesn't exist, but packaged binary does
    Object.defineProperty(process, 'platform', { value: 'linux', writable: true })
    Object.defineProperty(process, 'arch', { value: 'x64', writable: true })

    vi.mocked(fs.existsSync).mockImplementation((path) => {
      if (path === configPath) return false
      return typeof path === 'string' && path.includes('bin/linux-x64/crewchief-maproom')
    })

    const result = findMaproomBinary({ configPath })

    // Should warn about missing config path
    expect(loggerWarnSpy).toHaveBeenCalled()
    const warnCalls = loggerWarnSpy.mock.calls.flat().join(' ')
    expect(warnCalls).toContain('not found')
    expect(warnCalls).toContain(configPath)

    // Should fall through to packaged binary
    expect(result.source).toBe('packaged')

    loggerWarnSpy.mockRestore()
  })

  it('falls through to next priority when config path invalid', () => {
    const configPath = '/invalid/path/crewchief-maproom'

    // Config path doesn't exist
    vi.mocked(fs.existsSync).mockImplementation((path) => {
      return path !== configPath
    })

    // Global binary exists
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: Buffer.from('/usr/local/bin/crewchief-maproom'),
      stderr: Buffer.from(''),
      pid: 0,
      output: [],
      signal: null,
    })

    const result = findMaproomBinary({ configPath })

    // Should fall through to global
    expect(result.path).toBe('crewchief-maproom')
    expect(result.source).toBe('global')
  })

  // ===== EDGE CASES =====

  it('handles missing process.env.CREWCHIEF_MAPROOM_BIN gracefully', () => {
    delete process.env.CREWCHIEF_MAPROOM_BIN

    // Global binary exists
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: Buffer.from('/usr/local/bin/crewchief-maproom'),
      stderr: Buffer.from(''),
      pid: 0,
      output: [],
      signal: null,
    })

    const result = findMaproomBinary()

    // Should skip env check and use global
    expect(result.path).toBe('crewchief-maproom')
    expect(result.source).toBe('global')
  })

  it('handles undefined options parameter', () => {
    // Global binary exists
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: Buffer.from('/usr/local/bin/crewchief-maproom'),
      stderr: Buffer.from(''),
      pid: 0,
      output: [],
      signal: null,
    })

    const result = findMaproomBinary(undefined)

    expect(result.path).toBe('crewchief-maproom')
    expect(result.source).toBe('global')
  })

  it('handles empty string config path', () => {
    const configPath = ''

    // Global binary exists
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: Buffer.from('/usr/local/bin/crewchief-maproom'),
      stderr: Buffer.from(''),
      pid: 0,
      output: [],
      signal: null,
    })

    const result = findMaproomBinary({ configPath })

    // Empty string is falsy, should skip to global
    expect(result.path).toBe('crewchief-maproom')
    expect(result.source).toBe('global')
  })

  // ===== ADDITIONAL EDGE CASES =====

  it('handles env var that does not exist on filesystem', () => {
    process.env.CREWCHIEF_MAPROOM_BIN = '/nonexistent/env/path'

    // Env path doesn't exist
    vi.mocked(fs.existsSync).mockReturnValue(false)

    // Global binary exists
    vi.mocked(spawnSync).mockReturnValue({
      status: 0,
      stdout: Buffer.from('/usr/local/bin/crewchief-maproom'),
      stderr: Buffer.from(''),
      pid: 0,
      output: [],
      signal: null,
    })

    const result = findMaproomBinary()

    // Should fall through to global
    expect(result.path).toBe('crewchief-maproom')
    expect(result.source).toBe('global')
  })

  it('resolves packaged binary from bin root for backwards compatibility', () => {
    Object.defineProperty(process, 'platform', { value: 'linux', writable: true })
    Object.defineProperty(process, 'arch', { value: 'x64', writable: true })

    // Platform-specific path doesn't exist, but bin root does
    vi.mocked(fs.existsSync).mockImplementation((path) => {
      if (typeof path !== 'string') return false
      // Platform path doesn't exist
      if (path.includes('bin/linux-x64/')) return false
      // Root bin path exists
      if (path.includes('bin/crewchief-maproom') && !path.includes('linux-x64')) return true
      return false
    })

    const result = findMaproomBinary()

    expect(result.path).toContain('bin/crewchief-maproom')
    expect(result.path).not.toContain('linux-x64')
    expect(result.source).toBe('packaged')
  })

  it('checks monorepo sibling package when platform paths not found', () => {
    Object.defineProperty(process, 'platform', { value: 'linux', writable: true })
    Object.defineProperty(process, 'arch', { value: 'x64', writable: true })

    // Platform path and bin root don't exist, but sibling maproom-mcp does
    vi.mocked(fs.existsSync).mockImplementation((path) => {
      if (typeof path !== 'string') return false
      return path.includes('maproom-mcp/bin/linux-x64/crewchief-maproom')
    })

    const result = findMaproomBinary()

    expect(result.path).toContain('maproom-mcp/bin/linux-x64/crewchief-maproom')
    expect(result.source).toBe('packaged')
  })

  it('handles different architectures correctly', () => {
    Object.defineProperty(process, 'platform', { value: 'darwin', writable: true })
    Object.defineProperty(process, 'arch', { value: 'arm64', writable: true })

    vi.mocked(fs.existsSync).mockImplementation((path) => {
      return typeof path === 'string' && path.includes('bin/darwin-arm64/crewchief-maproom')
    })

    const result = findMaproomBinary()

    expect(result.path).toContain('darwin-arm64')
    expect(result.source).toBe('packaged')
  })

  it('handles errors during packaged binary resolution gracefully', () => {
    // Mock fs.existsSync to throw an error
    vi.mocked(fs.existsSync).mockImplementation(() => {
      throw new Error('File system error')
    })

    const result = findMaproomBinary()

    // Should return not-found instead of throwing
    expect(result.path).toBeNull()
    expect(result.source).toBe('not-found')
  })
})
