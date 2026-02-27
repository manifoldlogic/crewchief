/**
 * SQLite database availability checker for Maproom VSCode extension
 *
 * Provides unified database configuration and availability checking for SQLite.
 * This module is SQLite-only after removing PostgreSQL support.
 */

import { existsSync } from 'node:fs'
import { execFile } from 'node:child_process'
import { promisify } from 'node:util'
import * as vscode from 'vscode'
import { homedir } from 'node:os'
import { resolve, isAbsolute, join } from 'node:path'

const execFileAsync = promisify(execFile)

/**
 * Database configuration with path information
 */
export interface DatabaseConfig {
  /** Database backend type (always sqlite) */
  type: 'sqlite'
  /** Full database URL */
  url: string
  /** SQLite file path */
  path: string
}

/**
 * Expand tilde (~) in path to user's home directory
 *
 * @param p - Path that may contain tilde
 * @returns Path with tilde expanded to home directory
 */
export function expandPath(p: string): string {
  return p.startsWith('~') ? p.replace('~', homedir()) : p
}

/**
 * Resolve database configuration from VSCode settings
 *
 * SQLite uses sqlitePath setting or default ~/.maproom/maproom.db
 *
 * @returns Database configuration with type and URL
 */
export function resolveDatabaseConfig(): DatabaseConfig {
  const config = vscode.workspace.getConfiguration('maproom.database')
  const pathSetting = config.get<string>('sqlitePath') ?? ''
  const path = pathSetting || `${homedir()}/.maproom/maproom.db`
  const expanded = expandPath(path)
  const resolved = isAbsolute(expanded) ? expanded : resolve(process.cwd(), expanded)

  return {
    type: 'sqlite',
    url: `sqlite://${resolved}`,
    path: resolved,
  }
}

/**
 * Check if database is available
 *
 * For SQLite: checks if database file exists
 *
 * @param config - Database configuration to check
 * @returns Promise resolving to true if available, false otherwise
 */
export async function checkDatabaseAvailable(config: DatabaseConfig): Promise<boolean> {
  return existsSync(config.path)
}

/**
 * Get database URL string for ProcessOrchestrator
 *
 * @param config - Database configuration
 * @returns Database URL string
 */
export function getDatabaseUrl(config: DatabaseConfig): string {
  return config.url
}

/**
 * Get helpful error message when database is unavailable
 *
 * @param config - Database configuration
 * @returns User-friendly error message with setup instructions
 */
export function getDatabaseUnavailableMessage(config: DatabaseConfig): string {
  return (
    `SQLite database not found at: ${config.path}\n\n` +
    `To create an index, run:\n` +
    `  maproom scan /path/to/your/repo\n\n` +
    `The index will be created at ~/.maproom/maproom.db by default.\n` +
    `Or change the database path in Settings > Maproom > Database`
  )
}

/**
 * Status response from maproom status command
 */
interface StatusResponse {
  repos: Array<{
    name: string
    worktrees: Array<{
      name: string
      chunk_count: number
      last_updated: string | null
    }>
  }>
}

/**
 * Get platform identifier for binary path
 */
function getPlatform(): string {
  const platform = process.platform
  const arch = process.arch

  if (platform === 'darwin') {
    return arch === 'arm64' ? 'darwin-arm64' : 'darwin-x64'
  } else if (platform === 'win32') {
    return 'win32-x64'
  } else {
    return arch === 'arm64' ? 'linux-arm64' : 'linux-x64'
  }
}

/**
 * Get binary extension for current platform
 */
function getBinaryExtension(): string {
  return process.platform === 'win32' ? '.exe' : ''
}

/**
 * Check if a specific repository is indexed in the database
 *
 * Runs `maproom status --repo <repoName> --json` and checks
 * if the repo exists with at least one worktree containing chunks.
 *
 * @param extensionRoot - Extension root directory (for finding binary)
 * @param databaseUrl - Database URL
 * @param repoName - Repository name to check (e.g., "owner/repo")
 * @returns Promise resolving to true if repo is indexed with chunks, false otherwise
 */
export async function checkRepoIndexed(
  extensionRoot: string,
  databaseUrl: string,
  repoName: string
): Promise<boolean> {
  const platform = getPlatform()
  const binaryName = `maproom${getBinaryExtension()}`
  const binaryPath = join(extensionRoot, 'bin', platform, binaryName)

  // Check if binary exists
  if (!existsSync(binaryPath)) {
    console.log(`[checkRepoIndexed] Binary not found at: ${binaryPath}`)
    return false
  }

  console.log(`[checkRepoIndexed] Running: ${binaryPath} status --repo "${repoName}" --json`)
  console.log(`[checkRepoIndexed] Database URL: ${databaseUrl}`)

  try {
    const { stdout, stderr } = await execFileAsync(
      binaryPath,
      ['status', '--repo', repoName, '--json'],
      {
        env: {
          ...process.env,
          MAPROOM_DATABASE_URL: databaseUrl,
        },
        timeout: 30000, // 30 second timeout (migration may be slow first time)
      }
    )

    // Log stderr for debugging (binary debug output goes here)
    if (stderr) {
      console.log(`[checkRepoIndexed] stderr: ${stderr}`)
    }

    console.log(`[checkRepoIndexed] Got response, parsing JSON...`)
    const status: StatusResponse = JSON.parse(stdout)

    // Check if the repo exists and has at least one worktree with chunks
    const repo = status.repos.find((r) => r.name === repoName)
    if (!repo) {
      console.log(`[checkRepoIndexed] Repo "${repoName}" not found in status response`)
      return false
    }

    // Check if any worktree has chunks indexed
    const hasChunks = repo.worktrees.some((wt) => wt.chunk_count > 0)
    console.log(`[checkRepoIndexed] Repo "${repoName}" hasChunks: ${hasChunks}`)
    return hasChunks
  } catch (error: unknown) {
    // Binary execution failed, JSON parse failed, or repo not found
    const message = error instanceof Error ? error.message : String(error)
    console.log(`[checkRepoIndexed] Error: ${message}`)
    return false
  }
}
