/**
 * Initial workspace scan orchestration
 *
 * Provides functionality to trigger a one-time workspace scan after the setup
 * wizard completes. Spawns the crewchief-maproom scan process, parses progress
 * events, and displays a VSCode progress notification.
 *
 * Key features:
 * - Spawns `crewchief-maproom scan --path <workspace>` as child process
 * - Parses NDJSON progress events using StdoutParser
 * - Displays VSCode progress notification with file counts and percentage
 * - Updates StatusBarManager on completion
 * - Comprehensive error handling with user-friendly messages
 * - Graceful cleanup of resources
 */

import { spawn, type ChildProcess } from 'node:child_process'
import * as vscode from 'vscode'
import path from 'node:path'
import { access, constants } from 'node:fs/promises'
import { detectPlatform, getBinaryExtension, isWindows } from '../utils/platform.js'
import { StdoutParser } from './parser.js'
import type { WatchEvent } from './events.js'
import type { StatusBarManager } from '../ui/statusBar.js'

/**
 * Configuration for initial scan
 */
export interface ScanConfig {
  /** Path to extension root (where bin/ directory is located) */
  extensionRoot: string
  /** Workspace root directory to scan */
  workspaceRoot: string
  /** PostgreSQL connection string (DATABASE_URL) */
  databaseUrl: string
  /** Output channel for logging */
  outputChannel: vscode.OutputChannel
  /** Status bar manager for completion updates */
  statusBarManager: StatusBarManager
  /** Optional environment variables (e.g., embedding provider credentials) */
  env?: NodeJS.ProcessEnv
}

/**
 * Error thrown when scan operations fail
 */
export class ScanError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly exitCode?: number,
    public readonly stderr?: string
  ) {
    super(message)
    this.name = 'ScanError'
  }
}

/**
 * Run initial workspace scan with progress notification
 *
 * Spawns the scan process and displays a progress notification showing
 * file counts and percentage. The notification is dismissible but the
 * scan continues in the background.
 *
 * Progress notification structure:
 * - Location: Notification (visible popup)
 * - Title: "Indexing workspace"
 * - Cancellable: false (scan runs in background)
 * - Message: "Indexed N files" with file counts
 * - Increment: Based on progress percentage changes
 *
 * @param config - Scan configuration
 * @returns Promise resolving to total files indexed
 * @throws ScanError if binary not found, spawn fails, or scan process errors
 */
export async function runInitialScan(config: ScanConfig): Promise<number> {
  const { extensionRoot, workspaceRoot, databaseUrl, outputChannel, statusBarManager, env } = config

  // Get platform-specific binary path
  const platform = detectPlatform()
  const binaryName = `crewchief-maproom${getBinaryExtension()}`
  const binaryPath = path.join(extensionRoot, 'bin', platform, binaryName)

  // Verify binary exists and is executable
  await verifyBinary(binaryPath, outputChannel)

  log(outputChannel, `Starting initial scan for workspace: ${workspaceRoot}`)

  // Run scan with progress notification
  return vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: 'Indexing workspace',
      cancellable: false,
    },
    async (progress) => {
      return spawnScanProcess(binaryPath, workspaceRoot, databaseUrl, env || {}, progress, outputChannel, statusBarManager)
    }
  )
}

/**
 * Verify binary exists and is executable
 *
 * @param binaryPath - Path to binary
 * @param outputChannel - Output channel for logging
 * @throws ScanError if binary not found or not executable
 */
async function verifyBinary(binaryPath: string, outputChannel: vscode.OutputChannel): Promise<void> {
  try {
    // Check if file exists and is readable
    await access(binaryPath, constants.F_OK | constants.R_OK)

    // On Unix-like systems, also check executable permission
    if (!isWindows()) {
      await access(binaryPath, constants.X_OK)
    }

    log(outputChannel, `Binary verified: ${binaryPath}`)
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      throw new ScanError(
        `Binary not found: ${binaryPath}. The extension may not be properly installed for your platform.`,
        'BINARY_NOT_FOUND'
      )
    }

    if (error.code === 'EACCES') {
      throw new ScanError(
        `Binary not executable: ${binaryPath}. Try running: chmod +x ${binaryPath}`,
        'BINARY_NOT_EXECUTABLE'
      )
    }

    throw new ScanError(
      `Failed to verify binary: ${error.message}`,
      'BINARY_VERIFICATION_FAILED'
    )
  }
}

/**
 * Spawn scan process and track progress
 *
 * @param binaryPath - Path to crewchief-maproom binary
 * @param workspaceRoot - Workspace root to scan
 * @param databaseUrl - PostgreSQL connection string
 * @param env - Environment variables
 * @param progress - VSCode progress reporter
 * @param outputChannel - Output channel for logging
 * @param statusBarManager - Status bar manager for completion updates
 * @returns Promise resolving to total files indexed
 * @throws ScanError if process spawn fails or exits with error
 */
async function spawnScanProcess(
  binaryPath: string,
  workspaceRoot: string,
  databaseUrl: string,
  env: NodeJS.ProcessEnv,
  progress: vscode.Progress<{ message?: string; increment?: number }>,
  outputChannel: vscode.OutputChannel,
  statusBarManager: StatusBarManager
): Promise<number> {
  return new Promise<number>((resolve, reject) => {
    // Track progress state
    let lastPercent = 0
    let totalFiles = 0
    let processExited = false
    let stderrBuffer = ''

    // Handler for scan events
    const handleScanEvent = (event: WatchEvent) => {
      switch (event.type) {
        case 'progress': {
          // Calculate percentage
          const percent = event.files > 0 ? Math.floor((event.complete / event.files) * 100) : 0
          const increment = percent - lastPercent
          lastPercent = percent
          totalFiles = event.files

          // Update progress notification
          const message = `Indexed ${event.complete.toLocaleString()} of ${event.files.toLocaleString()} files`
          progress.report({ message, increment: increment > 0 ? increment : undefined })

          // Log progress periodically (every 10%)
          if (increment >= 10 || percent === 100) {
            log(outputChannel, `Progress: ${percent}% (${event.complete}/${event.files} files)`)
          }
          break
        }

        case 'complete': {
          totalFiles = event.files
          const duration = (event.duration / 1000).toFixed(2)
          log(outputChannel, `Scan complete: ${event.files} files indexed in ${duration}s`)

          // Update progress to 100%
          progress.report({ message: `Indexed ${event.files.toLocaleString()} files`, increment: 100 - lastPercent })
          break
        }

        case 'error': {
          log(outputChannel, `ERROR during scan: ${event.message}${event.file ? ` (file: ${event.file})` : ''}`)
          break
        }

        case 'status': {
          log(outputChannel, `Status: ${event.state}`)
          break
        }
      }
    }

    // Spawn scan process
    const args = ['scan', '--path', workspaceRoot]
    log(outputChannel, `Spawning: ${binaryPath} ${args.join(' ')}`)

    const child = spawn(binaryPath, args, {
      env: {
        ...process.env,
        ...env,
        DATABASE_URL: databaseUrl,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    })

    // Parse stdout for progress events
    if (child.stdout) {
      const parser = new StdoutParser(child.stdout)

      parser.on('event', handleScanEvent)

      parser.on('parseError', (error: Error, line: string) => {
        log(outputChannel, `Parse error: ${error.message}`)
        log(outputChannel, `Invalid line: ${line}`)
      })

      parser.on('close', () => {
        log(outputChannel, 'Parser closed')
      })
    }

    // Collect stderr for error reporting
    if (child.stderr) {
      child.stderr.on('data', (chunk: Buffer) => {
        const text = chunk.toString('utf8')
        stderrBuffer += text
        // Log stderr in real-time
        log(outputChannel, `[STDERR] ${text.trim()}`)
      })
    }

    // Handle process exit
    child.on('exit', (code: number | null, signal: string | null) => {
      processExited = true

      if (code === 0) {
        log(outputChannel, `Scan completed successfully. Indexed ${totalFiles} files.`)

        // Update status bar with completion
        statusBarManager.setState('watching', `Indexed ${totalFiles.toLocaleString()} files`)

        resolve(totalFiles)
      } else {
        const errorMessage = `Scan process exited with code ${code ?? 'unknown'}${signal ? ` (signal: ${signal})` : ''}`
        log(outputChannel, `ERROR: ${errorMessage}`)

        // Show user-friendly error notification
        const userMessage = extractErrorMessage(stderrBuffer) || 'Scan failed. Check Output channel for details.'
        vscode.window.showErrorMessage(`Maproom: ${userMessage}`)

        reject(new ScanError(
          errorMessage,
          'SCAN_FAILED',
          code ?? undefined,
          stderrBuffer
        ))
      }
    })

    // Handle spawn errors
    child.on('error', (error: NodeJS.ErrnoException) => {
      if (!processExited) {
        log(outputChannel, `ERROR: Failed to spawn scan process: ${error.message}`)

        vscode.window.showErrorMessage(`Maproom: Failed to start scan. ${error.message}`)

        reject(new ScanError(
          `Failed to spawn scan process: ${error.message}`,
          'SPAWN_FAILED'
        ))
      }
    })
  })
}

/**
 * Extract user-friendly error message from stderr
 *
 * Attempts to find actionable error messages in stderr output.
 *
 * @param stderr - Raw stderr output
 * @returns User-friendly error message or undefined
 */
function extractErrorMessage(stderr: string): string | undefined {
  if (!stderr.trim()) {
    return undefined
  }

  // Look for common error patterns
  const patterns = [
    /error: (.+)/i,
    /failed to (.+)/i,
    /cannot (.+)/i,
    /unable to (.+)/i,
  ]

  for (const pattern of patterns) {
    const match = stderr.match(pattern)
    if (match && match[1]) {
      return match[1].trim()
    }
  }

  // Return first non-empty line if no pattern matches
  const lines = stderr.split('\n').filter(line => line.trim())
  return lines[0]?.trim()
}

/**
 * Log a message to the output channel with timestamp
 *
 * @param outputChannel - VSCode output channel
 * @param message - Message to log
 */
function log(outputChannel: vscode.OutputChannel, message: string): void {
  const timestamp = new Date().toISOString()
  outputChannel.appendLine(`[${timestamp}] [Scan] ${message}`)
}
