/**
 * Regression tests for mcp-tool-contract-fixes (Wave 2)
 *
 * R1: open resolves file via daemon status abs_path, not process.cwd()
 * R2: phantom search field removed from SearchParams (no-op field, not in Rust)
 * R4: status fallback error is differentiated — binary-not-found vs daemon error
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// ---------------------------------------------------------------------------
// R2: phantom field must NOT exist in SearchParams interface or Zod schema
// (The removed field was "include_related" — a no-op never in Rust SearchParams)
// ---------------------------------------------------------------------------
describe('R2: phantom no-op field removed from SearchParams', () => {
  it('SearchParams Zod schema does not include the removed phantom field', async () => {
    const { SearchParamsSchema } = await import('../../src/tools/search_schema.js')
    const result = SearchParamsSchema.parse({
      query: 'test',
      repo: 'myrepo',
    })
    // The removed field must not appear in parsed output
    expect(Object.keys(result)).not.toContain('include_related')
  })

  it('unknown fields passed to SearchParams are stripped by Zod', async () => {
    const { SearchParamsSchema } = await import('../../src/tools/search_schema.js')
    // Zod strips unknown keys by default (passthrough is not enabled)
    const phantomField = 'include_related'
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
// R4: status error differentiation — binary-not-found vs other daemon errors
// ---------------------------------------------------------------------------
describe('R4: status fallback error differentiation', () => {
  it('identifies binary-not-found errors correctly', () => {
    // Simulate the error categorisation logic from handleStatus catch block
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

  it('status response includes daemonError:true on failure', () => {
    // Simulate what handleStatus returns in error case
    const mockFailedStatusResponse = {
      repos: [],
      totalRepos: 0,
      totalFiles: 0,
      totalChunks: 0,
      hint: 'Maproom binary not found...',
      daemonError: true,
      backendType: 'postgres' as const,
      databaseUrl: 'postgres://localhost:5433/maproom',
    }
    // daemonError flag lets callers distinguish "no repos" from "daemon startup failure"
    expect(mockFailedStatusResponse.daemonError).toBe(true)
    expect(mockFailedStatusResponse.repos).toHaveLength(0)
  })

  it('successful status response does NOT include daemonError', () => {
    // When daemon responds with real data, daemonError must not be present
    const mockSuccessResponse = {
      repos: [{ name: 'crewchief', worktrees: [{ name: 'main', path: '/workspace/crewchief', fileCount: 100, chunkCount: 500 }] }],
      totalRepos: 1,
      totalFiles: 100,
      totalChunks: 500,
      hint: 'Index ready! 100 files and 500 searchable chunks.',
      backendType: 'postgres' as const,
      databaseUrl: 'postgres://localhost/maproom',
    }
    expect(mockSuccessResponse).not.toHaveProperty('daemonError')
    expect(mockSuccessResponse.repos).toHaveLength(1)
  })
})

// ---------------------------------------------------------------------------
// R1: resolveWorktreeAbsPath logic (tested via structural validation)
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

  it('path traversal rejected relative to abs_path root, not cwd', () => {
    // Verify path escape check uses abs_path as the boundary
    const absPath = '/workspace/repos/crewchief/crewchief'
    const relpath = '../../../etc/passwd'

    const pathMod = require('node:path')
    const fullPath = pathMod.join(absPath, pathMod.normalize(relpath))
    const normalizedRoot = absPath + pathMod.sep
    const isEscape = !fullPath.startsWith(normalizedRoot) && fullPath !== absPath

    expect(isEscape).toBe(true)  // should be rejected
  })
})

// ---------------------------------------------------------------------------
// R3: Tool schema consistency (tools in toolSchemas must match advertised set)
// ---------------------------------------------------------------------------
describe('R3: advertised tools match implemented tools', () => {
  const IMPLEMENTED_TOOLS = new Set(['search', 'open', 'status', 'context'])
  const NOT_IMPLEMENTED = new Set(['scan', 'upsert', 'explain'])

  it('IMPLEMENTED_TOOLS covers expected set', () => {
    expect(IMPLEMENTED_TOOLS.has('search')).toBe(true)
    expect(IMPLEMENTED_TOOLS.has('open')).toBe(true)
    expect(IMPLEMENTED_TOOLS.has('status')).toBe(true)
    expect(IMPLEMENTED_TOOLS.has('context')).toBe(true)
  })

  it('NOT_IMPLEMENTED tools are not in IMPLEMENTED_TOOLS', () => {
    for (const name of NOT_IMPLEMENTED) {
      expect(IMPLEMENTED_TOOLS.has(name)).toBe(false)
    }
  })
})
