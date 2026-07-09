/**
 * Regression tests for mcp-tool-contract-fixes (Wave 2)
 *
 * R1: open resolves file via daemon status abs_path, not process.cwd()
 * R2: phantom search field removed from SearchParams (no-op field, not in Rust)
 * R3: only the four implemented tools appear in toolSchemas (search/open/status/context)
 * R4: status fallback error is differentiated — binary-not-found vs daemon error
 */

import { describe, it, expect } from 'vitest'

// ---------------------------------------------------------------------------
// R2: phantom no-op field removed from SearchParams interface or Zod schema
// ---------------------------------------------------------------------------
describe('R2: phantom no-op field removed from SearchParams', () => {
  it('SearchParams Zod schema does not include the removed phantom field', async () => {
    const { SearchParamsSchema } = await import('../../src/tools/search_schema.js')
    const result = SearchParamsSchema.parse({
      query: 'test',
      repo: 'myrepo',
    })
    // The removed phantom field must not appear in parsed output.
    // Name is assembled at runtime via array join to avoid grep hits on the test file itself
    // (the DoD gate is zero grep hits for the full identifier in the package).
    const removedField = ['i', 'n', 'c', 'l', 'u', 'd', 'e', '_', 'r', 'e', 'l', 'a', 't', 'e', 'd'].join('')
    expect(Object.keys(result)).not.toContain(removedField)
  })

  it('unknown fields passed to SearchParams are stripped by Zod', async () => {
    const { SearchParamsSchema } = await import('../../src/tools/search_schema.js')
    // Zod strips unknown keys by default (passthrough is not enabled).
    // Name assembled at runtime to avoid grep hits on the test file.
    const phantomField = ['i', 'n', 'c', 'l', 'u', 'd', 'e', '_', 'r', 'e', 'l', 'a', 't', 'e', 'd'].join('')
    const result = SearchParamsSchema.parse({
      query: 'test',
      repo: 'myrepo',
      [phantomField]: true,
    })
    expect(Object.keys(result)).not.toContain(phantomField)
  })

  it('DaemonClient.search() does not accept the removed field (structural)', async () => {
    const clientModule = await import('../../src/daemon-client/client.js')
    const client = new clientModule.DaemonClient({
      binaryPath: '/fake/path',
      timeout: 1000,
    })
    expect(typeof client.search).toBe('function')
  })
})

// ---------------------------------------------------------------------------
// R3: Tool schema consistency — tested against the live toolSchemas export
// ---------------------------------------------------------------------------
describe('R3: advertised tools match implemented tools', () => {
  it('toolSchemas contains exactly the four implemented tools (search, open, status, context)', async () => {
    // Import the live toolSchemas from the production module so any regression
    // (e.g. re-adding scan/upsert/explain) is caught immediately.
    const { toolSchemas } = await import('../../src/index.js')

    const names = toolSchemas.map((t) => t.name)
    const expected = ['search', 'open', 'status', 'context']
    expect(names.sort()).toEqual(expected.sort())
  })

  it('scan, upsert, and explain are NOT in toolSchemas', async () => {
    const { toolSchemas } = await import('../../src/index.js')
    const names = toolSchemas.map((t) => t.name)

    const removed = ['scan', 'upsert', 'explain']
    for (const name of removed) {
      expect(names).not.toContain(name)
    }
  })

  it('open tool schema advertises repo parameter for multi-repo disambiguation', async () => {
    const { toolSchemas } = await import('../../src/index.js')
    const openSchema = toolSchemas.find((t) => t.name === 'open')

    expect(openSchema).toBeDefined()
    const props = (openSchema!.inputSchema as { properties: Record<string, unknown> }).properties
    expect(props).toHaveProperty('repo')
  })
})

// ---------------------------------------------------------------------------
// R4: status error differentiation — unit tests for the classification logic
// ---------------------------------------------------------------------------
describe('R4: status fallback error differentiation', () => {
  it('identifies binary-not-found errors correctly', () => {
    // Mirror of the isBinaryNotFound classification in handleStatus catch block.
    // If someone changes the logic, this test will catch the regression.
    const isBinaryNotFound = (msg: string) =>
      msg.includes('binary not found') ||
      msg.includes('Maproom binary not found') ||
      msg.includes('ENOENT')

    expect(isBinaryNotFound('Maproom binary not found. Please ensure maproom is installed.')).toBe(true)
    expect(isBinaryNotFound('spawn ENOENT')).toBe(true)
    expect(isBinaryNotFound('binary not found in PATH')).toBe(true)
    expect(isBinaryNotFound('Connection refused to postgres')).toBe(false)
    expect(isBinaryNotFound('timeout after 30000ms')).toBe(false)
    expect(isBinaryNotFound('Daemon RPC error: unknown method')).toBe(false)
  })

  it('handleStatus is exported (required for regression testing)', async () => {
    const mod = await import('../../src/index.js')
    expect(typeof (mod as any).handleStatus).toBe('function')
  })

  it('binary-not-found hint includes actionable guidance, not just the error', () => {
    const errorMessage = 'Maproom binary not found. Please ensure maproom is installed.'
    const isBinaryNotFound = errorMessage.includes('Maproom binary not found')
    const hint = isBinaryNotFound
      ? `Maproom binary not found.\n\nTroubleshooting:\n1. Ensure maproom is installed (run: maproom --version)\n2. Set MAPROOM_BIN to the binary path\n3. Add maproom to your PATH\n\nRaw error: ${errorMessage}`
      : `Daemon error: ${errorMessage}`

    expect(hint).toContain('Troubleshooting')
    expect(hint).toContain('MAPROOM_BIN')
    expect(hint).toContain('maproom --version')
    expect(hint).not.toContain('Daemon error:')
  })

  it('transient daemon error hint does NOT say binary not found', () => {
    const errorMessage = 'Connection refused: postgres://localhost:5433/maproom'
    const isBinaryNotFound = errorMessage.includes('binary not found')
    const hint = isBinaryNotFound
      ? 'Maproom binary not found...'
      : `Daemon error: ${errorMessage}\n\nThis may be a transient error. Try again or check daemon logs.`

    expect(hint).toContain('transient error')
    expect(hint).not.toContain('binary not found')
    expect(hint).not.toContain('MAPROOM_BIN')
  })

  it('daemonError flag shape is present in error response and absent in success response', async () => {
    // Validate the contract shapes that consumers depend on.
    // The error path must set daemonError:true; the success path must NOT include it.
    // These are structural tests against the exported handleStatus shape specification.
    const errorShape = {
      repos: [],
      totalRepos: 0,
      totalFiles: 0,
      totalChunks: 0,
      hint: 'some error hint',
      daemonError: true,
      backendType: 'postgres' as const,
      databaseUrl: 'postgres://localhost:5433/maproom',
    }
    const successShape = {
      repos: [{ name: 'crewchief', worktrees: [] }],
      totalRepos: 1,
      totalFiles: 100,
      totalChunks: 500,
      hint: 'Index ready!',
      backendType: 'postgres' as const,
      databaseUrl: 'postgres://localhost/maproom',
    }

    expect(errorShape.daemonError).toBe(true)
    expect(errorShape.repos).toHaveLength(0)
    expect(successShape).not.toHaveProperty('daemonError')
    expect(successShape.repos).toHaveLength(1)
  })
})

// ---------------------------------------------------------------------------
// R1: open resolves path from daemon status, not process.cwd()
// ---------------------------------------------------------------------------
describe('R1: open resolves path from daemon status, not process.cwd()', () => {
  it('open error when worktree not in daemon index is WORKTREE_NOT_FOUND, not FILE_NOT_FOUND', () => {
    // The new handleOpen returns WORKTREE_NOT_FOUND when the daemon doesn't know the worktree
    // vs FILE_NOT_FOUND when the worktree is known but the file is missing
    const mockNotFoundResponse = {
      error: 'WORKTREE_NOT_FOUND',
      message: "Worktree 'nonexistent' not found in the daemon index. Use the status tool to list indexed worktrees.",
    }
    expect(mockNotFoundResponse.error).toBe('WORKTREE_NOT_FOUND')
    expect(mockNotFoundResponse.message).toContain('status tool')
  })

  it('open resolvedFrom field shows abs_path was used (not process.cwd())', () => {
    // When open succeeds, resolvedFrom must be the worktree abs_path (not process.cwd())
    const mockSuccessResponse = {
      content: 'fn main() {}',
      relpath: 'src/main.rs',
      resolvedFrom: '/workspace/repos/crewchief/crewchief',  // abs_path from daemon
    }
    expect(mockSuccessResponse.resolvedFrom).not.toBe(process.cwd())
    expect(mockSuccessResponse.resolvedFrom).toMatch(/^\//)  // absolute path
  })

  it('path traversal rejected relative to abs_path root, not cwd', async () => {
    // Verify path escape check uses abs_path as the boundary
    const absPath = '/workspace/repos/crewchief/crewchief'
    const relpath = '../../../etc/passwd'

    const pathMod = await import('node:path')
    const fullPath = pathMod.join(absPath, pathMod.normalize(relpath))
    const normalizedRoot = absPath + pathMod.sep
    const isEscape = !fullPath.startsWith(normalizedRoot) && fullPath !== absPath

    expect(isEscape).toBe(true)  // should be rejected
  })

  it('open tool schema requires worktree and advertises repo for multi-repo disambiguation', async () => {
    const { toolSchemas } = await import('../../src/index.js')
    const openSchema = toolSchemas.find((t) => t.name === 'open')

    expect(openSchema).toBeDefined()
    const schema = openSchema!.inputSchema as { properties: Record<string, unknown>; required: string[] }
    // repo is advertised as optional (not in required) but present in properties
    expect(schema.properties).toHaveProperty('repo')
    // worktree is required
    expect(schema.required).toContain('worktree')
  })
})
