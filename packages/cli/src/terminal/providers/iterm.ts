import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { TerminalProvider, WindowOptions, SplitDirection } from '../interface'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

export class ITermProvider implements TerminalProvider {
  readonly id = 'iterm'
  private scriptsDir: string | null = null

  constructor() {
    // Find scripts directory (same pattern as ITermSimpleService)
    const possiblePaths = [
      join(__dirname, '..', '..', '..', '..', 'scripts', 'iterm_scripts'),
      join(process.cwd(), 'scripts', 'iterm_scripts'),
      join(process.cwd(), '.crewchief', 'iterm_scripts'),
    ]

    for (const path of possiblePaths) {
      if (existsSync(join(path, 'spawn_agent.py'))) {
        this.scriptsDir = path
        break
      }
    }
  }

  async initialize(): Promise<void> {
    // NO MORE JSON-RPC BRIDGE - just check environment
    if (process.env.TERM_PROGRAM !== 'iTerm.app') {
      throw new Error('ITermProvider requires running in iTerm.app')
    }
    if (!this.scriptsDir) {
      throw new Error('iTerm scripts not found')
    }
  }

  async dispose(): Promise<void> {
    // No resources to clean up - direct script calls are stateless
  }

  async createWindow(options?: WindowOptions): Promise<string> {
    if (!this.scriptsDir) {
      throw new Error('iTerm scripts not found')
    }

    const args = [join(this.scriptsDir, 'spawn_agent.py'), 'claude']
    if (options?.title) {
      args.push('--name', options.title)
    }
    if (options?.workingDirectory) {
      args.push('--project-dir', options.workingDirectory)
    }

    const result = spawnSync('python3', args, { encoding: 'utf-8' })
    if (result.status !== 0) {
      throw new Error(`spawn_agent.py failed: ${result.stderr || result.stdout}`)
    }

    // Parse session ID from output - spawn_agent.py prints status messages
    // The session ID is not directly returned, but the pane is created
    // Return a generated ID for tracking
    const timestamp = Date.now()
    return options?.title || `iterm-pane-${timestamp}`
  }

  async createTab(_windowId: string): Promise<string> {
    // Creating a tab is essentially the same as creating a window in iTerm
    // The spawn_agent.py script will create a new pane
    return this.createWindow()
  }

  async splitPane(_targetId: string, direction: SplitDirection): Promise<string> {
    if (!this.scriptsDir) {
      throw new Error('iTerm scripts not found')
    }

    const scriptName = direction === 'vertical' ? 'split_vertical.py' : 'split_horizontal.py'
    const result = spawnSync('python3', [join(this.scriptsDir, scriptName)], {
      encoding: 'utf-8',
    })

    if (result.status !== 0) {
      throw new Error(`${scriptName} failed: ${result.stderr || result.stdout}`)
    }

    // Return a generated ID for the new pane
    return `iterm-pane-${Date.now()}`
  }

  async runCommand(paneId: string, command: string): Promise<void> {
    if (!this.scriptsDir) {
      throw new Error('iTerm scripts not found')
    }

    const result = spawnSync('python3', [join(this.scriptsDir, 'send_to_pane.py'), '--to', paneId, '--text', command], {
      encoding: 'utf-8',
    })

    if (result.status !== 0) {
      throw new Error(`send_to_pane.py failed: ${result.stderr || result.stdout}`)
    }
  }

  async focus(_paneId: string): Promise<void> {
    // Focus the iTerm app - individual pane focusing would require more complex scripting
    spawnSync('osascript', ['-e', 'tell application "iTerm2" to activate'])
  }
}
