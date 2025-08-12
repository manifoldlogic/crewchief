// Simple test harness: spawn the stdio server, send one request, print one response
import { spawn } from 'node:child_process'

const [, , method = 'search'] = process.argv

const req = {
  jsonrpc: '2.0',
  id: 1,
  method,
  params: method === 'search' ? {
    repo: process.env.MAPROOM_REPO || 'crewchief',
    worktree: process.env.MAPROOM_WORKTREE || 'maproom-cursor-gpt5',
    query: process.env.MAPROOM_QUERY || 'maproom',
    k: Number(process.env.MAPROOM_K || 5)
  } : method === 'open' ? {
    relpath: process.env.MAPROOM_RELPATH || 'crates/maproom/src/main.rs',
    range: { start: Number(process.env.MAPROOM_START || 1), end: Number(process.env.MAPROOM_END || 60) },
    worktree: process.env.MAPROOM_WORKTREE || 'maproom-cursor-gpt5'
  } : {}
}

const child = spawn('node', ['dist/index.js'], { cwd: new URL('..', import.meta.url).pathname, stdio: ['pipe', 'pipe', 'inherit'] })

child.stdout.setEncoding('utf8')
child.stdout.once('data', (line) => {
  try { console.log(JSON.parse(line.trim())) } catch { console.log(line.trim()) }
  child.kill()
})

child.stdin.write(JSON.stringify(req) + '\n')


