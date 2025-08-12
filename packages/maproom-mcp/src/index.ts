import { Client } from 'pg'
import pino from 'pino'
import { spawn } from 'node:child_process'
import { Readable } from 'node:stream'
import path from 'node:path'
import fs from 'node:fs'

// IMPORTANT: Never write logs to stdout; MCP JSON-RPC must be the only stdout output.
// Route pino logs to stderr to avoid corrupting the protocol stream.
const log = pino({ level: process.env.LOG_LEVEL || 'info' }, pino.destination(2))

type JsonRpcRequest = { jsonrpc: '2.0'; id?: number | string; method: string; params?: any }
type JsonRpcResponse = { jsonrpc: '2.0'; id: number | string | null; result?: any; error?: { code: number; message: string; data?: any } }

// MCP initialize response
function mcpInitialize() {
  return {
    protocolVersion: '2024-11-05',
    serverInfo: { name: 'maproom-mcp', version: '0.1.0' },
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

// Tool declarations for tools/list
const toolSchemas = [
  {
    name: 'search',
    description: 'Full-text search over Maproom chunks (ts_doc)',
    inputSchema: {
      type: 'object',
      properties: {
        repo: { type: 'string' },
        worktree: { anyOf: [{ type: 'string' }, { type: 'null' }] },
        query: { type: 'string' },
        k: { type: 'integer', minimum: 1, default: 10 }
      },
      required: ['repo', 'query']
    }
  },
  {
    name: 'open',
    description: 'Open a file slice from a worktree',
    inputSchema: {
      type: 'object',
      properties: {
        relpath: { type: 'string' },
        range: {
          type: 'object',
          properties: { start: { type: 'integer', minimum: 1 }, end: { type: 'integer', minimum: 1 } },
          required: []
        },
        worktree: { type: 'string' }
      },
      required: ['relpath', 'worktree']
    }
  },
  {
    name: 'upsert',
    description: 'Index/update specific files by spawning the Rust CLI',
    inputSchema: {
      type: 'object',
      properties: {
        paths: { type: 'array', items: { type: 'string' } },
        commit: { type: 'string' },
        repo: { type: 'string' },
        worktree: { type: 'string' },
        root: { type: 'string' }
      },
      required: ['paths', 'commit', 'repo', 'worktree', 'root']
    }
  }
]

async function getPg(): Promise<Client> {
  const client = new Client({ connectionString: process.env.DATABASE_URL })
  await client.connect()
  return client
}

async function handleSearch(params: any): Promise<any> {
  const { repo, worktree, query, k = 10 } = params
  const client = await getPg()
  try {
    const { rows: repoRows } = await client.query('SELECT id FROM maproom.repos WHERE name = $1', [repo])
    if (repoRows.length === 0) return { hits: [] }
    const repoId = repoRows[0].id
    let worktreeId: number | null = null
    if (typeof worktree === 'string' && worktree.length > 0) {
      const { rows: wt } = await client.query('SELECT id FROM maproom.worktrees WHERE repo_id=$1 AND name=$2', [repoId, worktree])
      worktreeId = wt[0]?.id ?? null
    }
    const tsParts = String(query)
      .split(/\s+/)
      .filter(Boolean)
      .map((t) => `${t.replace(/'/g, '')}:*`)
      .join(' & ')

    const args: any[] = [repoId]
    let sql = `
      SELECT c.id, f.relpath, c.symbol_name, c.kind::text, c.start_line, c.end_line, ts_rank_cd(c.ts_doc, to_tsquery('simple', $${worktreeId ? 3 : 2})) AS score
      FROM maproom.chunks c
      JOIN maproom.files f ON f.id = c.file_id
      WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $${worktreeId ? 3 : 2})
    `
    if (worktreeId) {
      sql += ' AND f.worktree_id = $2'
      args.push(worktreeId)
    }
    args.push(tsParts)
    sql += ' ORDER BY score DESC LIMIT $' + (args.length + 1)
    args.push(k)

    const { rows } = await client.query(sql, args)
    return { hits: rows.map((r) => ({
      chunk_id: r.id,
      relpath: r.relpath,
      symbol_name: r.symbol_name,
      kind: r.kind,
      start_line: r.start_line,
      end_line: r.end_line,
      score: Number(r.score)
    })) }
  } finally {
    await client.end().catch(() => {})
  }
}

async function handleOpen(params: any): Promise<any> {
  const { relpath, range, worktree } = params
  // For now, read directly from filesystem using provided worktree path via database
  const client = await getPg()
  try {
    const { rows } = await client.query(
      `SELECT w.abs_path FROM maproom.worktrees w JOIN maproom.files f ON f.worktree_id = w.id WHERE f.relpath = $1 AND w.name = $2 LIMIT 1`,
      [relpath, worktree]
    )
    if (rows.length === 0) throw new Error('worktree or file not found')
    const base = rows[0].abs_path as string
    const fs = await import('node:fs/promises')
    const content = await fs.readFile(`${base}/${relpath}`, 'utf8')
    const start = range?.start ?? 1
    const end = range?.end ?? content.split('\n').length
    const sliced = content.split('\n').slice(start - 1, end).join('\n')
    return { content: sliced }
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
      // We don't define prompts yet; return empty list
      respond(msg.id, { prompts: [] })
      log.info({ id: msg.id, count: 0 }, 'sent prompts list')
      return
    case 'resources/list':
      // We don't define resources yet; return empty list
      respond(msg.id, { resources: [] })
      log.info({ id: msg.id, count: 0 }, 'sent resources list')
      return
    case 'tools/call': {
      const name = msg.params?.name as string
      const args = msg.params?.arguments || {}
      if (name === 'search') {
        const res = await handleSearch(args)
        respond(msg.id ?? null, { content: [{ type: 'text', text: JSON.stringify(res) }] })
      } else if (name === 'open') {
        const res = await handleOpen(args)
        respond(msg.id ?? null, { content: [{ type: 'text', text: res.content }] })
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

