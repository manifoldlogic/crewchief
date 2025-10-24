/**
 * Upsert Tool - Re-index files in the semantic search database
 *
 * Provides file indexing capabilities by spawning the crewchief-maproom binary:
 * - Process spawning with argument validation
 * - Progress tracking from binary output
 * - Error handling and reporting
 * - Binary location discovery (env var, platform paths, system PATH)
 */

import pino from 'pino'
import type { UpsertParams, UpsertResult, UpsertToolConfig } from '../types.js'
import { validateUpsertParams } from './upsert_schema.js'
import { validatePath, ValidationError } from '../utils/validation.js'
import {
  getBinarycandidates,
  trySpawnWithCandidates,
  parseIndexingStats,
  ProcessError,
  type SpawnOptions,
} from '../utils/process.js'

const LOG_FILE = process.env.MAPROOM_MCP_LOG_FILE
const log = LOG_FILE
  ? pino({ level: process.env.LOG_LEVEL || 'info' }, pino.destination(LOG_FILE))
  : pino({ level: process.env.LOG_LEVEL || 'info' }, pino.destination(2))

/**
 * Default configuration for Upsert tool
 */
const DEFAULT_CONFIG: UpsertToolConfig = {
  timeout: 120000, // 2 minutes default timeout
}

/**
 * Validate paths array to prevent path traversal and other attacks
 * @param paths - Array of paths to validate
 * @returns Array of validated paths
 * @throws ValidationError if any path is invalid
 */
function validatePaths(paths: string[]): string[] {
  return paths.map((p) => {
    try {
      return validatePath(p)
    } catch (error) {
      if (error instanceof ValidationError) {
        throw error
      }
      throw new ValidationError(
        `Invalid path "${p}": ${error instanceof Error ? error.message : String(error)}`,
        'INVALID_PATH'
      )
    }
  })
}

/**
 * Execute Upsert tool handler
 * @param params - Tool parameters
 * @param config - Configuration options
 * @returns UpsertResult with indexing statistics
 * @throws ValidationError for invalid parameters
 * @throws ProcessError for process execution failures
 */
export async function handleUpsertTool(
  params: unknown,
  config: UpsertToolConfig = DEFAULT_CONFIG
): Promise<UpsertResult> {
  // Validate parameters with Zod schema
  const validatedParams = validateUpsertParams(params)
  const { paths, commit, repo, worktree, root } = validatedParams

  log.debug({ paths, commit, repo, worktree, root }, 'handleUpsertTool called')

  // Validate all paths to prevent security issues
  let validatedPaths: string[]
  try {
    validatedPaths = validatePaths(paths)
  } catch (error) {
    if (error instanceof ValidationError) {
      throw error
    }
    throw new ValidationError(
      `Path validation failed: ${error instanceof Error ? error.message : String(error)}`,
      'INVALID_PARAMS'
    )
  }

  // Build arguments for crewchief-maproom binary
  const args = [
    'upsert',
    '--paths',
    validatedPaths.join(','),
    '--commit',
    commit,
    '--repo',
    repo,
    '--worktree',
    worktree,
    '--root',
    root,
  ]

  log.debug({ args }, 'Spawning indexer process')

  // Get binary candidates to try
  const candidates = getBinarycandidates()

  if (candidates.length === 0) {
    throw new ProcessError(
      'No crewchief-maproom binary found.\n\nTroubleshooting:\n1. Build the binary: cargo build --release --bin crewchief-maproom\n2. Set CREWCHIEF_MAPROOM_BIN environment variable\n3. Add binary to system PATH',
      'BINARY_NOT_FOUND'
    )
  }

  log.debug({ candidates: candidates.map((c) => c.cmd) }, 'Binary candidates to try')

  // Spawn process with timeout
  const spawnOptions: SpawnOptions = {
    timeout: config.timeout || DEFAULT_CONFIG.timeout,
    env: config.env,
    captureStdout: true,
    captureStderr: true,
  }

  let processResult
  try {
    processResult = await trySpawnWithCandidates(candidates, args, spawnOptions)
  } catch (error) {
    if (error instanceof ProcessError) {
      log.error({ error: error.message, code: error.code }, 'Process execution failed')

      // Enhance error messages for common issues
      if (error.code === 'ENOENT') {
        throw new ProcessError(
          'crewchief-maproom binary not found.\n\nTroubleshooting:\n1. Build the binary: cargo build --release --bin crewchief-maproom\n2. Set CREWCHIEF_MAPROOM_BIN environment variable to binary path\n3. Ensure binary is in system PATH',
          'BINARY_NOT_FOUND',
          error.exitCode,
          error.stderr
        )
      } else if (error.code === 'TIMEOUT') {
        throw new ProcessError(
          `Indexing operation timed out after ${config.timeout}ms. Try indexing fewer files or increasing the timeout.`,
          'TIMEOUT',
          error.exitCode,
          error.stderr
        )
      }

      // Re-throw original error if not enhanced
      throw error
    }

    // Unknown error type
    throw new ProcessError(
      `Unexpected error during indexing: ${error instanceof Error ? error.message : String(error)}`,
      'UNKNOWN_ERROR'
    )
  }

  log.debug(
    { exitCode: processResult.code, stdoutLength: processResult.stdout.length },
    'Process completed successfully'
  )

  // Parse indexing statistics from output
  const stats = parseIndexingStats(processResult.stdout)

  log.debug({ stats }, 'Parsed indexing statistics')

  // Build result with parsed statistics
  // Provide defaults if parsing failed
  const result: UpsertResult = {
    updated_files: stats.files ?? 0,
    updated_chunks: stats.chunks ?? 0,
    duration_ms: stats.duration_ms ?? 0,
  }

  // Warn if we couldn't parse statistics
  if (stats.files === undefined || stats.chunks === undefined) {
    log.warn(
      { stdout: processResult.stdout },
      'Could not parse all indexing statistics from output'
    )
  }

  log.info({ result }, 'Upsert completed successfully')

  return result
}

/**
 * Error response helper for MCP protocol
 * @param error - Error object
 * @returns MCP-formatted error response
 */
export function formatUpsertError(error: unknown): any {
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
              hint: 'Check that all paths are valid relative paths within the repository',
            },
            null,
            2
          ),
        },
      ],
    }
  }

  if (error instanceof ProcessError) {
    return {
      isError: true,
      content: [
        {
          type: 'text',
          text: JSON.stringify(
            {
              error: error.code,
              message: error.message,
              exitCode: error.exitCode,
              stderr: error.stderr,
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
