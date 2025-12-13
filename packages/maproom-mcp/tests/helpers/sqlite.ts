/**
 * SQLite test utilities (SEPARATE from database.ts)
 *
 * Provides helper functions for SQLite-based integration tests without
 * modifying or interacting with PostgreSQL helpers. Uses pre-indexed
 * fixture from the Rust crate tests.
 *
 * Key functions:
 * - createTestSqliteDatabase(): Copy fixture to temp location for isolated testing
 * - cleanupTestSqliteDatabase(): Remove temp database file
 * - getSqliteFixturePath(): Get path to source fixture
 * - getSqliteTestUrl(): Generate sqlite:// URL for testing
 */

import { copyFileSync, unlinkSync, existsSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))

// Relative path from packages/maproom-mcp/tests/helpers to fixture
const FIXTURE_SOURCE = resolve(
  __dirname,
  '../../../../crates/maproom/tests/fixtures/pre-indexed-maproom.db'
)

/**
 * Create a copy of the SQLite test fixture for isolated testing
 *
 * Each test run gets its own database copy to prevent interference
 * between tests. The copy is placed in the system temp directory.
 *
 * @returns Path to the temporary test database
 * @throws Error if fixture not found (with instructions to regenerate)
 *
 * @example
 * ```typescript
 * const testDbPath = createTestSqliteDatabase()
 * process.env.MAPROOM_DATABASE_URL = getSqliteTestUrl(testDbPath)
 * // ... run tests ...
 * cleanupTestSqliteDatabase(testDbPath)
 * ```
 */
export function createTestSqliteDatabase(): string {
  if (!existsSync(FIXTURE_SOURCE)) {
    throw new Error(
      `SQLite fixture not found: ${FIXTURE_SOURCE}\n` +
        `Run: cargo test --test create_sqlite_fixture -- --ignored`
    )
  }

  const testDbPath = join(
    tmpdir(),
    `maproom-test-${Date.now()}-${Math.random().toString(36).slice(2)}.db`
  )
  copyFileSync(FIXTURE_SOURCE, testDbPath)
  return testDbPath
}

/**
 * Clean up a temporary test database
 *
 * Safe to call multiple times (idempotent). Silently handles
 * already-deleted files or permission errors.
 *
 * @param dbPath - Path to the test database to remove
 */
export function cleanupTestSqliteDatabase(dbPath: string): void {
  try {
    if (existsSync(dbPath)) {
      unlinkSync(dbPath)
    }
  } catch {
    // Ignore cleanup errors (file may already be deleted)
  }
}

/**
 * Get the path to the source SQLite fixture
 *
 * Useful for read-only tests that don't need isolation.
 * Throws if fixture doesn't exist.
 *
 * @returns Absolute path to the pre-indexed SQLite fixture
 * @throws Error if fixture not found
 */
export function getSqliteFixturePath(): string {
  if (!existsSync(FIXTURE_SOURCE)) {
    throw new Error(
      `SQLite fixture not found: ${FIXTURE_SOURCE}\n` +
        `Run: cargo test --test create_sqlite_fixture -- --ignored`
    )
  }
  return FIXTURE_SOURCE
}

/**
 * Get SQLite database URL for testing
 *
 * Generates a properly formatted sqlite:// URL suitable for
 * the MAPROOM_DATABASE_URL environment variable.
 *
 * @param dbPath - Optional path to database (uses fixture if not provided)
 * @returns SQLite URL suitable for MAPROOM_DATABASE_URL
 *
 * @example
 * ```typescript
 * // Use default fixture
 * process.env.MAPROOM_DATABASE_URL = getSqliteTestUrl()
 *
 * // Use custom database path
 * const testDb = createTestSqliteDatabase()
 * process.env.MAPROOM_DATABASE_URL = getSqliteTestUrl(testDb)
 * ```
 */
export function getSqliteTestUrl(dbPath?: string): string {
  const path = dbPath || getSqliteFixturePath()
  return `sqlite://${path}`
}
