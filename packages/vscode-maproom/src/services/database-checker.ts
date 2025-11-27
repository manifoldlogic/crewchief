/**
 * SQLite database availability checker for Maproom VSCode extension
 *
 * Provides unified database configuration and availability checking for SQLite.
 * This module is SQLite-only after removing PostgreSQL support.
 */

import { existsSync } from 'node:fs'
import * as vscode from 'vscode'
import { homedir } from 'node:os'
import { resolve, isAbsolute } from 'node:path'

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
    `  crewchief-maproom scan /path/to/your/repo\n\n` +
    `The index will be created at ~/.maproom/maproom.db by default.\n` +
    `Or change the database path in Settings > Maproom > Database`
  )
}
