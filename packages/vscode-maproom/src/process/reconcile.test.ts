/**
 * Tests for startup reconciliation
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { EventEmitter } from 'node:events'
import { Readable } from 'node:stream'

// Mock vscode module
vi.mock('vscode', () => ({
  workspace: {
    workspaceFolders: [{ uri: { fsPath: '/test/workspace' } }],
  },
}))

// Mock child_process
vi.mock('node:child_process', async (importOriginal) => {
  const actual = (await importOriginal()) as any
  return {
    ...actual,
    execFile: vi.fn(),
    spawn: vi.fn(),
  }
})

// Mock platform utilities
vi.mock('../utils/platform', () => ({
  detectPlatform: vi.fn().mockReturnValue('linux-x64'),
  getBinaryExtension: vi.fn().mockReturnValue(''),
}))

// Mock git utilities
vi.mock('../utils/git', () => ({
  getRepoName: vi.fn().mockResolvedValue('test-owner/test-repo'),
  getBranchName: vi.fn().mockResolvedValue('main'),
}))

// Import after mocks are set up
const { reconcileChanges, updateLastIndexedCommit } = await import('./reconcile')

/**
 * Mock ExtensionContext
 */
class MockExtensionContext {
  private workspaceStateData = new Map<string, any>()

  workspaceState = {
    get: <T>(key: string): T | undefined => this.workspaceStateData.get(key),
    update: async (key: string, value: any) => {
      this.workspaceStateData.set(key, value)
    },
  }

  extensionPath = '/test/extension'
}

/**
 * Mock child process
 */
class MockChildProcess extends EventEmitter {
  stdout: Readable
  stderr: Readable
  exitCode: number | null = null

  constructor() {
    super()
    this.stdout = new Readable({ read() {} })
    this.stderr = new Readable({ read() {} })
  }

  simulateSuccess(): void {
    setTimeout(() => {
      this.exitCode = 0
      this.emit('close', 0)
    }, 10)
  }

  simulateFailure(code: number, errorMsg?: string): void {
    setTimeout(() => {
      if (errorMsg) {
        this.stderr.push(errorMsg)
        this.stderr.push(null)
      }
      this.exitCode = code
      this.emit('close', code)
    }, 10)
  }
}

describe('reconcileChanges', () => {
  let context: MockExtensionContext
  let execFileMock: ReturnType<typeof vi.fn>
  let spawnMock: ReturnType<typeof vi.fn>

  beforeEach(async () => {
    context = new MockExtensionContext()

    const childProcess = await import('node:child_process')
    execFileMock = vi.mocked(childProcess.execFile)
    spawnMock = vi.mocked(childProcess.spawn)

    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('first run behavior', () => {
    it('should skip reconciliation on first run and store current commit', async () => {
      // No last commit stored
      // Mock git rev-parse HEAD
      execFileMock.mockImplementation((cmd: string, args: string[], opts: any, cb?: any) => {
        const callback = cb || opts
        if (args[0] === 'rev-parse' && args[1] === 'HEAD') {
          callback(null, { stdout: 'abc123\n', stderr: '' })
        }
        return {} as any
      })

      const result = await reconcileChanges(context as any, {
        extensionRoot: '/test/extension',
        databaseUrl: 'sqlite:///test/db',
      })

      expect(result.performed).toBe(false)
      expect(result.filesReconciled).toBe(0)
      expect(result.currentCommit).toBe('abc123')

      // Should have stored the commit
      expect(context.workspaceState.get('maproom.lastIndexedCommit')).toBe('abc123')
    })
  })

  describe('no changes scenario', () => {
    it('should skip when last commit equals current HEAD', async () => {
      // Set last commit same as current
      await context.workspaceState.update('maproom.lastIndexedCommit', 'abc123')

      execFileMock.mockImplementation((cmd: string, args: string[], opts: any, cb?: any) => {
        const callback = cb || opts
        if (args[0] === 'rev-parse' && args[1] === 'HEAD') {
          callback(null, { stdout: 'abc123\n', stderr: '' })
        }
        return {} as any
      })

      const result = await reconcileChanges(context as any, {
        extensionRoot: '/test/extension',
        databaseUrl: 'sqlite:///test/db',
      })

      expect(result.performed).toBe(false)
      expect(result.filesReconciled).toBe(0)
      expect(result.previousCommit).toBe('abc123')
      expect(result.currentCommit).toBe('abc123')
    })

    it('should update commit when git diff returns no files', async () => {
      await context.workspaceState.update('maproom.lastIndexedCommit', 'old123')

      execFileMock.mockImplementation((cmd: string, args: string[], opts: any, cb?: any) => {
        const callback = cb || opts
        if (args[0] === 'rev-parse' && args[1] === 'HEAD') {
          callback(null, { stdout: 'new456\n', stderr: '' })
        } else if (args[0] === 'diff' && args[1] === '--name-only') {
          callback(null, { stdout: '', stderr: '' }) // No changed files
        }
        return {} as any
      })

      const result = await reconcileChanges(context as any, {
        extensionRoot: '/test/extension',
        databaseUrl: 'sqlite:///test/db',
      })

      expect(result.performed).toBe(false)
      expect(result.filesReconciled).toBe(0)
      expect(context.workspaceState.get('maproom.lastIndexedCommit')).toBe('new456')
    })
  })

  describe('reconciliation with changes', () => {
    it('should run upsert for changed files', async () => {
      await context.workspaceState.update('maproom.lastIndexedCommit', 'old123')

      const mockProcess = new MockChildProcess()

      execFileMock.mockImplementation((cmd: string, args: string[], opts: any, cb?: any) => {
        const callback = cb || opts
        if (args[0] === 'rev-parse' && args[1] === 'HEAD') {
          callback(null, { stdout: 'new456\n', stderr: '' })
        } else if (args[0] === 'diff' && args[1] === '--name-only') {
          callback(null, { stdout: 'src/file1.ts\nsrc/file2.ts\n', stderr: '' })
        }
        return {} as any
      })

      spawnMock.mockImplementation(() => {
        mockProcess.simulateSuccess()
        return mockProcess as any
      })

      const result = await reconcileChanges(context as any, {
        extensionRoot: '/test/extension',
        databaseUrl: 'sqlite:///test/db',
      })

      expect(result.performed).toBe(true)
      expect(result.filesReconciled).toBe(2)
      expect(result.previousCommit).toBe('old123')
      expect(result.currentCommit).toBe('new456')

      // Verify upsert was called with correct arguments
      expect(spawnMock).toHaveBeenCalledWith(
        expect.stringContaining('maproom'),
        expect.arrayContaining([
          'upsert',
          '--commit', 'new456',
          '--repo', 'test-owner/test-repo',
          '--worktree', 'main',
          '--root', '/test/workspace',
          '--paths', 'src/file1.ts,src/file2.ts',
        ]),
        expect.objectContaining({
          env: expect.objectContaining({
            MAPROOM_DATABASE_URL: 'sqlite:///test/db',
          }),
        })
      )

      // Commit should be updated
      expect(context.workspaceState.get('maproom.lastIndexedCommit')).toBe('new456')
    })

    it('should call progress callback', async () => {
      await context.workspaceState.update('maproom.lastIndexedCommit', 'old123')

      const mockProcess = new MockChildProcess()
      const progressMessages: string[] = []

      execFileMock.mockImplementation((cmd: string, args: string[], opts: any, cb?: any) => {
        const callback = cb || opts
        if (args[0] === 'rev-parse' && args[1] === 'HEAD') {
          callback(null, { stdout: 'new456\n', stderr: '' })
        } else if (args[0] === 'diff' && args[1] === '--name-only') {
          callback(null, { stdout: 'src/changed.ts\n', stderr: '' })
        }
        return {} as any
      })

      spawnMock.mockImplementation(() => {
        mockProcess.simulateSuccess()
        return mockProcess as any
      })

      await reconcileChanges(context as any, {
        extensionRoot: '/test/extension',
        databaseUrl: 'sqlite:///test/db',
        onProgress: (msg) => progressMessages.push(msg),
      })

      expect(progressMessages).toContain('Finding changed files...')
      expect(progressMessages.some(m => m.includes('Reconciling'))).toBe(true)
      expect(progressMessages.some(m => m.includes('1 files'))).toBe(true)
    })
  })

  describe('error handling', () => {
    it('should return error result when git commands fail', async () => {
      await context.workspaceState.update('maproom.lastIndexedCommit', 'old123')

      execFileMock.mockImplementation((cmd: string, args: string[], opts: any, cb?: any) => {
        const callback = cb || opts
        callback(new Error('Not a git repository'), null)
        return {} as any
      })

      const result = await reconcileChanges(context as any, {
        extensionRoot: '/test/extension',
        databaseUrl: 'sqlite:///test/db',
      })

      expect(result.performed).toBe(false)
      expect(result.error).toBeDefined()
    })

    it('should handle upsert failure gracefully', async () => {
      await context.workspaceState.update('maproom.lastIndexedCommit', 'old123')

      const mockProcess = new MockChildProcess()

      execFileMock.mockImplementation((cmd: string, args: string[], opts: any, cb?: any) => {
        const callback = cb || opts
        if (args[0] === 'rev-parse' && args[1] === 'HEAD') {
          callback(null, { stdout: 'new456\n', stderr: '' })
        } else if (args[0] === 'diff' && args[1] === '--name-only') {
          callback(null, { stdout: 'src/file.ts\n', stderr: '' })
        }
        return {} as any
      })

      spawnMock.mockImplementation(() => {
        mockProcess.simulateFailure(1, 'Database error')
        return mockProcess as any
      })

      const result = await reconcileChanges(context as any, {
        extensionRoot: '/test/extension',
        databaseUrl: 'sqlite:///test/db',
      })

      expect(result.performed).toBe(false)
      expect(result.error).toBeDefined()
      expect(result.error).toContain('upsert exited with code 1')
    })
  })
})

describe('updateLastIndexedCommit', () => {
  let context: MockExtensionContext
  let execFileMock: ReturnType<typeof vi.fn>

  beforeEach(async () => {
    context = new MockExtensionContext()

    const childProcess = await import('node:child_process')
    execFileMock = vi.mocked(childProcess.execFile)

    vi.clearAllMocks()
  })

  it('should update the stored commit to current HEAD', async () => {
    execFileMock.mockImplementation((cmd: string, args: string[], opts: any, cb?: any) => {
      const callback = cb || opts
      callback(null, { stdout: 'commit789\n', stderr: '' })
      return {} as any
    })

    await updateLastIndexedCommit(context as any, '/test/workspace')

    expect(context.workspaceState.get('maproom.lastIndexedCommit')).toBe('commit789')
  })

  it('should not update if git command fails', async () => {
    await context.workspaceState.update('maproom.lastIndexedCommit', 'old123')

    execFileMock.mockImplementation((cmd: string, args: string[], opts: any, cb?: any) => {
      const callback = cb || opts
      callback(new Error('Git error'), null)
      return {} as any
    })

    await updateLastIndexedCommit(context as any, '/test/workspace')

    // Should still have old value
    expect(context.workspaceState.get('maproom.lastIndexedCommit')).toBe('old123')
  })
})
