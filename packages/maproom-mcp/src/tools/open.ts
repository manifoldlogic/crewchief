/**
 * Open Tool - Retrieve file contents with git integration
 *
 * Provides file reading capabilities with:
 * - Current worktree file reading
 * - Historical git commit retrieval
 * - Line range extraction
 * - Security validation (path traversal prevention, file size limits)
 */

import path from 'node:path'
import fs from 'node:fs/promises'
import { Client } from 'pg'
import pino from 'pino'
import type { OpenParams, FileContent, OpenToolConfig } from '../types.js'
import { validateOpenParams } from './open_schema.js'
import {
  validatePath,
  validateWithinRepo,
  validateFileSize,
  validateRange,
  extractRange,
  ValidationError,
  fileExists,
} from '../utils/validation.js'
import {
  isCommitCheckedOut,
  getFileFromGit,
  getRepoRoot,
} from '../utils/git.js'

const LOG_FILE = process.env.MAPROOM_MCP_LOG_FILE
const log = LOG_FILE
  ? pino({ level: process.env.LOG_LEVEL || 'info' }, pino.destination(LOG_FILE))
  : pino({ level: process.env.LOG_LEVEL || 'info' }, pino.destination(2))

/**
 * Default configuration for Open tool
 */
const DEFAULT_CONFIG: OpenToolConfig = {
  maxFileSize: 1024 * 1024, // 1MB
}

/**
 * Get worktree absolute path from database
 * @param client - PostgreSQL client
 * @param worktreeName - Name of worktree
 * @param relpath - Relative path to file (for validation)
 * @returns Absolute path to worktree
 * @throws Error if worktree not found
 */
async function getWorktreePath(
  client: Client,
  worktreeName: string,
  relpath: string
): Promise<string> {
  const { rows } = await client.query(
    `SELECT w.abs_path
     FROM maproom.worktrees w
     JOIN maproom.files f ON f.worktree_id = w.id
     WHERE f.relpath = $1 AND w.name = $2
     ORDER BY w.id DESC`,
    [relpath, worktreeName]
  )

  if (rows.length === 0) {
    // Provide helpful error message with available worktrees
    const availableWorktrees = await client.query(
      'SELECT DISTINCT w.name FROM maproom.worktrees w JOIN maproom.files f ON f.worktree_id = w.id WHERE f.relpath = $1',
      [relpath]
    )

    if (availableWorktrees.rows.length > 0) {
      throw new ValidationError(
        `File exists in other worktrees: ${availableWorktrees.rows.map((r) => r.name).join(', ')}. Check your worktree parameter.`,
        'FILE_NOT_FOUND'
      )
    } else {
      throw new ValidationError(
        `File '${relpath}' not found in worktree '${worktreeName}'. Use search tool to find the correct path.`,
        'FILE_NOT_FOUND'
      )
    }
  }

  // Try each candidate until we find one that exists
  for (const row of rows) {
    const fullPath = path.join(row.abs_path, relpath)
    if (await fileExists(fullPath)) {
      return row.abs_path as string
    }
  }

  // All candidates failed
  throw new ValidationError(
    `File '${relpath}' not accessible in worktree '${worktreeName}'. ` +
    `Tried ${rows.length} candidate paths but none exist on disk. ` +
    `This may indicate database pollution. Run 'maproom db cleanup-stale' to fix.`,
    'FILE_NOT_FOUND'
  )
}

/**
 * Read file from filesystem
 * @param worktreePath - Absolute path to worktree
 * @param relpath - Relative path to file
 * @param config - Configuration options
 * @returns File contents as string
 * @throws ValidationError if file not found or too large
 */
async function readFileFromFilesystem(
  worktreePath: string,
  relpath: string,
  config: OpenToolConfig
): Promise<string> {
  const absolutePath = path.join(worktreePath, relpath)

  // Validate file is within worktree boundaries
  validateWithinRepo(absolutePath, worktreePath)

  // Check file size before reading
  await validateFileSize(absolutePath, config.maxFileSize)

  try {
    const content = await fs.readFile(absolutePath, 'utf8')
    return content
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      throw new ValidationError(
        `File not found: ${relpath}`,
        'FILE_NOT_FOUND'
      )
    } else if (error.code === 'EISDIR') {
      throw new ValidationError(
        `Path is a directory, not a file: ${relpath}`,
        'INVALID_PATH'
      )
    }
    throw new ValidationError(
      `Failed to read file: ${error.message}`,
      'FILE_READ_ERROR'
    )
  }
}

/**
 * Execute Open tool handler
 * @param params - Tool parameters
 * @param client - PostgreSQL client
 * @param config - Configuration options
 * @returns FileContent object
 * @throws ValidationError for invalid parameters or file access errors
 */
export async function handleOpenTool(
  params: unknown,
  client: Client,
  config: OpenToolConfig = DEFAULT_CONFIG
): Promise<FileContent> {
  // Validate parameters with Zod schema
  const validatedParams = validateOpenParams(params)
  const { relpath, range, worktree, commit } = validatedParams

  log.debug({ relpath, range, worktree, commit }, 'handleOpenTool called')

  // Validate and normalize path to prevent traversal attacks
  const normalizedPath = validatePath(relpath)

  let content: string

  // Determine how to retrieve the file content
  if (commit) {
    // Check if commit is currently checked out
    const checkedOut = await isCommitCheckedOut(commit)

    if (checkedOut && worktree) {
      // Commit is checked out - read from filesystem for better performance
      log.debug({ commit, worktree }, 'Reading from filesystem (commit checked out)')
      const worktreePath = await getWorktreePath(client, worktree, normalizedPath)
      content = await readFileFromFilesystem(worktreePath, normalizedPath, config)
    } else {
      // Commit is not checked out - use git show
      log.debug({ commit }, 'Reading from git history (commit not checked out)')

      // Get repo root for git operations
      let repoRoot: string | undefined
      if (worktree) {
        const worktreePath = await getWorktreePath(client, worktree, normalizedPath)
        repoRoot = worktreePath
      }

      try {
        content = await getFileFromGit(commit, normalizedPath, repoRoot)
      } catch (error: any) {
        throw new ValidationError(
          error.message,
          'GIT_ERROR'
        )
      }
    }
  } else if (worktree) {
    // No commit specified - read from current worktree
    log.debug({ worktree }, 'Reading from current worktree')
    const worktreePath = await getWorktreePath(client, worktree, normalizedPath)
    content = await readFileFromFilesystem(worktreePath, normalizedPath, config)
  } else {
    // No worktree or commit specified
    throw new ValidationError(
      'Either worktree or commit parameter must be specified',
      'INVALID_PARAMS'
    )
  }

  // Extract line range if requested
  if (range) {
    log.debug({ range }, 'Extracting line range')
    const validated = validateRange(content, range.start, range.end)
    content = extractRange(content, validated.start, validated.end)
  }

  const result: FileContent = {
    content,
    relpath: normalizedPath,
    ...(range && { range }),
  }

  log.debug({ relpath: normalizedPath, hasRange: !!range }, 'handleOpenTool completed')

  return result
}

/**
 * Error response helper for MCP protocol
 * @param error - Error object
 * @returns MCP-formatted error response
 */
export function formatOpenError(error: unknown): any {
  if (error instanceof ValidationError) {
    return {
      isError: true,
      content: [
        {
          type: 'text',
          text: JSON.stringify(
            {
              error: error.code,
              message: error.message,
            },
            null,
            2
          ),
        },
      ],
    }
  }

  // Generic error
  const message = error instanceof Error ? error.message : String(error)
  return {
    isError: true,
    content: [
      {
        type: 'text',
        text: JSON.stringify(
          {
            error: 'INTERNAL_ERROR',
            message,
          },
          null,
          2
        ),
      },
    ],
  }
}
