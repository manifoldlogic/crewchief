/**
 * Explain Tool - Generate symbol cards for code chunks
 *
 * Provides detailed explanations of code symbols with:
 * - Chunk metadata (file path, line numbers, language)
 * - Symbol relationships (imports, exports, calls)
 * - Code preview
 * - Usage examples (if available)
 * - Intelligent caching for performance
 */

import { Client } from 'pg'
import pino from 'pino'
import path from 'node:path'
import fs from 'node:fs/promises'
import type { ExplainParams, ExplainToolConfig } from '../types.js'
import { validateExplainParams } from './explain_schema.js'
import { ValidationError } from '../utils/validation.js'
import {
  SymbolCard,
  formatSymbolCard,
  createEmptySymbolCard,
} from '../templates/symbol_card.js'
import { explainCache } from '../utils/cache.js'

const LOG_FILE = process.env.MAPROOM_MCP_LOG_FILE
const log = LOG_FILE
  ? pino({ level: process.env.LOG_LEVEL || 'info' }, pino.destination(LOG_FILE))
  : pino({ level: process.env.LOG_LEVEL || 'info' }, pino.destination(2))

/**
 * Default configuration for Explain tool
 */
const DEFAULT_CONFIG: ExplainToolConfig = {
  enabled: false, // Disabled by default (experimental)
  cacheTtlMs: 5 * 60 * 1000, // 5 minutes
}

/**
 * Query chunk details from database
 * @param client - PostgreSQL client
 * @param chunkId - Chunk ID
 * @returns Chunk data with file and worktree information
 */
async function queryChunkDetails(client: Client, chunkId: number) {
  const { rows } = await client.query(
    `SELECT
      c.id,
      c.symbol_name,
      c.kind::text,
      c.start_line,
      c.end_line,
      c.signature,
      c.docstring,
      c.preview,
      c.metadata,
      f.relpath,
      f.language,
      w.name as worktree_name,
      w.abs_path as worktree_path
    FROM maproom.chunks c
    JOIN maproom.files f ON f.id = c.file_id
    JOIN maproom.worktrees w ON w.id = f.worktree_id
    WHERE c.id = $1`,
    [chunkId]
  )

  if (rows.length === 0) {
    throw new ValidationError(
      `Chunk not found with id ${chunkId}. Use search tool to find valid chunk_id values.`,
      'CHUNK_NOT_FOUND'
    )
  }

  return rows[0]
}

/**
 * Query relationships for a chunk
 * @param client - PostgreSQL client
 * @param chunkId - Chunk ID
 * @returns Relationships grouped by type
 */
async function queryChunkRelationships(client: Client, chunkId: number) {
  // Query outgoing edges (imports, exports, calls)
  const { rows: outgoing } = await client.query(
    `SELECT
      ce.type::text as edge_type,
      c.symbol_name,
      f.relpath
    FROM maproom.chunk_edges ce
    JOIN maproom.chunks c ON c.id = ce.dst_chunk_id
    JOIN maproom.files f ON f.id = c.file_id
    WHERE ce.src_chunk_id = $1`,
    [chunkId]
  )

  // Query incoming edges (called_by)
  const { rows: incoming } = await client.query(
    `SELECT
      ce.type::text as edge_type,
      c.symbol_name,
      f.relpath
    FROM maproom.chunk_edges ce
    JOIN maproom.chunks c ON c.id = ce.src_chunk_id
    JOIN maproom.files f ON f.id = c.file_id
    WHERE ce.dst_chunk_id = $1`,
    [chunkId]
  )

  // Query test links
  const { rows: tests } = await client.query(
    `SELECT
      c.symbol_name,
      f.relpath
    FROM maproom.test_links tl
    JOIN maproom.chunks c ON c.id = tl.test_chunk_id
    JOIN maproom.files f ON f.id = c.file_id
    WHERE tl.target_chunk_id = $1`,
    [chunkId]
  )

  // Group relationships
  const relationships = {
    imports: [] as Array<{ symbol_name: string; relpath: string }>,
    exports: [] as Array<{ symbol_name: string; relpath: string }>,
    calls: [] as Array<{ symbol_name: string; relpath: string }>,
    called_by: [] as Array<{ symbol_name: string; relpath: string }>,
    tests: [] as Array<{ symbol_name: string; relpath: string }>,
  }

  for (const row of outgoing) {
    const rel = { symbol_name: row.symbol_name || 'unknown', relpath: row.relpath }
    switch (row.edge_type) {
      case 'imports':
        relationships.imports.push(rel)
        break
      case 'exports':
        relationships.exports.push(rel)
        break
      case 'calls':
        relationships.calls.push(rel)
        break
    }
  }

  for (const row of incoming) {
    const rel = { symbol_name: row.symbol_name || 'unknown', relpath: row.relpath }
    if (row.edge_type === 'calls') {
      relationships.called_by.push(rel)
    }
  }

  for (const row of tests) {
    relationships.tests.push({
      symbol_name: row.symbol_name || 'unknown',
      relpath: row.relpath,
    })
  }

  return relationships
}

/**
 * Load code preview from file
 * @param worktreePath - Absolute path to worktree
 * @param relpath - Relative file path
 * @param startLine - Start line number (1-indexed)
 * @param endLine - End line number (1-indexed)
 * @returns Code preview content and line count
 */
async function loadCodePreview(
  worktreePath: string,
  relpath: string,
  startLine: number,
  endLine: number
): Promise<{ content: string; line_count: number }> {
  const absolutePath = path.join(worktreePath, relpath)

  try {
    const fileContent = await fs.readFile(absolutePath, 'utf8')
    const lines = fileContent.split('\n')
    const previewLines = lines.slice(startLine - 1, endLine)
    return {
      content: previewLines.join('\n'),
      line_count: previewLines.length,
    }
  } catch (error: any) {
    // Fall back to preview from database if file read fails
    log.warn(
      { error: error.message, relpath, worktreePath },
      'Failed to read file for preview, using database preview'
    )
    return {
      content: '(Preview not available - file may have been moved or deleted)',
      line_count: endLine - startLine + 1,
    }
  }
}

/**
 * Generate symbol card from chunk data
 * @param client - PostgreSQL client
 * @param chunkId - Chunk ID
 * @returns Symbol card object
 */
async function generateSymbolCard(
  client: Client,
  chunkId: number
): Promise<SymbolCard> {
  const chunk = await queryChunkDetails(client, chunkId)
  const relationships = await queryChunkRelationships(client, chunkId)

  // Load code preview
  const preview = await loadCodePreview(
    chunk.worktree_path,
    chunk.relpath,
    chunk.start_line,
    chunk.end_line
  )

  // Build metadata object
  const metadata: any = {
    language: chunk.language,
  }

  // Extract additional metadata from JSONB metadata field
  if (chunk.metadata) {
    if (chunk.metadata.parent_heading) {
      metadata.parent_context = chunk.metadata.parent_heading
    }
    if (chunk.metadata.visibility) {
      metadata.visibility = chunk.metadata.visibility
    }
    // Include any other metadata
    for (const [key, value] of Object.entries(chunk.metadata)) {
      if (!['parent_heading', 'visibility'].includes(key)) {
        metadata[key] = value
      }
    }
  }

  const card: SymbolCard = {
    chunk: {
      id: chunk.id,
      symbol_name: chunk.symbol_name,
      kind: chunk.kind,
      start_line: chunk.start_line,
      end_line: chunk.end_line,
    },
    location: {
      relpath: chunk.relpath,
      worktree: chunk.worktree_name,
    },
    metadata,
    preview,
    relationships,
  }

  return card
}

/**
 * Execute Explain tool handler
 * @param params - Tool parameters
 * @param client - PostgreSQL client
 * @param config - Configuration options
 * @returns Formatted markdown explanation
 */
export async function handleExplainTool(
  params: unknown,
  client: Client,
  config: ExplainToolConfig = DEFAULT_CONFIG
): Promise<string> {
  // Check if tool is enabled
  if (!config.enabled) {
    throw new ValidationError(
      'Explain tool is currently disabled (experimental feature). Enable it in configuration to use.',
      'TOOL_DISABLED'
    )
  }

  // Validate parameters
  const validatedParams = validateExplainParams(params)
  const { chunk_id } = validatedParams

  log.debug({ chunk_id }, 'handleExplainTool called')

  // Check cache first
  const cacheKey = `explain:${chunk_id}`
  const cached = explainCache.get(cacheKey)

  if (cached) {
    log.debug({ chunk_id, cacheKey }, 'Cache hit for explain tool')
    return cached
  }

  log.debug({ chunk_id, cacheKey }, 'Cache miss, generating symbol card')

  // Generate symbol card
  const card = await generateSymbolCard(client, chunk_id)

  // Format as markdown
  const markdown = formatSymbolCard(card)

  // Cache the result
  explainCache.set(cacheKey, markdown, config.cacheTtlMs)
  log.debug({ chunk_id, cacheKey, size: markdown.length }, 'Cached explain result')

  return markdown
}

/**
 * Error response helper for MCP protocol
 * @param error - Error object
 * @returns MCP-formatted error response
 */
export function formatExplainError(error: unknown): any {
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
