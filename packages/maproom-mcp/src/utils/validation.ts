/**
 * Validation utilities for MCP server operations
 *
 * Provides security-focused validation for:
 * - Path traversal prevention
 * - File path normalization
 * - Repository boundary checks
 * - File size limits
 */

import path from 'node:path'
import fs from 'node:fs/promises'

/**
 * Custom error for validation failures
 */
export class ValidationError extends Error {
  constructor(message: string, public readonly code: string) {
    super(message)
    this.name = 'ValidationError'
  }
}

/**
 * Normalize and validate a file path to prevent path traversal attacks
 * @param relpath - Relative path to validate
 * @returns Normalized path
 * @throws ValidationError if path is invalid or contains traversal attempts
 */
export function validatePath(relpath: string): string {
  // Check for empty path
  if (!relpath || relpath.trim().length === 0) {
    throw new ValidationError('Path cannot be empty', 'INVALID_PATH')
  }

  // Normalize the path to remove ../, ./, etc.
  const normalized = path.normalize(relpath)

  // Check for path traversal attempts
  if (normalized.startsWith('..') || normalized.includes(path.sep + '..' + path.sep)) {
    throw new ValidationError(
      `Path traversal detected: ${relpath}`,
      'INVALID_PATH'
    )
  }

  // Check for absolute paths
  if (path.isAbsolute(normalized)) {
    throw new ValidationError(
      `Absolute paths not allowed: ${relpath}`,
      'INVALID_PATH'
    )
  }

  // Check for suspicious patterns
  if (relpath.includes('\0')) {
    throw new ValidationError(
      'Null bytes not allowed in path',
      'INVALID_PATH'
    )
  }

  return normalized
}

/**
 * Validate that a resolved path is within repository boundaries
 * @param resolvedPath - Absolute path to check
 * @param repoRoot - Absolute path to repository root
 * @throws ValidationError if path is outside repository
 */
export function validateWithinRepo(resolvedPath: string, repoRoot: string): void {
  const normalizedPath = path.normalize(resolvedPath)
  const normalizedRoot = path.normalize(repoRoot)

  if (!normalizedPath.startsWith(normalizedRoot)) {
    throw new ValidationError(
      'Path is outside repository boundaries',
      'INVALID_PATH'
    )
  }
}

/**
 * Validate file size against configured limit
 * @param filePath - Absolute path to file
 * @param maxSize - Maximum allowed file size in bytes (default 1MB)
 * @throws ValidationError if file exceeds size limit
 */
export async function validateFileSize(filePath: string, maxSize: number = 1024 * 1024): Promise<void> {
  try {
    const stats = await fs.stat(filePath)

    if (stats.size > maxSize) {
      throw new ValidationError(
        `File size (${stats.size} bytes) exceeds maximum (${maxSize} bytes)`,
        'FILE_TOO_LARGE'
      )
    }
  } catch (error: any) {
    if (error instanceof ValidationError) {
      throw error
    }
    throw new ValidationError(
      `Failed to check file size: ${error.message}`,
      'FILE_NOT_FOUND'
    )
  }
}

/**
 * Validate and extract line range parameters
 * @param content - File content as string
 * @param start - Start line number (1-indexed)
 * @param end - End line number (1-indexed)
 * @returns Object with validated start and end line numbers
 * @throws ValidationError if range is invalid
 */
export function validateRange(content: string, start: number, end: number): { start: number; end: number } {
  const lines = content.split('\n')
  const totalLines = lines.length

  // Validate start line
  if (start < 1) {
    throw new ValidationError(
      `Start line must be >= 1, got ${start}`,
      'INVALID_RANGE'
    )
  }

  // Validate end line
  if (end < 1) {
    throw new ValidationError(
      `End line must be >= 1, got ${end}`,
      'INVALID_RANGE'
    )
  }

  // Validate start <= end
  if (start > end) {
    throw new ValidationError(
      `Start line (${start}) must be <= end line (${end})`,
      'INVALID_RANGE'
    )
  }

  // Validate within bounds
  if (start > totalLines) {
    throw new ValidationError(
      `Start line (${start}) exceeds file length (${totalLines} lines)`,
      'INVALID_RANGE'
    )
  }

  if (end > totalLines) {
    throw new ValidationError(
      `End line (${end}) exceeds file length (${totalLines} lines)`,
      'INVALID_RANGE'
    )
  }

  return { start, end }
}

/**
 * Extract a line range from file content
 * @param content - File content as string
 * @param start - Start line number (1-indexed, inclusive)
 * @param end - End line number (1-indexed, inclusive)
 * @returns Extracted lines as string
 */
export function extractRange(content: string, start: number, end: number): string {
  const lines = content.split('\n')
  // Lines are 1-indexed, array is 0-indexed
  // slice(start-1, end) gives us lines [start, end) which is lines start through end-1
  // But we want [start, end] inclusive, so we use slice(start-1, end)
  return lines.slice(start - 1, end).join('\n')
}

/**
 * Check if a file exists and is readable
 * @param filePath - Absolute path to file
 * @returns true if file exists and is readable, false otherwise
 */
export async function fileExists(filePath: string): Promise<boolean> {
  try {
    await fs.access(filePath, fs.constants.R_OK)
    return true
  } catch {
    return false
  }
}
