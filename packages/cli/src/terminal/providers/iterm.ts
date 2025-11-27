import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { TerminalProvider, WindowOptions, SplitDirection, AgentInfo } from '../interface'

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

  /**
   * Send a message to an agent pane via send_to_pane.py
   * Matches TerminalProvider interface: sendMessage?(paneId: string, message: string): Promise<boolean>
   */
  async sendMessage(paneId: string, message: string): Promise<boolean> {
    if (!this.scriptsDir) return false

    // Extract agent type from paneId (format: name__type) for proper Enter key handling
    const agentType = this.parseAgentType(paneId)

    const args = [join(this.scriptsDir, 'send_to_pane.py'), '--to', paneId, '--text', message]
    if (agentType && agentType !== 'unknown') {
      args.push('--agent', agentType)
    }

    const result = spawnSync('python3', args, { encoding: 'utf-8' })
    return result.status === 0
  }

  /**
   * List all agent panes by calling list_panes.py and filtering for agent naming convention
   */
  async listAgents(): Promise<AgentInfo[]> {
    if (!this.scriptsDir) return []

    const result = spawnSync('python3', [join(this.scriptsDir, 'list_panes.py')], { encoding: 'utf-8' })

    if (result.status !== 0) return []
    return this.parsePaneList(result.stdout)
  }

  /**
   * Parse list_panes.py output to extract agent information
   * Output format: " N. [label]     Window:W Tab:T ID:session_id"
   */
  private parsePaneList(output: string): AgentInfo[] {
    const agents: AgentInfo[] = []
    const lines = output.trim().split('\n')

    for (const line of lines) {
      // Parse output like: " 1. [label]     Window:1 Tab:1 ID:3C31E1FF..."
      const match = line.match(/\[([^\]]+)\].*ID:(\S+)/)
      if (match) {
        const [, label, sessionId] = match
        // Filter for agent panes (name__type format)
        if (label.includes('__')) {
          const parts = label.split('__')
          agents.push({
            id: sessionId,
            name: label,
            type: parts[parts.length - 1],
            status: 'running', // iTerm panes are always running (if they exist)
          })
        }
      }
    }
    return agents
  }

  /**
   * Extract agent type from paneId (format: name__type)
   */
  private parseAgentType(paneId: string): string {
    const parts = paneId.split('__')
    return parts.length > 1 ? parts[parts.length - 1] : 'unknown'
  }
}
