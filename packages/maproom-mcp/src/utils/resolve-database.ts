/**
 * Database URL resolution for Maproom MCP Server
 *
 * Resolution hierarchy:
 * 1. Explicit MAPROOM_DATABASE_URL environment variable
 * 2. SQLite default (~/.maproom/maproom.db)
 *
 * Supported backends:
 * - SQLite: sqlite:///path/to/db
 * - PostgreSQL: postgres:// or postgresql://
 */

import { existsSync } from 'node:fs'
import { homedir } from 'node:os'
import { resolve, isAbsolute } from 'node:path'

/**
 * Database configuration with backend type information
 */
export type DatabaseConfig =
  | {
      /** SQLite backend */
      type: 'sqlite'
      /** Full database URL */
      url: string
      /** SQLite file path */
      path: string
    }
  | {
      /** PostgreSQL backend */
      type: 'postgres'
      /** Full database URL (with credentials redacted for logging) */
      url: string
      /** Redacted URL safe for logging/display (host+port+dbname only) */
      redactedUrl: string
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
 * Check if a URL is a PostgreSQL URL
 *
 * @param url - Database URL to check
 * @returns true if URL starts with 'postgres://' or 'postgresql://'
 */
export function isPostgresUrl(url: string): boolean {
  return url.startsWith('postgres://') || url.startsWith('postgresql://')
}

/**
 * Redact credentials from a PostgreSQL URL, keeping only host, port, and dbname.
 * Never includes username or password in the output.
 *
 * @param url - Full postgres:// or postgresql:// URL
 * @returns URL with credentials removed: postgres://host:port/dbname
 *
 * @example
 * redactPostgresUrl('postgres://user:pass@host:5433/maproom')
 *   // → 'postgres://host:5433/maproom'
 */
export function redactPostgresUrl(url: string): string {
  try {
    // Normalise scheme for URL parsing
    const normalised = url.replace(/^postgresql:\/\//, 'postgres://')
    const parsed = new URL(normalised)
    // Reconstruct with no credentials
    const hostPart = parsed.port ? `${parsed.hostname}:${parsed.port}` : parsed.hostname
    return `postgres://${hostPart}${parsed.pathname}`
  } catch {
    // If URL is malformed, return a fully-redacted placeholder
    return 'postgres://[redacted]'
  }
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
  const rawPath = url.slice('sqlite://'.length)
  const expanded = expandPath(rawPath)
  const resolved = isAbsolute(expanded) ? expanded : resolve(process.cwd(), expanded)

  return {
    type: 'sqlite',
    url: `sqlite://${resolved}`,
    path: resolved,
  }
}

/**
 * Parse PostgreSQL URL and return DatabaseConfig
 */
function parsePostgresUrl(url: string): DatabaseConfig {
  return {
    type: 'postgres',
    url,
    redactedUrl: redactPostgresUrl(url),
  }
}

/**
 * Resolve database configuration using environment-based hierarchy
 *
 * Resolution priority:
 * 1. Explicit MAPROOM_DATABASE_URL environment variable
 *    - sqlite://... → SQLite backend
 *    - postgres://... / postgresql://... → PostgreSQL backend (passed through untouched)
 * 2. SQLite default (~/.maproom/maproom.db)
 *
 * @returns Database configuration with type and URL
 * @throws Error if an unrecognised URL scheme is provided
 */
export function resolveDatabaseConfig(): DatabaseConfig {
  const url = process.env.MAPROOM_DATABASE_URL

  // Tier 1: Explicit URL
  if (url) {
    if (isSqliteUrl(url)) {
      return parseSqliteUrl(url)
    }
    if (isPostgresUrl(url)) {
      return parsePostgresUrl(url)
    }
    throw new Error(
      `Unsupported database URL scheme: ${url}\n` +
        'Supported schemes: sqlite:///path/to/database.db, postgres://host/db, postgresql://host/db'
    )
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
