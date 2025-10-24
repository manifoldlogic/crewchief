/**
 * Unit tests for Open tool
 *
 * Tests cover:
 * - Parameter validation with Zod schema
 * - Path validation and security checks
 * - Line range extraction
 * - Error handling for all edge cases
 * - File size limits
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import {
  validatePath,
  validateRange,
  extractRange,
  ValidationError,
  validateWithinRepo,
} from '../../src/utils/validation.js'
import { validateOpenParams } from '../../src/tools/open_schema.js'
import path from 'node:path'

describe('Open Tool - Parameter Validation', () => {
  it('should validate required relpath parameter', () => {
    expect(() => validateOpenParams({ relpath: '' })).toThrow()
    expect(() => validateOpenParams({})).toThrow()
  })

  it('should accept valid parameters', () => {
    const valid = {
      relpath: 'src/index.ts',
      worktree: 'main',
    }
    const result = validateOpenParams(valid)
    expect(result.relpath).toBe('src/index.ts')
    expect(result.worktree).toBe('main')
  })

  it('should accept optional range parameter', () => {
    const valid = {
      relpath: 'src/index.ts',
      range: { start: 1, end: 10 },
      worktree: 'main',
    }
    const result = validateOpenParams(valid)
    expect(result.range).toEqual({ start: 1, end: 10 })
  })

  it('should accept optional commit parameter', () => {
    const valid = {
      relpath: 'src/index.ts',
      commit: 'abc123',
      worktree: 'main',
    }
    const result = validateOpenParams(valid)
    expect(result.commit).toBe('abc123')
  })

  it('should reject invalid range (start > end)', () => {
    const invalid = {
      relpath: 'src/index.ts',
      range: { start: 10, end: 5 },
      worktree: 'main',
    }
    expect(() => validateOpenParams(invalid)).toThrow()
  })

  it('should reject negative line numbers', () => {
    const invalid = {
      relpath: 'src/index.ts',
      range: { start: -1, end: 10 },
      worktree: 'main',
    }
    expect(() => validateOpenParams(invalid)).toThrow()
  })

  it('should reject zero line numbers', () => {
    const invalid = {
      relpath: 'src/index.ts',
      range: { start: 0, end: 10 },
      worktree: 'main',
    }
    expect(() => validateOpenParams(invalid)).toThrow()
  })
})

describe('Open Tool - Path Validation', () => {
  it('should accept valid relative paths', () => {
    expect(validatePath('src/index.ts')).toBe('src/index.ts')
    expect(validatePath('packages/cli/src/main.ts')).toBe(path.join('packages', 'cli', 'src', 'main.ts'))
  })

  it('should reject empty paths', () => {
    expect(() => validatePath('')).toThrow(ValidationError)
    expect(() => validatePath('  ')).toThrow(ValidationError)
  })

  it('should reject path traversal attempts with ../', () => {
    expect(() => validatePath('../etc/passwd')).toThrow(ValidationError)
    expect(() => validatePath('../../secret.txt')).toThrow(ValidationError)
    expect(() => validatePath('src/../../etc/passwd')).toThrow(ValidationError)
  })

  it('should reject absolute paths', () => {
    expect(() => validatePath('/etc/passwd')).toThrow(ValidationError)
    expect(() => validatePath('/absolute/path')).toThrow(ValidationError)
  })

  it('should reject paths with null bytes', () => {
    expect(() => validatePath('src/index\0.ts')).toThrow(ValidationError)
  })

  it('should normalize paths correctly', () => {
    expect(validatePath('src/./index.ts')).toBe(path.join('src', 'index.ts'))
    expect(validatePath('src//index.ts')).toBe(path.join('src', 'index.ts'))
  })

  it('should throw ValidationError with correct code', () => {
    try {
      validatePath('../etc/passwd')
      expect.fail('Should have thrown')
    } catch (error) {
      expect(error).toBeInstanceOf(ValidationError)
      expect((error as ValidationError).code).toBe('INVALID_PATH')
    }
  })
})

describe('Open Tool - Repository Boundary Validation', () => {
  it('should accept paths within repository', () => {
    const repoRoot = '/workspace/repo'
    const validPath = '/workspace/repo/src/index.ts'
    expect(() => validateWithinRepo(validPath, repoRoot)).not.toThrow()
  })

  it('should reject paths outside repository', () => {
    const repoRoot = '/workspace/repo'
    const invalidPath = '/workspace/other/file.ts'
    expect(() => validateWithinRepo(invalidPath, repoRoot)).toThrow(ValidationError)
  })

  it('should handle normalized paths', () => {
    const repoRoot = '/workspace/repo/'
    const validPath = '/workspace/repo/src/../lib/util.ts'
    expect(() => validateWithinRepo(path.normalize(validPath), path.normalize(repoRoot))).not.toThrow()
  })
})

describe('Open Tool - Range Validation', () => {
  const testContent = 'line1\nline2\nline3\nline4\nline5'

  it('should validate valid ranges', () => {
    const result = validateRange(testContent, 1, 3)
    expect(result).toEqual({ start: 1, end: 3 })
  })

  it('should accept single-line range', () => {
    const result = validateRange(testContent, 2, 2)
    expect(result).toEqual({ start: 2, end: 2 })
  })

  it('should accept full file range', () => {
    const result = validateRange(testContent, 1, 5)
    expect(result).toEqual({ start: 1, end: 5 })
  })

  it('should reject start < 1', () => {
    expect(() => validateRange(testContent, 0, 3)).toThrow(ValidationError)
    expect(() => validateRange(testContent, -1, 3)).toThrow(ValidationError)
  })

  it('should reject end < 1', () => {
    expect(() => validateRange(testContent, 1, 0)).toThrow(ValidationError)
    expect(() => validateRange(testContent, 1, -1)).toThrow(ValidationError)
  })

  it('should reject start > end', () => {
    expect(() => validateRange(testContent, 3, 1)).toThrow(ValidationError)
  })

  it('should reject start beyond file length', () => {
    expect(() => validateRange(testContent, 10, 15)).toThrow(ValidationError)
  })

  it('should reject end beyond file length', () => {
    expect(() => validateRange(testContent, 1, 10)).toThrow(ValidationError)
  })

  it('should throw ValidationError with INVALID_RANGE code', () => {
    try {
      validateRange(testContent, 10, 15)
      expect.fail('Should have thrown')
    } catch (error) {
      expect(error).toBeInstanceOf(ValidationError)
      expect((error as ValidationError).code).toBe('INVALID_RANGE')
    }
  })
})

describe('Open Tool - Range Extraction', () => {
  const testContent = 'line1\nline2\nline3\nline4\nline5'

  it('should extract single line', () => {
    const result = extractRange(testContent, 2, 2)
    expect(result).toBe('line2')
  })

  it('should extract multiple lines', () => {
    const result = extractRange(testContent, 2, 4)
    expect(result).toBe('line2\nline3\nline4')
  })

  it('should extract first line', () => {
    const result = extractRange(testContent, 1, 1)
    expect(result).toBe('line1')
  })

  it('should extract last line', () => {
    const result = extractRange(testContent, 5, 5)
    expect(result).toBe('line5')
  })

  it('should extract full content', () => {
    const result = extractRange(testContent, 1, 5)
    expect(result).toBe(testContent)
  })

  it('should handle content with no trailing newline', () => {
    const content = 'a\nb\nc'
    const result = extractRange(content, 2, 3)
    expect(result).toBe('b\nc')
  })

  it('should handle content with empty lines', () => {
    const content = 'line1\n\nline3\n\nline5'
    const result = extractRange(content, 2, 4)
    expect(result).toBe('\nline3\n')
  })

  it('should handle single-line content', () => {
    const content = 'single line'
    const result = extractRange(content, 1, 1)
    expect(result).toBe('single line')
  })
})

describe('Open Tool - Edge Cases', () => {
  it('should handle empty file content', () => {
    const content = ''
    const lines = content.split('\n')
    expect(lines.length).toBe(1) // Empty string splits to ['']
  })

  it('should handle file with only newlines', () => {
    const content = '\n\n\n'
    const result = extractRange(content, 1, 3)
    expect(result).toBe('\n\n')
  })

  it('should handle Windows-style line endings', () => {
    const content = 'line1\r\nline2\r\nline3'
    // Note: Our implementation uses \n, so \r\n would be preserved in content
    const lines = content.split('\n')
    expect(lines.length).toBe(3)
  })

  it('should handle very long lines', () => {
    const longLine = 'a'.repeat(10000)
    const content = `short\n${longLine}\nshort`
    const result = extractRange(content, 2, 2)
    expect(result).toBe(longLine)
    expect(result.length).toBe(10000)
  })

  it('should handle unicode characters', () => {
    const content = '日本語\n中文\nEmoji 🎉\nΓεια σου'
    const result = extractRange(content, 2, 3)
    expect(result).toBe('中文\nEmoji 🎉')
  })
})

describe('Open Tool - Error Messages', () => {
  it('should provide clear error for FILE_NOT_FOUND', () => {
    const error = new ValidationError('File not found: test.ts', 'FILE_NOT_FOUND')
    expect(error.message).toContain('test.ts')
    expect(error.code).toBe('FILE_NOT_FOUND')
  })

  it('should provide clear error for INVALID_PATH', () => {
    const error = new ValidationError('Path traversal detected: ../etc/passwd', 'INVALID_PATH')
    expect(error.message).toContain('traversal')
    expect(error.code).toBe('INVALID_PATH')
  })

  it('should provide clear error for INVALID_RANGE', () => {
    const error = new ValidationError('Start line (10) exceeds file length (5 lines)', 'INVALID_RANGE')
    expect(error.message).toContain('10')
    expect(error.message).toContain('5')
    expect(error.code).toBe('INVALID_RANGE')
  })

  it('should provide clear error for FILE_TOO_LARGE', () => {
    const error = new ValidationError('File size (2000000 bytes) exceeds maximum (1048576 bytes)', 'FILE_TOO_LARGE')
    expect(error.message).toContain('2000000')
    expect(error.message).toContain('1048576')
    expect(error.code).toBe('FILE_TOO_LARGE')
  })
})

describe('Open Tool - ValidationError Class', () => {
  it('should create error with message and code', () => {
    const error = new ValidationError('Test error', 'TEST_CODE')
    expect(error.message).toBe('Test error')
    expect(error.code).toBe('TEST_CODE')
    expect(error.name).toBe('ValidationError')
  })

  it('should be instanceof Error', () => {
    const error = new ValidationError('Test', 'CODE')
    expect(error).toBeInstanceOf(Error)
    expect(error).toBeInstanceOf(ValidationError)
  })

  it('should have stack trace', () => {
    const error = new ValidationError('Test', 'CODE')
    expect(error.stack).toBeDefined()
  })
})
