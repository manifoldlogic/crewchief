/**
 * Process spawning utilities for MCP server tools
 *
 * Provides reusable infrastructure for spawning and managing child processes:
 * - Binary location discovery (env vars, platform paths, system PATH)
 * - Stream handling (stdout/stderr capture)
 * - Progress parsing from output
 * - Timeout support
 * - Graceful error handling
 */

import { spawn, type ChildProcess } from 'node:child_process'
import { Readable } from 'node:stream'
import path from 'node:path'
import fs from 'node:fs'
import { fileURLToPath } from 'node:url'

/**
 * Result from process execution
 */
export interface ProcessResult {
  /** Exit code from process */
  code: number
  /** Standard output as string */
  stdout: string
  /** Standard error as string */
  stderr: string
  /** Command that was executed */
  command: string
  /** Arguments passed to command */
  args: string[]
}

/**
 * Options for process spawning
 */
export interface SpawnOptions {
  /** Timeout in milliseconds (0 = no timeout) */
  timeout?: number
  /** Working directory for process */
  cwd?: string
  /** Environment variables to pass to process */
  env?: Record<string, string>
  /** Whether to capture stdout (default true) */
  captureStdout?: boolean
  /** Whether to capture stderr (default true) */
  captureStderr?: boolean
}

/**
 * Error thrown when process execution fails
 */
export class ProcessError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly exitCode?: number,
    public readonly stderr?: string,
    public readonly command?: string
  ) {
    super(message)
    this.name = 'ProcessError'
  }
}

/**
 * Convert a readable stream to a string
 * @param stream - Readable stream to convert
 * @returns Promise resolving to accumulated string
 */
export async function streamToString(stream: Readable): Promise<string> {
  const chunks: Buffer[] = []
  for await (const chunk of stream) {
    chunks.push(Buffer.from(chunk))
  }
  return Buffer.concat(chunks).toString('utf8')
}

/**
 * Find the crewchief-maproom binary using multiple fallback strategies
 * @returns Path to binary if found, null otherwise
 */
export function findMaproomBinary(): string | null {
  // Strategy 1: Environment variable override
  if (process.env.CREWCHIEF_MAPROOM_BIN) {
    const binPath = process.env.CREWCHIEF_MAPROOM_BIN
    if (fs.existsSync(binPath)) {
      return binPath
    }
  }

  // Strategy 2: Platform-specific packaged binary
  try {
    const execName = process.platform === 'win32' ? 'crewchief-maproom.exe' : 'crewchief-maproom'

    // Try to find __dirname equivalent in ESM
    const currentFileUrl = import.meta.url
    const currentFilePath = fileURLToPath(currentFileUrl)
    const currentDir = path.dirname(currentFilePath)

    // Navigate up to package root (from src/utils to package root)
    const packageRoot = path.resolve(currentDir, '..', '..')
    const packagedPath = path.join(packageRoot, 'bin', `${process.platform}-${process.arch}`, execName)

    if (fs.existsSync(packagedPath)) {
      return packagedPath
    }
  } catch (error) {
    // Ignore errors in binary discovery
  }

  // Strategy 3: Development build path (relative to workspace root)
  try {
    const devPaths = [
      './target/release/crewchief-maproom',
      './crates/maproom/target/release/crewchief-maproom',
      '../../../crates/maproom/target/release/crewchief-maproom',
    ]

    for (const devPath of devPaths) {
      if (fs.existsSync(devPath)) {
        return path.resolve(devPath)
      }
    }
  } catch (error) {
    // Ignore errors
  }

  // Strategy 4: System PATH will be tried by spawn itself
  // Return common binary names to try
  return null
}

/**
 * Get candidate binary locations to try
 * @returns Array of {cmd, description} objects to try in order
 */
export function getBinarycandidates(): Array<{ cmd: string; description: string }> {
  const candidates: Array<{ cmd: string; description: string }> = []

  // Try discovered binary first
  const discovered = findMaproomBinary()
  if (discovered) {
    candidates.push({ cmd: discovered, description: 'Discovered binary' })
  }

  // Try system PATH binaries
  candidates.push(
    { cmd: 'crewchief-maproom', description: 'System PATH binary' },
    { cmd: 'crewchief', description: 'CrewChief CLI wrapper' }
  )

  return candidates
}

/**
 * Spawn a child process and capture output
 * @param command - Command to execute
 * @param args - Arguments to pass to command
 * @param options - Spawn options
 * @returns Promise resolving to ProcessResult
 * @throws ProcessError if process fails or times out
 */
export async function spawnProcess(
  command: string,
  args: string[],
  options: SpawnOptions = {}
): Promise<ProcessResult> {
  const {
    timeout = 0,
    cwd,
    env,
    captureStdout = true,
    captureStderr = true,
  } = options

  return new Promise((resolve, reject) => {
    let child: ChildProcess
    let timeoutHandle: NodeJS.Timeout | undefined

    try {
      // Spawn the child process
      child = spawn(command, args, {
        cwd,
        env: env ? { ...process.env, ...env } : process.env,
        stdio: ['ignore', captureStdout ? 'pipe' : 'ignore', captureStderr ? 'pipe' : 'ignore'],
      })

      let stdoutData = ''
      let stderrData = ''

      // Capture stdout if enabled
      if (captureStdout && child.stdout) {
        child.stdout.on('data', (chunk) => {
          stdoutData += chunk.toString('utf8')
        })
      }

      // Capture stderr if enabled
      if (captureStderr && child.stderr) {
        child.stderr.on('data', (chunk) => {
          stderrData += chunk.toString('utf8')
        })
      }

      // Handle process exit
      child.on('close', (code) => {
        if (timeoutHandle) {
          clearTimeout(timeoutHandle)
        }

        const result: ProcessResult = {
          code: code ?? -1,
          stdout: stdoutData,
          stderr: stderrData,
          command,
          args,
        }

        if (code === 0) {
          resolve(result)
        } else {
          reject(
            new ProcessError(
              `Process exited with code ${code}: ${stderrData || 'No error message'}`,
              'PROCESS_EXIT_ERROR',
              code ?? -1,
              stderrData,
              command
            )
          )
        }
      })

      // Handle spawn errors
      child.on('error', (error: Error & { code?: string }) => {
        if (timeoutHandle) {
          clearTimeout(timeoutHandle)
        }

        const errorCode = error.code || 'SPAWN_ERROR'
        reject(
          new ProcessError(
            `Failed to spawn process: ${error.message}`,
            errorCode,
            undefined,
            undefined,
            command
          )
        )
      })

      // Set up timeout if specified
      if (timeout > 0) {
        timeoutHandle = setTimeout(() => {
          child.kill('SIGTERM')

          // Force kill after 5 seconds if process doesn't respond to SIGTERM
          setTimeout(() => {
            if (!child.killed) {
              child.kill('SIGKILL')
            }
          }, 5000)

          reject(
            new ProcessError(
              `Process timed out after ${timeout}ms`,
              'TIMEOUT',
              undefined,
              undefined,
              command
            )
          )
        }, timeout)
      }
    } catch (error: any) {
      if (timeoutHandle) {
        clearTimeout(timeoutHandle)
      }
      reject(
        new ProcessError(
          `Failed to spawn process: ${error.message}`,
          'SPAWN_ERROR',
          undefined,
          undefined,
          command
        )
      )
    }
  })
}

/**
 * Try spawning a process with multiple candidate binaries
 *
 * Spawning is the appropriate pattern for one-time operations where spawn
 * overhead (~100-200ms) is negligible.
 *
 * For repeated operations (like search), use DaemonClient instead:
 * @see packages/maproom-mcp/src/daemon.ts - Singleton daemon pattern for repeated operations
 * @see packages/daemon-client/README.md - Migration guide and API documentation
 *
 * @param candidateBinaries - Array of {cmd, description} to try
 * @param args - Arguments to pass (without subcommand)
 * @param options - Spawn options
 * @returns Promise resolving to ProcessResult
 * @throws ProcessError if all candidates fail
 */
export async function trySpawnWithCandidates(
  candidateBinaries: Array<{ cmd: string; description: string }>,
  args: string[],
  options: SpawnOptions = {}
): Promise<ProcessResult> {
  const errors: Array<{ cmd: string; error: string }> = []

  for (const candidate of candidateBinaries) {
    try {
      // Determine if we need to add 'maproom' subcommand
      const fullArgs = candidate.cmd.includes('crewchief-maproom')
        ? args
        : ['maproom', ...args]

      const result = await spawnProcess(candidate.cmd, fullArgs, options)
      return result
    } catch (error: any) {
      errors.push({
        cmd: candidate.cmd,
        error: error.message || String(error),
      })
    }
  }

  // All candidates failed
  const errorDetails = errors.map((e) => `  - ${e.cmd}: ${e.error}`).join('\n')
  throw new ProcessError(
    `Failed to execute command with all candidates:\n${errorDetails}\n\nTroubleshooting:\n1. Build the binary: cargo build --release --bin crewchief-maproom\n2. Set CREWCHIEF_MAPROOM_BIN environment variable\n3. Add binary to system PATH`,
    'ALL_CANDIDATES_FAILED',
    undefined,
    errorDetails
  )
}

/**
 * Parse indexing statistics from maproom output
 * Expected format includes lines like:
 * - "Processed X files"
 * - "Created Y chunks"
 * - "Duration: Zms"
 *
 * @param output - stdout from maproom binary
 * @returns Object with parsed statistics
 */
export interface IndexingStats {
  files?: number
  chunks?: number
  duration_ms?: number
}

export function parseIndexingStats(output: string): IndexingStats {
  const stats: IndexingStats = {}

  // Parse file count - look for patterns like "Processed 5 files" or "5 files indexed"
  const filesMatch = output.match(/(?:Processed|Indexed)\s+(\d+)\s+files?/i) ||
                     output.match(/(\d+)\s+files?\s+(?:processed|indexed)/i)
  if (filesMatch) {
    stats.files = parseInt(filesMatch[1], 10)
  }

  // Parse chunk count - look for patterns like "Created 42 chunks" or "42 chunks created"
  const chunksMatch = output.match(/(?:Created|Generated)\s+(\d+)\s+chunks?/i) ||
                      output.match(/(\d+)\s+chunks?\s+(?:created|generated)/i)
  if (chunksMatch) {
    stats.chunks = parseInt(chunksMatch[1], 10)
  }

  // Parse duration - look for patterns like "Duration: 1234ms" or "Completed in 1234ms"
  const durationMatch = output.match(/(?:Duration|Completed\s+in):\s+(\d+)ms/i) ||
                        output.match(/took\s+(\d+)ms/i)
  if (durationMatch) {
    stats.duration_ms = parseInt(durationMatch[1], 10)
  }

  return stats
}
