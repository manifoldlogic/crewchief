import { Client } from 'pg'
import pino from 'pino'
import { spawn } from 'node:child_process'
import { Readable } from 'node:stream'
import path from 'node:path'
import fs from 'node:fs'

// IMPORTANT: Never write logs to stdout; MCP JSON-RPC must be the only stdout output.
// Route pino logs to stderr to avoid corrupting the protocol stream.
const LOG_FILE = process.env.MAPROOM_MCP_LOG_FILE
const log = LOG_FILE 
  ? pino({ level: process.env.LOG_LEVEL || 'info' }, pino.destination(LOG_FILE))
  : pino({ level: process.env.LOG_LEVEL || 'info' }, pino.destination(2))

type JsonRpcRequest = { jsonrpc: '2.0'; id?: number | string; method: string; params?: any }
type JsonRpcResponse = { jsonrpc: '2.0'; id: number | string | null; result?: any; error?: { code: number; message: string; data?: any } }

// MCP initialize response
function mcpInitialize() {
  return {
    protocolVersion: '2024-11-05',
    serverInfo: { 
      name: 'maproom-mcp', 
      version: '0.1.0',
      description: 'Semantic code search for indexed repositories. Start with status tool, then search with simple terms.'
    },
    // Advertise capabilities explicitly per MCP expectations
    capabilities: {
      tools: { listChanged: false },
      prompts: { listChanged: false },
      resources: { listChanged: false }
    }
  }
}

// Emit an initial server-info line for clients that probe stdout before JSON-RPC
// This is NOT a JSON-RPC message; it's a lightweight hint some clients look for.
const serverInfoProbe = {
  serverInfo: { name: 'maproom-mcp', version: '0.1.0' },
  protocolVersion: '2024-11-05',
  transports: ['stdio']
}

// Emit a structured server-info line to stderr early to satisfy UIs that probe logs
log.info(serverInfoProbe, 'server-info')

// Log startup environment for debugging
log.debug({
  env: {
    MAPROOM_DATABASE_URL: process.env.MAPROOM_DATABASE_URL ? '[SET]' : '[NOT SET]',
    LOG_LEVEL: process.env.LOG_LEVEL,
    MAPROOM_MCP_LOG_FILE: process.env.MAPROOM_MCP_LOG_FILE,
    NODE_ENV: process.env.NODE_ENV,
    CLIENT: process.env.MCP_CLIENT || process.env.CLAUDE_CODE_CLIENT || 'unknown'
  }
}, 'maproom-mcp startup environment')

// Discovery/inspect mode: if invoked with special flags or env, print server info to stdout and exit.
// This allows clients that probe for server metadata before starting JSON-RPC to succeed.
const probeFlags = new Set(['--server-info', '--stdio-info', '--mcp-server-info'])
const hasProbeFlag = process.argv.some((a) => probeFlags.has(a))
const hasProbeEnv = ['MCP_SERVER_INFO', 'CURSOR_MCP_SERVER_INFO', 'MCP_STDIO_INFO']
  .some((k) => process.env[k])
if (hasProbeFlag || hasProbeEnv) {
  const info = {
    serverInfo: serverInfoProbe.serverInfo,
    protocolVersion: serverInfoProbe.protocolVersion,
    transports: serverInfoProbe.transports,
    capabilities: { tools: true }
  }
  // Intentionally write to stdout and exit; this is a separate discovery run
  process.stdout.write(JSON.stringify(info) + '\n')
  process.exit(0)
}

// Prompt declarations for prompts/list - provides ready-made search patterns
const promptSchemas = [
  {
    name: 'find_main_entry',
    description: 'Find the main entry point of the application',
    arguments: [
      { name: 'repo', description: 'Repository name', required: true }
    ],
    prompt: 'Search for main entry point:\nrepo: {{repo}}\nquery: "main entry index.ts"'
  },
  {
    name: 'find_tests',
    description: 'Find test files for a specific feature',
    arguments: [
      { name: 'repo', description: 'Repository name', required: true },
      { name: 'feature', description: 'Feature to find tests for', required: true }
    ],
    prompt: 'Search for tests:\nrepo: {{repo}}\nquery: "{{feature}} test describe"\nfilter: "code"'
  },
  {
    name: 'understand_architecture',
    description: 'Get an overview of the codebase architecture',
    arguments: [
      { name: 'repo', description: 'Repository name', required: true }
    ],
    prompt: 'First run status to see structure, then search:\n1. repo: {{repo}} query: "README" filter: "docs"\n2. repo: {{repo}} query: "config" filter: "config"\n3. repo: {{repo}} query: "main class service"'
  },
  {
    name: 'find_error_handling',
    description: 'Find error handling patterns in the code',
    arguments: [
      { name: 'repo', description: 'Repository name', required: true }
    ],
    prompt: 'Search for error handling:\nrepo: {{repo}}\nquery: "error catch throw try"\nk: 20'
  }
]

// Tool declarations for tools/list
const toolSchemas = [
  {
    name: 'search',
    description: `Semantic code search optimized for AI agents - BEST FOR: finding functions/classes by concept, understanding code relationships, exploring unfamiliar codebases. FASTER THAN: Grep for conceptual searches. USE WHEN: searching for functionality rather than exact text matches.

🤖 AI AGENT QUERY FORMULATION:

Transform natural language questions into optimal search queries:

TRANSFORMATION PATTERNS:
1. Extract 2-3 core technical terms
2. Remove: how, what, where, when, why, does, is, are, the, a, an
3. Prefer code-like terminology

EXAMPLES:
  "How does authentication work?" → "authentication"
  "What handles errors?" → "error handler"
  "Find auth logic" → "authentication"
  "Where is WebSocket disconnect?" → "WebSocket disconnect"

QUERY BEST PRACTICES:
✅ Good: 2-3 words, concepts, code terms
  - "error handling"
  - "cart validation"
  - "WebSocket disconnect"
  - "processCheckout"

❌ Avoid: Full sentences, questions, special chars
  - "How do I handle errors in the checkout?"
  - "function_that_validates_cart_items"
  - "src/cart/checkout.ts"

MULTI-QUERY STRATEGY:
If first query returns <3 results, try variations:
  Query 1: "error handling"
  → <3 results?
  Query 2: "exception handler"
  → <3 results?
  Query 3: "try catch error"

SEARCH MODES:
- "fts" (full-text search): Best for exact keyword matches, identifiers
- "vector" (semantic search): Best for conceptual queries, similar code
- "hybrid" (default): Combines both for optimal results

⚠️ NOT FOR:
- Exact string matching: "TODO", "FIXME"
- File paths (use Glob instead)
- Very long queries (>4 words)

FILTERS: Narrow by file_type, recency, repo_id, worktree_id
DEBUG: Set debug=true to see score breakdowns`,
    inputSchema: {
      type: 'object',
      properties: {
        repo: { type: 'string', description: 'Repository name to search in (use "crewchief" for this codebase)' },
        worktree: { anyOf: [{ type: 'string' }, { type: 'null' }], description: 'Optional worktree name to limit search scope' },
        query: { type: 'string', description: 'Search query - can be concepts, function names, or multiple terms. Works best with 1-3 words. Examples: "maproom search", "worktree create", "message bus"' },
        k: { type: 'integer', minimum: 1, default: 10, description: 'Number of results to return (default: 10, max useful: 20)' },
        mode: {
          type: 'string',
          enum: ['fts', 'vector', 'hybrid'],
          default: 'hybrid',
          description: 'Search mode: "fts" for full-text keyword search, "vector" for semantic similarity, "hybrid" (default) for combined approach'
        },
        filter: {
          type: 'string',
          enum: ['all', 'code', 'docs', 'config'],
          default: 'all',
          description: 'Filter results by file type: all (default), code (ts/js/rs), docs (md/mdx), config (json/yaml/toml)'
        },
        filters: {
          type: 'object',
          description: 'Advanced filters for precise result targeting',
          properties: {
            repo_id: { type: 'integer', description: 'Filter by specific repository ID' },
            worktree_id: { type: 'integer', description: 'Filter by specific worktree ID' },
            file_type: { type: 'string', description: 'Filter by file extension (e.g., "ts", "rs", "md")' },
            recency_threshold: { type: 'string', description: 'Filter by file modification time (PostgreSQL interval, e.g., "7 days", "1 month")' }
          }
        },
        debug: {
          type: 'boolean',
          default: false,
          description: 'Enable debug mode to see score breakdowns (FTS, vector, graph signals, fusion method)'
        }
      },
      required: ['repo', 'query']
    }
  },
  {
    name: 'open',
    description: 'Retrieve specific code from indexed files - USE AFTER: getting search results. REQUIRES: exact relpath and worktree from search results. SUPPORTS: line ranges (from start_line/end_line in results) and context lines. TIP: Use the exact relpath and worktree values from search results.',
    inputSchema: {
      type: 'object',
      properties: {
        relpath: { type: 'string', description: 'Relative path to the file (copy exactly from search results)' },
        range: {
          type: 'object',
          description: 'Optional line range to retrieve (use start_line/end_line from search results)',
          properties: { start: { type: 'integer', minimum: 1 }, end: { type: 'integer', minimum: 1 } },
          required: []
        },
        context: { type: 'integer', minimum: 0, default: 0, description: 'Number of context lines to show before and after the range (try 5-10 for more context)' },
        worktree: { type: 'string', description: 'Worktree name where the file is located (copy exactly from search results or status)' }
      },
      required: ['relpath', 'worktree']
    }
  },
  {
    name: 'status',
    description: 'Get maproom index status - ALWAYS USE THIS FIRST before searching! Shows indexed repos, worktrees, statistics, and last update times. Tells you what\'s searchable and helps diagnose why searches might fail.',
    inputSchema: {
      type: 'object',
      properties: {
        repo: { type: 'string', description: 'Optional: filter status to specific repo (e.g., "crewchief")' }
      },
      required: []
    }
  },
  {
    name: 'scan',
    description: 'Scan and index an entire repository or worktree with automatic embedding generation - USE FOR: initial indexing of a new repository, re-indexing after major changes, or when you need to ensure all files are indexed. This is a comprehensive operation that discovers and indexes all supported files in the specified path. FASTER THAN: calling upsert on individual files. USE WHEN: setting up a new codebase for search, or when search results seem incomplete.\n\nMULTI-PROVIDER SUPPORT: Automatically detects and uses available embedding provider (Ollama, OpenAI, or Google Vertex AI). Provider selection is cached for session performance.',
    inputSchema: {
      type: 'object',
      properties: {
        repo: { type: 'string', description: 'Repository name (e.g., "crewchief"). Will auto-detect from git remote if not provided.' },
        worktree: { type: 'string', description: 'Worktree name (e.g., "main", "feature-branch"). Will auto-detect from current git branch if not provided.' },
        path: { type: 'string', description: 'Path to scan (absolute or relative). Defaults to current directory if not provided.' },
        commit: { type: 'string', description: 'Git commit hash (use "HEAD" for current). Defaults to HEAD if not provided.' },
        concurrency: { type: 'integer', minimum: 1, maximum: 16, default: 4, description: 'Number of concurrent file processing workers (default: 4)' },
        parallel: { type: 'boolean', default: false, description: 'Enable parallel batch processing for better performance with large codebases' },
        languages: { type: 'array', items: { type: 'string' }, description: 'Optional: limit to specific languages (e.g., ["typescript", "rust"])' },
        exclude: { type: 'array', items: { type: 'string' }, description: 'Optional: glob patterns to exclude (e.g., ["node_modules/**", "*.test.ts"])' }
      },
      required: []
    }
  },
  {
    name: 'upsert',
    description: 'Index/update specific files in maproom with automatic embedding generation - USE WHEN: files have changed and need reindexing. FOR FULL REPO: use "scan" instead. Only use upsert for targeted updates of a few specific files. Spawns the Rust indexer.\n\nMULTI-PROVIDER SUPPORT: Automatically detects and uses available embedding provider (Ollama, OpenAI, or Google Vertex AI). Provider selection is cached for session performance.',
    inputSchema: {
      type: 'object',
      properties: {
        paths: { type: 'array', items: { type: 'string' }, description: 'Array of file paths to index' },
        commit: { type: 'string', description: 'Git commit hash (use HEAD for current)' },
        repo: { type: 'string', description: 'Repository name (e.g., "crewchief")' },
        worktree: { type: 'string', description: 'Worktree name to index' },
        root: { type: 'string', description: 'Root directory path of the repository' }
      },
      required: ['paths', 'commit', 'repo', 'worktree', 'root']
    }
  },
  {
    name: 'context',
    description: 'Retrieve contextually relevant code sections around a given chunk. Assembles a ContextBundle with the target chunk plus related context (imports, callers, tests, etc.) within a token budget. USE AFTER: getting chunk_id from search results. BEST FOR: understanding code in context, gathering related functionality.',
    inputSchema: {
      type: 'object',
      properties: {
        chunk_id: { type: 'string', description: 'UUID of the target chunk to retrieve context for (from search results)' },
        budget_tokens: {
          type: 'integer',
          minimum: 1000,
          maximum: 20000,
          default: 6000,
          description: 'Maximum number of tokens to include in the context bundle (default: 6000, range: 1000-20000)'
        },
        expand: {
          type: 'object',
          description: 'Optional expansion configuration to control which related chunks to include',
          properties: {
            callers: { type: 'boolean', default: true, description: 'Include chunks that call this function' },
            callees: { type: 'boolean', default: true, description: 'Include chunks called by this function' },
            tests: { type: 'boolean', default: true, description: 'Include test chunks for this code' },
            docs: { type: 'boolean', default: false, description: 'Include documentation chunks' },
            config: { type: 'boolean', default: false, description: 'Include related configuration files' },
            max_depth: { type: 'integer', minimum: 1, maximum: 5, default: 2, description: 'Maximum relationship traversal depth' }
          }
        }
      },
      required: ['chunk_id']
    }
  },
  {
    name: 'explain',
    description: 'Generate a detailed symbol card for a code chunk. Provides markdown-formatted explanation with metadata, relationships, code preview, and usage examples. USE AFTER: getting chunk_id from search results. EXPERIMENTAL: Must be enabled in configuration. Uses intelligent caching for performance.',
    inputSchema: {
      type: 'object',
      properties: {
        chunk_id: {
          anyOf: [{ type: 'string' }, { type: 'integer' }],
          description: 'Chunk ID to explain (from search results). Can be string or number.'
        }
      },
      required: ['chunk_id']
    }
  }
]

async function getPg(): Promise<Client> {
  // Default to maproom-postgres connection for zero-config experience
  const DEFAULT_DATABASE_URL = 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'
  const connectionString = process.env.MAPROOM_DATABASE_URL || process.env.PG_DATABASE_URL || DEFAULT_DATABASE_URL

  log.debug({ connectionString: connectionString.replace(/:[^@]+@/, ':***@') }, 'Connecting to database')
  const client = new Client({ connectionString })
  await client.connect()
  return client
}

async function getAvailableRepos(client: Client): Promise<string[]> {
  try {
    const { rows } = await client.query('SELECT DISTINCT name FROM maproom.repos ORDER BY name')
    return rows.map(r => r.name)
  } catch (err) {
    log.error({ err }, 'Failed to get available repos')
    return []
  }
}

async function handleStatus(params: any): Promise<any> {
  const { repo } = params
  const client = await getPg()
  try {
    let repoFilter = ''
    const args: any[] = []
    
    if (repo) {
      repoFilter = 'WHERE r.name = $1'
      args.push(repo)
    }
    
    // Get repo and worktree statistics
    const statsQuery = `
      SELECT 
        r.name as repo_name,
        w.name as worktree_name,
        w.abs_path,
        COUNT(DISTINCT f.id) as file_count,
        COUNT(DISTINCT c.id) as chunk_count,
        NOW() as last_indexed
      FROM maproom.repos r
      LEFT JOIN maproom.worktrees w ON w.repo_id = r.id
      LEFT JOIN maproom.files f ON f.worktree_id = w.id
      LEFT JOIN maproom.chunks c ON c.file_id = f.id
      ${repoFilter}
      GROUP BY r.name, w.name, w.abs_path
      ORDER BY r.name, w.name
    `
    
    const { rows } = await client.query(statsQuery, args)
    
    // Group by repo
    const repos: any = {}
    for (const row of rows) {
      if (!repos[row.repo_name]) {
        repos[row.repo_name] = {
          name: row.repo_name,
          worktrees: []
        }
      }
      if (row.worktree_name) {
        repos[row.repo_name].worktrees.push({
          name: row.worktree_name,
          path: row.abs_path,
          fileCount: parseInt(row.file_count),
          chunkCount: parseInt(row.chunk_count),
          lastIndexed: row.last_indexed
        })
      }
    }
    
    // Get file type statistics
    const fileTypesQuery = `
      SELECT 
        regexp_replace(f.relpath, '^.*\\.', '.') as extension,
        COUNT(*) as count
      FROM maproom.files f
      JOIN maproom.worktrees w ON w.id = f.worktree_id
      JOIN maproom.repos r ON r.id = w.repo_id
      ${repoFilter}
      GROUP BY extension
      ORDER BY count DESC
      LIMIT 10
    `
    
    const { rows: fileTypes } = await client.query(fileTypesQuery, args)
    
    const totalFiles = Object.values(repos).reduce((sum: number, repo: any) => 
      sum + repo.worktrees.reduce((wsum: number, wt: any) => wsum + wt.fileCount, 0), 0)
    const totalChunks = Object.values(repos).reduce((sum: number, repo: any) => 
      sum + repo.worktrees.reduce((wsum: number, wt: any) => wsum + wt.chunkCount, 0), 0)
    
    let hint = ''
    let nextStep: string | undefined

    if (Object.keys(repos).length === 0) {
      hint = '⚠️ No repositories indexed yet.\n\nTo get started:\n1. Use the scan tool to index a repository\n2. Then use search to find your code'
      nextStep = 'Run scan tool to index your first repository'
    } else if (totalFiles === 0) {
      hint = '⚠️ Repository found but no files indexed.\n\nTo fix:\n1. Run scan tool to index files in this repository\n2. Check that the path contains supported file types (.ts, .js, .rs, .md, etc.)'
      nextStep = 'Run scan tool to index files'
    } else {
      hint = `✓ Index ready! ${totalFiles} files and ${totalChunks} searchable chunks.\n\nCommon searches: "main function", "error handling", "database query"`
    }
    
    return {
      repos: Object.values(repos),
      fileTypes: fileTypes.map(ft => ({ extension: ft.extension, count: parseInt(ft.count) })),
      totalRepos: Object.keys(repos).length,
      totalFiles,
      totalChunks,
      hint,
      nextStep,
      searchTips: [
        'Use simple terms: "auth" instead of "authentication_handler"',
        'Search concepts: "message bus" or "event handling"',
        'Filter by type: use filter:"code" or filter:"docs"',
        'Default repo for this codebase: "crewchief"'
      ]
    }
  } finally {
    await client.end().catch(() => {})
  }
}

// Helper function to build filter WHERE clauses
function buildFilterClauses(filters: any, filter: string, args: any[]): string {
  let clauses = ''

  // Legacy file type filter
  if (filter !== 'all') {
    if (filter === 'code') {
      clauses += ` AND f.relpath NOT LIKE '%.md' AND f.relpath NOT LIKE '%.mdx' AND f.relpath NOT LIKE '%.json' AND f.relpath NOT LIKE '%.yaml' AND f.relpath NOT LIKE '%.yml'`
    } else if (filter === 'docs') {
      clauses += ` AND (f.relpath LIKE '%.md' OR f.relpath LIKE '%.mdx')`
    } else if (filter === 'config') {
      clauses += ` AND (f.relpath LIKE '%.json' OR f.relpath LIKE '%.yaml' OR f.relpath LIKE '%.yml' OR f.relpath LIKE '%.toml')`
    }
  }

  // Advanced filters
  if (filters.file_type) {
    args.push(`%.${filters.file_type}`)
    clauses += ` AND f.relpath LIKE $${args.length}`
  }

  if (filters.recency_threshold) {
    args.push(filters.recency_threshold)
    clauses += ` AND f.last_modified > NOW() - INTERVAL $${args.length}`
  }

  if (filters.repo_id) {
    args.push(filters.repo_id)
    clauses += ` AND f.repo_id = $${args.length}`
  }

  return clauses
}

// FTS-only search implementation
async function executeFtsSearch(
  client: any,
  query: string,
  repoId: number,
  worktreeId: number | null,
  k: number,
  filter: string,
  filters: any,
  debug: boolean
): Promise<{ rows: any[], debugInfo: any }> {
  const tsParts = String(query)
    .split(/\s+/)
    .filter(Boolean)
    .map((t) => `${t.replace(/'/g, '')}:*`)
    .join(' & ')

  const args: any[] = [repoId]
  let paramIndex = 2

  if (worktreeId) {
    args.push(worktreeId)
    paramIndex = 3
  }

  const tsQueryParam = paramIndex
  args.push(tsParts)

  let sql = `
    SELECT c.id, f.relpath, c.symbol_name, c.kind::text, c.start_line, c.end_line, c.metadata,
      c.recency_score, c.churn_score,
      CASE
        WHEN c.kind IN ('heading_1', 'heading_2') THEN
          ts_rank_cd(c.ts_doc, to_tsquery('simple', $${tsQueryParam})) * 2.0
        WHEN c.kind = 'heading_3' THEN
          ts_rank_cd(c.ts_doc, to_tsquery('simple', $${tsQueryParam})) * 1.5
        WHEN c.kind IN ('heading_4', 'heading_5', 'heading_6') THEN
          ts_rank_cd(c.ts_doc, to_tsquery('simple', $${tsQueryParam})) * 1.2
        WHEN c.kind = 'json_key' THEN
          ts_rank_cd(c.ts_doc, to_tsquery('simple', $${tsQueryParam})) * 1.3
        ELSE
          ts_rank_cd(c.ts_doc, to_tsquery('simple', $${tsQueryParam}))
      END AS fts_score
    FROM maproom.chunks c
    JOIN maproom.files f ON f.id = c.file_id
    WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $${tsQueryParam})
  `

  if (worktreeId) {
    sql += ' AND f.worktree_id = $2'
  }

  sql += buildFilterClauses(filters, filter, args)

  args.push(k)
  sql += ` ORDER BY fts_score DESC LIMIT $${args.length}`

  const { rows } = await client.query(sql, args)

  const debugInfo = debug ? {
    mode: 'fts',
    query_terms: tsParts,
    total_results: rows.length
  } : null

  return { rows, debugInfo }
}

// Vector-only search implementation
async function executeVectorSearch(
  client: any,
  query: string,
  repoId: number,
  worktreeId: number | null,
  k: number,
  filter: string,
  filters: any,
  debug: boolean
): Promise<{ rows: any[], debugInfo: any }> {
  // For vector search, we need to generate an embedding for the query
  // Since we don't have an embedding service integrated yet, we'll return an informative error

  // Check if any embeddings exist in code_embeddings table
  const { rows: embeddingCheck } = await client.query(
    'SELECT COUNT(*) as count FROM maproom.code_embeddings LIMIT 1'
  )

  if (embeddingCheck[0].count === '0') {
    throw new Error(
      'Vector search requires embeddings. No embeddings found in database.\n\n' +
      'To use vector search:\n' +
      '1. Generate embeddings using the embedding generation pipeline\n' +
      '2. Run: crewchief maproom:generate-embeddings\n\n' +
      'Falling back to FTS mode is recommended.'
    )
  }

  // TODO: Integrate with embedding service to generate query embedding
  // For now, return a placeholder response
  throw new Error(
    'Vector search requires query embedding generation.\n\n' +
    'This feature requires:\n' +
    '1. Integration with OpenAI text-embedding-3-small API\n' +
    '2. Query text → vector(1536) conversion\n\n' +
    'Use mode:"fts" or mode:"hybrid" as alternatives.\n' +
    'Vector search implementation is in progress (HYBRID_SEARCH-2001).'
  )
}

// Hybrid search implementation (FTS + Vector with RRF fusion)
async function executeHybridSearch(
  client: any,
  query: string,
  repoId: number,
  worktreeId: number | null,
  k: number,
  filter: string,
  filters: any,
  debug: boolean
): Promise<{ rows: any[], debugInfo: any }> {
  // For now, fall back to FTS until vector embedding service is integrated
  // This maintains backward compatibility while the hybrid search backend is being completed

  const result = await executeFtsSearch(client, query, repoId, worktreeId, k, filter, filters, debug)

  if (debug && result.debugInfo) {
    result.debugInfo.mode = 'hybrid (fts-only fallback)'
    result.debugInfo.note = 'Hybrid search falls back to FTS until vector embeddings are available. Full hybrid implementation with RRF fusion is in progress.'
  }

  return result
}

async function handleSearch(params: any): Promise<any> {
  const {
    repo,
    worktree,
    query,
    k = 10,
    filter = 'all',
    mode = 'hybrid',
    filters = {},
    debug = false
  } = params

  const client = await getPg()
  try {
    // Validate mode parameter
    if (!['fts', 'vector', 'hybrid'].includes(mode)) {
      return {
        hits: [],
        error: 'Invalid search mode',
        hint: `Mode must be one of: "fts", "vector", "hybrid". Got: "${mode}"\n\nMode selection guide:\n- "fts": Full-text search for exact keywords\n- "vector": Semantic similarity search\n- "hybrid": Combined approach (recommended)`,
        suggestion: 'Use mode:"hybrid" for best results'
      }
    }

    const { rows: repoRows } = await client.query('SELECT id FROM maproom.repos WHERE name = $1', [repo])
    if (repoRows.length === 0) {
      const availableRepos = await getAvailableRepos(client)
      const suggestion = availableRepos.includes('crewchief') && repo.toLowerCase().includes('crew')
        ? 'Did you mean repo:"crewchief"?'
        : availableRepos.length > 0
          ? `Available repos: ${availableRepos.join(', ')}`
          : 'No repos indexed yet. Use upsert tool to index files.'

      return {
        hits: [],
        error: 'Repository not found',
        hint: `Repository '${repo}' is not indexed.\n\nTo fix this:\n1. Run status tool to see available repos\n2. Run scan tool to index this repository\n3. Then search again`,
        availableRepos,
        suggestion,
        nextStep: 'Use the scan tool to index this repository before searching'
      }
    }
    const repoId = repoRows[0].id
    let worktreeId: number | null = null
    let worktreeInfo: any = null

    // Handle worktree filtering (from parameter or advanced filters)
    const targetWorktreeId = filters.worktree_id || null
    if (typeof worktree === 'string' && worktree.length > 0) {
      const { rows: wt } = await client.query('SELECT id, name FROM maproom.worktrees WHERE repo_id=$1 AND name=$2', [repoId, worktree])
      if (wt.length > 0) {
        worktreeId = wt[0].id
        worktreeInfo = wt[0]
      }
    } else if (targetWorktreeId) {
      worktreeId = targetWorktreeId
    }
    
    // Execute mode-specific search
    let rows: any[] = []
    let debugInfo: any = null

    if (mode === 'fts') {
      const result = await executeFtsSearch(client, query, repoId, worktreeId, k, filter, filters, debug)
      rows = result.rows
      debugInfo = result.debugInfo
    } else if (mode === 'vector') {
      const result = await executeVectorSearch(client, query, repoId, worktreeId, k, filter, filters, debug)
      rows = result.rows
      debugInfo = result.debugInfo
    } else {
      // hybrid mode
      const result = await executeHybridSearch(client, query, repoId, worktreeId, k, filter, filters, debug)
      rows = result.rows
      debugInfo = result.debugInfo
    }
    
    const result: any = {
      hits: rows.map((r) => {
        const hit: any = {
          chunk_id: r.id,
          relpath: r.relpath,
          symbol_name: r.symbol_name,
          kind: r.kind,
          start_line: r.start_line,
          end_line: r.end_line,
          score: Number(r.fts_score || r.vector_score || r.hybrid_score || 0)
        }

        // Add debug score breakdown if requested
        if (debug) {
          hit.debug = {
            fts_score: r.fts_score ? Number(r.fts_score) : null,
            vector_score: r.vector_score ? Number(r.vector_score) : null,
            recency_score: r.recency_score ? Number(r.recency_score) : null,
            churn_score: r.churn_score ? Number(r.churn_score) : null,
            final_score: hit.score
          }
        }

        // Add metadata context if available
        if (r.metadata) {
          if (r.metadata.parent_heading) {
            hit.parent_context = r.metadata.parent_heading
          }
          if (r.metadata.language) {
            hit.language = r.metadata.language
          }
        }

        // Add type information for better context
        if (r.kind.startsWith('heading_')) {
          hit.type = 'markdown'
          hit.heading_level = parseInt(r.kind.split('_')[1])
        } else if (r.relpath.endsWith('.md') || r.relpath.endsWith('.mdx')) {
          hit.type = 'markdown'
        } else if (r.relpath.endsWith('.json')) {
          hit.type = 'config'
        } else {
          hit.type = 'code'
        }

        return hit
      })
    }

    // Add debug info if requested
    if (debug && debugInfo) {
      result.debug = debugInfo
    }
    
    // Add comprehensive hints and suggestions for empty results
    if (rows.length === 0) {
      const suggestions = []
      const examples = []
      
      // Analyze the query to provide better suggestions
      const terms = query.split(/\s+/)
      const queryLength = query.length
      const termCount = terms.length
      
      // Query too complex
      if (termCount > 4) {
        suggestions.push(`Your query has ${termCount} terms. Try fewer terms (2-3 work best)`)
        suggestions.push(`Try just: "${terms.slice(0, 2).join(' ')}"`)
      }
      
      // Try individual terms for multi-word queries
      if (termCount > 1) {
        suggestions.push(`Try individual terms: "${terms[0]}" or "${terms[terms.length - 1]}"`)
      }
      
      // Suggest variations
      if (query.toLowerCase() !== query) {
        suggestions.push(`Try lowercase: "${query.toLowerCase()}"`)
      }
      
      // Check for common patterns that might need adjustment
      if (query.includes('_')) {
        suggestions.push(`Try without underscores: "${query.replace(/_/g, ' ')}"`)
      }
      if (query.includes('-')) {
        suggestions.push(`Try without hyphens: "${query.replace(/-/g, ' ')}"`)
      }
      if (query.includes('.')) {
        suggestions.push(`Try without dots: "${query.replace(/\./g, ' ')}"`)
      }
      
      // Suggest related conceptual searches
      if (query.includes('function') || query.includes('method')) {
        examples.push('Try searching for the action instead: "create", "update", "delete"')
      }
      if (query.includes('class') || query.includes('interface')) {
        examples.push('Try searching for the entity name: "Service", "Manager", "Controller"')
      }
      if (query.includes('test')) {
        examples.push('Try: "describe", "test(" or "spec"')
      }
      
      // Provide filter suggestions
      if (filter === 'all') {
        suggestions.push('Try filtering by type: add filter:"code" or filter:"docs"')
      }
      
      // Check index status suggestion
      const statusHint = 'Run the status tool first to see what\'s indexed and available for search'
      
      // Build comprehensive hint
      result.hint = worktreeInfo
        ? `No results in worktree '${worktree}'.\n\nPossible reasons:\n1. Files not indexed yet - use scan tool to index the repository\n2. Search terms too specific - try simpler terms\n3. Wrong worktree - check status tool`
        : `No results found for "${query}".\n\n${statusHint}\n\nSearch tips:\n• Use 1-3 word queries\n• Try conceptual terms: "authentication", "database", "error handling"\n• Separate words with spaces, not underscores\n• Start broad, then refine\n\nIf repository is not indexed: Use scan tool to index it first`
      
      if (suggestions.length > 0) {
        result.suggestions = suggestions
      }
      
      if (examples.length > 0) {
        result.examples = examples
      }
      
      // Add query analysis
      result.queryAnalysis = {
        termCount,
        queryLength,
        hasSpecialChars: /[_\-.()]/.test(query),
        recommendation: termCount > 3 ? 'simplify' : termCount === 1 ? 'try related terms' : 'good length'
      }
    } else if (rows.length === 1) {
      // Single result - suggest how to find more
      result.hint = 'Found 1 result. To find more: try broader terms or increase k parameter'
    } else if (rows.length === k) {
      // Hit the limit - suggest increasing k
      result.hint = `Showing top ${k} results. More may exist - increase k parameter to see more`
    }
    
    return result
  } finally {
    await client.end().catch(() => {})
  }
}

async function handleOpen(params: any): Promise<any> {
  const client = await getPg()
  try {
    const { handleOpenTool } = await import('./tools/open.js')
    const result = await handleOpenTool(params, client)
    return result
  } finally {
    await client.end().catch(() => {})
  }
}

async function handleScan(params: any): Promise<any> {
  const { spawn } = await import('node:child_process')

  try {
    // Detect provider before scanning
    const { getProviderConfig } = await import('./utils/provider-detection.js')
    let providerConfig
    try {
      providerConfig = await getProviderConfig()
      log.info({ provider: providerConfig.provider, dimension: providerConfig.dimension }, 'Scanning with provider')
    } catch (error: any) {
      if (error.message && error.message.includes('No embedding provider available')) {
        return {
          success: false,
          error: 'Cannot scan with embeddings: No provider available.\n' + error.message,
          hint: 'Configure an embedding provider to enable semantic search. See error message for setup instructions.'
        }
      }
      throw error
    }

    // Build command arguments
    const args: string[] = ['scan']

    if (params.repo) args.push('--repo', params.repo)
    if (params.worktree) args.push('--worktree', params.worktree)
    if (params.path) args.push('--path', params.path)
    if (params.commit) args.push('--commit', params.commit)
    if (params.concurrency) args.push('--concurrency', String(params.concurrency))
    if (params.parallel) args.push('--parallel')
    if (params.languages && Array.isArray(params.languages)) {
      params.languages.forEach((lang: string) => args.push('--languages', lang))
    }
    if (params.exclude && Array.isArray(params.exclude)) {
      params.exclude.forEach((pattern: string) => args.push('--exclude', pattern))
    }

    // Add provider flag
    args.push('--provider', providerConfig.provider)

    log.info({ args }, 'spawning crewchief-maproom scan')

    // Spawn the Rust binary
    const { findMaproomBinary } = await import('./utils/process.js')
    const binaryPath = await findMaproomBinary()

    if (!binaryPath) {
      throw new Error('Could not find crewchief-maproom binary')
    }

    const proc = spawn(binaryPath, args, {
      stdio: ['ignore', 'pipe', 'pipe'],
      env: { ...process.env }
    })

    let stdout = ''
    let stderr = ''

    proc.stdout?.on('data', (chunk: Buffer) => {
      stdout += chunk.toString()
    })

    proc.stderr?.on('data', (chunk: Buffer) => {
      stderr += chunk.toString()
      log.info({ line: chunk.toString().trim() }, 'scan output')
    })

    const exitCode = await new Promise<number>((resolve) => {
      proc.on('exit', (code: number | null) => resolve(code ?? 1))
    })

    if (exitCode !== 0) {
      log.error({ exitCode, stderr }, 'scan command failed')
      return {
        success: false,
        error: `Scan command failed with exit code ${exitCode}`,
        stderr: stderr.trim(),
        hint: 'Check that the path is a valid git repository and that you have the necessary permissions'
      }
    }

    // Parse output for statistics
    const lines = stderr.split('\n')
    const stats: any = {
      filesProcessed: 0,
      chunksCreated: 0,
      duration: null
    }

    for (const line of lines) {
      const filesMatch = line.match(/Processed (\d+) files/)
      if (filesMatch) stats.filesProcessed = parseInt(filesMatch[1], 10)

      const chunksMatch = line.match(/Created (\d+) chunks/)
      if (chunksMatch) stats.chunksCreated = parseInt(chunksMatch[1], 10)

      const durationMatch = line.match(/Completed in ([\d.]+)s/)
      if (durationMatch) stats.duration = durationMatch[1] + 's'
    }

    log.info({ stats }, 'scan completed')

    return {
      success: true,
      message: 'Repository scan completed successfully',
      stats,
      repo: params.repo || 'auto-detected',
      worktree: params.worktree || 'auto-detected',
      path: params.path || 'current directory',
      provider: providerConfig.provider,
      dimension: providerConfig.dimension,
      hint: 'Use the status tool to verify indexing results, then search to find your code'
    }
  } catch (error: any) {
    log.error({ error: error.message }, 'scan error')
    return {
      success: false,
      error: error.message,
      hint: 'Ensure crewchief-maproom binary is available and the path is a valid git repository'
    }
  }
}

async function handleUpsert(params: any): Promise<any> {
  try {
    const { handleUpsertTool, formatUpsertError } = await import('./tools/upsert.js')
    const result = await handleUpsertTool(params)
    return result
  } catch (error) {
    const { formatUpsertError } = await import('./tools/upsert.js')
    throw formatUpsertError(error)
  }
}

async function handleExplain(params: any): Promise<any> {
  // Check if explain tool is enabled via environment variable
  const explainEnabled = process.env.MAPROOM_EXPLAIN_ENABLED === 'true'

  const client = await getPg()
  try {
    const { handleExplainTool } = await import('./tools/explain.js')
    const result = await handleExplainTool(params, client, { enabled: explainEnabled })
    return result
  } finally {
    await client.end().catch(() => {})
  }
}

/**
 * handleContext - Retrieve contextually relevant code sections around a target chunk
 *
 * Integrates with the context assembler to provide intelligent context gathering
 * with relationship traversal, budget management, and multi-chunk assembly.
 */
async function handleContext(params: any): Promise<any> {
  const client = await getPg()
  try {
    const { handleContextTool } = await import('./tools/context.js')
    const result = await handleContextTool(params, client)
    return result
  } finally {
    await client.end().catch(() => {})
  }
}

async function streamToString(s: Readable): Promise<string> {
  const chunks: Buffer[] = []
  for await (const c of s) chunks.push(Buffer.from(c))
  return Buffer.concat(chunks).toString('utf8')
}

let useContentLengthFraming = false

function writeJson(json: any) {
  const payload = JSON.stringify(json)
  if (useContentLengthFraming) {
    const bytes = Buffer.byteLength(payload, 'utf8')
    process.stdout.write(`Content-Length: ${bytes}\r\n\r\n${payload}`)
  } else {
    process.stdout.write(payload + '\n')
  }
}

function respond(id: number | string | null | undefined, result?: any, error?: any) {
  // Per JSON-RPC, notifications (no id) MUST NOT be responded to
  // Only respond when id is a string or number
  if (!(typeof id === 'string' || typeof id === 'number')) return
  const resp: JsonRpcResponse = { jsonrpc: '2.0', id }
  if (error) resp.error = { code: -32000, message: String(error?.message || error), data: error?.stack }
  else resp.result = result
  writeJson(resp)
}

process.stdin.setEncoding('utf8')
let buf = ''

async function handleMessage(msg: JsonRpcRequest) {
  log.info({ id: msg.id, method: msg.method }, 'recv request')
  switch (msg.method) {
    case 'initialize':
      const init = mcpInitialize()
      respond(msg.id, init)
      log.info({ id: msg.id }, 'sent initialize result')
      return
    case 'notifications/initialized':
      // JSON-RPC notification from client after initialize; do not respond
      log.info({ method: msg.method }, 'client initialized')
      return
    case 'tools/list':
      // Provide both camelCase and snake_case keys on tool schemas
      respond(msg.id, { tools: toolSchemas })
      log.info({ id: msg.id, count: toolSchemas.length }, 'sent tools list')
      return
    case 'prompts/list':
      respond(msg.id, { prompts: promptSchemas })
      log.info({ id: msg.id, count: promptSchemas.length }, 'sent prompts list')
      return
    case 'prompts/get': {
      const name = msg.params?.name as string
      const prompt = promptSchemas.find(p => p.name === name)
      if (prompt) {
        respond(msg.id, { prompt })
      } else {
        respond(msg.id, undefined, new Error(`Unknown prompt: ${name}`))
      }
      return
    }
    case 'resources/list':
      // We don't define resources yet; return empty list
      respond(msg.id, { resources: [] })
      log.info({ id: msg.id, count: 0 }, 'sent resources list')
      return
    case 'tools/call': {
      const name = msg.params?.name as string
      const args = msg.params?.arguments || {}
      if (name === 'status') {
        const res = await handleStatus(args)
        respond(msg.id ?? null, { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] })
        log.info({ id: msg.id, tool: name }, 'sent tool result')
      } else if (name === 'search') {
        const res = await handleSearch(args)
        respond(msg.id ?? null, { content: [{ type: 'text', text: JSON.stringify(res) }] })
        log.info({ id: msg.id, tool: name }, 'sent tool result')
      } else if (name === 'open') {
        try {
          const res = await handleOpen(args)
          respond(msg.id ?? null, { content: [{ type: 'text', text: res.content }] })
          log.info({ id: msg.id, tool: name }, 'sent tool result')
        } catch (error) {
          const { formatOpenError } = await import('./tools/open.js')
          const errorResponse = formatOpenError(error)
          respond(msg.id ?? null, errorResponse)
          log.error({ id: msg.id, tool: name, error }, 'tool error')
        }
      } else if (name === 'upsert') {
        try {
          const res = await handleUpsert(args)
          respond(msg.id ?? null, { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] })
          log.info({ id: msg.id, tool: name }, 'sent tool result')
        } catch (error) {
          const { formatUpsertError } = await import('./tools/upsert.js')
          const errorResponse = formatUpsertError(error)
          respond(msg.id ?? null, errorResponse)
          log.error({ id: msg.id, tool: name, error }, 'tool error')
        }
      } else if (name === 'context') {
        try {
          const res = await handleContext(args)
          respond(msg.id ?? null, { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] })
          log.info({ id: msg.id, tool: name }, 'sent tool result')
        } catch (error) {
          const { formatContextError } = await import('./tools/context.js')
          const errorResponse = formatContextError(error)
          respond(msg.id ?? null, errorResponse)
          log.error({ id: msg.id, tool: name, error }, 'tool error')
        }
      } else if (name === 'scan') {
        try {
          const res = await handleScan(args)
          respond(msg.id ?? null, { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] })
          log.info({ id: msg.id, tool: name }, 'sent tool result')
        } catch (error: any) {
          respond(msg.id ?? null, undefined, new Error(error.message || 'Scan failed'))
          log.error({ id: msg.id, tool: name, error }, 'tool error')
        }
      } else if (name === 'explain') {
        try {
          const res = await handleExplain(args)
          respond(msg.id ?? null, { content: [{ type: 'text', text: res }] })
          log.info({ id: msg.id, tool: name }, 'sent tool result')
        } catch (error) {
          const { formatExplainError } = await import('./tools/explain.js')
          const errorResponse = formatExplainError(error)
          respond(msg.id ?? null, errorResponse)
          log.error({ id: msg.id, tool: name, error }, 'tool error')
        }
      } else {
        respond(msg.id ?? null, undefined, new Error(`unknown tool: ${name}`))
      }
      return
    }
    case 'search':
      respond(msg.id, await handleSearch(msg.params))
      log.info({ id: msg.id }, 'sent search result')
      return
    case 'open':
      respond(msg.id, await handleOpen(msg.params))
      log.info({ id: msg.id }, 'sent open result')
      return
    case 'scan':
      respond(msg.id, await handleScan(msg.params))
      log.info({ id: msg.id }, 'sent scan result')
      return
    case 'upsert':
      respond(msg.id, await handleUpsert(msg.params))
      log.info({ id: msg.id }, 'sent upsert result')
      return
    default:
      respond(msg.id, undefined, new Error(`unknown method: ${msg.method}`))
      return
  }
}

process.stdin.on('data', async (chunk) => {
  buf += chunk
  while (true) {
    // Prefer LSP-style framing if we see a header block
    const headerEnd = buf.indexOf('\r\n\r\n')
    if (headerEnd !== -1) {
      const headers = buf.slice(0, headerEnd)
      const m = /Content-Length:\s*(\d+)/i.exec(headers)
      if (!m) {
        // Headers present but no content-length → discard header noise and continue
        log.warn({ headers }, 'header block without content-length; discarding')
        buf = buf.slice(headerEnd + 4)
        continue
      }
      useContentLengthFraming = true
      const length = parseInt(m[1], 10)
      const start = headerEnd + 4
      if (buf.length - start < length) return
      const body = buf.slice(start, start + length)
      buf = buf.slice(start + length)
      let msg: JsonRpcRequest
      try { msg = JSON.parse(body) } catch (e) { log.warn({ body }, 'invalid json'); continue }
      try { await handleMessage(msg) } catch (err) { log.error({ err }, 'handler error'); respond(msg.id ?? null, undefined, err) }
      continue
    }

    // Fallback to newline-delimited JSON
    const nl = buf.indexOf('\n')
    if (nl === -1) return
    const line = buf.slice(0, nl)
    buf = buf.slice(nl + 1)
    if (!line.trim()) continue
    let msg: JsonRpcRequest
    try { msg = JSON.parse(line) } catch (e) { log.warn({ line }, 'invalid json'); continue }
    try { await handleMessage(msg) } catch (err) { log.error({ err }, 'handler error'); respond(msg.id ?? null, undefined, err) }
  }
})

// Do not log to stdout here; keep idle.

