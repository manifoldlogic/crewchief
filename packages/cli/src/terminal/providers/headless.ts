import { spawn, ChildProcess } from 'node:child_process'
import treeKill from 'tree-kill'
import { logger } from '../../utils/logger'
import { TerminalProvider, WindowOptions, SplitDirection, AgentInfo } from '../interface'

interface HeadlessAgent {
  child: ChildProcess
  name: string
  type: string
}

export class HeadlessProvider implements TerminalProvider {
  readonly id = 'headless'
  private agents = new Map<string, HeadlessAgent>()
  private logicalPaneCounter = 0

  async initialize(): Promise<void> {
    logger.info('Initializing Headless Terminal Provider')

    // Setup cleanup handlers
    process.on('SIGINT', this.handleSignal.bind(this))
    process.on('SIGTERM', this.handleSignal.bind(this))
    process.on('exit', this.handleSignal.bind(this))
  }

  async dispose(): Promise<void> {
    logger.info('Disposing Headless Terminal Provider - killing all processes')
    const promises: Promise<void>[] = []

    for (const [paneId, agent] of this.agents.entries()) {
      if (agent.child && agent.child.pid) {
        logger.info(`Killing process tree for pane ${paneId} (PID: ${agent.child.pid})`)
        const promise = new Promise<void>((resolve) => {
          treeKill(agent.child.pid!, 'SIGTERM', (err) => {
            if (err) {
              logger.error(`Failed to kill process ${agent.child.pid}: ${err.message}`)
            }
            resolve()
          })
        })
        promises.push(promise)
      }
    }

    await Promise.all(promises)
    this.agents.clear()
  }

  private async handleSignal(): Promise<void> {
    await this.dispose()
    // We don't exit here; let the main process handle the exit based on the signal event
  }

  async createWindow(_options?: WindowOptions): Promise<string> {
    return `headless-window-${Date.now()}`
  }

  async createTab(_windowId: string): Promise<string> {
    this.logicalPaneCounter++
    return `headless-pane-${this.logicalPaneCounter}`
  }

  async splitPane(_targetId: string, _direction: SplitDirection): Promise<string> {
    this.logicalPaneCounter++
    return `headless-pane-${this.logicalPaneCounter}`
  }

  async runCommand(paneId: string, command: string): Promise<void> {
    logger.info(`[${paneId}] Spawning: ${command}`)

    const child = spawn(command, {
      shell: true,
      stdio: 'pipe',
      detached: false,
    })

    if (child.pid) {
      this.agents.set(paneId, {
        child,
        name: paneId,
        type: this.parseAgentType(paneId),
      })
    }

    child.stdout?.on('data', (data: Buffer) => {
      this.logOutput(paneId, data, 'stdout')
    })

    child.stderr?.on('data', (data: Buffer) => {
      this.logOutput(paneId, data, 'stderr')
    })

    child.on('exit', (code) => {
      logger.info(`[${paneId}] Process exited with code ${code}`)
      // NOTE: Do NOT delete from agents map on exit - keep for listAgents()
      // to show stopped agents. Cleanup only happens on explicit dispose() call.
    })

    child.on('error', (err) => {
      logger.error(`[${paneId}] Process error: ${err.message}`)
    })
  }

  async focus(_paneId: string): Promise<void> {
    // No-op in headless
  }

  /**
   * Send a message to an agent via stdin pipe
   */
  async sendMessage(paneId: string, message: string): Promise<boolean> {
    const agent = this.agents.get(paneId)
    if (!agent) {
      logger.warn(`[sendMessage] No agent found with paneId: ${paneId}`)
      return false
    }

    // Check if process is still running before attempting to write
    if (agent.child.exitCode !== null) {
      logger.warn(`[sendMessage] Agent ${paneId} has already exited (code: ${agent.child.exitCode})`)
      return false
    }

    if (agent.child.stdin?.writable) {
      agent.child.stdin.write(message + '\n')
      logger.info(`[${paneId}] Sent message: ${message}`)
      return true
    }

    logger.warn(`[sendMessage] stdin not writable for agent ${paneId}`)
    return false
  }

  /**
   * List all agents managed by this provider
   */
  async listAgents(): Promise<AgentInfo[]> {
    return Array.from(this.agents.entries()).map(([id, agent]) => ({
      id,
      name: agent.name,
      type: agent.type,
      status: agent.child.exitCode === null ? 'running' : 'stopped',
    }))
  }

  /**
   * Parse agent type from paneId (format: name__type or just name)
   */
  private parseAgentType(paneId: string): string {
    const parts = paneId.split('__')
    return parts.length > 1 ? parts[parts.length - 1] : 'unknown'
  }

  private logOutput(paneId: string, data: Buffer, _stream: 'stdout' | 'stderr'): void {
    const lines = data.toString().split('\n')
    for (const line of lines) {
      if (line.trim()) {
        logger.info(`[${paneId}] ${line}`)
      }
    }
  }
}
