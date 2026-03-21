import pino from 'pino'
import { spawn } from 'node:child_process'
import { Readable } from 'node:stream'
import path from 'node:path'
import fs from 'node:fs'
import { resolveDatabaseConfig } from './utils/resolve-database.js'
import { getDaemonClient } from './daemon.js'

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

Examples:
  filters: {file_type: "ts"}          → Only TypeScript files
  filters: {file_type: "ts,tsx,js"}   → TypeScript or JavaScript files
  filters: {file_type: "md,mdx"}      → Markdown documentation
  filters: {file_type: "rs"}          → Rust source files
  filters: {
    file_type: "ts,tsx",
    recency_threshold: "7 days"
  }                                   → Recent TypeScript files only

FILTER SYNTAX:
- Comma-separated for multiple types: "ts,tsx,js"
- Case insensitive: "TS" same as "ts"
- With or without dot: ".ts" same as "ts"
- Max 20 extensions per filter

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
          description: 'Search mode: "fts" for keyword search (fast), "vector" for semantic similarity (requires embeddings), "hybrid" (default) for combined ranking (slower but comprehensive)'
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
            file_type: { type: 'string', description: 'Filter by file extension(s). Single: "ts" or multiple: "ts,tsx,js" (comma-separated, max 20 extensions)' },
            recency_threshold: { type: 'string', description: 'Filter by file modification time (e.g., "7 days", "1 month")' }
          }
        },
        debug: {
          type: 'boolean',
          default: false,
          description: 'Enable debug mode to see score breakdowns (FTS, vector, graph signals, fusion method)'
        },
        deduplicate: {
          type: 'boolean',
          default: true,
          description: 'Deduplicate results across worktrees. When true, results with the same file path, symbol name, and line number are grouped, returning only the highest-scoring instance. Set false to see all results including duplicates. (default: true)'
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
]


async function handleStatus(params: any): Promise<any> {
  const { repo } = params

  // Check database type
  const dbConfig = resolveDatabaseConfig()
  if (dbConfig.type === 'sqlite') {
    log.info({ sqlitePath: dbConfig.path }, 'SQLite mode: querying status via daemon')

    try {
      const daemon = getDaemonClient()
      const statusResult = await daemon.status({ repo })

      // Calculate totals from daemon response
      const totalFiles = statusResult.total_files
      const totalChunks = statusResult.total_chunks

      // Format repos for MCP response (convert snake_case to camelCase)
      const repos = statusResult.repos.map(r => ({
        name: r.name,
        worktrees: r.worktrees.map(wt => ({
          name: wt.name,
          path: wt.path,
          fileCount: wt.file_count,
          chunkCount: wt.chunk_count,
        }))
      }))

      let hint = ''
      let nextStep: string | undefined

      if (repos.length === 0) {
        hint = 'No repositories indexed yet.\n\nTo get started:\n1. Run `maproom scan` to index a repository\n2. Then use search to find your code'
        nextStep = 'Run maproom scan to index your first repository'
      } else if (totalFiles === 0) {
        hint = 'Repository found but no files indexed.\n\nTo fix:\n1. Run `maproom scan` to index files in this repository\n2. Check that the path contains supported file types (.ts, .js, .rs, .md, etc.)'
        nextStep = 'Run maproom scan to index files'
      } else {
        hint = `Index ready! ${totalFiles} files and ${totalChunks} searchable chunks.\n\nCommon searches: "main function", "error handling", "database query"`
      }

      return {
        repos,
        totalRepos: repos.length,
        totalFiles,
        totalChunks,
        hint,
        nextStep,
        backendType: 'sqlite',
        sqlitePath: dbConfig.path,
        searchTips: [
          'Use simple terms: "auth" instead of "authentication_handler"',
          'Search concepts: "message bus" or "event handling"',
          'Filter by type: use filter:"code" or filter:"docs"',
        ],
      }
    } catch (error) {
      log.error({ error }, 'Failed to get status from daemon')
      // Fallback to empty response on error
      return {
        repos: [],
        totalRepos: 0,
        totalFiles: 0,
        totalChunks: 0,
        hint: `Failed to query status: ${error instanceof Error ? error.message : String(error)}`,
        backendType: 'sqlite',
        sqlitePath: dbConfig.path,
        searchTips: [
          'Use simple terms: "auth" instead of "authentication_handler"',
          'Search concepts: "message bus" or "event handling"',
          'Filter by type: use filter:"code" or filter:"docs"',
        ],
      }
    }
  }

  // PostgreSQL backend has been removed. Only SQLite via the Rust daemon is supported.
  throw new Error('Unsupported database configuration. Only SQLite mode is supported. Set MAPROOM_DATABASE_URL to a sqlite:// path.')
}

/**
 * Parse and normalize file type filter input into array of extensions.
 *
 * Handles comma-separated extension lists with flexible formatting:
 * - Case insensitive: "TS" → "ts"
 * - Dot tolerant: ".ts" → "ts"
 * - Whitespace tolerant: " ts , tsx " → ["ts", "tsx"]
 * - Empty safe: "" → [], ",,," → []
 *
 * @param input - Raw file_type filter string from MCP request
 * @returns Array of normalized extension strings (lowercase, no dots)
 *
 * @example Single extension
 * parseFileTypeFilter("ts") → ["ts"]
 *
 * @example Multi-extension
 * parseFileTypeFilter("ts,tsx,js") → ["ts", "tsx", "js"]
 *
 * @example Flexible formatting
 * parseFileTypeFilter(".TS, .tsx , js") → ["ts", "tsx", "js"]
 *
 * @example Empty handling
 * parseFileTypeFilter("") → []
 * parseFileTypeFilter(",,,") → []
 */
export function parseFileTypeFilter(input: string): string[] {
  return input
    .split(',')                           // Split on comma delimiter
    .map(ext => ext.trim())               // Remove leading/trailing whitespace
    .map(ext => ext.replace(/^\./, ''))   // Strip leading dot if present
    .map(ext => ext.toLowerCase())        // Normalize to lowercase
    .filter(ext => ext.length > 0)        // Remove empty strings after processing
}

// Helper function to build filter WHERE clauses
export function buildFilterClauses(filters: any, filter: string, args: any[]): string {
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

  // Advanced file_type filter with multi-extension support
  if (filters.file_type) {
    const extensions = parseFileTypeFilter(filters.file_type)

    // Skip filter if parsing produced no valid extensions
    if (extensions.length === 0) {
      // Graceful fallback - search all files
    } else {
      // Enforce extension count limit to prevent DoS via complex OR queries
      if (extensions.length > 20) {
        extensions.splice(20)  // Truncate to maximum allowed
      }

      // Single extension: backward-compatible simple LIKE clause
      if (extensions.length === 1) {
        args.push(`%.${extensions[0]}`)
        clauses += ` AND f.relpath LIKE $${args.length}`
      }
      // Multiple extensions: OR clause for union of all types
      else {
        const likeConditions = extensions.map(ext => {
          args.push(`%.${ext}`)
          return `f.relpath LIKE $${args.length}`
        })
        // Use parentheses to ensure correct precedence with other filters
        clauses += ` AND (${likeConditions.join(' OR ')})`
      }
    }
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

export async function handleSearch(params: any): Promise<any> {
  const {
    repo,
    worktree,
    query,
    k = 10,
    filter = 'all',
    mode = 'fts', // Changed default to 'fts' to use Rust binary
    filters = {},
    debug = false
  } = params

  // All search modes use the daemon-based search via Rust binary
  const { handleSearchTool } = await import('./tools/search.js')

  // For hybrid mode, fall back to FTS (hybrid fusion handled by daemon in future)
  const effectiveMode = mode === 'hybrid' ? 'fts' : mode
  if (mode === 'hybrid') {
    log.warn('Hybrid mode not yet supported, falling back to FTS')
  }

  const result = await handleSearchTool(
    { query, repo, worktree, limit: k, mode: effectiveMode, debug },
    null as any // Client not used by daemon-based search
  )
  // Transform SearchBundle to old format for backward compatibility
  return {
    hits: result.hits,
    error: result.error,
    hint: result.hint,
    suggestion: result.suggestion,
  }
}

async function handleOpen(params: any): Promise<any> {
  // Check database type
  const dbConfig = resolveDatabaseConfig()
  if (dbConfig.type === 'sqlite') {
    // SQLite mode: read file directly from filesystem
    // In SQLite mode, we don't have worktree abs_path mappings in the database,
    // so we read from the current working directory or use relpath as-is
    const fs = await import('node:fs/promises')
    const path = await import('node:path')

    try {
      const { relpath, range } = params

      // Try to read from current working directory
      const cwd = process.cwd()
      const fullPath = path.resolve(cwd, relpath)

      // Security: ensure path is within cwd
      if (!fullPath.startsWith(cwd)) {
        return {
          error: 'INVALID_PATH',
          message: 'Path traversal not allowed',
        }
      }

      let content = await fs.readFile(fullPath, 'utf8')

      // Apply line range if specified
      if (range && range.start && range.end) {
        const lines = content.split('\n')
        const startIdx = Math.max(0, range.start - 1)
        const endIdx = Math.min(lines.length, range.end)
        content = lines.slice(startIdx, endIdx).join('\n')
      }

      return {
        content,
        relpath,
        ...(range && { range }),
      }
    } catch (error: any) {
      if (error.code === 'ENOENT') {
        return {
          error: 'FILE_NOT_FOUND',
          message: `File not found: ${params.relpath}`,
        }
      }
      return {
        error: 'OPEN_FAILED',
        message: error.message || 'Failed to open file',
      }
    }
  }

  // PostgreSQL backend has been removed. Only SQLite mode is supported.
  throw new Error('Unsupported database configuration. Only SQLite mode is supported. Set MAPROOM_DATABASE_URL to a sqlite:// path.')
}

/**
 * handleContext - Retrieve contextually relevant code sections around a target chunk
 *
 * Integrates with the context assembler to provide intelligent context gathering
 * with relationship traversal, budget management, and multi-chunk assembly.
 */
async function handleContext(params: any): Promise<any> {
  const { handleContextTool } = await import('./tools/context.js')
  return await handleContextTool(params)
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
        try {
          const res = await handleSearch(args)
          respond(msg.id ?? null, { content: [{ type: 'text', text: JSON.stringify(res) }] })
          log.info({ id: msg.id, tool: name }, 'sent tool result')
        } catch (error) {
          const { formatSearchError } = await import('./tools/search.js')
          const errorResponse = formatSearchError(error)
          respond(msg.id ?? null, errorResponse)
          log.error({ id: msg.id, tool: name, error }, 'tool error')
        }
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

