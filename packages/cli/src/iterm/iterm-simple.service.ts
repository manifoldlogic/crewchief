import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { logger } from '../utils/logger'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

export class ITermSimpleService {
  private scriptsDir: string | null = null

  constructor() {
    // Find the iterm_scripts directory
    const possiblePaths = [
      join(__dirname, '..', '..', '..', '..', 'iterm_scripts'),
      join(process.cwd(), 'iterm_scripts'),
      join(process.cwd(), '.crewchief', 'iterm_scripts'),
    ]

    for (const path of possiblePaths) {
      if (existsSync(join(path, 'send_to_pane.py'))) {
        this.scriptsDir = path
        break
      }
    }
  }

  isAvailable(): boolean {
    return process.env.TERM_PROGRAM === 'iTerm.app' && this.scriptsDir !== null
  }

  /**
   * Send text to a pane by label or agent name
   * This is the main method used by agent message command
   */
  sendKeys(targetLabel: string, text: string, agentType?: string): boolean {
    if (!this.scriptsDir) {
      logger.error('iTerm scripts not found')
      return false
    }

    const scriptPath = join(this.scriptsDir, 'send_to_pane.py')
    
    const args = [
      scriptPath,
      '--to', targetLabel,
      '--text', text,
    ]

    // Add agent type for proper Enter key handling (chr(13) for Claude, etc.)
    if (agentType) {
      args.push('--agent', agentType)
    }

    const result = spawnSync('python3', args, {
      encoding: 'utf-8',
      stdio: 'pipe',
    })

    if (result.status !== 0) {
      logger.error(`Failed to send to pane: ${result.stderr || result.stdout}`)
      return false
    }

    logger.info(`Sent to ${targetLabel}: ${text}`)
    return true
  }

  /**
   * List all panes with their labels
   */
  listPanes(): Array<{ index: number; label: string; sessionId: string }> {
    if (!this.scriptsDir) {
      logger.error('iTerm scripts not found')
      return []
    }

    const scriptPath = join(this.scriptsDir, 'list_panes.py')
    
    const result = spawnSync('python3', [scriptPath], {
      encoding: 'utf-8',
      stdio: 'pipe',
    })

    if (result.status !== 0) {
      logger.error(`Failed to list panes: ${result.stderr || result.stdout}`)
      return []
    }

    const panes: Array<{ index: number; label: string; sessionId: string }> = []
    const lines = result.stdout.trim().split('\n')
    
    for (const line of lines) {
      // Parse output like: "1. [agent-name] (session_id)"
      const match = line.match(/^(\d+)\.\s+\[(.*?)\]\s+\((.*?)\)/)
      if (match) {
        panes.push({
          index: parseInt(match[1], 10),
          label: match[2],
          sessionId: match[3],
        })
      }
    }

    return panes
  }

  /**
   * Find a pane by agent name/label
   */
  findPaneByLabel(label: string): string | null {
    const panes = this.listPanes()
    const pane = panes.find(p => p.label === label)
    return pane ? pane.sessionId : null
  }

  /**
   * Compatibility methods for drop-in replacement of TmuxService
   */
  
  hasSession(): boolean {
    // In iTerm2, we always have a session if iTerm is running
    return this.isAvailable()
  }

  ensureSession(): void {
    if (!this.isAvailable()) {
      throw new Error('iTerm2 is not available. Please ensure iTerm2 is running.')
    }
  }

  attach(): void {
    // iTerm2 doesn't need explicit attach like tmux
    // Just focus the iTerm2 app
    spawnSync('osascript', ['-e', 'tell application "iTerm2" to activate'])
  }

  captureOutput(paneId: string): string {
    // This would require more complex iTerm2 API usage
    // For now, return empty string
    logger.warn('captureOutput not fully implemented for iTerm2')
    return ''
  }

  createWindowWithCommand(command: string): string {
    // Use spawn_agent.py for this functionality
    logger.warn('createWindowWithCommand not fully implemented for iTerm2 - use spawn command instead')
    return 'iterm-pane-' + Date.now()
  }

  getWindowIdForPane(paneId: string): string | null {
    // iTerm2 doesn't expose window IDs the same way
    return null
  }

  pipePaneToFile(paneId: string, filePath: string, append: boolean): void {
    // This would require iTerm2 Python API
    logger.warn('pipePaneToFile not implemented for iTerm2')
  }
}