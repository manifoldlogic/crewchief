/**
 * Daemon process lifecycle management
 */

import { spawn,type ChildProcess } from 'node:child_process'
import type { Readable, Writable } from 'node:stream'
import { DaemonStartError, DaemonCrashError } from './errors.js'

/**
 * Configuration for daemon process
 */
export interface DaemonConfig {
  /** Path to the daemon binary */
  binaryPath: string

  /** Environment variables for the daemon */
  env?: NodeJS.ProcessEnv

  /** Timeout for daemon startup (ms) */
  startTimeout?: number

  /** Timeout for graceful shutdown (ms) */
  shutdownTimeout?: number

  /** Maximum number of restart attempts */
  maxRestartAttempts?: number

  /** Initial backoff delay for restarts (ms) */
  restartBackoffMs?: number

  /** Request timeout (ms) */
  timeout?: number

  /** Enable auto-restart on crash */
  autoRestart?: boolean
}

/**
 * Daemon process handle with streams
 */
export interface DaemonProcessDef {
  process: ChildProcess
  stdin: Writable
  stdout: Readable
  stderr: Readable
}

/**
 * Manages daemon process lifecycle
 */
export class DaemonLifecycle {
  private restartAttempts = 0
  private lastRestartTime = 0
  private readonly resetWindowMs = 60000 // Reset attempt counter after 60s of uptime

  constructor(private readonly config: DaemonConfig) {}

  /**
   * Start the daemon process
   */
  async start(): Promise<DaemonProcessDef> {
    const startTimeout = this.config.startTimeout ?? 5000

    try {
      const daemonProcess = spawn(this.config.binaryPath, ['serve'], {
        env: {
          ...process.env,
          ...this.config.env,
        },
        stdio: ['pipe', 'pipe', 'pipe'],
        windowsHide: true,
      })

      // Ensure streams are available
      if (!daemonProcess.stdin || !daemonProcess.stdout || !daemonProcess.stderr) {
        throw new DaemonStartError(
          'Failed to create daemon process streams',
          new Error('stdio pipes not available')
        )
      }

      // Wait for process to stabilize (avoid immediate crashes)
      await new Promise<void>((resolve, reject) => {
        const stabilizationTime = Math.min(500, startTimeout)
        const timer = setTimeout(() => {
          cleanup()
          resolve()
        }, stabilizationTime)

        const cleanup = () => {
          clearTimeout(timer)
          daemonProcess.off('error', onError)
          daemonProcess.off('exit', onExit)
        }

        const onError = (error: Error) => {
          cleanup()
          reject(new DaemonStartError('Daemon process failed to start', error))
        }

        const onExit = (code: number | null, signal: string | null) => {
          cleanup()
          reject(
            new DaemonCrashError(
              'Daemon process crashed immediately after start',
              code ?? undefined,
              signal ?? undefined
            )
          )
        }

        daemonProcess.once('error', onError)
        daemonProcess.once('exit', onExit)
      })

      return {
        process: daemonProcess,
        stdin: daemonProcess.stdin,
        stdout: daemonProcess.stdout,
        stderr: daemonProcess.stderr,
      }
    } catch (error) {
      if (error instanceof DaemonStartError || error instanceof DaemonCrashError) {
        throw error
      }
      throw new DaemonStartError(
        `Failed to spawn daemon process: ${error instanceof Error ? error.message : String(error)}`,
        error instanceof Error ? error : undefined
      )
    }
  }

  /**
   * Stop the daemon process gracefully
   */
  async stop(daemonProcess: DaemonProcessDef): Promise<void> {
    const shutdownTimeout = this.config.shutdownTimeout ?? 5000

    return new Promise<void>((resolve) => {
      const { process } = daemonProcess

      // Process already exited
      if (process.exitCode !== null || process.killed) {
        resolve()
        return
      }

      // Set up timeout for forceful kill
      const killTimer = setTimeout(() => {
        if (process.exitCode === null && !process.killed) {
          process.kill('SIGKILL')
        }
      }, shutdownTimeout)

      // Wait for exit
      process.once('exit', () => {
        clearTimeout(killTimer)
        resolve()
      })

      // Send SIGTERM for graceful shutdown
      process.kill('SIGTERM')
    })
  }

  /**
   * Check if daemon should be restarted after a crash
   */
  shouldRestart(): boolean {
    const maxAttempts = this.config.maxRestartAttempts ?? 5
    const autoRestart = this.config.autoRestart ?? true

    if (!autoRestart) {
      return false
    }

    // Reset attempt counter if enough time has passed
    const now = Date.now()
    if (now - this.lastRestartTime > this.resetWindowMs) {
      this.restartAttempts = 0
    }

    return this.restartAttempts < maxAttempts
  }

  /**
   * Get backoff delay for next restart attempt (exponential backoff)
   */
  getBackoffDelay(): number {
    const initialBackoff = this.config.restartBackoffMs ?? 1000
    const delay = initialBackoff * Math.pow(2, this.restartAttempts)
    this.restartAttempts++
    this.lastRestartTime = Date.now()
    return delay
  }

  /**
   * Reset restart attempt counter (call after successful operation)
   */
  resetRestartAttempts(): void {
    this.restartAttempts = 0
  }
}
