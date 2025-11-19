/**
 * Search Tool - Semantic code search via Rust binary subprocess
 *
 * Provides MCP tool interface for semantic code search by spawning the
 * crewchief-maproom Rust binary and parsing its JSON output.
 *
 * Architecture: Calls Rust binary to avoid duplicating FTS SQL logic in TypeScript.
 * This ensures single source of truth - Phase 2 enhancements only need Rust updates.
 */

import { Client } from 'pg'
import pino from 'pino'
import type { SearchParams, SearchResult, SearchBundle } from '../types.js'
import { validateSearchParams } from './search_schema.js'
import { ValidationError } from '../utils/validation.js'
import {
  getBinarycandidates,
  trySpawnWithCandidates,
  ProcessError,
  type ProcessResult,
} from '../utils/process.js'

const LOG_FILE = process.env.MAPROOM_MCP_LOG_FILE
const log = LOG_FILE
  ? pino({ level: process.env.LOG_LEVEL || 'info' }, pino.destination(LOG_FILE))
  : pino({ level: process.env.LOG_LEVEL || 'info' }, pino.destination(2))

/**
 * Normalize query for exact match detection
 *
 * Handles acronym-aware camelCase to snake_case conversion:
 * - "validateProvider" → "validate_provider"
 * - "XMLParser" → "xml_parser"
 * - "validateHTTPRequest" → "validate_http_request"
 * - "HTTPSHandler" → "https_handler"
 * - "Base64Encoder" → "base64_encoder"
 * - "validate-provider" → "validate_provider"
 *
 * @param query - Original search query
 * @returns Normalized snake_case query for ILIKE matching
 */
export function normalizeForExactMatch(query: string): string {
  let normalized = query

  // Step 1: Handle consecutive uppercase (acronyms) before lowercase
  // "XMLParser" → "XML_Parser", "HTTPSHandler" → "HTTPS_Handler"
  normalized = normalized.replace(/([A-Z]+)([A-Z][a-z])/g, '$1_$2')

  // Step 2: Handle transition from lowercase to multiple capitals (acronym after lowercase)
  // "validateHTTP" → "validate_HTTP"
  normalized = normalized.replace(/([a-z\d])([A-Z]{2,})/g, '$1_$2')

  // Step 3: Handle camelCase → snake_case (single capital after lowercase)
  // "validateProvider" → "validate_Provider"
  normalized = normalized.replace(/([a-z\d])([A-Z])/g, '$1_$2')

  // Step 4: Handle kebab-case, spaces, and dots → snake_case
  normalized = normalized.replace(/[\s\-\.]/g, '_')

  // Step 5: Lowercase everything
  normalized = normalized.toLowerCase()

  // Step 6: Clean up multiple/trailing/leading underscores
  normalized = normalized.replace(/_+/g, '_').replace(/^_|_$/g, '')

  return normalized
}

/**
 * Rust binary search result format
 */
interface RustSearchHit {
  score: number
  file_relpath: string
  symbol_name: string | null
  kind: string
  start_line: number
  end_line: number
}

/**
 * Rust binary output format
 */
interface RustSearchOutput {
  hits: RustSearchHit[]
}

/**
 * Fetch chunk IDs from database based on file path and line range
 *
 * The Rust binary doesn't return chunk IDs, so we need to query them
 * from the database using relpath + start_line + end_line.
 *
 * @param client - PostgreSQL client
 * @param repo - Repository name
 * @param hits - Search hits from Rust binary
 * @returns Map of "relpath:start_line:end_line" -> chunk_id
 */
async function fetchChunkIds(
  client: Client,
  repo: string,
  hits: RustSearchHit[]
): Promise<Map<string, number>> {
  if (hits.length === 0) {
    return new Map()
  }

  // Build composite keys for lookup
  const keys = hits.map(
    (hit) => `${hit.file_relpath}:${hit.start_line}:${hit.end_line}`
  )

  // Query chunk IDs for all hits in one database call
  const query = `
    SELECT c.id, f.relpath, c.start_line, c.end_line
    FROM maproom.chunks c
    JOIN maproom.files f ON f.id = c.file_id
    JOIN maproom.repos r ON r.id = f.repo_id
    WHERE r.name = $1
      AND (f.relpath, c.start_line, c.end_line) IN (${hits.map((_, i) => `($${i * 3 + 2}, $${i * 3 + 3}, $${i * 3 + 4})`).join(', ')})
  `

  const params = [
    repo,
    ...hits.flatMap((hit) => [hit.file_relpath, hit.start_line, hit.end_line]),
  ]

  try {
    const { rows } = await client.query(query, params)

    const idMap = new Map<string, number>()
    for (const row of rows) {
      const key = `${row.relpath}:${row.start_line}:${row.end_line}`
      idMap.set(key, row.id)
    }

    return idMap
  } catch (error) {
    log.error({ error, repo, hitCount: hits.length }, 'Failed to fetch chunk IDs')
    // Return empty map rather than failing - chunk_id will be 0 for these hits
    return new Map()
  }
}

/**
 * Execute search tool handler
 *
 * Calls the Rust binary via subprocess to perform FTS search, then enriches
 * results with chunk IDs from the database.
 *
 * @param params - Tool parameters
 * @param client - PostgreSQL client (for chunk ID lookup)
 * @returns SearchBundle with results
 * @throws ValidationError for invalid parameters
 * @throws ProcessError for binary execution failures
 */
export async function handleSearchTool(
  params: unknown,
  client: Client
): Promise<SearchBundle> {
  // Validate parameters with Zod schema
  const validatedParams = validateSearchParams(params)
  const { query, repo, worktree, limit, mode, debug } = validatedParams

  log.debug({ query, repo, worktree, limit, mode, debug }, 'handleSearchTool called')

  // Validate repo parameter is provided
  if (!repo) {
    throw new ValidationError(
      'repo parameter is required for search',
      'MISSING_REPO'
    )
  }

  // Note: Rust binary only supports FTS mode currently
  // Vector and hybrid modes are handled by TypeScript SQL in index.ts
  if (mode !== 'fts') {
    throw new ValidationError(
      `Search mode "${mode}" not supported by Rust binary. Only "fts" mode is available via binary. Use TypeScript implementation for vector/hybrid modes.`,
      'UNSUPPORTED_MODE'
    )
  }

  // Get binary candidates to try
  const candidates = getBinarycandidates()
  if (candidates.length === 0) {
    throw new ProcessError(
      'No crewchief-maproom binary found.\n\nTroubleshooting:\n1. Build the binary: cargo build --release --bin crewchief-maproom\n2. Set CREWCHIEF_MAPROOM_BIN environment variable\n3. Add binary to system PATH',
      'BINARY_NOT_FOUND'
    )
  }

  log.debug({ candidates }, 'Found binary candidates')

  // Build command arguments
  const args = [
    'search',
    '--repo',
    repo,
    '--query',
    query,
    '--k',
    String(limit),
  ]

  if (worktree) {
    args.push('--worktree', worktree)
  }

  // Note: Rust binary doesn't support --debug flag yet
  // Debug mode will need to be added to Rust in Phase 2

  log.debug({ args }, 'Spawning Rust binary for search')

  // Spawn Rust binary and collect output
  let result: ProcessResult
  try {
    result = await trySpawnWithCandidates(candidates, args, {
      timeout: 30000, // 30 second timeout for search
      captureStdout: true,
      captureStderr: true,
    })
  } catch (error) {
    if (error instanceof ProcessError) {
      // Handle specific error cases
      if (error.stderr?.includes('query returned an unexpected number of rows')) {
        throw new ValidationError(
          `Repository '${repo}' not found or no data indexed.`,
          'REPO_NOT_FOUND'
        )
      }
      throw error
    }
    throw new ProcessError(
      `Failed to execute search: ${error instanceof Error ? error.message : String(error)}`,
      'EXECUTION_FAILED'
    )
  }

  // Parse JSON output from Rust
  let rustOutput: RustSearchOutput
  try {
    rustOutput = JSON.parse(result.stdout)
  } catch (error) {
    log.error({ stdout: result.stdout }, 'Failed to parse Rust output as JSON')
    throw new ProcessError(
      'Failed to parse search results from Rust binary. Output was not valid JSON.',
      'PARSE_ERROR',
      undefined,
      result.stderr
    )
  }

  // Fetch chunk IDs from database
  const chunkIdMap = await fetchChunkIds(client, repo, rustOutput.hits)

  // Transform Rust hits to SearchResult format
  const hits: SearchResult[] = rustOutput.hits.map((hit) => {
    const key = `${hit.file_relpath}:${hit.start_line}:${hit.end_line}`
    const chunk_id = chunkIdMap.get(key) || 0

    if (chunk_id === 0) {
      log.warn({ hit }, 'Chunk ID not found for search result')
    }

    return {
      chunk_id,
      symbol_name: hit.symbol_name,
      kind: hit.kind,
      relpath: hit.file_relpath,
      start_line: hit.start_line,
      end_line: hit.end_line,
      score: hit.score,
      // preview field not available from Rust binary yet
      // Will be added in Phase 2
    }
  })

  const bundle: SearchBundle = {
    hits,
    total: hits.length,
    query,
    mode,
    repo,
    worktree,
  }

  log.debug(
    {
      query,
      hits: hits.length,
      mode,
      repo,
      worktree,
    },
    'Search completed successfully via Rust binary'
  )

  return bundle
}

/**
 * Error response helper for MCP protocol
 * @param error - Error object
 * @returns MCP-formatted error response
 */
export function formatSearchError(error: unknown): any {
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
              hint:
                error.code === 'REPO_NOT_FOUND'
                  ? 'Use the status tool to see available repositories, or use scan tool to index a new repository.'
                  : error.code === 'MISSING_REPO'
                    ? 'The repo parameter is required. Example: {repo: "crewchief", query: "search"}'
                    : error.code === 'UNSUPPORTED_MODE'
                      ? 'Currently only "fts" mode is supported via Rust binary. Vector and hybrid modes coming in Phase 2.'
                      : 'Check your parameters and try again.',
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
              hint:
                error.code === 'BINARY_NOT_FOUND'
                  ? 'Install the crewchief-maproom Rust binary or set CREWCHIEF_MAPROOM_BIN environment variable.'
                  : error.code === 'EXECUTION_FAILED'
                    ? 'The Rust binary failed to execute. Check that it is properly installed and has execute permissions.'
                    : 'Search execution failed. Check logs for details.',
              stderr: error.stderr,
            },
            null,
            2
          ),
        },
      ],
    }
  }

  // Validation errors from Zod
  if (error && typeof error === 'object' && 'issues' in error) {
    const zodError = error as any
    return {
      isError: true,
      content: [
        {
          type: 'text',
          text: JSON.stringify(
            {
              error: 'VALIDATION_ERROR',
              message: 'Invalid parameters',
              details: zodError.issues,
              hint: 'Check parameter types and constraints. query and repo are required, limit must be 1-100, mode must be "fts", "vector", or "hybrid".',
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
