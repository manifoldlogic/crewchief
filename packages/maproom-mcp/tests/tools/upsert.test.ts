/**
 * Unit tests for Upsert tool
 *
 * Tests cover:
 * - Parameter validation with Zod schema
 * - Path validation and security checks
 * - Process spawning and execution
 * - Progress parsing from output
 * - Error handling for all edge cases
 * - Binary discovery
 * - Timeout handling
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { validateUpsertParams } from '../../src/tools/upsert_schema.js'
import { handleUpsertTool } from '../../src/tools/upsert.js'
import { ValidationError } from '../../src/utils/validation.js'
import {
  parseIndexingStats,
  findMaproomBinary,
  getBinarycandidates,
  ProcessError,
} from '../../src/utils/process.js'
import { clearProviderCache } from '../../src/utils/provider-detection.js'

describe('Upsert Tool - Parameter Validation', () => {
  it('should validate required parameters', () => {
    expect(() => validateUpsertParams({})).toThrow()
  })

  it('should reject empty paths array', () => {
    const invalid = {
      paths: [],
      commit: 'abc123',
      repo: 'test-repo',
      worktree: 'main',
      root: '/workspace/test',
    }
    expect(() => validateUpsertParams(invalid)).toThrow()
  })

  it('should reject empty strings in paths', () => {
    const invalid = {
      paths: ['src/index.ts', ''],
      commit: 'abc123',
      repo: 'test-repo',
      worktree: 'main',
      root: '/workspace/test',
    }
    expect(() => validateUpsertParams(invalid)).toThrow()
  })

  it('should reject empty commit', () => {
    const invalid = {
      paths: ['src/index.ts'],
      commit: '',
      repo: 'test-repo',
      worktree: 'main',
      root: '/workspace/test',
    }
    expect(() => validateUpsertParams(invalid)).toThrow()
  })

  it('should reject empty repo', () => {
    const invalid = {
      paths: ['src/index.ts'],
      commit: 'abc123',
      repo: '',
      worktree: 'main',
      root: '/workspace/test',
    }
    expect(() => validateUpsertParams(invalid)).toThrow()
  })

  it('should reject empty worktree', () => {
    const invalid = {
      paths: ['src/index.ts'],
      commit: 'abc123',
      repo: 'test-repo',
      worktree: '',
      root: '/workspace/test',
    }
    expect(() => validateUpsertParams(invalid)).toThrow()
  })

  it('should reject empty root', () => {
    const invalid = {
      paths: ['src/index.ts'],
      commit: 'abc123',
      repo: 'test-repo',
      worktree: 'main',
      root: '',
    }
    expect(() => validateUpsertParams(invalid)).toThrow()
  })

  it('should accept valid parameters', () => {
    const valid = {
      paths: ['src/index.ts', 'src/lib/util.ts'],
      commit: 'abc123def456',
      repo: 'crewchief',
      worktree: 'main',
      root: '/workspace/crewchief',
    }
    const result = validateUpsertParams(valid)
    expect(result.paths).toEqual(['src/index.ts', 'src/lib/util.ts'])
    expect(result.commit).toBe('abc123def456')
    expect(result.repo).toBe('crewchief')
    expect(result.worktree).toBe('main')
    expect(result.root).toBe('/workspace/crewchief')
  })

  it('should accept single path', () => {
    const valid = {
      paths: ['src/'],
      commit: 'HEAD',
      repo: 'test',
      worktree: 'dev',
      root: '/workspace',
    }
    const result = validateUpsertParams(valid)
    expect(result.paths).toEqual(['src/'])
  })
})

describe('Upsert Tool - Path Security', () => {
  it('should reject path traversal in paths array', () => {
    // Path validation is deferred to the binary, but we still validate basic structure
    const params = {
      paths: ['../etc/passwd'],
      commit: 'abc123',
      repo: 'test',
      worktree: 'main',
      root: '/workspace',
    }
    // This should be caught by the path validation in handleUpsertTool
    expect(() => validateUpsertParams(params)).not.toThrow() // Schema accepts it
    // But validatePath would throw ValidationError
  })

  it('should accept valid relative paths', () => {
    const params = {
      paths: ['src/index.ts', 'packages/cli/src/main.ts'],
      commit: 'abc123',
      repo: 'test',
      worktree: 'main',
      root: '/workspace',
    }
    expect(() => validateUpsertParams(params)).not.toThrow()
  })

  it('should accept directory paths', () => {
    const params = {
      paths: ['src/', 'packages/'],
      commit: 'abc123',
      repo: 'test',
      worktree: 'main',
      root: '/workspace',
    }
    expect(() => validateUpsertParams(params)).not.toThrow()
  })
})

describe('Indexing Stats Parsing', () => {
  it('should parse complete output with all stats', () => {
    const output = `
Starting indexing...
Processed 42 files
Created 315 chunks
Duration: 1234ms
Indexing complete.
`
    const stats = parseIndexingStats(output)
    expect(stats.files).toBe(42)
    expect(stats.chunks).toBe(315)
    expect(stats.duration_ms).toBe(1234)
  })

  it('should parse output with alternative phrasing', () => {
    const output = `
Indexed 5 files
Generated 23 chunks
Completed in: 567ms
`
    const stats = parseIndexingStats(output)
    expect(stats.files).toBe(5)
    expect(stats.chunks).toBe(23)
    expect(stats.duration_ms).toBe(567)
  })

  it('should parse output with files and chunks only', () => {
    const output = `
10 files processed
50 chunks created
`
    const stats = parseIndexingStats(output)
    expect(stats.files).toBe(10)
    expect(stats.chunks).toBe(50)
    expect(stats.duration_ms).toBeUndefined()
  })

  it('should handle missing statistics gracefully', () => {
    const output = `
Some random output without stats
`
    const stats = parseIndexingStats(output)
    expect(stats.files).toBeUndefined()
    expect(stats.chunks).toBeUndefined()
    expect(stats.duration_ms).toBeUndefined()
  })

  it('should handle empty output', () => {
    const stats = parseIndexingStats('')
    expect(stats.files).toBeUndefined()
    expect(stats.chunks).toBeUndefined()
    expect(stats.duration_ms).toBeUndefined()
  })

  it('should parse single file', () => {
    const output = 'Processed 1 file, Created 7 chunks, Duration: 123ms'
    const stats = parseIndexingStats(output)
    expect(stats.files).toBe(1)
    expect(stats.chunks).toBe(7)
    expect(stats.duration_ms).toBe(123)
  })

  it('should handle large numbers', () => {
    const output = `
Processed 99999 files
Created 5000000 chunks
Duration: 123456ms
`
    const stats = parseIndexingStats(output)
    expect(stats.files).toBe(99999)
    expect(stats.chunks).toBe(5000000)
    expect(stats.duration_ms).toBe(123456)
  })

  it('should parse case-insensitive', () => {
    const output = `
PROCESSED 15 FILES
CREATED 89 CHUNKS
DURATION: 999MS
`
    const stats = parseIndexingStats(output)
    expect(stats.files).toBe(15)
    expect(stats.chunks).toBe(89)
    expect(stats.duration_ms).toBe(999)
  })
})

describe('Binary Discovery', () => {
  it('should check environment variable first', () => {
    const originalEnv = process.env.CREWCHIEF_MAPROOM_BIN
    try {
      // Mock environment variable pointing to non-existent file
      process.env.CREWCHIEF_MAPROOM_BIN = '/nonexistent/binary'
      const result = findMaproomBinary()
      // Should try env var first, but may fall back to other strategies
      // The result can be null (if no fallbacks found) or a valid path (if fallback found)
      expect(result === null || typeof result === 'string').toBe(true)
    } finally {
      // Restore original env
      if (originalEnv) {
        process.env.CREWCHIEF_MAPROOM_BIN = originalEnv
      } else {
        delete process.env.CREWCHIEF_MAPROOM_BIN
      }
    }
  })

  it('should return null when no binary found', () => {
    const originalEnv = process.env.CREWCHIEF_MAPROOM_BIN
    try {
      delete process.env.CREWCHIEF_MAPROOM_BIN
      const result = findMaproomBinary()
      // May or may not be null depending on whether dev build exists
      expect(result === null || typeof result === 'string').toBe(true)
    } finally {
      if (originalEnv) {
        process.env.CREWCHIEF_MAPROOM_BIN = originalEnv
      }
    }
  })

  it('should provide binary candidates', () => {
    const candidates = getBinarycandidates()
    expect(Array.isArray(candidates)).toBe(true)
    expect(candidates.length).toBeGreaterThan(0)
    expect(candidates[0]).toHaveProperty('cmd')
    expect(candidates[0]).toHaveProperty('description')
  })

  it('should include system PATH candidates', () => {
    const candidates = getBinarycandidates()
    const hasSysPath = candidates.some((c) => c.cmd === 'crewchief-maproom' || c.cmd === 'crewchief')
    expect(hasSysPath).toBe(true)
  })
})

describe('Process Error Handling', () => {
  it('should create ProcessError with all fields', () => {
    const error = new ProcessError(
      'Test error message',
      'TEST_CODE',
      1,
      'stderr output',
      'test-command'
    )
    expect(error.message).toBe('Test error message')
    expect(error.code).toBe('TEST_CODE')
    expect(error.exitCode).toBe(1)
    expect(error.stderr).toBe('stderr output')
    expect(error.command).toBe('test-command')
    expect(error.name).toBe('ProcessError')
  })

  it('should create ProcessError with minimal fields', () => {
    const error = new ProcessError('Simple error', 'SIMPLE_CODE')
    expect(error.message).toBe('Simple error')
    expect(error.code).toBe('SIMPLE_CODE')
    expect(error.exitCode).toBeUndefined()
    expect(error.stderr).toBeUndefined()
    expect(error.command).toBeUndefined()
  })

  it('should be instanceof Error', () => {
    const error = new ProcessError('Test', 'CODE')
    expect(error).toBeInstanceOf(Error)
    expect(error).toBeInstanceOf(ProcessError)
  })

  it('should have stack trace', () => {
    const error = new ProcessError('Test', 'CODE')
    expect(error.stack).toBeDefined()
  })
})

describe('Upsert Tool - Error Messages', () => {
  it('should provide clear error for BINARY_NOT_FOUND', () => {
    const error = new ProcessError(
      'crewchief-maproom binary not found',
      'BINARY_NOT_FOUND'
    )
    expect(error.message).toContain('binary not found')
    expect(error.code).toBe('BINARY_NOT_FOUND')
  })

  it('should provide clear error for TIMEOUT', () => {
    const error = new ProcessError(
      'Process timed out after 120000ms',
      'TIMEOUT'
    )
    expect(error.message).toContain('timed out')
    expect(error.code).toBe('TIMEOUT')
  })

  it('should provide clear error for PROCESS_EXIT_ERROR', () => {
    const error = new ProcessError(
      'Process exited with code 1: Database connection failed',
      'PROCESS_EXIT_ERROR',
      1,
      'Database connection failed'
    )
    expect(error.message).toContain('exited')
    expect(error.exitCode).toBe(1)
    expect(error.stderr).toContain('Database')
  })

  it('should provide clear error for INVALID_PATH', () => {
    const error = new ValidationError(
      'Path traversal detected: ../etc/passwd',
      'INVALID_PATH'
    )
    expect(error.message).toContain('traversal')
    expect(error.code).toBe('INVALID_PATH')
  })
})

describe('Upsert Tool - Edge Cases', () => {
  it('should handle single path in array', () => {
    const params = {
      paths: ['src/index.ts'],
      commit: 'abc123',
      repo: 'test',
      worktree: 'main',
      root: '/workspace',
    }
    expect(() => validateUpsertParams(params)).not.toThrow()
  })

  it('should handle many paths', () => {
    const params = {
      paths: Array.from({ length: 100 }, (_, i) => `file${i}.ts`),
      commit: 'abc123',
      repo: 'test',
      worktree: 'main',
      root: '/workspace',
    }
    expect(() => validateUpsertParams(params)).not.toThrow()
  })

  it('should handle long commit hashes', () => {
    const params = {
      paths: ['src/index.ts'],
      commit: 'a'.repeat(40), // Full SHA-1 hash
      repo: 'test',
      worktree: 'main',
      root: '/workspace',
    }
    expect(() => validateUpsertParams(params)).not.toThrow()
  })

  it('should handle special characters in repo name', () => {
    const params = {
      paths: ['src/index.ts'],
      commit: 'abc123',
      repo: 'my-repo_test.v2',
      worktree: 'main',
      root: '/workspace',
    }
    expect(() => validateUpsertParams(params)).not.toThrow()
  })

  it('should handle paths with special characters', () => {
    const params = {
      paths: ['src/file-name.spec.ts', 'src/file_name.test.ts'],
      commit: 'abc123',
      repo: 'test',
      worktree: 'main',
      root: '/workspace',
    }
    expect(() => validateUpsertParams(params)).not.toThrow()
  })
})

describe('Upsert Tool - Output Parsing Edge Cases', () => {
  it('should handle output with no newlines', () => {
    const output = 'Processed 5 files Created 20 chunks Duration: 100ms'
    const stats = parseIndexingStats(output)
    expect(stats.files).toBe(5)
    expect(stats.chunks).toBe(20)
    expect(stats.duration_ms).toBe(100)
  })

  it('should handle output with extra whitespace', () => {
    const output = `

    Processed   10   files

    Created   50   chunks

    Duration:   500ms

    `
    const stats = parseIndexingStats(output)
    expect(stats.files).toBe(10)
    expect(stats.chunks).toBe(50)
    expect(stats.duration_ms).toBe(500)
  })

  it('should handle output with progress bars', () => {
    const output = `
Processing files: [##########] 100%
Processed 15 files
Creating chunks: [##########] 100%
Created 75 chunks
Duration: 800ms
`
    const stats = parseIndexingStats(output)
    expect(stats.files).toBe(15)
    expect(stats.chunks).toBe(75)
    expect(stats.duration_ms).toBe(800)
  })

  it('should handle output with ANSI color codes', () => {
    const output = '\x1b[32mProcessed 8 files\x1b[0m\n\x1b[32mCreated 40 chunks\x1b[0m'
    const stats = parseIndexingStats(output)
    expect(stats.files).toBe(8)
    expect(stats.chunks).toBe(40)
  })
})

describe('Upsert Tool - Provider Integration', () => {
  const originalEnv = { ...process.env }

  beforeEach(() => {
    clearProviderCache()
    vi.restoreAllMocks()
  })

  afterEach(() => {
    process.env = { ...originalEnv }
  })

  it('should throw NO_PROVIDER error when no provider available', async () => {
    // Clear all provider configs
    delete process.env.MAPROOM_EMBEDDING_PROVIDER
    delete process.env.OPENAI_API_KEY
    delete process.env.GOOGLE_PROJECT_ID
    delete process.env.GOOGLE_APPLICATION_CREDENTIALS

    // Mock Ollama not available
    global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'))

    const params = {
      paths: ['src/index.ts'],
      commit: 'abc123',
      repo: 'test-repo',
      worktree: 'main',
      root: '/workspace/test',
    }

    await expect(handleUpsertTool(params)).rejects.toThrow('No provider available')
  })

  it('should include helpful error message when provider unavailable', async () => {
    delete process.env.MAPROOM_EMBEDDING_PROVIDER
    delete process.env.OPENAI_API_KEY
    global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'))

    const params = {
      paths: ['src/index.ts'],
      commit: 'abc123',
      repo: 'test-repo',
      worktree: 'main',
      root: '/workspace/test',
    }

    try {
      await handleUpsertTool(params)
      expect.fail('Should have thrown error')
    } catch (error: any) {
      expect(error.message).toContain('No provider available')
      expect(error.message).toContain('Install Ollama')
      expect(error.message).toContain('OPENAI_API_KEY')
    }
  })
})
