const { spawn } = require('node:child_process')
const path = require('node:path')

const cwd = path.join(__dirname, '..')
const child = spawn('node', ['dist/index.js'], { cwd, stdio: ['pipe', 'pipe', 'inherit'] })

child.stdout.setEncoding('utf8')
child.stdout.on('data', (d) => process.stdout.write(d))

function send(obj) {
  const s = JSON.stringify(obj)
  child.stdin.write(s + '\n')
}

send({ jsonrpc: '2.0', id: 1, method: 'initialize' })
setTimeout(() => send({ jsonrpc: '2.0', id: 2, method: 'tools/list' }), 100)
setTimeout(() => child.kill(), 800)







