/**
 * Database URL resolution for Maproom MCP Server
 *
 * Four-tier hierarchy for database connection:
 * 1. Explicit MAPROOM_DATABASE_URL environment variable
 * 2. DevContainer detection (IN_DEVCONTAINER=true)
 * 3. SQLite default (~/.maproom/maproom.db if exists)
 * 4. Default localhost:5433 (VSCode extension port)
 */

import { existsSync } from 'node:fs'
import { homedir } from 'node:os'
import { resolve, isAbsolute } from 'node:path'

/**
 * Database configuration with backend type information
 */
export interface DatabaseConfig {
  /** Database backend type */
  type: 'postgresql' | 'sqlite'
  /** Full database URL */
  url: string
  /** SQLite file path (only for sqlite type) */
  path?: string
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
 * 1. Explicit MAPROOM_DATABASE_URL environment variable
 * 2. DevContainer detection (IN_DEVCONTAINER=true) → PostgreSQL container
 * 3. SQLite default (~/.maproom/maproom.db) if file exists
 * 4. PostgreSQL fallback (localhost:5433)
 *
 * @returns Database configuration with type and URL
 */
export function resolveDatabaseConfig(): DatabaseConfig {
  const url = process.env.MAPROOM_DATABASE_URL

  // Tier 1: Explicit URL
  if (url) {
    if (isSqliteUrl(url)) {
      return parseSqliteUrl(url)
    }
    return { type: 'postgresql', url }
  }

  // Tier 2: DevContainer
  if (process.env.IN_DEVCONTAINER === 'true') {
    return {
      type: 'postgresql',
      url: 'postgresql://maproom:maproom@maproom-postgres:5432/maproom',
    }
  }

  // Tier 3: SQLite default
  const sqlitePath = expandPath('~/.maproom/maproom.db')
  if (existsSync(sqlitePath)) {
    return {
      type: 'sqlite',
      url: `sqlite://${sqlitePath}`,
      path: sqlitePath,
    }
  }

  // Tier 4: PostgreSQL fallback
  return {
    type: 'postgresql',
    url: 'postgresql://maproom:maproom@localhost:5433/maproom',
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
