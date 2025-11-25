/**
 * Database URL resolution for Maproom MCP Server
 *
 * Three-tier hierarchy for database connection:
 * 1. Explicit MAPROOM_DATABASE_URL environment variable
 * 2. DevContainer detection (IN_DEVCONTAINER=true)
 * 3. Default localhost:5433 (VSCode extension port)
 */

/**
 * Resolve database URL using environment-based hierarchy
 *
 * @returns Database connection string
 */
export function resolveDatabase(): string {
  // 1. Explicit override
  if (process.env.MAPROOM_DATABASE_URL) {
    return process.env.MAPROOM_DATABASE_URL
  }

  // 2. DevContainer
  if (process.env.IN_DEVCONTAINER === 'true') {
    return 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'
  }

  // 3. Default localhost
  return 'postgresql://maproom:maproom@localhost:5433/maproom'
}
