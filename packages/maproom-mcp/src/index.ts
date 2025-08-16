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
    DATABASE_URL: process.env.DATABASE_URL ? '[SET]' : '[NOT SET]',
    PG_DATABASE_URL: process.env.PG_DATABASE_URL ? '[SET]' : '[NOT SET]',
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
    description: 'Semantic code search - BEST FOR: finding functions/classes by concept, understanding code relationships, exploring unfamiliar codebases. FASTER THAN: Grep for conceptual searches. USE WHEN: searching for functionality rather than exact text matches. EXAMPLES: "authentication flow", "error handling", "database connection", "React component state". TIP: Start with simple terms, then refine. Use status tool first to see what\'s indexed.',
    inputSchema: {
      type: 'object',
      properties: {
        repo: { type: 'string', description: 'Repository name to search in (use "crewchief" for this codebase)' },
        worktree: { anyOf: [{ type: 'string' }, { type: 'null' }], description: 'Optional worktree name to limit search scope' },
        query: { type: 'string', description: 'Search query - can be concepts, function names, or multiple terms. Works best with 1-3 words. Examples: "maproom search", "worktree create", "message bus"' },
        k: { type: 'integer', minimum: 1, default: 10, description: 'Number of results to return (default: 10, max useful: 20)' },
        filter: { 
          type: 'string', 
          enum: ['all', 'code', 'docs', 'config'],
          default: 'all',
          description: 'Filter results by file type: all (default), code (ts/js/rs), docs (md/mdx), config (json/yaml/toml)'
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
    name: 'upsert',
    description: 'Index/update files in maproom - USE WHEN: files have changed and need reindexing. RARELY NEEDED: maproom auto-indexes on file changes. Only use if search returns outdated results. Spawns the Rust indexer.',
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
  }
]

async function getPg(): Promise<Client> {
  const connectionString = process.env.DATABASE_URL || process.env.PG_DATABASE_URL
  if (!connectionString) {
    log.error('No DATABASE_URL or PG_DATABASE_URL environment variable set')
    throw new Error('Database connection string not configured')
  }
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
    if (Object.keys(repos).length === 0) {
      hint = 'No repositories indexed yet. Use the upsert tool to index files first.'
    } else if (totalFiles === 0) {
      hint = 'Repository exists but no files indexed. Use the upsert tool to index files.'
    } else {
      hint = `Index ready! ${totalFiles} files and ${totalChunks} searchable chunks. Common searches: "main function", "error handling", "database query"`
    }
    
    return {
      repos: Object.values(repos),
      fileTypes: fileTypes.map(ft => ({ extension: ft.extension, count: parseInt(ft.count) })),
      totalRepos: Object.keys(repos).length,
      totalFiles,
      totalChunks,
      hint,
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

async function handleSearch(params: any): Promise<any> {
  const { repo, worktree, query, k = 10, filter = 'all' } = params
  const client = await getPg()
  try {
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
        hint: `Repository '${repo}' is not indexed.\n\nTo fix this:\n1. Run status tool to see available repos\n2. For this codebase, use repo:"crewchief"\n3. If needed, run upsert tool to index files`,
        availableRepos,
        suggestion
      }
    }
    const repoId = repoRows[0].id
    let worktreeId: number | null = null
    let worktreeInfo: any = null
    
    if (typeof worktree === 'string' && worktree.length > 0) {
      const { rows: wt } = await client.query('SELECT id, name FROM maproom.worktrees WHERE repo_id=$1 AND name=$2', [repoId, worktree])
      if (wt.length > 0) {
        worktreeId = wt[0].id
        worktreeInfo = wt[0]
      }
    }
    
    const tsParts = String(query)
      .split(/\s+/)
      .filter(Boolean)
      .map((t) => `${t.replace(/'/g, '')}:*`)
      .join(' & ')

    const args: any[] = [repoId]
    let sql = `
      SELECT c.id, f.relpath, c.symbol_name, c.kind::text, c.start_line, c.end_line, c.metadata,
        CASE 
          WHEN c.kind IN ('heading_1', 'heading_2') THEN 
            ts_rank_cd(c.ts_doc, to_tsquery('simple', $${worktreeId ? 3 : 2})) * 2.0
          WHEN c.kind = 'heading_3' THEN
            ts_rank_cd(c.ts_doc, to_tsquery('simple', $${worktreeId ? 3 : 2})) * 1.5
          WHEN c.kind IN ('heading_4', 'heading_5', 'heading_6') THEN
            ts_rank_cd(c.ts_doc, to_tsquery('simple', $${worktreeId ? 3 : 2})) * 1.2
          WHEN c.kind = 'json_key' THEN
            ts_rank_cd(c.ts_doc, to_tsquery('simple', $${worktreeId ? 3 : 2})) * 1.3
          ELSE 
            ts_rank_cd(c.ts_doc, to_tsquery('simple', $${worktreeId ? 3 : 2}))
        END AS score
      FROM maproom.chunks c
      JOIN maproom.files f ON f.id = c.file_id
      WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $${worktreeId ? 3 : 2})
    `
    if (worktreeId) {
      sql += ' AND f.worktree_id = $2'
      args.push(worktreeId)
    }
    
    // Add filter conditions
    if (filter !== 'all') {
      if (filter === 'code') {
        sql += ` AND f.relpath NOT LIKE '%.md' AND f.relpath NOT LIKE '%.mdx' AND f.relpath NOT LIKE '%.json' AND f.relpath NOT LIKE '%.yaml' AND f.relpath NOT LIKE '%.yml'`
      } else if (filter === 'docs') {
        sql += ` AND (f.relpath LIKE '%.md' OR f.relpath LIKE '%.mdx')`
      } else if (filter === 'config') {
        sql += ` AND (f.relpath LIKE '%.json' OR f.relpath LIKE '%.yaml' OR f.relpath LIKE '%.yml' OR f.relpath LIKE '%.toml')`
      }
    }
    
    args.push(tsParts)
    sql += ' ORDER BY score DESC LIMIT $' + (args.length + 1)
    args.push(k)

    const { rows } = await client.query(sql, args)
    
    const result: any = {
      hits: rows.map((r) => {
        const hit: any = {
          chunk_id: r.id,
          relpath: r.relpath,
          symbol_name: r.symbol_name,
          kind: r.kind,
          start_line: r.start_line,
          end_line: r.end_line,
          score: Number(r.score)
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
        ? `No results in worktree '${worktree}'.\n\nPossible reasons:\n1. Files not indexed yet - use upsert tool\n2. Search terms too specific - try simpler terms\n3. Wrong worktree - check status tool`
        : `No results found for "${query}".\n\n${statusHint}\n\nSearch tips:\n• Use 1-3 word queries\n• Try conceptual terms: "authentication", "database", "error handling"\n• Separate words with spaces, not underscores\n• Start broad, then refine`
      
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
  const { relpath, range, worktree, context = 0 } = params
  // Read directly from filesystem using provided worktree path via database
  const client = await getPg()
  try {
    const { rows } = await client.query(
      `SELECT w.abs_path FROM maproom.worktrees w JOIN maproom.files f ON f.worktree_id = w.id WHERE f.relpath = $1 AND w.name = $2 LIMIT 1`,
      [relpath, worktree]
    )
    if (rows.length === 0) {
      // Provide helpful error message
      const availableWorktrees = await client.query(
        'SELECT DISTINCT w.name FROM maproom.worktrees w JOIN maproom.files f ON f.worktree_id = w.id WHERE f.relpath = $1',
        [relpath]
      )
      if (availableWorktrees.rows.length > 0) {
        throw new Error(`File exists in other worktrees: ${availableWorktrees.rows.map(r => r.name).join(', ')}. Check your worktree parameter.`)
      } else {
        throw new Error(`File '${relpath}' not found in worktree '${worktree}'. Use search tool to find the correct path.`)
      }
    }
    const base = rows[0].abs_path as string
    const fs = await import('node:fs/promises')
    const content = await fs.readFile(`${base}/${relpath}`, 'utf8')
    const lines = content.split('\n')
    
    // Calculate line range with context
    let start = range?.start ?? 1
    let end = range?.end ?? lines.length
    
    // Add context lines if requested
    if (context > 0) {
      start = Math.max(1, start - context)
      end = Math.min(lines.length, end + context)
    }
    
    const sliced = lines.slice(start - 1, end).join('\n')
    
    return { 
      content: sliced,
      actualRange: { start, end },
      requestedRange: range,
      contextLines: context
    }
  } finally {
    await client.end().catch(() => {})
  }
}

async function handleUpsert(params: any): Promise<any> {
  const { paths = [], commit, repo, worktree, root } = params
  const crewchiefArgs = ['maproom', 'upsert', '--paths', paths.join(','), '--commit', commit, '--repo', repo, '--worktree', worktree, '--root', root]
  const maproomArgs = ['upsert', '--paths', paths.join(','), '--commit', commit, '--repo', repo, '--worktree', worktree, '--root', root]

  const candidates: Array<{ cmd: string, args: string[] }> = []
  if (process.env.CREWCHIEF_MAPROOM_BIN) candidates.push({ cmd: process.env.CREWCHIEF_MAPROOM_BIN, args: maproomArgs })
  // Packaged binary fallback (platform-arch)
  try {
    const execName = process.platform === 'win32' ? 'crewchief-maproom.exe' : 'crewchief-maproom'
    const packaged = path.join(__dirname, '..', 'bin', `${process.platform}-${process.arch}`, execName)
    if (fs.existsSync(packaged)) {
      candidates.push({ cmd: packaged, args: maproomArgs })
    }
  } catch {}
  candidates.push(
    { cmd: 'crewchief', args: crewchiefArgs },
    { cmd: 'crewchief-maproom', args: maproomArgs },
    { cmd: './target/debug/crewchief-maproom', args: maproomArgs },
  )

  let lastErr = ''
  for (const c of candidates) {
    try {
      const child = spawn(c.cmd, c.args, { stdio: ['ignore', 'pipe', 'pipe'] })
      const out = await streamToString(child.stdout as Readable)
      const err = await streamToString(child.stderr as Readable)
      const code: number = await new Promise((res) => child.on('close', res))
      if (code === 0) return { ok: true, cmd: c.cmd, out }
      lastErr = `cmd ${c.cmd} exited ${code}: ${err}`
    } catch (e: any) {
      lastErr = `spawn ${c.cmd} failed: ${e?.message || e}`
    }
  }
  throw new Error(`upsert failed: ${lastErr}`)
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
        const res = await handleOpen(args)
        respond(msg.id ?? null, { content: [{ type: 'text', text: res.content }] })
        log.info({ id: msg.id, tool: name }, 'sent tool result')
      } else if (name === 'upsert') {
        const res = await handleUpsert(args)
        respond(msg.id ?? null, { content: [{ type: 'text', text: JSON.stringify(res) }] })
        log.info({ id: msg.id, tool: name }, 'sent tool result')
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

