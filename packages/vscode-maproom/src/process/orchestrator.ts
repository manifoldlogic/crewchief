/**
 * Process orchestrator for crewchief-maproom watch processes
 *
 * Manages the lifecycle of two long-running Rust processes:
 * 1. watch - Monitors file changes and triggers indexing
 * 2. branch-watch - Monitors git branch switches
 *
 * Key features:
 * - Platform-aware binary selection (darwin-x64, linux-arm64, etc.)
 * - Graceful shutdown with SIGTERM → SIGKILL cascade
 * - Comprehensive error handling and reporting
 * - VSCode Output channel integration for logs
 * - PostgreSQL environment variable injection
 */

import { spawn, type ChildProcess } from 'node:child_process'
import { EventEmitter } from 'node:events'
import type { OutputChannel } from 'vscode'
import * as vscode from 'vscode'
import path from 'node:path'
import { access, constants } from 'node:fs/promises'
import { detectPlatform, getBinaryExtension, isWindows } from '../utils/platform'
import { StdoutParser } from './parser'
import type { WatchEvent } from './events'
import { CrashRecovery } from './recovery'

import type { SecretsManager } from '../config/secrets'
import type { EmbeddingProvider } from '../ui/setupWizard'
import { getRepoName, getBranchName } from '../utils/git'

/**
 * PostgreSQL connection configuration
 */
export interface PostgresConfig {
  host: string
  port: number
  user: string
  password: string
  database: string
}

/**
 * Process orchestrator configuration
 */
export interface OrchestratorConfig {
  /** Path to extension root (where bin/ directory is located) */
  extensionRoot: string
  /** Workspace root directory for branch-watch */
  workspaceRoot: string
  /** PostgreSQL connection configuration */
  postgres: PostgresConfig
  /** Secrets manager for API credentials (optional) */
  secretsManager?: SecretsManager
  /** Embedding provider (optional) */
  provider?: EmbeddingProvider
}

/**
 * Error thrown when process operations fail
 */
export class ProcessError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly processName?: string,
    public readonly exitCode?: number,
    public readonly stderr?: string
  ) {
    super(message)
    this.name = 'ProcessError'
  }
}

/**
 * Managed process information
 */
interface ManagedProcess {
  name: string
  child: ChildProcess
  args: string[]
  crashed: boolean
  exitCode?: number
  parser?: StdoutParser // NDJSON parser for structured events
}

/**
 * Process orchestrator events emitted via EventEmitter
 */
export interface OrchestratorEvents {
  /** Emitted when a process emits a parsed watch event */
  watchEvent: (processName: string, event: WatchEvent) => void
  /** Emitted when a process encounters a parse error */
  parseError: (processName: string, error: Error, line: string) => void
}

/**
 * Process orchestrator for watch processes
 *
 * Spawns and manages two long-running processes:
 * - watch: Monitors file changes with throttling
 * - branch-watch: Monitors git branch switches
 *
 * Extends EventEmitter to emit parsed watch events from processes.
 */
export class ProcessOrchestrator extends EventEmitter {
  private readonly outputChannel: OutputChannel
  private readonly config: OrchestratorConfig
  private readonly binaryPath: string
  private readonly processes: Map<string, ManagedProcess> = new Map()
  private readonly recoveryManagers: Map<string, CrashRecovery> = new Map()
  private isShuttingDown = false

  /**
   * Create a new process orchestrator
   *
   * @param outputChannel - VSCode output channel for logging
   * @param config - Orchestrator configuration
   * @throws ProcessError if platform is unsupported or binary not found
   */
  constructor(outputChannel: OutputChannel, config: OrchestratorConfig) {
    super()
    this.outputChannel = outputChannel
    this.config = config

    // Detect platform and construct binary path
    try {
      const platform = detectPlatform()
      const binaryName = `crewchief-maproom${getBinaryExtension()}`
      this.binaryPath = path.join(config.extensionRoot, 'bin', platform, binaryName)

      this.log(`Process orchestrator initialized`)
      this.log(`Platform: ${platform}`)
      this.log(`Binary path: ${this.binaryPath}`)
      this.log(`Workspace root: ${config.workspaceRoot}`)
    } catch (error: any) {
      const message = `Failed to initialize process orchestrator: ${error.message}`
      this.logError(message, error)
      throw new ProcessError(message, 'INIT_FAILED')
    }
  }

  /**
   * Start watching processes
   *
   * Spawns both watch and branch-watch processes with proper environment setup.
   *
   * @throws ProcessError if binary not found or spawn fails
   */
  async startWatching(): Promise<void> {
    this.log('Starting watch processes...')

    try {
      // Verify binary exists and is executable
      await this.verifyBinary()

      // Prepare environment variables (includes credentials if configured)
      const env = await this.buildEnvironment()

      // Get git repository and branch information
      const repoName = await getRepoName(this.config.workspaceRoot)
      const branchName = await getBranchName(this.config.workspaceRoot)
      this.log(`Repository: ${repoName}, Branch: ${branchName}`)

      // Start watch process (file change monitoring)
      await this.startProcess(
        'watch',
        [
          'watch',
          '--repo',
          repoName,
          '--worktree',
          branchName,
          '--path',
          this.config.workspaceRoot,
          '--throttle',
          '3s',
        ],
        env
      )

      // Start branch-watch process (git branch monitoring)
      await this.startProcess('branch-watch', ['branch-watch', '--repo', this.config.workspaceRoot], env)

      this.log('All watch processes started successfully')
    } catch (error: any) {
      this.logError('Failed to start watch processes', error)
      // Clean up any started processes
      await this.stopWatching()
      throw error
    }
  }

  /**
   * Stop all watching processes gracefully
   *
   * Implements graceful shutdown with SIGTERM → wait 5s → SIGKILL cascade.
   * Also disposes all recovery managers to clean up timers.
   */
  async stopWatching(): Promise<void> {
    if (this.isShuttingDown) {
      this.log('Shutdown already in progress, skipping...')
      return
    }

    this.isShuttingDown = true
    this.log('Stopping watch processes...')

    const stopPromises: Promise<void>[] = []

    for (const [name, managed] of this.processes) {
      stopPromises.push(this.stopProcess(name, managed))
    }

    await Promise.all(stopPromises)

    this.processes.clear()

    // Dispose recovery managers to clean up timers
    for (const [name, recovery] of this.recoveryManagers) {
      recovery.dispose()
      this.log(`Disposed recovery manager for ${name}`)
    }
    this.recoveryManagers.clear()

    this.isShuttingDown = false
    this.log('All watch processes stopped')
  }

  /**
   * Check if processes are running
   *
   * @returns true if at least one process is running
   */
  isRunning(): boolean {
    for (const managed of this.processes.values()) {
      if (!managed.crashed && managed.child.exitCode === null) {
        return true
      }
    }
    return false
  }

  /**
   * Get status of all managed processes
   *
   * @returns Map of process names to their status
   */
  getStatus(): Map<string, { running: boolean; crashed: boolean; exitCode?: number }> {
    const status = new Map<string, { running: boolean; crashed: boolean; exitCode?: number }>()

    for (const [name, managed] of this.processes) {
      status.set(name, {
        running: !managed.crashed && managed.child.exitCode === null,
        crashed: managed.crashed,
        exitCode: managed.exitCode,
      })
    }

    return status
  }

  /**
   * Verify binary exists and is executable
   *
   * @throws ProcessError if binary not found or not executable
   */
  private async verifyBinary(): Promise<void> {
    try {
      // Check if file exists and is readable
      await access(this.binaryPath, constants.F_OK | constants.R_OK)

      // On Unix-like systems, also check executable permission
      if (!isWindows()) {
        await access(this.binaryPath, constants.X_OK)
      }

      this.log(`Binary verified: ${this.binaryPath}`)
    } catch (error: any) {
      if (error.code === 'ENOENT') {
        throw new ProcessError(
          `Binary not found: ${this.binaryPath}. The extension may not be properly installed for your platform.`,
          'BINARY_NOT_FOUND'
        )
      }

      if (error.code === 'EACCES') {
        throw new ProcessError(
          `Binary not executable: ${this.binaryPath}. Try running: chmod +x ${this.binaryPath}`,
          'BINARY_NOT_EXECUTABLE'
        )
      }

      throw new ProcessError(
        `Failed to verify binary: ${error.message}`,
        'BINARY_VERIFICATION_FAILED'
      )
    }
  }

  /**
   * Build environment variables for spawned processes
   *
   * Includes:
   * - PostgreSQL connection details
   * - Embedding provider API credentials (if configured)
   * - Parent process environment
   *
   * SECURITY: API credentials are retrieved from encrypted SecretStorage
   * and passed as environment variables to Rust binary. While env vars
   * are visible in process listings, this is the standard mechanism for
   * passing credentials to child processes.
   *
   * @returns Promise resolving to environment object for child process
   */
  private async buildEnvironment(): Promise<NodeJS.ProcessEnv> {
    const { postgres, secretsManager, provider } = this.config

    // Start with PostgreSQL config
    const env: NodeJS.ProcessEnv = {
      ...process.env,
      PGHOST: postgres.host,
      PGPORT: postgres.port.toString(),
      PGUSER: postgres.user,
      PGPASSWORD: postgres.password,
      PGDATABASE: postgres.database,
    }

    // Add embedding provider credentials if available
    if (secretsManager && provider) {
      try {
        const credentialEnv = await secretsManager.getEnvironmentVars(provider)
        Object.assign(env, credentialEnv)

        // Log credential env var names (NOT values) for debugging
        const credentialKeys = Object.keys(credentialEnv)
        if (credentialKeys.length > 0) {
          this.log(`Embedding credentials configured: ${credentialKeys.join(', ')}`)
        }
      } catch (error: any) {
        // Log error but don't fail - Ollama doesn't need credentials
        this.log(`Warning: Failed to retrieve embedding credentials: ${error.message}`)
      }
    }

    return env
  }

  /**
   * Start a managed process
   *
   * @param name - Process name (for logging and tracking)
   * @param args - Command-line arguments
   * @param env - Environment variables
   * @throws ProcessError if spawn fails
   */
  private async startProcess(name: string, args: string[], env: NodeJS.ProcessEnv): Promise<void> {
    this.log(`Starting ${name} process: ${this.binaryPath} ${args.join(' ')}`)

    try {
      const child = spawn(this.binaryPath, args, {
        env,
        stdio: ['ignore', 'pipe', 'pipe'],
        // On Windows, we need to handle SIGTERM differently
        windowsHide: true,
      })

      const managed: ManagedProcess = {
        name,
        child,
        args,
        crashed: false,
      }

      // Handle stdout - parse NDJSON events and log
      if (child.stdout) {
        // Create parser for structured events
        const parser = new StdoutParser(child.stdout)
        managed.parser = parser

        // Emit parsed events from orchestrator
        parser.on('event', (event: WatchEvent) => {
          this.log(`[${name}] Event: ${JSON.stringify(event)}`)
          this.emit('watchEvent', name, event)
        })

        // Log parse errors
        parser.on('parseError', (error: Error, line: string) => {
          this.log(`[${name}] Parse error: ${error.message}`)
          this.log(`[${name}] Invalid line: ${line}`)
          this.emit('parseError', name, error, line)
        })

        // Log when parser closes
        parser.on('close', () => {
          this.log(`[${name}] Parser closed`)
        })
      }

      // Handle stderr - log errors to output channel
      if (child.stderr) {
        child.stderr.on('data', (chunk: Buffer) => {
          const text = chunk.toString('utf8').trim()
          if (text) {
            this.log(`[${name}] ERROR: ${text}`)
          }
        })
      }

      // Handle process exit
      child.on('exit', (code: number | null, signal: string | null) => {
        if (!this.isShuttingDown) {
          managed.crashed = true
          managed.exitCode = code ?? undefined

          if (code !== 0) {
            this.logError(
              `Process ${name} exited unexpectedly with code ${code ?? 'unknown'}` +
                (signal ? ` (signal: ${signal})` : '')
            )

            // Attempt crash recovery
            void this.handleProcessCrash(name, code ?? -1, signal)
          } else {
            this.log(`Process ${name} exited normally`)
          }
        }
      })

      // Handle spawn errors
      child.on('error', (error: NodeJS.ErrnoException) => {
        managed.crashed = true
        this.logError(`Failed to spawn ${name} process`, error)
      })

      this.processes.set(name, managed)

      // Give process a moment to crash if it's going to fail immediately
      await this.sleep(100)

      // Check if process crashed immediately
      if (managed.crashed) {
        throw new ProcessError(
          `Process ${name} crashed immediately after spawn`,
          'PROCESS_CRASHED_IMMEDIATELY',
          name,
          managed.exitCode
        )
      }

      this.log(`Process ${name} started successfully`)
    } catch (error: any) {
      if (error instanceof ProcessError) {
        throw error
      }

      throw new ProcessError(
        `Failed to start ${name} process: ${error.message}`,
        'SPAWN_FAILED',
        name
      )
    }
  }

  /**
   * Stop a managed process gracefully
   *
   * Implements SIGTERM → wait 5s → SIGKILL cascade.
   *
   * @param name - Process name
   * @param managed - Managed process information
   */
  private async stopProcess(name: string, managed: ManagedProcess): Promise<void> {
    const { child } = managed

    // Skip if already exited
    if (child.exitCode !== null || child.killed) {
      this.log(`Process ${name} already stopped`)
      return
    }

    this.log(`Stopping ${name} process (PID: ${child.pid})...`)

    return new Promise<void>((resolve) => {
      let isResolved = false

      const cleanup = () => {
        if (!isResolved) {
          isResolved = true
          // Close parser if it exists
          if (managed.parser) {
            managed.parser.close()
          }
          // Remove all event listeners
          child.removeAllListeners()
          child.stdout?.removeAllListeners()
          child.stderr?.removeAllListeners()
          resolve()
        }
      }

      // Handle process exit
      child.once('exit', () => {
        this.log(`Process ${name} exited`)
        cleanup()
      })

      // Send SIGTERM for graceful shutdown
      if (isWindows()) {
        // On Windows, SIGTERM is not supported - use taskkill
        this.log(`Sending taskkill to ${name} (PID: ${child.pid})...`)
        child.kill()
      } else {
        this.log(`Sending SIGTERM to ${name} (PID: ${child.pid})...`)
        child.kill('SIGTERM')
      }

      // Wait 5 seconds, then SIGKILL if still running
      const killTimer = setTimeout(() => {
        if (child.exitCode === null && !child.killed) {
          this.log(`Process ${name} did not respond to SIGTERM, sending SIGKILL...`)
          child.kill('SIGKILL')

          // Force cleanup after SIGKILL
          setTimeout(() => {
            cleanup()
          }, 1000)
        }
      }, 5000)

      // Clean up timer when process exits
      child.once('exit', () => {
        clearTimeout(killTimer)
      })
    })
  }

  /**
   * Log a message to the output channel
   *
   * @param message - Message to log
   */
  private log(message: string): void {
    const timestamp = new Date().toISOString()
    this.outputChannel.appendLine(`[${timestamp}] ${message}`)
  }

  /**
   * Log an error to the output channel
   *
   * @param message - Error message
   * @param error - Optional error object
   */
  private logError(message: string, error?: Error): void {
    this.log(`ERROR: ${message}`)
    if (error) {
      this.log(`  ${error.message}`)
      if (error.stack) {
        this.log(`  ${error.stack}`)
      }
    }
  }

  /**
   * Handle process crash with recovery logic
   *
   * Implements automatic restart with exponential backoff and circuit breaker.
   * Shows error notification after max attempts exceeded.
   *
   * @param processName - Name of crashed process
   * @param exitCode - Process exit code
   * @param signal - Signal that caused exit (if any)
   */
  private async handleProcessCrash(
    processName: string,
    exitCode: number,
    signal: string | null
  ): Promise<void> {
    // Get or create recovery manager for this process
    let recovery = this.recoveryManagers.get(processName)
    if (!recovery) {
      recovery = new CrashRecovery({
        maxAttempts: 5,
        maxBackoffMs: 16000,
        successResetMs: 60000,
      })
      this.recoveryManagers.set(processName, recovery)
    }

    // Attempt recovery
    const restarted = await recovery.handleCrash(
      processName,
      exitCode,
      signal,
      async () => {
        // Restart function - rebuild environment and restart process
        const managed = this.processes.get(processName)
        if (!managed) {
          throw new Error(`Process ${processName} not found in managed processes`)
        }

        // Remove crashed process from map
        this.processes.delete(processName)

        // Rebuild environment (includes credentials)
        const env = await this.buildEnvironment()

        // Restart the process
        await this.startProcess(processName, managed.args, env)

        this.log(`Process ${processName} restarted successfully after crash`)
      }
    )

    // Check if recovery is blocked (max attempts exceeded)
    if (!restarted && recovery.isBlocked()) {
      this.log(
        `Process ${processName} recovery blocked after ${recovery.getAttemptCount()} attempts`
      )

      // Show error notification with actionable buttons
      const selection = await vscode.window.showErrorMessage(
        `Maproom watcher "${processName}" crashed after ${recovery.getAttemptCount()} restart attempts`,
        'Show Logs',
        'Restart Manually'
      )

      if (selection === 'Show Logs') {
        this.outputChannel.show()
      } else if (selection === 'Restart Manually') {
        await this.restartWatchers()
      }
    }
  }

  /**
   * Manually restart all watchers
   *
   * Stops all processes, resets recovery state, and starts fresh.
   * Used when user manually triggers restart or recovery is blocked.
   */
  async restartWatchers(): Promise<void> {
    this.log('Manual restart requested...')

    try {
      // Stop all processes
      await this.stopWatching()

      // Reset all recovery managers
      for (const [name, recovery] of this.recoveryManagers) {
        recovery.reset()
        this.log(`Reset recovery state for ${name}`)
      }

      // Clear recovery managers
      this.recoveryManagers.clear()

      // Start watching again
      await this.startWatching()

      this.log('Manual restart completed successfully')

      vscode.window.showInformationMessage('Maproom watchers restarted successfully')
    } catch (error: any) {
      const errorMessage = `Failed to restart watchers: ${error.message}`
      this.logError(errorMessage, error)
      vscode.window.showErrorMessage(errorMessage)
      throw error
    }
  }

  /**
   * Sleep for the specified duration
   *
   * @param ms - Milliseconds to sleep
   */
  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms))
  }
}
