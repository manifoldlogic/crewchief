/**
 * Unified database availability checker for Maproom VSCode extension
 *
 * Supports both SQLite (file existence) and PostgreSQL (TCP check) backends.
 * Resolution priority is settings-based (VSCode settings).
 *
 * The VSCode extension uses settings-based configuration (user preference persisted in settings.json),
 * while the daemon receives the resolved URL via environment variable when spawned.
 */

import { existsSync } from 'node:fs'
import * as vscode from 'vscode'
import { homedir } from 'node:os'
import { resolve, isAbsolute } from 'node:path'
import {
  checkPostgresAvailable,
  getPostgresConfigFromSettings,
  getPostgresUrl,
  getPostgresUnavailableMessage,
  type PostgresConfig,
} from './postgres-checker'

/**
 * Database configuration with backend type information
 */
export interface DatabaseConfig {
  /** Database backend type */
  type: 'sqlite' | 'postgresql'
  /** Full database URL */
  url: string
  /** SQLite file path (only for sqlite type) */
  path?: string
}

// Re-export PostgresConfig for consumers
export type { PostgresConfig }

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
 * Priority:
 * 1. Settings: maproom.database.provider determines type
 * 2. For SQLite: Use sqlitePath setting or default ~/.maproom/maproom.db
 * 3. For PostgreSQL: Build URL from host/port/user/password/name settings
 *
 * @returns Database configuration with type and URL
 */
export function resolveDatabaseConfig(): DatabaseConfig {
  const config = vscode.workspace.getConfiguration('maproom.database')
  const provider = config.get<string>('provider') ?? 'sqlite'

  if (provider === 'sqlite') {
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

  // PostgreSQL mode
  const pgConfig = getPostgresConfigFromSettings()
  return {
    type: 'postgresql',
    url: getPostgresUrl(pgConfig),
  }
}

/**
 * Check if database is available
 *
 * For SQLite: checks if database file exists
 * For PostgreSQL: performs TCP connectivity check (delegates to postgres-checker.ts)
 *
 * @param config - Database configuration to check
 * @returns Promise resolving to true if available, false otherwise
 */
export async function checkDatabaseAvailable(config: DatabaseConfig): Promise<boolean> {
  if (config.type === 'sqlite') {
    return existsSync(config.path!)
  }

  // PostgreSQL mode - delegate to existing postgres-checker
  const pgConfig = getPostgresConfigFromSettings()
  return checkPostgresAvailable(pgConfig)
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
  if (config.type === 'sqlite') {
    return (
      `SQLite database not found at: ${config.path}\n\n` +
      `To create an index, run:\n` +
      `  crewchief-maproom scan /path/to/your/repo\n\n` +
      `The index will be created at ~/.maproom/maproom.db by default.\n` +
      `Or change the database path in Settings > Maproom > Database`
    )
  }

  // PostgreSQL mode - delegate to existing message
  return getPostgresUnavailableMessage()
}
