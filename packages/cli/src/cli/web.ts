import { spawn } from 'child_process'
import { resolve } from 'path'
import { Command } from 'commander'
import open from 'open'

const web = new Command('web')
  .description('Start CrewChief Web UI')
  .option('-p, --port <port>', 'Port to run on', '3456')
  .action(async (options) => {
    const port = options.port
    const serverPath = resolve(__dirname, '../../web-ui/dist/server.js')
    const server = spawn('node', [serverPath], {
      env: { ...process.env, PORT: port },
      stdio: 'inherit',
    })
    server.on('error', (err) => {
      console.error('Failed to start server:', err)
      process.exit(1)
    })
    console.log(`Server starting on port ${port}`)
    await open(`http://localhost:${port}`)
  })

export default web
