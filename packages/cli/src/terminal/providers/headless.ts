import { spawn, ChildProcess } from 'node:child_process'
import treeKill from 'tree-kill'
import { logger } from '../../utils/logger'
import { TerminalProvider, WindowOptions, SplitDirection } from '../interface'

export class HeadlessProvider implements TerminalProvider {
  readonly id = 'headless'
  private processes = new Map<string, ChildProcess>()
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

    for (const [paneId, proc] of this.processes.entries()) {
      if (proc && proc.pid) {
        logger.debug(`Killing process tree for pane ${paneId} (PID: ${proc.pid})`)
        const promise = new Promise<void>((resolve) => {
          treeKill(proc.pid!, 'SIGTERM', (err) => {
            if (err) {
              logger.error(`Failed to kill process ${proc.pid}: ${err.message}`)
            }
            resolve()
          })
        })
        promises.push(promise)
      }
    }

    await Promise.all(promises)
    this.processes.clear()
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
      this.processes.set(paneId, child)
    }

    child.stdout?.on('data', (data: Buffer) => {
      this.logOutput(paneId, data, 'stdout')
    })

    child.stderr?.on('data', (data: Buffer) => {
      this.logOutput(paneId, data, 'stderr')
    })

    child.on('exit', (code) => {
      logger.info(`[${paneId}] Process exited with code ${code}`)
      this.processes.delete(paneId)
    })

    child.on('error', (err) => {
      logger.error(`[${paneId}] Process error: ${err.message}`)
    })
  }

  async focus(_paneId: string): Promise<void> {
    // No-op in headless
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
