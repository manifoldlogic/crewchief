/**
 * PostgreSQL availability checker for Maproom VSCode extension
 *
 * Checks if PostgreSQL is available at localhost:5433 (maproom-mcp standard port).
 * Does NOT start Docker containers - assumes maproom-mcp is already running.
 *
 * This lightweight approach avoids container conflicts and keeps the extension simple.
 */

import { createConnection, type Socket } from 'node:net'

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
 * Default postgres configuration matching maproom-mcp
 */
export const DEFAULT_POSTGRES_CONFIG: PostgresConfig = {
  host: 'maproom-postgres', // Docker network hostname
  port: 5432, // Internal postgres port
  user: 'maproom',
  password: 'maproom',
  database: 'maproom',
}

/**
 * Check if PostgreSQL is available and accepting connections
 *
 * Performs a simple TCP connection check to verify postgres is listening.
 * Does NOT validate credentials or database existence - just connectivity.
 *
 * @param config - PostgreSQL connection configuration
 * @param timeoutMs - Connection timeout in milliseconds (default: 2000)
 * @returns Promise resolving to true if available, false otherwise
 */
export async function checkPostgresAvailable(
  config: PostgresConfig = DEFAULT_POSTGRES_CONFIG,
  timeoutMs: number = 2000
): Promise<boolean> {
  return new Promise((resolve) => {
    const socket: Socket = createConnection({
      host: config.host,
      port: config.port,
      timeout: timeoutMs,
    })

    // Connection successful - postgres is listening
    socket.on('connect', () => {
      socket.destroy()
      resolve(true)
    })

    // Connection failed - postgres not available
    socket.on('error', () => {
      socket.destroy()
      resolve(false)
    })

    // Timeout - postgres not responding
    socket.on('timeout', () => {
      socket.destroy()
      resolve(false)
    })
  })
}

/**
 * Get connection URL for PostgreSQL
 *
 * @param config - PostgreSQL connection configuration
 * @returns Connection URL string
 */
export function getPostgresUrl(config: PostgresConfig = DEFAULT_POSTGRES_CONFIG): string {
  return `postgresql://${config.user}:${config.password}@${config.host}:${config.port}/${config.database}`
}

/**
 * Get helpful error message when postgres is not available
 *
 * @returns User-friendly error message with setup instructions
 */
export function getPostgresUnavailableMessage(): string {
  return (
    'PostgreSQL is not running at maproom-postgres:5432. ' +
    'Please start Maproom services:\n\n' +
    '  npx @crewchief/maproom-mcp setup --provider=openai\n\n' +
    'Or if using Ollama:\n\n' +
    '  npx @crewchief/maproom-mcp setup --provider=ollama'
  )
}
