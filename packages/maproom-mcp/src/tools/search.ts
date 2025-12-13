/**
 * Search Tool - Semantic code search via Rust binary subprocess
 *
 * Provides MCP tool interface for semantic code search by spawning the
 * crewchief-maproom Rust binary and parsing its JSON output.
 *
 * Architecture: Calls Rust binary to avoid duplicating FTS SQL logic in TypeScript.
 * This ensures single source of truth - Phase 2 enhancements only need Rust updates.
 *
 * ## Edge Case Handling (SEMRANK-2007)
 *
 * This module handles edge cases gracefully:
 *
 * 1. **Empty Query**: Validated by Zod schema (search_schema.ts) with .min(1) constraint.
 *    - Empty strings, whitespace-only, null, undefined → ValidationError
 *    - Error message: "query is required and cannot be empty"
 *
 * 2. **NULL symbol_name**: Handled in Rust FTS executor (fts.rs:137-140)
 *    - Documentation/markdown chunks may have NULL symbol_name
 *    - CASE ELSE clause applies exact_mult = 1.0 (neutral, no boost, no crash)
 *
 * 3. **Unknown/NULL kind**: Handled in Rust FTS executor (fts.rs:152-154)
 *    - Unknown kinds from parsing edge cases or future tree-sitter updates
 *    - CASE ELSE clause applies kind_mult = 1.0 (neutral baseline)
 *    - Explicit NULL handler: WHEN c.kind IS NULL THEN 1.0
 *
 * 4. **Multi-word Queries**: Normalized via normalizeForExactMatch() function
 *    - "HTTP handler" → "http_handler"
 *    - "validate HTTP request" → "validate_http_request"
 *    - Enables exact match detection across different naming conventions
 *
 * 5. **Special Characters**: Protected by parameterized queries
 *    - All SQL queries use $1, $2, $3... placeholders (not string concatenation)
 *    - Prevents SQL injection: "'; DROP TABLE;" is treated as literal text
 *    - No crashes from quotes, backslashes, Unicode, or other special chars
 *
 * 6. **Graceful Degradation**: No crashes for missing data
 *    - No matches → Empty hits array (not error)
 *    - Missing chunk IDs → chunk_id = 0 with warning log
 *    - Non-existent repo → Helpful error message with troubleshooting hint
 *
 * All edge cases are tested in tests/integration/semrank-edge-cases.test.ts
 */

import { Client } from 'pg'
import pino from 'pino'
import type { SearchParams, SearchResult, SearchBundle } from '../types.js'
import { validateSearchParams } from './search_schema.js'
import { ValidationError } from '../utils/validation.js'
import { ProcessError } from '../utils/process.js'
import { getDaemonClient } from '../daemon.js'
import {
  DaemonError,
  DaemonStartError,
  DaemonTimeoutError,
  RpcError,
} from '../daemon-client/index.js'

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
 * Rust binary search result format (SEMRANK-2006: includes optional debug fields)
 */
interface RustSearchHit {
  score: number
  file_relpath: string
  symbol_name: string | null
  kind: string
  start_line: number
  end_line: number
  base_score?: number
  kind_mult?: number
  exact_mult?: number
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
 * @param client - Database client
 * @param repo - Repository name
 * @param hits - Search hits from Rust binary
 * @returns Map of "relpath:start_line:end_line" -> chunk_id
 * @deprecated Legacy function for PostgreSQL. Not used with SQLite backend.
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
 * @param client - Database client (legacy, not used with SQLite)
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
  const { query, repo, worktree, limit, mode, debug, deduplicate } = validatedParams

  log.debug({ query, repo, worktree, limit, mode, debug, deduplicate }, 'handleSearchTool called')

  // SEMRANK-2006: Permission check for debug mode
  // If auth system exists in the future, check user.hasPermission('debug_mode')
  // For now, log warning when debug is enabled (acceptable for MVP, metadata not sensitive)
  if (debug) {
    log.warn(
      'Debug mode enabled without permission check. ' +
        'In production, this should verify user.hasPermission("debug_mode"). ' +
        'Score breakdown metadata is not sensitive for MVP.'
    )
  }

  // Validate repo parameter is provided
  if (!repo) {
    throw new ValidationError(
      'repo parameter is required for search',
      'MISSING_REPO'
    )
  }

  // Note: Rust binary only supports FTS mode currently
  // Vector and hybrid modes are handled by TypeScript SQL in index.ts
  if (!['fts', 'vector'].includes(mode)) {
    throw new ValidationError(
      `Search mode "${mode}" not supported. Use mode="fts" or mode="vector".`,
      'UNSUPPORTED_MODE'
    )
  }

  // ============================================================================
  // MIGRATION NOTE (DAEMIGR-2001):
  // Replaced process spawning with daemon client for 20-50x performance improvement.
  // Old spawning code preserved in utils/process.ts for VSCode extension use.
  // ============================================================================

  log.debug({ mode, query, repo, worktree, limit, debug }, 'Calling daemon for search')

  // Get daemon client singleton
  const daemon = getDaemonClient()

  // Call daemon search method
  let daemonResult: Awaited<ReturnType<typeof daemon.search>>
  try {
    daemonResult = await daemon.search({
      query,
      repo,
      worktree,
      limit,
      // Note: mode parameter not yet supported by daemon (Phase 2 enhancement)
      // Daemon uses hybrid search by default
      debug,
      deduplicate,
    })
  } catch (error) {
    // Convert daemon errors to MCP-friendly errors
    if (error instanceof DaemonStartError) {
      throw new ProcessError(
        `Failed to start maproom daemon: ${error.message}\n\nTroubleshooting:\n1. Ensure crewchief-maproom binary is installed\n2. Check MAPROOM_DATABASE_URL environment variable\n3. Verify database is running and accessible`,
        'DAEMON_START_FAILED'
      )
    }

    if (error instanceof DaemonTimeoutError) {
      throw new ProcessError(
        `Search request timed out: ${error.message}\n\nTroubleshooting:\n1. Check database connectivity\n2. Verify network is not slow\n3. Try reducing search scope with filters`,
        'SEARCH_TIMEOUT'
      )
    }

    if (error instanceof RpcError) {
      // Check for repository not found error (same as old spawning logic)
      if (error.message.includes('query returned an unexpected number of rows')) {
        throw new ValidationError(
          `Repository '${repo}' not found or no data indexed.`,
          'REPO_NOT_FOUND'
        )
      }

      throw new ProcessError(
        `Daemon RPC error: ${error.message}`,
        'RPC_ERROR'
      )
    }

    if (error instanceof DaemonError) {
      throw new ProcessError(
        `Daemon error: ${error.message}`,
        'DAEMON_ERROR'
      )
    }

    // Unknown error type
    throw new ProcessError(
      `Search failed: ${error instanceof Error ? error.message : String(error)}`,
      'SEARCH_FAILED'
    )
  }

  // Transform daemon result to RustSearchOutput format
  // Daemon returns { hits: [...], total, ... } but hits have different field names
  const rustOutput: RustSearchOutput = {
    hits: daemonResult.hits.map((hit) => ({
      file_relpath: hit.file_path, // Daemon uses file_path, MCP expects file_relpath
      start_line: hit.start_line,
      end_line: hit.end_line,
      symbol_name: hit.symbol_name || '', // Daemon provides symbol_name directly
      kind: hit.kind, // Daemon provides kind directly
      score: hit.score,
      // Debug fields not yet available from daemon
      base_score: undefined,
      kind_mult: undefined,
      exact_mult: undefined,
    })),
  }

  log.debug({ hitCount: rustOutput.hits.length }, 'Received search results from daemon')

  // Transform Rust hits to SearchResult format (SEMRANK-2006: include score_breakdown if debug=true)
  const hits: SearchResult[] = rustOutput.hits.map((hit, index) => {
    const daemonHit = daemonResult.hits[index]

    // Daemon provides chunk_id directly
    const chunk_id = daemonHit.chunk_id

    if (!chunk_id || chunk_id === 0) {
      log.warn({ hit: daemonHit }, 'Invalid chunk_id in search result')
    }

    // Build SearchResult with optional score_breakdown (SEMRANK-2006)
    const result: SearchResult = {
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

    // Add score breakdown if debug mode enabled and fields are present
    if (debug && hit.base_score !== undefined && hit.kind_mult !== undefined && hit.exact_mult !== undefined) {
      result.score_breakdown = {
        base_fts: hit.base_score,
        kind_multiplier: hit.kind_mult,
        exact_match_multiplier: hit.exact_mult,
        final: hit.score,
      }
    }

    return result
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
  // Check for RpcError with structured details (SRCHTRN-1005)
  if (error instanceof RpcError) {
    const details = error.getDetails()

    if (details) {
      // Format structured error with error type, stage, context, and suggestions
      return {
        isError: true,
        content: [
          {
            type: 'text',
            text: JSON.stringify(
              {
                error: details.error_type,
                stage: details.stage,
                message: error.message,
                context: details.context,
                suggestions: details.suggestions,
              },
              null,
              2
            ),
          },
        ],
      }
    }

    // Fallback to existing error handling if no details available
    // (backward compatibility)
    return {
      isError: true,
      content: [
        {
          type: 'text',
          text: JSON.stringify(
            {
              error: 'RPC_ERROR',
              message: error.message,
            },
            null,
            2
          ),
        },
      ],
    }
  }

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
                      ? 'Use mode="fts" for keyword search or mode="vector" for semantic search.'
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
