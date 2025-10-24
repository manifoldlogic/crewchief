/**
 * Context Tool - Retrieve contextually relevant code sections
 *
 * Assembles a ContextBundle with the target chunk plus related context
 * (imports, callers, callees, tests, etc.) within a token budget.
 *
 * This implementation integrates with the Rust context assembler via
 * direct database access and file loading, matching the assembler's logic.
 */

import path from 'node:path'
import fs from 'node:fs/promises'
import { Client } from 'pg'
import pino from 'pino'
import type { ContextParams, ExpandOptions } from './context_schema.js'
import { validateContextParams } from './context_schema.js'
import { ValidationError } from '../utils/validation.js'

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
  role: 'primary' | 'caller' | 'callee' | 'test' | 'doc' | 'config' | 'import'
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
    worktree: string
    expand_options: ExpandOptions
  }
  warnings?: string[]
}

/**
 * Chunk metadata from database
 */
interface ChunkMetadata {
  id: number
  file_relpath: string
  worktree_name: string
  worktree_path: string
  symbol_name: string | null
  kind: string
  start_line: number
  end_line: number
  signature: string | null
  docstring: string | null
}

/**
 * Estimate token count from text
 * Uses rough approximation: ~4 characters per token for code
 * This matches the Rust implementation's simple estimation
 */
function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4)
}

/**
 * Extract line range from file content
 */
function extractLines(content: string, start: number, end: number): string {
  const lines = content.split('\n')
  return lines.slice(start - 1, end).join('\n')
}

/**
 * Retrieve chunk metadata from database
 */
async function getChunkMetadata(
  client: Client,
  chunkId: number
): Promise<ChunkMetadata | null> {
  const { rows } = await client.query(
    `
    SELECT
      c.id,
      f.relpath as file_relpath,
      w.name as worktree_name,
      w.abs_path as worktree_path,
      c.symbol_name,
      c.kind::text,
      c.start_line,
      c.end_line,
      c.metadata->>'signature' as signature,
      c.metadata->>'docstring' as docstring
    FROM maproom.chunks c
    JOIN maproom.files f ON f.id = c.file_id
    JOIN maproom.worktrees w ON w.id = f.worktree_id
    WHERE c.id = $1
  `,
    [chunkId]
  )

  if (rows.length === 0) {
    return null
  }

  const row = rows[0]
  return {
    id: row.id,
    file_relpath: row.file_relpath,
    worktree_name: row.worktree_name,
    worktree_path: row.worktree_path,
    symbol_name: row.symbol_name,
    kind: row.kind,
    start_line: row.start_line,
    end_line: row.end_line,
    signature: row.signature,
    docstring: row.docstring,
  }
}

/**
 * Read file content from filesystem
 */
async function readFileContent(
  worktreePath: string,
  relpath: string
): Promise<string> {
  const absolutePath = path.join(worktreePath, relpath)

  try {
    const content = await fs.readFile(absolutePath, 'utf8')
    return content
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      throw new ValidationError(
        `File not found: ${relpath}. File may have been moved or deleted since indexing.`,
        'FILE_NOT_FOUND'
      )
    }
    throw new ValidationError(
      `Failed to read file: ${error.message}`,
      'FILE_READ_ERROR'
    )
  }
}

/**
 * Get related chunks via relationships
 * This queries the maproom.relationships table to find connected chunks
 */
async function getRelatedChunks(
  client: Client,
  chunkId: number,
  expand: ExpandOptions
): Promise<Array<{ chunk_id: number; relationship_type: string; score: number }>> {
  if (!expand.callers && !expand.callees && !expand.tests && !expand.docs && !expand.config) {
    // No expansion requested
    return []
  }

  // Build relationship type filters
  const relationshipTypes: string[] = []
  if (expand.callers) {
    relationshipTypes.push('calls', 'invokes', 'uses')
  }
  if (expand.callees) {
    relationshipTypes.push('called_by', 'invoked_by', 'used_by')
  }
  if (expand.tests) {
    relationshipTypes.push('tests', 'tested_by')
  }
  if (expand.docs) {
    relationshipTypes.push('documents', 'documented_by')
  }
  if (expand.config) {
    relationshipTypes.push('configures', 'configured_by')
  }

  if (relationshipTypes.length === 0) {
    return []
  }

  // Query relationships
  // Note: The relationships table may not exist yet in all environments
  // We'll handle this gracefully by checking if the table exists first
  try {
    const { rows } = await client.query(
      `
      SELECT
        CASE
          WHEN from_chunk_id = $1 THEN to_chunk_id
          ELSE from_chunk_id
        END as chunk_id,
        relationship_type,
        1.0 as score
      FROM maproom.relationships
      WHERE (from_chunk_id = $1 OR to_chunk_id = $1)
        AND relationship_type = ANY($2::text[])
      LIMIT 20
    `,
      [chunkId, relationshipTypes]
    )

    return rows.map((row) => ({
      chunk_id: row.chunk_id,
      relationship_type: row.relationship_type,
      score: parseFloat(row.score),
    }))
  } catch (error: any) {
    // If relationships table doesn't exist, just log and return empty
    if (error.code === '42P01') {
      // undefined_table
      log.debug('Relationships table does not exist yet, skipping relationship expansion')
      return []
    }
    throw error
  }
}

/**
 * Assemble context bundle for a chunk
 */
export async function handleContextTool(
  params: unknown,
  client: Client
): Promise<ContextBundle> {
  // Validate parameters
  const validatedParams = validateContextParams(params)
  const { chunk_id, budget_tokens, expand } = validatedParams

  log.debug({ chunk_id, budget_tokens, expand }, 'handleContextTool called')

  // Parse chunk_id to integer
  const chunkIdNum = parseInt(chunk_id, 10)

  // Retrieve chunk metadata
  const chunkMetadata = await getChunkMetadata(client, chunkIdNum)
  if (!chunkMetadata) {
    throw new ValidationError(
      `Chunk not found with id ${chunkIdNum}. Verify the chunk_id from search results.`,
      'CHUNK_NOT_FOUND'
    )
  }

  // Read file content
  const fileContent = await readFileContent(
    chunkMetadata.worktree_path,
    chunkMetadata.file_relpath
  )

  // Extract primary chunk content
  const primaryContent = extractLines(
    fileContent,
    chunkMetadata.start_line,
    chunkMetadata.end_line
  )
  const primaryTokens = estimateTokens(primaryContent)

  // Initialize bundle
  const bundle: ContextBundle = {
    items: [],
    total_tokens: 0,
    budget_tokens,
    budget_remaining: budget_tokens,
    truncated: false,
    metadata: {
      chunk_id: chunkIdNum,
      worktree: chunkMetadata.worktree_name,
      expand_options: expand,
    },
    warnings: [],
  }

  // Add primary chunk
  const primaryItem: ContextItem = {
    relpath: chunkMetadata.file_relpath,
    range: {
      start: chunkMetadata.start_line,
      end: chunkMetadata.end_line,
    },
    role: 'primary',
    reason: 'Target chunk requested by user',
    content: primaryContent,
    tokens: primaryTokens,
    symbol_name: chunkMetadata.symbol_name || undefined,
    kind: chunkMetadata.kind,
  }

  bundle.items.push(primaryItem)
  bundle.total_tokens = primaryTokens
  bundle.budget_remaining = budget_tokens - primaryTokens

  // Check if primary chunk alone exceeds budget
  if (primaryTokens > budget_tokens) {
    bundle.truncated = true
    bundle.warnings?.push(
      `Primary chunk (${primaryTokens} tokens) exceeds budget (${budget_tokens} tokens). Only primary chunk included.`
    )
    log.warn({ chunkIdNum, primaryTokens, budget_tokens }, 'Primary chunk exceeds budget')
    return bundle
  }

  // Get related chunks if expansion is enabled
  const relatedChunks = await getRelatedChunks(client, chunkIdNum, expand)

  log.debug({ relatedCount: relatedChunks.length }, 'Found related chunks')

  // Add related chunks within budget
  for (const related of relatedChunks) {
    // Check if we have budget remaining
    if (bundle.budget_remaining <= 100) {
      // Reserve at least 100 tokens for meaningful content
      bundle.truncated = true
      bundle.warnings?.push(
        `Budget exhausted after ${bundle.items.length} items. Some related context omitted.`
      )
      break
    }

    // Fetch related chunk metadata
    const relatedMetadata = await getChunkMetadata(client, related.chunk_id)
    if (!relatedMetadata) {
      log.warn({ chunk_id: related.chunk_id }, 'Related chunk not found, skipping')
      continue
    }

    // Read related file content
    let relatedFileContent: string
    try {
      relatedFileContent = await readFileContent(
        relatedMetadata.worktree_path,
        relatedMetadata.file_relpath
      )
    } catch (error) {
      log.warn(
        { chunk_id: related.chunk_id, error },
        'Failed to read related chunk file, skipping'
      )
      continue
    }

    // Extract related chunk content
    const relatedContent = extractLines(
      relatedFileContent,
      relatedMetadata.start_line,
      relatedMetadata.end_line
    )
    const relatedTokens = estimateTokens(relatedContent)

    // Check if adding this chunk would exceed budget
    if (bundle.total_tokens + relatedTokens > budget_tokens) {
      bundle.truncated = true
      bundle.warnings?.push(
        `Budget limit reached. ${relatedChunks.length - bundle.items.length + 1} related chunks omitted.`
      )
      break
    }

    // Determine role based on relationship type
    let role: ContextItem['role'] = 'caller'
    if (related.relationship_type.includes('test')) {
      role = 'test'
    } else if (related.relationship_type.includes('doc')) {
      role = 'doc'
    } else if (related.relationship_type.includes('config')) {
      role = 'config'
    } else if (
      related.relationship_type.includes('called_by') ||
      related.relationship_type.includes('invoked_by')
    ) {
      role = 'callee'
    } else if (related.relationship_type.includes('import')) {
      role = 'import'
    }

    // Add related chunk
    const relatedItem: ContextItem = {
      relpath: relatedMetadata.file_relpath,
      range: {
        start: relatedMetadata.start_line,
        end: relatedMetadata.end_line,
      },
      role,
      reason: `Related via ${related.relationship_type} relationship`,
      content: relatedContent,
      tokens: relatedTokens,
      symbol_name: relatedMetadata.symbol_name || undefined,
      kind: relatedMetadata.kind,
    }

    bundle.items.push(relatedItem)
    bundle.total_tokens += relatedTokens
    bundle.budget_remaining = budget_tokens - bundle.total_tokens
  }

  log.debug(
    {
      chunk_id: chunkIdNum,
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
                    : 'Check your parameters and try again.',
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
