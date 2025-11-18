/**
 * Unit tests for validation utilities
 *
 * Tests cover the fileExists() helper function with comprehensive
 * coverage of all code paths and edge cases.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { fileExists } from '../../src/utils/validation.js'
import fs from 'node:fs/promises'
import path from 'node:path'
import os from 'node:os'

describe('fileExists', () => {
  let testDir: string
  let testFile: string
  let testSubDir: string

  beforeEach(async () => {
    // Create temporary test directory
    testDir = path.join(os.tmpdir(), `fileExists-test-${Date.now()}-${Math.random().toString(36).substring(7)}`)
    await fs.mkdir(testDir, { recursive: true })

    // Create a test file
    testFile = path.join(testDir, 'test-file.txt')
    await fs.writeFile(testFile, 'test content for fileExists unit tests')

    // Create a test subdirectory (not a file)
    testSubDir = path.join(testDir, 'test-subdir')
    await fs.mkdir(testSubDir)
  })

  afterEach(async () => {
    // Cleanup test files
    try {
      await fs.rm(testDir, { recursive: true, force: true })
    } catch (error) {
      // Ignore cleanup errors
      console.warn('Cleanup error (non-fatal):', error)
    }
  })

  it('should return true for existing file', async () => {
    const result = await fileExists(testFile)
    expect(result).toBe(true)
  })

  it('should return false for non-existent file', async () => {
    const nonExistentPath = path.join(testDir, 'does-not-exist.txt')
    const result = await fileExists(nonExistentPath)
    expect(result).toBe(false)
  })

  it('should return false for directory', async () => {
    // fileExists should return false for directories - only files should return true
    // Uses stats.isFile() to distinguish between files and directories
    const result = await fileExists(testSubDir)
    expect(result).toBe(false)
  })

  it.skipIf(process.platform === 'win32')('should return false for inaccessible file', async () => {
    // Unix/Linux/macOS only test
    // Create a file then remove all permissions
    const inaccessibleFile = path.join(testDir, 'inaccessible.txt')
    await fs.writeFile(inaccessibleFile, 'cannot read this')

    // Remove all permissions (mode 0o000)
    await fs.chmod(inaccessibleFile, 0o000)

    const result = await fileExists(inaccessibleFile)
    expect(result).toBe(false)

    // Restore permissions for cleanup
    await fs.chmod(inaccessibleFile, 0o644)
  })

  it('should handle absolute paths correctly', async () => {
    // Test with absolute path (our typical use case)
    const absolutePath = path.resolve(testFile)
    const result = await fileExists(absolutePath)
    expect(result).toBe(true)
  })

  it('should not throw errors for any input', async () => {
    // Verify the function never throws - always returns boolean
    await expect(fileExists('/nonexistent/path/file.txt')).resolves.toBe(false)
    await expect(fileExists('')).resolves.toBe(false)
    await expect(fileExists('/etc/passwd/../../../etc/passwd')).resolves.toBeDefined()
  })
})
