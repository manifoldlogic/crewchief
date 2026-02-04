import { spawn, ChildProcess } from 'node:child_process'
import { createWriteStream, WriteStream } from 'node:fs'
import { mkdir, open, readFile, readdir, stat } from 'node:fs/promises'
import path from 'node:path'
import treeKill from 'tree-kill'
import { logger } from '../../utils/logger'
import { TerminalProvider, WindowOptions, SplitDirection, AgentInfo } from '../interface'

interface LogStreams {
  stdout: WriteStream
  stderr: WriteStream
  combined: WriteStream
}

interface HeadlessAgent {
  child: ChildProcess
  name: string
  type: string
  logStreams?: LogStreams
  runId?: string
}

/**
 * Create log directory and log files with secure permissions.
 * Directory: 0o700 (owner-only), Files: 0o600 (owner read/write only).
 */
async function createLogFiles(runsDir: string, runId: string): Promise<LogStreams> {
  const logDir = path.join(runsDir, runId, 'logs')

  // Create directory with owner-only permissions
  await mkdir(logDir, { recursive: true, mode: 0o700 })

  const stdoutPath = path.join(logDir, 'stdout.log')
  const stderrPath = path.join(logDir, 'stderr.log')
  const combinedPath = path.join(logDir, 'combined.log')

  // Create files with restricted permissions, then close file handles
  const stdoutHandle = await open(stdoutPath, 'w', 0o600)
  await stdoutHandle.close()
  const stderrHandle = await open(stderrPath, 'w', 0o600)
  await stderrHandle.close()
  const combinedHandle = await open(combinedPath, 'w', 0o600)
  await combinedHandle.close()

  return {
    stdout: createWriteStream(stdoutPath, { flags: 'a' }),
    stderr: createWriteStream(stderrPath, { flags: 'a' }),
    combined: createWriteStream(combinedPath, { flags: 'a' }),
  }
}

/**
 * Recursively calculate total size of a directory in bytes.
 */
async function calculateDirectorySize(dirPath: string): Promise<number> {
  let totalSize = 0

  try {
    const entries = await readdir(dirPath, { withFileTypes: true })
    for (const entry of entries) {
      const fullPath = path.join(dirPath, entry.name)
      if (entry.isDirectory()) {
        totalSize += await calculateDirectorySize(fullPath)
      } else if (entry.isFile()) {
        const fileStat = await stat(fullPath)
        totalSize += fileStat.size
      }
    }
  } catch {
    // Directory may not exist or be inaccessible
  }

  return totalSize
}

/**
 * Check log directory size and warn if it exceeds 1GB.
 * Returns total size in bytes.
 */
async function checkLogDirectorySize(runsDir: string): Promise<number> {
  const totalSize = await calculateDirectorySize(runsDir)

  const GB = 1024 * 1024 * 1024
  if (totalSize > GB) {
    logger.warn(
      `Log directory exceeds 1GB (${(totalSize / GB).toFixed(2)} GB). ` +
        'Consider cleanup: rm -rf .crewchief/runs/<old-runId>',
    )
  }

  return totalSize
}

export interface HeadlessProviderOptions {
  /**
   * Base directory for .crewchief/runs log storage.
   * Defaults to process.cwd() if not specified.
   * Primarily useful for testing.
   */
  baseDir?: string
}

export class HeadlessProvider implements TerminalProvider {
  readonly id = 'headless'
  private agents = new Map<string, HeadlessAgent>()
  private logicalPaneCounter = 0
  private readonly baseDir: string

  constructor(options?: HeadlessProviderOptions) {
    this.baseDir = options?.baseDir ?? process.cwd()
  }

  /**
   * Get the runs directory path for log storage.
   */
  private get runsDir(): string {
    return path.join(this.baseDir, '.crewchief/runs')
  }

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
      // Close log streams before killing
      if (agent.logStreams) {
        agent.logStreams.stdout.end()
        agent.logStreams.stderr.end()
        agent.logStreams.combined.end()
      }

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

  async runCommand(paneId: string, command: string, runId?: string): Promise<void> {
    logger.info(`[${paneId}] Spawning: ${command}`)

    // Create log streams BEFORE spawning the process to ensure no output
    // is lost for fast-exiting commands.
    let logStreams: LogStreams | undefined
    if (runId) {
      try {
        logStreams = await createLogFiles(this.runsDir, runId)
        // Check log directory size after creating new log files
        checkLogDirectorySize(this.runsDir).catch(() => {
          // Size check is advisory; ignore errors
        })
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err)
        logger.error(`Failed to create log files: ${message}`)
        // Continue without file logging
      }
    }

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
        logStreams,
        runId,
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
      // Close log streams on exit
      const agent = this.agents.get(paneId)
      if (agent?.logStreams) {
        agent.logStreams.stdout.end()
        agent.logStreams.stderr.end()
        agent.logStreams.combined.end()
      }
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
   * Get the log file path for a given pane and stream type.
   * Returns undefined if the agent has no runId (no log persistence).
   */
  getLogPath(paneId: string, stream: 'stdout' | 'stderr' | 'combined' = 'combined'): string | undefined {
    const agent = this.agents.get(paneId)
    if (!agent?.runId) return undefined

    return path.join(this.runsDir, agent.runId, 'logs', `${stream}.log`)
  }

  /**
   * Get log content for a pane. Returns combined log by default.
   * If `lines` is specified, returns only the last N lines.
   */
  async getLogs(paneId: string, lines?: number): Promise<string> {
    const logPath = this.getLogPath(paneId)
    if (!logPath) throw new Error(`No logs found for pane ${paneId}`)

    const content = await readFile(logPath, 'utf-8')
    if (!lines) return content

    const allLines = content.split('\n')
    return allLines.slice(-lines).join('\n')
  }

  /**
   * Parse agent type from paneId (format: name__type or just name)
   */
  private parseAgentType(paneId: string): string {
    const parts = paneId.split('__')
    return parts.length > 1 ? parts[parts.length - 1] : 'unknown'
  }

  private logOutput(paneId: string, data: Buffer, stream: 'stdout' | 'stderr'): void {
    const agent = this.agents.get(paneId)
    const text = data.toString()

    // Existing console logging
    const lines = text.split('\n')
    for (const line of lines) {
      if (line.trim()) {
        logger.info(`[${paneId}] ${line}`)
      }
    }

    // Write to log files
    if (agent?.logStreams) {
      if (stream === 'stdout') {
        agent.logStreams.stdout.write(text)
        agent.logStreams.combined.write(text)
      } else {
        agent.logStreams.stderr.write(text)
        agent.logStreams.combined.write(text)
      }
    }
  }
}

// Export for testing
export { checkLogDirectorySize, createLogFiles }
