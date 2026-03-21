import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createFileChangeHandler } from '../fileChangeHandler.js'
import type { AgentMessage } from '../message.types.js'

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

// Capture spawn calls
const spawnMock = vi.fn().mockReturnValue({
  on: vi.fn(),
  unref: vi.fn(),
})

vi.mock('node:child_process', () => ({
  spawn: (...args: unknown[]) => spawnMock(...args),
}))

vi.mock('../../utils/maproom-binary.js', () => ({
  findMaproomBinary: vi.fn().mockReturnValue({ path: '/usr/local/bin/maproom', source: 'global' }),
}))

vi.mock('../../utils/logger.js', () => ({
  logger: {
    warn: vi.fn(),
    info: vi.fn(),
    debug: vi.fn(),
  },
}))

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeFileChangeMessage(files: Array<{ path: string; status: 'added' | 'modified' | 'deleted' }>): AgentMessage {
  return {
    type: 'file-change',
    from: 'test-agent',
    to: 'orchestrator',
    payload: { files },
    timestamp: new Date(),
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('createFileChangeHandler', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('calls maproom upsert with modified file paths', () => {
    const handler = createFileChangeHandler('/workspace/repos/my-project')

    handler(
      makeFileChangeMessage([
        { path: 'src/index.ts', status: 'modified' },
        { path: 'src/utils.ts', status: 'added' },
      ]),
    )

    expect(spawnMock).toHaveBeenCalledTimes(1)
    expect(spawnMock).toHaveBeenCalledWith(
      '/usr/local/bin/maproom',
      ['upsert', 'src/index.ts', 'src/utils.ts'],
      expect.objectContaining({
        cwd: '/workspace/repos/my-project',
        stdio: 'ignore',
        detached: true,
      }),
    )
  })

  it('excludes deleted files from upsert', () => {
    const handler = createFileChangeHandler('/workspace/repos/my-project')

    handler(
      makeFileChangeMessage([
        { path: 'src/index.ts', status: 'modified' },
        { path: 'src/removed.ts', status: 'deleted' },
      ]),
    )

    expect(spawnMock).toHaveBeenCalledTimes(1)
    const args = spawnMock.mock.calls[0][1] as string[]
    expect(args).toContain('src/index.ts')
    expect(args).not.toContain('src/removed.ts')
  })

  it('does not spawn when all files are deleted', () => {
    const handler = createFileChangeHandler('/workspace/repos/my-project')

    handler(makeFileChangeMessage([{ path: 'src/removed.ts', status: 'deleted' }]))

    expect(spawnMock).not.toHaveBeenCalled()
  })

  it('ignores non-file-change messages', () => {
    const handler = createFileChangeHandler('/workspace/repos/my-project')

    handler({
      type: 'status',
      from: 'test-agent',
      to: 'orchestrator',
      payload: { activity: 'working' },
      timestamp: new Date(),
    })

    expect(spawnMock).not.toHaveBeenCalled()
  })

  it('ignores file-change messages with empty file list', () => {
    const handler = createFileChangeHandler('/workspace/repos/my-project')

    handler(makeFileChangeMessage([]))

    expect(spawnMock).not.toHaveBeenCalled()
  })

  it('does not throw when maproom binary is not found', async () => {
    const { findMaproomBinary } = await import('../../utils/maproom-binary.js')
    vi.mocked(findMaproomBinary).mockReturnValue({ path: null, source: 'not-found' })

    const handler = createFileChangeHandler('/workspace/repos/my-project')

    // Should not throw
    expect(() => handler(makeFileChangeMessage([{ path: 'src/index.ts', status: 'modified' }]))).not.toThrow()

    expect(spawnMock).not.toHaveBeenCalled()
  })

  it('does not throw when spawn fails', () => {
    spawnMock.mockImplementationOnce(() => {
      throw new Error('spawn ENOENT')
    })

    const handler = createFileChangeHandler('/workspace/repos/my-project')

    // Should not throw (fire-and-forget)
    expect(() => handler(makeFileChangeMessage([{ path: 'src/index.ts', status: 'modified' }]))).not.toThrow()
  })
})
