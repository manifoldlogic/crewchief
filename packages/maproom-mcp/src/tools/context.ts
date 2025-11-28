/**
 * Context Tool - Retrieve contextually relevant code sections
 *
 * Assembles a ContextBundle with the target chunk plus related context
 * (imports, callers, callees, tests, etc.) within a token budget.
 *
 * This implementation uses the daemon client to communicate with the
 * Rust context assembler, avoiding PostgreSQL duplication.
 *
 * Migration note: CTXCLI-3002 replaced direct PostgreSQL queries with
 * daemon client calls for 20-50x performance improvement.
 */

import pino from 'pino'
import type { ContextParams as SchemaContextParams, ExpandOptions } from './context_schema.js'
import { validateContextParams } from './context_schema.js'
import { ValidationError } from '../utils/validation.js'
import { ProcessError } from '../utils/process.js'
import { getDaemonClient } from '../daemon.js'
import {
  DaemonError,
  DaemonStartError,
  DaemonTimeoutError,
  RpcError,
  type RustContextBundle,
  type RustContextItem,
} from '../daemon-client/index.js'

const LOG_FILE = process.env.MAPROOM_MCP_LOG_FILE
const log = LOG_FILE
  ? pino({ level: process.env.LOG_LEVEL || 'info' }, pino.destination(LOG_FILE))
  : pino({ level: process.env.LOG_LEVEL || 'info' }, pino.destination(2))

/**
 * A single item in a context bundle
 */
export interface ContextItem {
  relpath: string
  range: {
    start: number
    end: number
  }
  role: 'primary' | 'caller' | 'callee' | 'test' | 'doc' | 'config' | 'import' | 'hook' | 'jsx_parent' | 'jsx_child' | string
  reason: string
  content: string
  tokens: number
  symbol_name?: string
  kind?: string
}

/**
 * Context bundle with primary chunk and related context
 */
export interface ContextBundle {
  items: ContextItem[]
  total_tokens: number
  budget_tokens: number
  budget_remaining: number
  truncated: boolean
  metadata: {
    chunk_id: number
    worktree?: string
    expand_options: ExpandOptions
  }
  warnings?: string[]
}

/**
 * Map Rust context item to MCP context item format
 */
function mapRustItemToMcp(item: RustContextItem): ContextItem {
  return {
    relpath: item.relpath,
    range: {
      start: item.range.start,
      end: item.range.end,
    },
    role: item.role as ContextItem['role'],
    reason: item.reason,
    content: item.content,
    tokens: item.tokens,
  }
}

/**
 * Map Rust context bundle to MCP context bundle format
 *
 * The Rust daemon returns a simpler ContextBundle structure.
 * This function adds the MCP-specific computed fields:
 * - budget_tokens (from request params)
 * - budget_remaining (computed)
 * - metadata (from request params)
 */
function mapRustToMcpBundle(
  rustBundle: RustContextBundle,
  requestParams: SchemaContextParams,
): ContextBundle {
  const budgetTokens = requestParams.budget_tokens
  const expand = requestParams.expand

  return {
    items: rustBundle.items.map(mapRustItemToMcp),
    total_tokens: rustBundle.total_tokens,
    budget_tokens: budgetTokens,
    budget_remaining: budgetTokens - rustBundle.total_tokens,
    truncated: rustBundle.truncated,
    metadata: {
      chunk_id: parseInt(requestParams.chunk_id, 10),
      expand_options: expand,
    },
  }
}

/**
 * Assemble context bundle for a chunk using daemon client
 */
export async function handleContextTool(
  params: unknown,
): Promise<ContextBundle> {
  // Validate parameters
  const validatedParams = validateContextParams(params)
  const { chunk_id, budget_tokens, expand } = validatedParams

  log.debug({ chunk_id, budget_tokens, expand }, 'handleContextTool called')

  // Get daemon client singleton
  const daemon = getDaemonClient()

  // Call daemon context method
  let rustBundle: RustContextBundle
  try {
    rustBundle = await daemon.context({
      chunk_id,
      budget_tokens,
      expand: {
        callers: expand.callers,
        callees: expand.callees,
        tests: expand.tests,
        docs: expand.docs,
        config: expand.config,
        max_depth: expand.max_depth,
        routes: expand.routes,
        hooks: expand.hooks,
        jsx_parents: expand.jsx_parents,
        jsx_children: expand.jsx_children,
      },
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
        `Context request timed out: ${error.message}\n\nTroubleshooting:\n1. Check database connectivity\n2. Verify network is not slow\n3. Try reducing budget_tokens`,
        'CONTEXT_TIMEOUT'
      )
    }

    if (error instanceof RpcError) {
      // Check for chunk not found error (error code -32000)
      if (error.rpcCode === -32000 || error.message.includes('not found')) {
        throw new ValidationError(
          `Chunk not found with id ${chunk_id}. Verify the chunk_id from search results.`,
          'CHUNK_NOT_FOUND'
        )
      }

      // Check for invalid params (error code -32602)
      if (error.isInvalidParams()) {
        throw new ValidationError(
          `Invalid parameters: ${error.message}`,
          'INVALID_PARAMS'
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
      `Context assembly failed: ${error instanceof Error ? error.message : String(error)}`,
      'CONTEXT_FAILED'
    )
  }

  log.debug(
    {
      chunk_id,
      items: rustBundle.items.length,
      total_tokens: rustBundle.total_tokens,
      truncated: rustBundle.truncated,
    },
    'Received context bundle from daemon'
  )

  // Map Rust bundle to MCP format
  const bundle = mapRustToMcpBundle(rustBundle, validatedParams)

  log.debug(
    {
      chunk_id,
      items: bundle.items.length,
      total_tokens: bundle.total_tokens,
      truncated: bundle.truncated,
    },
    'Context bundle assembled'
  )

  return bundle
}

/**
 * Format context tool error for MCP protocol
 */
export function formatContextError(error: unknown): any {
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
                error.code === 'CHUNK_NOT_FOUND'
                  ? 'Use the search tool to find valid chunks and get their chunk_id values.'
                  : error.code === 'FILE_NOT_FOUND'
                    ? 'Try re-indexing with the upsert tool if files have been moved or deleted.'
                    : error.code === 'INVALID_PARAMS'
                      ? 'Check chunk_id format (must be positive integer) and budget_tokens range (1000-20000).'
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
                error.code === 'DAEMON_START_FAILED'
                  ? 'Check that the crewchief-maproom binary is installed and database is accessible.'
                  : error.code === 'CONTEXT_TIMEOUT'
                    ? 'The request timed out. Try reducing budget_tokens or checking database performance.'
                    : 'Context assembly failed. Check logs for details.',
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
              hint: 'Check parameter types and constraints. chunk_id must be a positive integer, budget_tokens must be between 1000 and 20000.',
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
