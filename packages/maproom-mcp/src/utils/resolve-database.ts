/**
 * Database URL resolution for Maproom MCP Server
 *
 * Resolution hierarchy:
 * 1. Explicit MAPROOM_DATABASE_URL environment variable
 * 2. SQLite default (~/.maproom/maproom.db)
 *
 * Note: Only SQLite is supported. PostgreSQL support was removed.
 */

import { existsSync } from 'node:fs'
import { homedir } from 'node:os'
import { resolve, isAbsolute } from 'node:path'

/**
 * Database configuration with backend type information
 */
export interface DatabaseConfig {
  /** Database backend type (SQLite only) */
  type: 'sqlite'
  /** Full database URL */
  url: string
  /** SQLite file path */
  path: string
}

/**
 * Check if a URL is a SQLite URL
 *
 * @param url - Database URL to check
 * @returns true if URL starts with 'sqlite://'
 */
export function isSqliteUrl(url: string): boolean {
  return url.startsWith('sqlite://')
}

/**
 * Expand tilde (~) in path to user's home directory
 */
function expandPath(p: string): string {
  return p.startsWith('~') ? p.replace('~', homedir()) : p
}

/**
 * Parse SQLite URL and return DatabaseConfig
 */
function parseSqliteUrl(url: string): DatabaseConfig {
  const path = url.slice('sqlite://'.length)
  const expanded = expandPath(path)
  const resolved = isAbsolute(expanded) ? expanded : resolve(process.cwd(), expanded)

  return {
    type: 'sqlite',
    url: `sqlite://${resolved}`,
    path: resolved,
  }
}

/**
 * Resolve database configuration using environment-based hierarchy
 *
 * Resolution priority:
 * 1. Explicit MAPROOM_DATABASE_URL environment variable (must be sqlite://)
 * 2. SQLite default (~/.maproom/maproom.db)
 *
 * @returns Database configuration with type and URL
 * @throws Error if non-SQLite URL is provided
 */
export function resolveDatabaseConfig(): DatabaseConfig {
  const url = process.env.MAPROOM_DATABASE_URL

  // Tier 1: Explicit URL
  if (url) {
    if (!isSqliteUrl(url)) {
      throw new Error(
        `Invalid database URL: ${url}\n` +
          'Only SQLite is supported. Use: sqlite:///path/to/database.db'
      )
    }
    return parseSqliteUrl(url)
  }

  // Tier 2: SQLite default
  const sqlitePath = expandPath('~/.maproom/maproom.db')
  return {
    type: 'sqlite',
    url: `sqlite://${sqlitePath}`,
    path: sqlitePath,
  }
}

/**
 * Resolve database URL using environment-based hierarchy
 *
 * @returns Database connection string
 * @deprecated Use resolveDatabaseConfig() for access to backend type information
 */
export function resolveDatabase(): string {
  return resolveDatabaseConfig().url
}
