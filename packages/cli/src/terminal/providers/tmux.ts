import { spawnSync } from 'node:child_process'
import { logger } from '../../utils/logger'
import { TerminalProvider, WindowOptions, SplitDirection, AgentInfo } from '../interface'

export class TmuxProvider implements TerminalProvider {
  readonly id = 'tmux'
  private sessionName: string

  constructor(options?: { sessionName?: string }) {
    const sessionName = options?.sessionName ?? 'crewchief'

    // Validate session name characters (tmux limitation)
    if (!/^[a-zA-Z0-9_.]+$/.test(sessionName)) {
      throw new Error(
        `Session name '${sessionName}' contains invalid characters. ` +
          'Use only letters, digits, underscores, and periods.',
      )
    }

    this.sessionName = sessionName
  }

  async initialize(): Promise<void> {
    logger.info('Initializing Tmux Terminal Provider')

    // Check tmux availability (POSIX compatible — no `which`)
    const whichResult = spawnSync('command', ['-v', 'tmux'], { shell: true })
    if (whichResult.status !== 0) {
      throw new Error(
        'tmux not found. Please install tmux:\n' +
          '  Ubuntu/Debian: sudo apt install tmux\n' +
          '  macOS: brew install tmux',
      )
    }

    // Check tmux version
    const versionResult = spawnSync('tmux', ['-V'])
    const versionOutput = versionResult.stdout.toString()
    const versionMatch = versionOutput.match(/tmux (\d+)\.(\d+)/)

    if (!versionMatch) {
      throw new Error('Could not determine tmux version. ' + `Output from 'tmux -V': ${versionOutput.trim()}`)
    }

    const major = Number(versionMatch[1])
    const minor = Number(versionMatch[2])
    if (major < 2 || (major === 2 && minor < 1)) {
      throw new Error(
        `tmux version ${major}.${minor} is too old. Minimum required: 2.1\n` +
          'Please upgrade tmux:\n' +
          '  Ubuntu/Debian: sudo apt update && sudo apt install tmux\n' +
          '  macOS: brew upgrade tmux',
      )
    }

    // Create session if it doesn't exist
    const sessions = spawnSync('tmux', ['list-sessions'])
    if (!sessions.stdout.toString().includes(this.sessionName)) {
      const createResult = spawnSync('tmux', ['new-session', '-d', '-s', this.sessionName])
      if (createResult.status !== 0) {
        throw new Error(
          `Failed to create tmux session '${this.sessionName}': ${createResult.stderr.toString().trim()}\n` +
            'Troubleshooting:\n' +
            '  - Ensure tmux server is running: tmux start-server\n' +
            '  - Check for permission issues with /tmp/tmux-*',
        )
      }
    }

    logger.info(`Tmux provider initialized (session: ${this.sessionName})`)
  }

  async dispose(): Promise<void> {
    // No-op: tmux sessions persist intentionally (survives CLI exit)
    logger.info('Tmux provider disposed (session persists)')
  }

  async createWindow(options?: WindowOptions): Promise<string> {
    const name = options?.title || `agent-${Date.now()}`
    const args = ['new-window', '-t', this.sessionName, '-n', name, '-P', '-F', '#{pane_id}']

    if (options?.workingDirectory) {
      args.push('-c', options.workingDirectory)
    }

    const result = spawnSync('tmux', args)

    if (result.status !== 0) {
      throw new Error(`Failed to create tmux window: ${result.stderr.toString().trim()}`)
    }

    const paneId = result.stdout.toString().trim()
    logger.info(`Created tmux window '${name}' with pane ${paneId}`)
    return paneId
  }

  async createTab(windowId: string): Promise<string> {
    // In tmux, a "tab" is effectively a new window within the same session.
    // The windowId parameter is acknowledged but tmux windows are session-scoped.
    const result = spawnSync('tmux', ['new-window', '-t', this.sessionName, '-P', '-F', '#{pane_id}'])

    if (result.status !== 0) {
      throw new Error(`Failed to create tmux tab in window ${windowId}: ${result.stderr.toString().trim()}`)
    }

    const paneId = result.stdout.toString().trim()
    logger.info(`Created tmux tab (pane ${paneId}) for window ${windowId}`)
    return paneId
  }

  async splitPane(targetId: string, direction: SplitDirection): Promise<string> {
    const dirFlag = direction === 'horizontal' ? '-h' : '-v'
    const result = spawnSync('tmux', ['split-window', dirFlag, '-t', targetId, '-P', '-F', '#{pane_id}'])

    if (result.status !== 0) {
      throw new Error(`Failed to split tmux pane ${targetId}: ${result.stderr.toString().trim()}`)
    }

    const paneId = result.stdout.toString().trim()
    logger.info(`Split pane ${targetId} ${direction}ly, new pane: ${paneId}`)
    return paneId
  }

  async runCommand(paneId: string, command: string): Promise<void> {
    const escaped = this.escapeTmuxMessage(command)
    const result = spawnSync('tmux', ['send-keys', '-t', paneId, escaped, 'Enter'])

    if (result.status !== 0) {
      throw new Error(`Failed to run command in pane ${paneId}: ${result.stderr.toString().trim()}`)
    }

    logger.info(`[${paneId}] Sent command: ${command}`)
  }

  async focus(paneId: string): Promise<void> {
    const result = spawnSync('tmux', ['select-pane', '-t', paneId])

    if (result.status !== 0) {
      logger.error(`Failed to focus pane ${paneId}: ${result.stderr.toString().trim()}`)
    }
  }

  /**
   * Send a message to an agent in a specific pane via tmux send-keys.
   * @param paneId - The tmux pane identifier (e.g., %0)
   * @param message - The message text to send
   * @returns true if message was sent successfully, false otherwise
   */
  async sendMessage(paneId: string, message: string): Promise<boolean> {
    const escaped = this.escapeTmuxMessage(message)
    const result = spawnSync('tmux', ['send-keys', '-t', paneId, escaped, 'Enter'])

    if (result.status !== 0) {
      logger.warn(`[sendMessage] Failed to send to pane ${paneId}: ${result.stderr.toString().trim()}`)
      return false
    }

    logger.info(`[${paneId}] Sent message: ${message}`)
    return true
  }

  /**
   * List all agents managed by this provider.
   * Filters tmux panes for the task__platform naming convention.
   * @returns Array of agent information objects
   */
  async listAgents(): Promise<AgentInfo[]> {
    const result = spawnSync('tmux', ['list-panes', '-s', '-t', this.sessionName, '-F', '#{pane_id}:#{window_name}'])

    if (result.status !== 0) {
      logger.warn(`Failed to list tmux panes: ${result.stderr.toString().trim()}`)
      return []
    }

    const output = result.stdout.toString().trim()
    if (!output) {
      return []
    }

    const lines = output.split('\n')
    return lines
      .filter((line: string) => line.includes('__')) // Filter for agent naming pattern
      .map((line: string) => {
        const [id, name] = line.split(':')
        const platform = name.split('__')[1] || 'unknown'
        return { id, name, platform, status: 'running' as const }
      })
  }

  /**
   * Escape special characters for tmux send-keys.
   *
   * tmux send-keys passes text to the shell, which interprets special characters.
   * Without escaping:
   *   - $HOME would expand to the user's home directory
   *   - Backticks or $() would execute commands
   *   - Newlines would split into multiple lines unexpectedly
   */
  private escapeTmuxMessage(message: string): string {
    return message
      .replace(/\\/g, '\\\\') // Backslashes first
      .replace(/"/g, '\\"') // Double quotes
      .replace(/'/g, "\\'") // Single quotes
      .replace(/\$/g, '\\$') // Dollar signs (variable expansion)
      .replace(/`/g, '\\`') // Backticks (command substitution)
      .replace(/\n/g, '\\n') // Newlines (literal \n, not line break)
  }
}
